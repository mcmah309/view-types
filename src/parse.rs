use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, format_ident};
use syn::{
    parse::{Parse, ParseStream, Result},
    punctuated::Punctuated,
    token::{Brace, Paren},
    Expr, Ident, Pat, Token, Type, braced, parenthesized,
};
use std::collections::HashMap;

/// Top-level view specification with fragments and structs
#[derive(Debug, Clone)]
pub struct ViewSpec {
    pub fragments: Vec<Fragment>,
    pub view_structs: Vec<ViewStruct>,
}

/// A reusable fragment of fields
#[derive(Debug, Clone)]
pub struct Fragment {
    pub name: Ident,
    pub fields: Vec<FieldSpec>,
}

/// A view struct definition
#[derive(Debug, Clone)]
pub struct ViewStruct {
    pub name: Ident,
    pub generics: Option<syn::Generics>,
    pub items: Vec<StructItem>,
}

/// Items that can appear in a struct definition
#[derive(Debug, Clone)]
pub enum StructItem {
    /// Spread a fragment: `..fragment_name`
    Spread(Ident),
    /// Individual field: `field_name` or pattern
    Field(FieldSpec),
}

/// Individual field specification with optional transformation
#[derive(Debug, Clone)]
pub struct FieldSpec {
    pub pattern: Pat,
    pub transformation: Option<FieldTransformation>,
    pub field_name: Option<Ident>, // extracted from pattern if simple
}

/// Field transformation options
#[derive(Debug, Clone)]
pub enum FieldTransformation {
    /// Simple assignment: `field = expr`
    Assignment(Expr),
    /// Function call: `field(args)`
    FunctionCall(Expr),
    /// No transformation, just include the field
    None,
}

impl Parse for ViewSpec {
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
                    return Err(syn::Error::new(ident.span(), "Expected 'fragment' or 'struct'"));
                }
            } else if lookahead.peek(Token![struct]) {
                let view_struct = input.parse::<ViewStruct>()?;
                view_structs.push(view_struct);
            } else {
                return Err(lookahead.error());
            }
        }

        Ok(ViewSpec {
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
                "Expected 'fragment'"
            ));
        }
        let name: Ident = input.parse()?;

        let content;
        braced!(content in input);

        let mut fields = Vec::new();
        while !content.is_empty() {
            let field_spec = content.parse::<FieldSpec>()?;
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
                items.push(StructItem::Spread(fragment_name));
            } else {
                // Individual field
                let field_spec = content.parse::<FieldSpec>()?;
                items.push(StructItem::Field(field_spec));
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
        })
    }
}

impl Parse for FieldSpec {
    fn parse(input: ParseStream) -> Result<Self> {
        // Parse the pattern manually since Pat doesn't implement Parse directly
        let pattern = parse_field_pattern(input)?;
        
        // Extract field name if it's a simple identifier
        let field_name = extract_field_name_from_pattern(&pattern);

        // Check for transformation
        let transformation = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let expr: Expr = input.parse()?;
            Some(FieldTransformation::Assignment(expr))
        } else {
            None
        };

        Ok(FieldSpec {
            pattern,
            transformation,
            field_name,
        })
    }
}

/// Parse a field pattern manually
fn parse_field_pattern(input: ParseStream) -> Result<Pat> {
    // Parse a general pattern - this handles most common cases
    if input.peek(Ident) {
        // Could be a simple ident or start of a path/tuple struct pattern
        let lookahead = input.lookahead1();
        
        if lookahead.peek(Ident) && input.peek2(Token![::]) {
            // Path pattern like EnumType::Branch(value) or std::option::Option::Some(x)
            parse_path_pattern(input)
        } else if lookahead.peek(Ident) && input.peek2(Paren) {
            // Tuple struct pattern like Some(field) or MyStruct(x, y)
            parse_tuple_struct_pattern(input)
        } else if lookahead.peek(Ident) {
            // Simple identifier pattern
            let ident: Ident = input.parse()?;
            Ok(Pat::Ident(syn::PatIdent {
                attrs: vec![],
                by_ref: None,
                mutability: None,
                ident,
                subpat: None,
            }))
        } else {
            Err(lookahead.error())
        }
    } else {
        Err(syn::Error::new(input.span(), "Expected field pattern"))
    }
}

/// Parse a path-based pattern like EnumType::Branch(value)
fn parse_path_pattern(input: ParseStream) -> Result<Pat> {
    let mut segments = Punctuated::new();
    
    // Parse the path segments
    loop {
        let ident: Ident = input.parse()?;
        segments.push(syn::PathSegment {
            ident,
            arguments: syn::PathArguments::None,
        });
        
        if input.peek(Token![::]) {
            input.parse::<Token![::]>()?;
        } else {
            break;
        }
    }
    
    let path = syn::Path {
        leading_colon: None,
        segments,
    };
    
    // Check if this is followed by parentheses (tuple struct pattern)
    if input.peek(Paren) {
        let content;
        parenthesized!(content in input);
        
        let mut elems = Punctuated::new();
        while !content.is_empty() {
            let inner_pattern = parse_inner_pattern(&content)?;
            elems.push(inner_pattern);
            
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            } else {
                break;
            }
        }
        
        Ok(Pat::TupleStruct(syn::PatTupleStruct {
            attrs: vec![],
            qself: None,
            path,
            paren_token: syn::token::Paren::default(),
            elems,
        }))
    } else {
        // Just a path pattern
        Ok(Pat::Path(syn::PatPath {
            attrs: vec![],
            qself: None,
            path,
        }))
    }
}

