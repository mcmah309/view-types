use quote::{format_ident, quote};
use syn::ItemStruct;

use crate::resolve::{Builder, BuilderViewField, ViewStructBuilder};

pub(crate) fn expand<'a>(
    original_struct: &'a ItemStruct,
    mut builder: Builder<'a>,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut generated_code = Vec::new();

    for view_structs in &mut builder.view_structs {
        let view_struct = generate_view_struct(view_structs)?;
        let ref_structs = generate_ref_view_structs_and_methods(view_structs)?;

        generated_code.push(view_struct);
        generated_code.push(ref_structs);
    }
    let views_enum = generate_views_enum(original_struct, &builder.view_structs)?;
    generated_code.push(views_enum);

    let conversion_impl = generate_original_conversion_methods(original_struct, &builder)?;
    generated_code.push(conversion_impl);

    // todo
    // let inter_view_conversions = generate_inter_view_conversions(&context)?;
    // generated_code.extend(inter_view_conversions);

    Ok(quote! {
        #(#generated_code)*
    })
}

fn generate_view_struct(view_struct: &ViewStructBuilder) -> syn::Result<proc_macro2::TokenStream> {
    let ViewStructBuilder {
        name,
        builder_fields,
        attributes,
        visibility,
        ..
    } = view_struct;

    let mut struct_fields = Vec::new();
    for builder_field in builder_fields {
        // todo get any attributes from the view struct/fragment fields (we don't want to use the original)
        let vis = builder_field.vis;
        let field_name = builder_field.name;
        let ty = &builder_field.this_regular_struct_field_type;

        struct_fields.push(quote! {
            #vis #field_name: #ty
        });
    }

    let generics_clause = if let Some(g) = view_struct.get_regular_generics() {
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

fn generate_views_enum(
    original_struct: &ItemStruct,
    view_structs: &Vec<ViewStructBuilder>,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut branches = Vec::new();
    for view_struct in view_structs {
        let name = view_struct.name;
        let ty_generics = view_struct.get_regular_generics().map(|e| {
            let (_, ty_generics, _) = e.split_for_impl();
            ty_generics
        });
        branches.push(quote! {
            #name(#name #ty_generics)
        });
    }

    let ItemStruct { attrs, vis, struct_token, ident, generics, fields, semi_token } = original_struct;

    let mut enum_name = ident.to_string();
    enum_name.push_str("Kind");
    let enum_name = syn::Ident::new(enum_name.as_str(), ident.span());

    Ok(quote! {
        #(#attrs)*
        #vis enum #enum_name #generics {
            #(#branches,)*
        }
    })
}

/// Generate a reference and mutable reference structs
fn generate_ref_view_structs_and_methods(
    view_struct: &mut ViewStructBuilder,
) -> syn::Result<proc_macro2::TokenStream> {
    // todo check this lifetime does not exist
    let all_owned_fields_additional_immutable_ref = quote! { &'original_struct };
    let all_owned_fields_additional_mutable_ref = quote! { &'original_struct mut};
    let mut uses_additional_lifetime = false;

    let mut immutable_struct_fields = Vec::new();
    let mut mutable_struct_fields = Vec::new();
    for builder_field in &view_struct.builder_fields {
        // todo get any attributes from the view struct/fragment fields (we don't want to use the original)
        let vis = builder_field.vis;
        let field_name = builder_field.name;
        let ref_ty = &builder_field.this_ref_struct_field_type;
        let mut_ty = &builder_field.this_mut_struct_field_type;

        // Note: no need to check both, they both will be references or not
        let (additional_immutable_ref, additional_mutable_ref) = match ref_ty {
            syn::Type::Reference(_) => (None, None),
            _ => {
                uses_additional_lifetime = true;
                (
                    Some(all_owned_fields_additional_immutable_ref.clone()),
                    Some(all_owned_fields_additional_mutable_ref.clone()),
                )
            }
        };

        immutable_struct_fields.push(quote! {
            #vis #field_name : #additional_immutable_ref #ref_ty
        });
        mutable_struct_fields.push(quote! {
            #vis #field_name : #additional_mutable_ref #mut_ty
        });
    }

    let ref_struct_name = format_ident!("{}Ref", view_struct.name);
    let mut_struct_name = format_ident!("{}Mut", view_struct.name);

    // Add lifetime parameter if does not already exist and needed
    let struct_generics: Option<proc_macro2::TokenStream>;
    if uses_additional_lifetime {
        view_struct.add_original_struct_lifetime_to_refs();
        let (_, type_generics, where_clause) =
            view_struct.get_ref_generics().unwrap().split_for_impl();
        struct_generics = Some(quote! { #type_generics #where_clause });
    } else {
        struct_generics = None;
    }

    let attributes = view_struct.attributes;
    let visibility = view_struct.visibility;

    Ok(quote! {
        #(#attributes)*
        #visibility struct #ref_struct_name #struct_generics {
            #(#immutable_struct_fields,)*
        }

        #(#attributes)*
        #visibility struct #mut_struct_name #struct_generics {
            #(#mutable_struct_fields,)*
        }
    })
}

/// Generate conversion methods on the original struct
fn generate_original_conversion_methods(
    original_struct: &ItemStruct,
    context: &Builder,
) -> syn::Result<proc_macro2::TokenStream> {
    let original_name = &original_struct.ident;
    let original_generics = &original_struct.generics;
    let (_, original_ty_generics, original_where_clause) = original_generics.split_for_impl();
    let mut generics_with_new_lifetime = original_generics.clone();
    generics_with_new_lifetime
        .params
        .insert(0, syn::parse_quote!('original_struct));
    let (impl_generics, _, _) = generics_with_new_lifetime.split_for_impl();

    let mut methods = Vec::new();

    for view_struct in &context.view_structs {
        let view_name = view_struct.name;
        let snake_case_name = pascal_to_snake_case(&view_name.to_string());

        let into_method = format_ident!("into_{}", snake_case_name);
        let as_ref_method = format_ident!("as_{}_ref", snake_case_name);
        let as_mut_method = format_ident!("as_{}_mut", snake_case_name);

        // Generate field assignments
        let into_assignments = generate_into_assignments(&view_struct.builder_fields)?;
        let ref_assignments = generate_ref_assignments(&view_struct.builder_fields)?;
        let mut_assignments = generate_mut_assignments(&view_struct.builder_fields)?;

        // Determine return types
        let view_generics = view_struct.get_regular_generics();

        // Check if any field requires unwrapping (pattern matching)
        let has_unwrapping = view_struct
            .builder_fields
            .iter()
            .any(|e| e.pattern_to_match.is_some() || e.validation.is_some());
        let into_return_type = if has_unwrapping {
            quote! { Option<#view_name #view_generics> }
        } else {
            quote! { #view_name #view_generics }
        };

        let ref_struct_name = format_ident!("{}Ref", view_name);
        let mut_struct_name = format_ident!("{}Mut", view_name);

        let ref_struct_generics = view_struct.get_ref_generics().map(|e| {
            let (_, type_generics, _) = e.split_for_impl();
            type_generics
        });

        let ref_return_type = if has_unwrapping {
            quote! { Option<#ref_struct_name # ref_struct_generics> }
        } else {
            quote! { #ref_struct_name #ref_struct_generics }
        };

        let mut_return_type = if has_unwrapping {
            quote! { Option<#mut_struct_name #ref_struct_generics> }
        } else {
            quote! { #mut_struct_name #ref_struct_generics }
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

            pub fn #as_ref_method(&'original_struct self) -> #ref_return_type {
                #ref_body
            }

            pub fn #as_mut_method(&'original_struct mut self) -> #mut_return_type {
                #mut_body
            }
        });
    }

    Ok(quote! {
        impl #impl_generics #original_name #original_ty_generics #original_where_clause {
            #(#methods)*
        }
    })
}

fn generate_into_assignments(
    builder_fields: &[BuilderViewField],
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut assignments = Vec::new();

    for builder_field in builder_fields {
        let field_name = builder_field.name;

        let assignment = if let Some(pattern_path) = builder_field.pattern_to_match {
            if let Some(validation) = builder_field.validation {
                quote! {
                    #field_name: if let #pattern_path(#field_name) = self.#field_name {
                        {
                            let #field_name = &#field_name;
                            if !(#validation) {
                                return None;
                            }
                        }
                        #field_name
                    } else {
                        return None;
                    }
                }
            } else {
                quote! {
                    #field_name: if let #pattern_path(#field_name) = self.#field_name { #field_name } else { return None }
                }
            }
        } else {
            if let Some(validation) = builder_field.validation {
                quote! {
                    #field_name: {
                        let #field_name = &self.#field_name;
                        if !(#validation) {
                            return None;
                        }
                        self.#field_name
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
    builder_fields: &[BuilderViewField],
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut assignments = Vec::new();

    for builder_field in builder_fields {
        let field_name = builder_field.name;

        let assignment = if let Some(pattern_path) = builder_field.pattern_to_match {
            // Generate explicit pattern matching for references
            if let Some(validation) = builder_field.validation {
                quote! {
                    #field_name: if let #pattern_path(#field_name) = &self.#field_name {
                        if !(#validation) {
                            return None;
                        }
                        #field_name
                    } else {
                        return None;
                    }
                }
            } else {
                quote! {
                    #field_name: if let #pattern_path(#field_name) = &self.#field_name { #field_name } else { return None }
                }
            }
        } else {
            if let Some(validation) = builder_field.validation {
                quote! {
                    #field_name: {
                        let #field_name = &self.#field_name;
                        if !(#validation) {
                            return None;
                        }
                        #field_name
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
    builder_fields: &[BuilderViewField],
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut assignments = Vec::new();

    for builder_field in builder_fields {
        let field_name = builder_field.name;
        let final_deref = if builder_field.is_refs_and_original_struct_lifetime {
            quote! { &mut *#field_name }
        } else {
            quote! { #field_name }
        };

        let assignment = if let Some(pattern_path) = builder_field.pattern_to_match {
            if let Some(validation) = builder_field.validation {
                quote! {
                    #field_name: if let #pattern_path(#field_name) = &mut self.#field_name {
                        {
                            let #field_name = &*#field_name;
                            if !(#validation) {
                                return None;
                            }
                        }
                        #final_deref
                    } else {
                        return None;
                    }
                }
            } else {
                quote! {
                    #field_name: if let #pattern_path(#field_name) = &mut self.#field_name { #final_deref } else { return None }
                }
            }
        } else {
            if let Some(validation) = builder_field.validation {
                quote! {
                    #field_name: {
                        let #field_name = &mut self.#field_name;
                        {
                            let #field_name = &*#field_name;
                            if !(#validation) {
                                return None;
                            }
                        }
                        #final_deref
                    }
                }
            } else {
                quote! {
                    #field_name: {
                        let #field_name = &mut self.#field_name;
                        #final_deref
                    }
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
