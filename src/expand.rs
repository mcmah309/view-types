use quote::{format_ident, quote};
use syn::{Field, ItemStruct, token::Type};

use crate::resolve::{Resolution, ResolvedViewField, ResolvedViewStruct};

pub(crate) fn expand<'a>(
    original_struct: &'a ItemStruct,
    resolution: Resolution<'a>,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut generated_code = Vec::new();

    for resolved_view in &resolution.view_structs {
        let view_struct = generate_view_struct(resolved_view)?;
        let ref_struct = generate_ref_view_struct(resolved_view)?;
        let mut_struct = generate_mut_view_struct(resolved_view)?;

        generated_code.push(view_struct);
        generated_code.push(ref_struct);
        generated_code.push(mut_struct);
    }

    let conversion_impl = generate_original_conversion_methods(original_struct, &resolution)?;
    generated_code.push(conversion_impl);

    // todo
    // let inter_view_conversions = generate_inter_view_conversions(&resolution)?;
    // generated_code.extend(inter_view_conversions);

    Ok(quote! {
        #(#generated_code)*
    })
}

fn generate_view_struct(
    resolved_view: &ResolvedViewStruct,
) -> syn::Result<proc_macro2::TokenStream> {
    let ResolvedViewStruct {
        name,
        generics,
        resolved_fields,
        attributes,
        visibility,
    } = resolved_view;

    let mut struct_fields = Vec::new();
    for resolved_field in resolved_fields {
        let Field {
            attrs: _, // todo get any attributes from the view struct/fragment fields (we don't want to use the original)
            vis,
            mutability: _,
            ident,
            colon_token: _,
            ty,
        } = resolved_field.original_struct_field;

        struct_fields.push(quote! {
            #vis #ident: #ty
        });
    }

    let generics_clause = if let Some(g) = generics {
        quote! { #g }
    } else {
        quote! {}
    };

    Ok(quote! {
        #(#attributes)*
        #visibility struct #name #generics_clause {
            #(#struct_fields,)*
        }
    })
}