/// Parse a simple tuple struct pattern like Some(field)
fn parse_tuple_struct_pattern(input: ParseStream) -> Result<Pat> {
    let ident: Ident = input.parse()?;
    let path = syn::Path::from(ident);
    
    let content;
    parenthesized!(content in input);
    
    let mut elems = Punctuated::new();
    while !content.is_empty() {
        let inner_pattern = parse_inner_pattern(&content)?;
        elems.push(inner_pattern);
        
        if content.peek(Token![,]) {
            content.parse::<Token![,]>()?;
        } else {
            break;
        }
    }
    
    Ok(Pat::TupleStruct(syn::PatTupleStruct {
        attrs: vec![],
        qself: None,
        path,
        paren_token: syn::token::Paren::default(),
        elems,
    }))
}

/// Parse inner patterns within parentheses
fn parse_inner_pattern(input: ParseStream) -> Result<Pat> {
    if input.peek(Ident) {
        let ident: Ident = input.parse()?;
        Ok(Pat::Ident(syn::PatIdent {
            attrs: vec![],
            by_ref: None,
            mutability: None,
            ident,
            subpat: None,
        }))
    } else if input.peek(Token![_]) {
        input.parse::<Token![_]>()?;
        Ok(Pat::Wild(syn::PatWild {
            attrs: vec![],
            underscore_token: Token![_](Span::call_site()),
        }))
    } else {
        Err(syn::Error::new(input.span(), "Expected identifier or wildcard pattern"))
    }
}

/// Extract a simple field name from a pattern if possible
fn extract_field_name_from_pattern(pattern: &Pat) -> Option<Ident> {
    match pattern {
        // Simple identifier: `field_name`
        Pat::Ident(pat_ident) => Some(pat_ident.ident.clone()),
        
        // Tuple struct pattern: `Some(field_name)` or `EnumType::Branch(field_name)`
        Pat::TupleStruct(pat_tuple_struct) => {
            if pat_tuple_struct.elems.len() == 1 {
                if let Pat::Ident(pat_ident) = &pat_tuple_struct.elems[0] {
                    return Some(pat_ident.ident.clone());
                }
            }
            None
        }
        
        // Path pattern: `EnumType::Branch`
        Pat::Path(pat_path) => {
            pat_path.path.get_ident().cloned()
        }
        
        _ => None,
    }
}

/// Helper function to get all fields for a view struct by resolving fragments
pub fn resolve_view_fields<'a>(
    view_struct: &'a ViewStruct, 
    fragments: &'a [Fragment]
) -> Result<Vec<&'a FieldSpec>> {
    let fragment_map: HashMap<String, &Fragment> = fragments
        .iter()
        .map(|f| (f.name.to_string(), f))
        .collect();

    let mut resolved_fields = Vec::new();

    for item in &view_struct.items {
        match item {
            StructItem::Spread(fragment_name) => {
                let fragment_name_str = fragment_name.to_string();
                if let Some(fragment) = fragment_map.get(&fragment_name_str) {
                    resolved_fields.extend(&fragment.fields);
                } else {
                    return Err(syn::Error::new(
                        fragment_name.span(),
                        format!("Fragment '{}' not found", fragment_name_str)
                    ));
                }
            }
            StructItem::Field(field_spec) => {
                resolved_fields.push(field_spec);
            }
        }
    }

    Ok(resolved_fields)
}

/// Helper function to check if a pattern represents an unwrapping operation
pub fn is_unwrap_pattern(pattern: &Pat) -> bool {
    match pattern {
        Pat::TupleStruct(pat_tuple_struct) => {
            // Check if this is any tuple struct pattern with arguments
            !pat_tuple_struct.elems.is_empty()
        }
        _ => false,
    }
}

/// Helper function to get the inner pattern from an unwrap pattern
pub fn get_inner_pattern_from_unwrap(pattern: &Pat) -> Option<&Pat> {
    match pattern {
        Pat::TupleStruct(pat_tuple_struct) => {
            if pat_tuple_struct.elems.len() == 1 {
                Some(&pat_tuple_struct.elems[0])
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Helper function to check if this is a specific pattern type (e.g., "Some")
pub fn is_pattern_type(pattern: &Pat, pattern_name: &str) -> bool {
    match pattern {
        Pat::TupleStruct(pat_tuple_struct) => {
            if let Some(last_segment) = pat_tuple_struct.path.segments.last() {
                last_segment.ident == pattern_name
            } else {
                false
            }
        }
        Pat::Path(pat_path) => {
            if let Some(ident) = pat_path.path.get_ident() {
                ident == pattern_name
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Helper to determine if a field spec has a transformation
pub fn has_transformation(field_spec: &FieldSpec) -> bool {
    field_spec.transformation.is_some()
}

/// Helper to get the target field name (either from pattern or explicit)
pub fn get_target_field_name(field_spec: &FieldSpec) -> Option<&Ident> {
    field_spec.field_name.as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

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
        if let StructItem::Spread(name) = &view_struct.items[0] {
            assert_eq!(name.to_string(), "all");
        } else {
            panic!("Expected spread item");
        }
    }

    #[test]
    fn test_parse_fragment_with_transformations() {
        let input = parse_quote! {
            fragment semantic {
                Some(semantic) = valid_semantic_value(semantic),
                Some(query)
            }
        };
        
        let fragment: Fragment = syn::parse2(input).unwrap();
        assert_eq!(fragment.fields.len(), 2);
        assert!(has_transformation(&fragment.fields[0]));
        assert!(!has_transformation(&fragment.fields[1]));
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
        
        let view_spec: ViewSpec = syn::parse2(input).unwrap();
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
        
        let view_spec: ViewSpec = syn::parse2(input).unwrap();
        let keyword_struct = &view_spec.view_structs[0];
        
        let resolved = resolve_view_fields(keyword_struct, &view_spec.fragments).unwrap();
        assert_eq!(resolved.len(), 4); // offset, limit, query, custom_field
    }
}