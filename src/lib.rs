use parse::Views;
use syn::ItemStruct;

mod expand;
mod parse;
mod resolve;

/// The main views procedural macro
/// 
/// # Example
/// ```rust
/// use view_types::views;
/// 
/// fn validate_ratio(ratio: &f32) -> bool {
///     (*ratio >= 0.0 && *ratio <= 1.0)
/// }
/// 
/// #[views(
///     fragment all {
///         offset,
///         limit,
///     }
///     fragment keyword {
///         Some(query),
///         words_limit
///     }
///     fragment semantic {
///         vector
///     }
///     pub view KeywordSearch {
///         ..all,
///         ..keyword,
///     }
///     pub view SemanticSearch<'a> {
///         ..all,
///         ..semantic,
///     }
///     pub view HybridSearch<'a> {
///         ..all,
///         ..keyword,
///         ..semantic,
///         Some(ratio) if validate_ratio(ratio)
///     }
/// )]
/// pub struct Search<'a> {
///     query: Option<String>,
///     offset: usize,
///     limit: usize,
///     words_limit: Option<usize>,
///     vector: Option<&'a Vec<u8>>,
///     ratio: Option<f32>,
/// }
/// ```
#[proc_macro_attribute]
pub fn views(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match views_impl(args, input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

fn views_impl(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> syn::Result<proc_macro::TokenStream> {
    let view_spec = syn::parse::<Views>(args.into())?;
    
    let mut original_struct = syn::parse::<ItemStruct>(input.into())?;
    let enum_attributes = crate::parse::extract_nested_attributes("Variant", &mut original_struct.attrs)?;
    let resolution = resolve::resolve(&original_struct, &view_spec, enum_attributes)?;
    
    let generated_code = expand::expand(&original_struct, resolution)?;
    
    Ok(quote::quote! {
        #original_struct
        #generated_code
    }.into())
}