/// Generate a reference struct (ViewRef)
fn generate_ref_view_struct(
    resolved_view: &ResolvedViewStruct,
) -> syn::Result<proc_macro2::TokenStream> {
    let ResolvedViewStruct {
        name,
        generics,
        resolved_fields,
        attributes,
        visibility,
    } = resolved_view;

    let ref_struct_name = format_ident!("{}Ref", name);

    let all_owned_fields_additional_ref = quote! { &'original_struct }; // todo check this lifetime does not exist
    let mut uses_all_owned_fields_additional_ref = false;

    let mut struct_fields = Vec::new();
    for resolved_field in resolved_fields {
        let Field {
            attrs: _, // todo get any attributes from the view struct/fragment fields (we don't want to use the original which is this)
            vis,
            mutability: _,
            ident,
            colon_token: _,
            ty,
        } = resolved_field.original_struct_field;

        let additional_ref = match ty {
            syn::Type::Reference(_) => None,
            _ => {
                uses_all_owned_fields_additional_ref = true;
                Some(all_owned_fields_additional_ref.clone())
            }
        };

        struct_fields.push(quote! {
            #vis #ident : #additional_ref #ty
        });
    }

    // Add lifetime parameter if does not already exist and needed
    let struct_lifetime_generics: Option<proc_macro2::TokenStream>;
    if uses_all_owned_fields_additional_ref {
        if let Some(generics) = generics {
            let mut new_generics = generics.clone();
            new_generics
                .params
                .insert(0, syn::parse_quote!('original_struct));
            struct_lifetime_generics = Some(quote! { #new_generics });
        } else {
            struct_lifetime_generics = Some(quote! { <'original_struct> });
        };
    } else {
        if let Some(generics) = generics {
            struct_lifetime_generics = Some(quote! { #generics });
        }
        else {
            struct_lifetime_generics = None;
        }
    }

    Ok(quote! {
        #(#attributes)*
        #visibility struct #ref_struct_name #struct_lifetime_generics {
            #(#struct_fields,)*
        }
    })
}

/// Generate a mutable reference struct (ViewMut)
fn generate_mut_view_struct(
    resolved_view: &ResolvedViewStruct,
) -> syn::Result<proc_macro2::TokenStream> {
    let ResolvedViewStruct {
        name,
        generics,
        resolved_fields,
        attributes,
        visibility,
    } = resolved_view;

    let mut_struct_name = format_ident!("{}Mut", name);

        let all_owned_fields_additional_ref = quote! { &'original_struct mut }; // todo check this lifetime does not exist
    let mut uses_all_owned_fields_additional_ref = false;

    let mut struct_fields = Vec::new();
    for resolved_field in resolved_fields {
        let Field {
            attrs: _, // todo get any attributes from the view struct/fragment fields (we don't want to use the original which is this)
            vis,
            mutability: _,
            ident,
            colon_token: _,
            ty,
        } = resolved_field.original_struct_field;

        let additional_ref = match ty {
            syn::Type::Reference(_) => None,
            _ => {
                uses_all_owned_fields_additional_ref = true;
                Some(all_owned_fields_additional_ref.clone())
            }
        };

        struct_fields.push(quote! {
            #vis #ident : #additional_ref #ty
        });
    }

    // Add lifetime parameter if does not already exist and needed
    let struct_lifetime_generics: Option<proc_macro2::TokenStream>;
    if uses_all_owned_fields_additional_ref {
        if let Some(generics) = generics {
            let mut new_generics = generics.clone();
            new_generics
                .params
                .insert(0, syn::parse_quote!('original_struct));
            struct_lifetime_generics = Some(quote! { #new_generics });
        } else {
            struct_lifetime_generics = Some(quote! { <'original_struct> });
        };
    } else {
        if let Some(generics) = generics {
            struct_lifetime_generics = Some(quote! { #generics });
        }
        else {
            struct_lifetime_generics = None;
        }
    }

    Ok(quote! {
        #(#attributes)*
        #visibility struct #mut_struct_name #struct_lifetime_generics {
            #(#struct_fields,)*
        }
    })
}

/// Generate conversion methods on the original struct
fn generate_original_conversion_methods(
    original_struct: &ItemStruct,
    resolution: &Resolution,
) -> syn::Result<proc_macro2::TokenStream> {
    let original_name = &original_struct.ident;
    let original_generics = &original_struct.generics;
    let (impl_generics, ty_generics, where_clause) = original_generics.split_for_impl();

    let mut methods = Vec::new();

    for resolved_view in &resolution.view_structs {
        let view_name = resolved_view.name;
        let snake_case_name = pascal_to_snake_case(&view_name.to_string());

        let into_method = format_ident!("into_{}", snake_case_name);
        let as_ref_method = format_ident!("as_{}_ref", snake_case_name);
        let as_mut_method = format_ident!("as_{}_mut", snake_case_name);

        // Generate field assignments
        let into_assignments = generate_into_assignments(&resolved_view.resolved_fields)?;
        let ref_assignments = generate_ref_assignments(&resolved_view.resolved_fields)?;
        let mut_assignments = generate_mut_assignments(&resolved_view.resolved_fields)?;

        // Determine return types
        let view_generics = if let Some(g) = resolved_view.generics {
            quote! { #g }
        } else {
            quote! {}
        };

        // Check if any field requires unwrapping (pattern matching)
        let has_unwrapping = resolved_view
            .resolved_fields
            .iter()
            .any(|f| f.pattern_to_match.is_some());
        let into_return_type = if has_unwrapping {
            quote! { Option<#view_name #view_generics> }
        } else {
            quote! { #view_name #view_generics }
        };

        let ref_struct_name = format_ident!("{}Ref", view_name);
        let mut_struct_name = format_ident!("{}Mut", view_name);

        let ref_return_type = if has_unwrapping {
            quote! { Option<#ref_struct_name<'_>> }
        } else {
            quote! { #ref_struct_name<'_> }
        };

        let mut_return_type = if has_unwrapping {
            quote! { Option<#mut_struct_name<'_>> }
        } else {
            quote! { #mut_struct_name<'_> }
        };

        // Method bodies
        let into_body = if has_unwrapping {
            quote! {
                Some(#view_name {
                    #(#into_assignments,)*
                })
            }
        } else {
            quote! {
                #view_name {
                    #(#into_assignments,)*
                }
            }
        };

        let ref_body = if has_unwrapping {
            quote! {
                Some(#ref_struct_name {
                    #(#ref_assignments,)*
                })
            }
        } else {
            quote! {
                #ref_struct_name {
                    #(#ref_assignments,)*
                }
            }
        };

        let mut_body = if has_unwrapping {
            quote! {
                Some(#mut_struct_name {
                    #(#mut_assignments,)*
                })
            }
        } else {
            quote! {
                #mut_struct_name {
                    #(#mut_assignments,)*
                }
            }
        };

        methods.push(quote! {
            pub fn #into_method(self) -> #into_return_type {
                #into_body
            }

            pub fn #as_ref_method(&self) -> #ref_return_type {
                #ref_body
            }

            pub fn #as_mut_method(&mut self) -> #mut_return_type {
                #mut_body
            }
        });
    }

    Ok(quote! {
        impl #impl_generics #original_name #ty_generics #where_clause {
            #(#methods)*
        }
    })
}

fn generate_into_assignments(
    resolved_fields: &[ResolvedViewField],
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut assignments = Vec::new();

    for resolved_field in resolved_fields {
        let field_name = resolved_field
            .original_struct_field
            .ident
            .as_ref()
            .expect("Should not be a tuple struct");

        let assignment = if let Some(pattern_path) = resolved_field.pattern_to_match {
            // Generate explicit pattern matching assignment
            if let Some(transformation) = resolved_field.transformation {
                // Pattern with transformation: if let Pattern(field) = transform(original_field) { field } else { return None }
                quote! {
                    #field_name: if let #pattern_path(#field_name) = {
                        let #field_name = self.#field_name;
                        #transformation
                    } { #field_name } else { return None }
                }
            } else {
                // Simple pattern: if let Pattern(field) = original_field { field } else { return None }
                quote! {
                    #field_name: if let #pattern_path(#field_name) = self.#field_name { #field_name } else { return None }
                }
            }
        } else {
            // No pattern matching needed
            if let Some(transformation) = resolved_field.transformation {
                quote! {
                    #field_name: {
                        let #field_name = self.#field_name;
                        #transformation
                    }
                }
            } else {
                quote! {
                    #field_name: self.#field_name
                }
            }
        };

        assignments.push(assignment);
    }

    Ok(assignments)
}

fn generate_ref_assignments(
    resolved_fields: &[ResolvedViewField],
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut assignments = Vec::new();

    for resolved_field in resolved_fields {
        let field_name = resolved_field
            .original_struct_field
            .ident
            .as_ref()
            .expect("Should not be a tuple struct");

        let assignment = if let Some(pattern_path) = resolved_field.pattern_to_match {
            // Generate explicit pattern matching for references
            if let Some(transformation) = resolved_field.transformation {
                // Pattern with transformation for refs
                quote! {
                    #field_name: if let #pattern_path(ref #field_name) = {
                        let #field_name = &self.#field_name;
                        #transformation
                    } { #field_name } else { return None }
                }
            } else {
                // Simple pattern: if let Pattern(ref field) = &original_field { field } else { return None }
                quote! {
                    #field_name: if let #pattern_path(ref #field_name) = &self.#field_name { #field_name } else { return None }
                }
            }
        } else {
            // No pattern matching, just take reference
            if let Some(transformation) = resolved_field.transformation {
                quote! {
                    #field_name: {
                        let #field_name = &self.#field_name;
                        &#transformation
                    }
                }
            } else {
                quote! {
                    #field_name: &self.#field_name
                }
            }
        };

        assignments.push(assignment);
    }

    Ok(assignments)
}

/// Generate field assignments for as_mut methods
fn generate_mut_assignments(
    resolved_fields: &[ResolvedViewField],
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut assignments = Vec::new();

    for resolved_field in resolved_fields {
        let field_name = resolved_field
            .original_struct_field
            .ident
            .as_ref()
            .expect("Should not be a tuple struct");

        let assignment = if let Some(pattern_path) = resolved_field.pattern_to_match {
            // Generate explicit pattern matching for mutable references
            if let Some(transformation) = resolved_field.transformation {
                // Pattern with transformation for mut refs
                quote! {
                    #field_name: if let #pattern_path(ref mut #field_name) = {
                        let #field_name = &mut self.#field_name;
                        #transformation
                    } { #field_name } else { return None }
                }
            } else {
                // Simple pattern: if let Pattern(ref mut field) = &mut original_field { field } else { return None }
                quote! {
                    #field_name: if let #pattern_path(ref mut #field_name) = &mut self.#field_name { #field_name } else { return None }
                }
            }
        } else {
            // No pattern matching, just take mutable reference
            if let Some(transformation) = resolved_field.transformation {
                quote! {
                    #field_name: {
                        let #field_name = &mut self.#field_name;
                        &mut #transformation
                    }
                }
            } else {
                quote! {
                    #field_name: &mut self.#field_name
                }
            }
        };

        assignments.push(assignment);
    }

    Ok(assignments)
}

fn pascal_to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    if let Some(ch) = chars.next() {
        result.push(ch.to_lowercase().next().unwrap());
    }

    while let Some(ch) = chars.next() {
        if ch.is_uppercase() {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }

    result
}
