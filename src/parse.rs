use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, format_ident};
use syn::{
    parse::{Parse, ParseStream, Result},
    punctuated::Punctuated,
    token::{Brace, Paren},
    Expr, Ident, Pat, Token, Type, braced, parenthesized,
};
use std::collections::HashSet;

/// Top-level view specification containing types and field mappings
#[derive(Debug, Clone)]
pub struct ViewSpec {
    pub types: Vec<Type>,
    pub field_groups: Vec<FieldGroup>,
}

/// A group of views with their field specifications
#[derive(Debug, Clone)]
pub struct FieldGroup {
    pub view_names: Vec<Ident>,
    pub fields: Vec<FieldSpec>,
}

/// Individual field specification with optional transformation
#[derive(Debug, Clone)]
pub struct FieldSpec {
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
        let mut types = Vec::new();
        let mut field_groups = Vec::new();

        // Parse the types() declaration first
        if input.peek(Ident) && input.peek2(Paren) {
            let types_ident: Ident = input.parse()?;
            if types_ident != "types" {
                return Err(syn::Error::new(types_ident.span(), "Expected 'types'"));
            }

            let content;
            parenthesized!(content in input);
            let type_list: Punctuated<Type, Token![,]> = content.parse_terminated(Type::parse, Token![,])?;
            types = type_list.into_iter().collect();

            // Consume optional comma after types()
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        // Parse field groups
        while !input.is_empty() {
            let field_group = input.parse::<FieldGroup>()?;
            field_groups.push(field_group);

            // Consume optional trailing comma
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(ViewSpec {
            types,
            field_groups,
        })
    }
}

impl Parse for FieldGroup {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut view_names = Vec::new();

        // Parse view names separated by |
        loop {
            let view_name: Ident = input.parse()?;
            view_names.push(view_name);

            if input.peek(Token![|]) {
                input.parse::<Token![|]>()?;
            } else {
                break;
            }
        }

        // Parse the field block
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

        Ok(FieldGroup {
            view_names,
            fields,
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

/// Helper function to extract view names from field groups
pub fn extract_all_view_names(field_groups: &[FieldGroup]) -> HashSet<String> {
    let mut view_names = HashSet::new();
    for group in field_groups {
        for view_name in &group.view_names {
            view_names.insert(view_name.to_string());
        }
    }
    view_names
}

/// Helper function to get fields for a specific view
pub fn get_fields_for_view<'a>(field_groups: &'a [FieldGroup], view_name: &str) -> Vec<&'a FieldSpec> {
    let mut fields = Vec::new();
    for group in field_groups {
        if group.view_names.iter().any(|name| name.to_string() == view_name) {
            fields.extend(&group.fields);
        }
    }
    fields
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
    fn test_parse_simple_field_group() {
        let input = parse_quote! {
            KeywordSearch | SemanticSearch {
                offset,
                limit
            }
        };
        
        let field_group: FieldGroup = syn::parse2(input).unwrap();
        assert_eq!(field_group.view_names.len(), 2);
        assert_eq!(field_group.view_names[0].to_string(), "KeywordSearch");
        assert_eq!(field_group.view_names[1].to_string(), "SemanticSearch");
        assert_eq!(field_group.fields.len(), 2);
    }

    #[test]
    fn test_parse_unwrap_pattern() {
        let input = parse_quote! {
            KeywordSearch {
                Some(query),
                Option::Some(searchable_attributes),
                EnumType::Branch(value)
            }
        };
        
        let field_group: FieldGroup = syn::parse2(input).unwrap();
        assert_eq!(field_group.fields.len(), 3);
        
        for field in &field_group.fields {
            assert!(is_unwrap_pattern(&field.pattern));
        }
        
        // Test specific pattern types
        assert!(is_pattern_type(&field_group.fields[0].pattern, "Some"));
        assert!(is_pattern_type(&field_group.fields[1].pattern, "Some"));
        assert!(is_pattern_type(&field_group.fields[2].pattern, "Branch"));
    }

    #[test]
    fn test_parse_complex_patterns() {
        let input = parse_quote! {
            KeywordSearch {
                EnumType::Branch(value),
                std::option::Option::Some(query),
                MyEnum::Variant(x, y)
            }
        };
        
        let field_group: FieldGroup = syn::parse2(input).unwrap();
        assert_eq!(field_group.fields.len(), 3);
        
        // All should be recognized as unwrap patterns
        for field in &field_group.fields {
            assert!(is_unwrap_pattern(&field.pattern));
        }
    }

    #[test]
    fn test_parse_with_transformation() {
        let input = parse_quote! {
            SemanticSearch {
                Some(semantic) = valid_semantic_value(semantic)
            }
        };
        
        let field_group: FieldGroup = syn::parse2(input).unwrap();
        assert_eq!(field_group.fields.len(), 1);
        assert!(has_transformation(&field_group.fields[0]));
    }

    #[test]
    fn test_parse_full_view_spec() {
        let input = parse_quote! {
            types(KeywordSearch<'a>, SemanticSearch, HybridSearch<'a>),
            KeywordSearch | SemanticSearch | HybridSearch {
                offset,
                limit
            },
            SemanticSearch | HybridSearch {
                Some(semantic) = valid_semantic_value(semantic)
            },
            KeywordSearch | HybridSearch {
                Some(query),
                Some(searchable_attributes)
            }
        };
        
        let view_spec: ViewSpec = syn::parse2(input).unwrap();
        assert_eq!(view_spec.types.len(), 3);
        assert_eq!(view_spec.field_groups.len(), 3);
        
        // Check that we can extract all view names
        let view_names = extract_all_view_names(&view_spec.field_groups);
        assert!(view_names.contains("KeywordSearch"));
        assert!(view_names.contains("SemanticSearch"));
        assert!(view_names.contains("HybridSearch"));
    }

    #[test]
    fn test_get_fields_for_view() {
        let input = parse_quote! {
            types(KeywordSearch, SemanticSearch),
            KeywordSearch | SemanticSearch {
                offset,
                limit
            },
            KeywordSearch {
                Some(query)
            }
        };
        
        let view_spec: ViewSpec = syn::parse2(input).unwrap();
        
        let keyword_fields = get_fields_for_view(&view_spec.field_groups, "KeywordSearch");
        assert_eq!(keyword_fields.len(), 3); // offset, limit, query
        
        let semantic_fields = get_fields_for_view(&view_spec.field_groups, "SemanticSearch");
        assert_eq!(semantic_fields.len(), 2); // offset, limit
    }
}