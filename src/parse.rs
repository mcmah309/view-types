use syn::{
    Expr, Ident, Token, Visibility, braced, parenthesized,
    parse::{Parse, ParseStream, Result},
    token::Paren,
};

/// Top-level view specification with fragments and structs
#[derive(Debug)]
pub(crate) struct Views {
    pub fragments: Vec<Fragment>,
    pub view_structs: Vec<ViewStruct>,
}

#[derive(Debug)]
pub(crate) struct Fragment {
    pub name: Ident,
    pub fields: Vec<FieldItem>,
}

#[derive(Debug)]
pub(crate) struct ViewStruct {
    pub name: Ident,
    pub generics: Option<syn::Generics>,
    pub items: Vec<ViewStructFieldKind>,
    pub attributes: Vec<syn::Attribute>,
    pub visibility: Option<Visibility>,
}

/// Items that can appear in a view struct definition
#[derive(Debug)]
pub(crate) enum ViewStructFieldKind {
    /// Spread a fragment: `..fragment_name`
    FragmentSpread(Ident),
    /// Individual field: `field_name` or pattern
    Field(FieldItem),
}

/// Individual field specification with optional validation
#[derive(Debug)]
pub(crate) struct FieldItem {
    pub field_name: Ident,
    /// e.g. `std::option::Option::Some` in `std::option::Option::Some(field)`
    pub pattern_to_match: Option<syn::Path>,
    /// e.g. `validate(field)` in `field if validate(field)`
    pub validation: Option<Expr>,
    /// Explicit type annotation, e.g. `field: Type` or EnumName::Branch(field: Type)
    pub explicit_type: Option<syn::Type>,
}

impl Parse for Views {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut fragments = Vec::new();
        let mut view_structs = Vec::new();

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(Ident) {
                // Check if it's "fragment"
                let fork = input.fork();
                let ident: Ident = fork.parse()?;

                if ident == "fragment" {
                    let fragment = input.parse::<Fragment>()?;
                    fragments.push(fragment);
                } else {
                    return Err(syn::Error::new(
                        ident.span(),
                        "Expected 'fragment' or 'struct'",
                    ));
                }
            } else if lookahead.peek(Token![struct])
                || lookahead.peek(Token![#])
                || lookahead.peek(Token![pub])
            {
                let view_struct = input.parse::<ViewStruct>()?;
                view_structs.push(view_struct);
            } else {
                return Err(lookahead.error());
            }
        }

        Ok(Views {
            fragments,
            view_structs,
        })
    }
}

impl Parse for Fragment {
    fn parse(input: ParseStream) -> Result<Self> {
        let fragment_keyword: Ident = input.parse()?;
        if fragment_keyword != "fragment" {
            return Err(syn::Error::new(
                fragment_keyword.span(),
                "Expected 'fragment'",
            ));
        }
        let name: Ident = input.parse()?;

        let content;
        braced!(content in input);

        let mut fields = Vec::new();
        while !content.is_empty() {
            let field_spec = content.parse::<FieldItem>()?;
            fields.push(field_spec);

            // Consume optional comma
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(Fragment { name, fields })
    }
}

impl Parse for ViewStruct {
    fn parse(input: ParseStream) -> Result<Self> {
        let attributes = input.call(syn::Attribute::parse_outer)?;
        let visibility = input.parse::<Visibility>().ok();
        input.parse::<Token![struct]>()?;
        let name: Ident = input.parse()?;

        // Parse optional generics
        let generics = if input.peek(Token![<]) {
            Some(input.parse::<syn::Generics>()?)
        } else {
            None
        };

        let content;
        braced!(content in input);

        let mut items = Vec::new();
        while !content.is_empty() {
            if content.peek(Token![..]) {
                // Spread syntax
                content.parse::<Token![..]>()?;
                let fragment_name: Ident = content.parse()?;
                items.push(ViewStructFieldKind::FragmentSpread(fragment_name));
            } else {
                // Individual field
                let field_spec = content.parse::<FieldItem>()?;
                items.push(ViewStructFieldKind::Field(field_spec));
            }

            // Consume optional comma
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(ViewStruct {
            name,
            generics,
            items,
            attributes,
            visibility,
        })
    }
}

impl Parse for FieldItem {
    fn parse(input: ParseStream) -> Result<Self> {
        let (field_name, pattern_to_match, explicit_type) = parse_field_pattern(input)?;

        let validation = if input.peek(Token![if]) {
            input.parse::<Token![if]>()?;
            let validation: Expr = input.parse()?;
            Some(validation)
        } else {
            None
        };

        Ok(FieldItem {
            pattern_to_match,
            explicit_type,
            validation,
            field_name,
        })
    }
}

/// name, pattern, explicit type
fn parse_field_pattern(
    input: ParseStream,
) -> Result<(Ident, Option<syn::Path>, Option<syn::Type>)> {
    let lookahead = input.lookahead1();
    if lookahead.peek(Ident) && (input.peek2(Paren) || input.peek2(Token![::])) {
        // Pattern like Some(field) or std::option::Option::Some(field)
        let pattern_to_match = input.parse::<syn::Path>()?;
        if input.peek(Paren) {
            let inner;
            parenthesized!(inner in input);
            let field = inner.parse::<Ident>()?;
            if inner.peek(Token![:]) {
                inner.parse::<Token![:]>()?;
                let inner_type = inner.parse::<syn::Type>()?;
                return Ok((field, Some(pattern_to_match), Some(inner_type)));
            }
            return Ok((field, Some(pattern_to_match), None));
        } else {
            return Err(syn::Error::new(
                input.span(),
                "Expected parentheses containing field to match on",
            ));
        }
    } else {
        // Simple identifier pattern
        let ident: Ident = input.parse()?;
        let lookahead = input.lookahead1();
        if lookahead.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            let inner_type = input.parse::<syn::Type>()?;
            return Ok((ident, None, Some(inner_type)));
        }
        return Ok((ident, None, None));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use syn::parse_quote;

    /// Helper function to get all fields for a view struct by resolving fragments
    fn resolve_view_fields<'a>(
        view_struct: &'a ViewStruct,
        fragments: &'a [Fragment],
    ) -> Result<Vec<&'a FieldItem>> {
        let fragment_map: HashMap<String, &Fragment> =
            fragments.iter().map(|f| (f.name.to_string(), f)).collect();

