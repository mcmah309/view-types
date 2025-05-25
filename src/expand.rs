use syn::ItemStruct;

use crate::resolve::ResolvedViewStruct;



pub(crate) fn expand<'a>(original_struct: &'a ItemStruct, resolved_view_structs: Vec<ResolvedViewStruct<'a>>) -> syn::Result<proc_macro2::TokenStream> {
    todo!();
}