        let mut resolved_fields = Vec::new();

        for item in &view_struct.items {
            match item {
                ViewStructFieldKind::FragmentSpread(fragment_name) => {
                    let fragment_name_str = fragment_name.to_string();
                    if let Some(fragment) = fragment_map.get(&fragment_name_str) {
                        resolved_fields.extend(&fragment.fields);
                    } else {
                        return Err(syn::Error::new(
                            fragment_name.span(),
                            format!("Fragment '{}' not found", fragment_name_str),
                        ));
                    }
                }
                ViewStructFieldKind::Field(field_spec) => {
                    resolved_fields.push(field_spec);
                }
            }
        }

        Ok(resolved_fields)
    }

    /// Helper to determine if a field spec has a validation
    fn has_validation(field_spec: &FieldItem) -> bool {
        field_spec.validation.is_some()
    }

    #[test]
    fn test_parse_fragment() {
        let input = parse_quote! {
            fragment all {
                offset,
                limit
            }
        };

        let fragment: Fragment = syn::parse2(input).unwrap();
        assert_eq!(fragment.name.to_string(), "all");
        assert_eq!(fragment.fields.len(), 2);
    }

    #[test]
    fn test_parse_view_struct() {
        let input = parse_quote! {
            struct KeywordSearch<'a> {
                ..all,
                ..keyword,
                custom_field
            }
        };

        let view_struct: ViewStruct = syn::parse2(input).unwrap();
        assert_eq!(view_struct.name.to_string(), "KeywordSearch");
        assert!(view_struct.generics.is_some());
        assert_eq!(view_struct.items.len(), 3);

        // Check spread items
        if let ViewStructFieldKind::FragmentSpread(name) = &view_struct.items[0] {
            assert_eq!(name.to_string(), "all");
        } else {
            panic!("Expected spread item");
        }
    }

    #[test]
    fn test_parse_fragment_with_validations() {
        let input = parse_quote! {
            fragment semantic {
                Some(semantic) = valid_semantic_value(semantic),
                Some(query)
            }
        };

        let fragment: Fragment = syn::parse2(input).unwrap();
        assert_eq!(fragment.fields.len(), 2);
        assert!(has_validation(&fragment.fields[0]));
        assert!(!has_validation(&fragment.fields[1]));
    }

    #[test]
    fn test_parse_full_view_spec() {
        let input = parse_quote! {
            fragment all {
                offset,
                limit
            }
            fragment keyword {
                Some(query),
                words_limit
            }
            struct KeywordSearch<'a> {
                ..all,
                ..keyword
            }
            struct SemanticSearch {
                ..all,
                semantic_field
            }
        };

        let view_spec: Views = syn::parse2(input).unwrap();
        assert_eq!(view_spec.fragments.len(), 2);
        assert_eq!(view_spec.view_structs.len(), 2);
    }

    #[test]
    fn test_resolve_view_fields() {
        let input = parse_quote! {
            fragment all {
                offset,
                limit
            }
            fragment keyword {
                Some(query)
            }
            struct KeywordSearch {
                ..all,
                ..keyword,
                custom_field
            }
        };

        let view_spec: Views = syn::parse2(input).unwrap();
        let keyword_struct = &view_spec.view_structs[0];

        let resolved = resolve_view_fields(keyword_struct, &view_spec.fragments).unwrap();
        assert_eq!(resolved.len(), 4); // offset, limit, query, custom_field
        let names = resolved
            .iter()
            .map(|f| f.field_name.to_string())
            .collect::<Vec<_>>();
        assert!(names.contains(&"offset".to_owned()));
        assert!(names.contains(&"limit".to_owned()));
        assert!(names.contains(&"query".to_owned()));
        assert!(names.contains(&"custom_field".to_owned()));
    }
}
