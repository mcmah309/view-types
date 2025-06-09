use quote::{format_ident, quote};
use std::collections::{HashMap, hash_map::Entry};
use syn::ItemStruct;

use crate::resolve::{Builder, BuilderViewField, ViewStructBuilder};

pub(crate) fn expand<'a>(
    original_struct: &'a ItemStruct,
    mut builder: Builder<'a>,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut generated_code = Vec::new();

    for view_structs in &mut builder.view_structs {
        let view_struct = generate_view_struct(view_structs)?;
        let ref_structs = generate_ref_view_structs_and_methods(view_structs)?; // Note: This mutates, order matters

        generated_code.push(view_struct);
        generated_code.push(ref_structs);
    }
    let views_enum = generate_views_enum_and_impl(original_struct, &builder)?;
    generated_code.extend(views_enum);

    let conversion_impl = generate_original_conversion_methods(original_struct, &builder)?;
    generated_code.push(conversion_impl);

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
        let vis = builder_field.vis;
        let field_name = builder_field.name;
        let ty = &builder_field.regular_struct_field_type;

        struct_fields.push(quote! {
            #vis #field_name: #ty
        });
    }

    let generics_clause = if let Some(g) = view_struct.get_regular_generics() {
        let (_, ty_generics, where_generics) = g.split_for_impl();
        quote! { #ty_generics #where_generics }
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

fn generate_views_enum_and_impl(
    original_struct: &ItemStruct,
    builder: &Builder<'_>,
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut branches = Vec::new();
    for view_struct in &builder.view_structs {
        let name = view_struct.name;
        let ty_generics = view_struct.get_regular_generics().map(|e| {
            let (_, ty_generics, _) = e.split_for_impl();
            ty_generics
        });
        branches.push(quote! {
            #name(#name #ty_generics)
        });
    }

    let ItemStruct {
        attrs: _,
        vis,
        struct_token: _,
        ident,
        generics,
        fields: _,
        semi_token: _,
    } = original_struct;

    let mut enum_name = ident.to_string();
    enum_name.push_str("Variant");
    let enum_name = syn::Ident::new(enum_name.as_str(), ident.span());

    let attrs = &builder.enum_attributes;

    let mut tokens = Vec::new();

    tokens.push(quote! {
        #(#attrs)*
        #vis enum #enum_name #generics {
            #(#branches,)*
        }
    });

    // Determine the common types for fields - what should be the return type of the variant methods
    let mut common_types_for_fields = HashMap::new();

    for field in builder.view_structs.iter().flat_map(|e| &e.builder_fields) {
        let entry = common_types_for_fields.entry(field.name);
        match entry {
            Entry::Occupied(mut occupied_entry) => {
                let current_common_ty: &mut CommmonType = occupied_entry.get_mut();
                current_common_ty.is_there_an_option =
                    current_common_ty.is_there_an_option || field.is_option;
                current_common_ty.is_there_an_owned =
                    current_common_ty.is_there_an_owned || !field.is_ref;
                current_common_ty.is_there_a_ref = current_common_ty.is_there_a_ref || field.is_ref;
                current_common_ty.is_there_a_mut = current_common_ty.is_there_a_mut || field.is_mut;
            }
            Entry::Vacant(vacant_entry) => {
                let common_type = CommmonType {
                    stripped_type: &field.stripped_type,
                    is_there_an_option: field.is_option,
                    is_there_an_owned: !field.is_ref,
                    is_there_a_ref: field.is_ref,
                    is_there_a_mut: field.is_mut,
                };
                vacant_entry.insert(common_type);
            }
        };
    }
    for (name, common_ty) in common_types_for_fields.iter_mut() { 
        for view_struct in builder.view_structs.iter() {
            if !view_struct.builder_fields.iter().any(|e| &e.name == name) {
                // At least one view does not contain these field so we need option
                common_ty.is_there_an_option = true;
            }
        }
    }

    let mut methods = Vec::new();
    let mut field_to_arms = HashMap::new();
    for view in &builder.view_structs {
        let view_name = view.name;
        for field in view.builder_fields.iter() {
            let arms_of_field = field_to_arms
                .entry(&field.name)
                .or_insert_with(|| Vec::new());

            let target_common_type = common_types_for_fields.get(&field.name).unwrap();

            let name = &field.name;

            // Add ref arms
            if target_common_type.is_there_an_option {
                if field.is_option {
                    if field.is_stripped_type_ref {
                        arms_of_field.push(quote! {
                            #enum_name::#view_name(view) => view.#name
                        });
                    }
                    else {
                        arms_of_field.push(quote! {
                            #enum_name::#view_name(view) => view.#name.as_ref()
                        });
                    }
                }
                else {
                    arms_of_field.push(quote! {
                        #enum_name::#view_name(view) => Some(&view.#name)
                    });
                }
            } else {
                arms_of_field.push(quote! {
                    #enum_name::#view_name(view) => &view.#name
                });
            }

            let can_add_mut_method = !target_common_type.is_there_a_ref;

            if can_add_mut_method {
                // todo
            }

            let can_add_owned_method =
                !target_common_type.is_there_a_ref && !target_common_type.is_there_a_mut;

            if can_add_mut_method {
                // todo
            }
        }
    }

    for (name,target_common_type) in common_types_for_fields.iter() {
        let arms = field_to_arms.get(name).unwrap();
        let stripped_type = target_common_type.stripped_type;
        let is_ref = match stripped_type {
            syn::Type::Reference(_) => true,
            _ => false,
        };
        let ref_token = if is_ref {
            quote! {}
        }
        else {
            quote! {&}
        };

        // Generate ref method
        if target_common_type.is_there_an_option {
            methods.push(quote! {
                pub fn #name(&self) -> Option<#ref_token #stripped_type> {
                    match self {
                        #(#arms,)*
                        _ => None,
                    }
                }
            });
        } else {
            methods.push(quote! {
                pub fn #name(&self) -> #ref_token #stripped_type {
                    match self {
                        #(#arms,)*
                    }
                }
            });
        }
    }

    // for view in &builder.view_structs {
    //     for field in view.builder_fields {}
    // }
    let (impl_ty, reg_ty, where_ty,) = generics.split_for_impl();
    tokens.push(quote! {
        impl #impl_ty #enum_name #reg_ty #where_ty { // todo split
            #(#methods)*
        }
    });

    Ok(tokens)
}

struct CommmonType<'a> {
    stripped_type: &'a syn::Type,
    is_there_an_option: bool,
    is_there_an_owned: bool,
    is_there_a_ref: bool,
    is_there_a_mut: bool,
}

/// Generate a reference and mutable reference structs
fn generate_ref_view_structs_and_methods(
    view_struct: &mut ViewStructBuilder,
) -> syn::Result<proc_macro2::TokenStream> {
    // todo check this lifetime does not exist
    let all_owned_fields_additional_immutable_ref = quote! { &'original };
    let all_owned_fields_additional_mutable_ref = quote! { &'original mut};
    let mut uses_additional_lifetime = false;

    let mut immutable_struct_fields = Vec::new();
    let mut mutable_struct_fields = Vec::new();
    let mut immutable_struct_method_fields = Vec::new();
    let mut mutable_struct_method_fields = Vec::new();
    for builder_field in &view_struct.builder_fields {
        let vis = builder_field.vis;
        let field_name = builder_field.name;
        let ref_ty = &builder_field.ref_struct_field_type;
        let mut_ty = &builder_field.mut_struct_field_type;

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
            #vis #field_name: #additional_immutable_ref #ref_ty
        });
        mutable_struct_fields.push(quote! {
            #vis #field_name: #additional_mutable_ref #mut_ty
        });
        immutable_struct_method_fields.push(quote! {
            #field_name: &self.#field_name
        });
        mutable_struct_method_fields.push(quote! {
            #field_name: &mut self.#field_name
        });
    }

    let ref_struct_name = format_ident!("{}Ref", view_struct.name);
    let mut_struct_name = format_ident!("{}Mut", view_struct.name);

    // Add lifetime parameter if does not already exist and needed
    let (ref_impl_generics, ref_type_generics, ref_where_clause) = if uses_additional_lifetime {
        view_struct.add_original_struct_lifetime_to_refs();
        let (impl_generics, type_generics, where_clause) = view_struct
            .get_ref_generics()
            .expect("If refs use an additional lifetime, then it must have had this generic added")
            .split_for_impl();
        (Some(impl_generics), Some(type_generics), Some(where_clause))
    } else {
        (None, None, None)
    };

    let ref_attributes = view_struct.ref_attributes;
    let mut_attributes = view_struct.mut_attributes;
    let visibility = view_struct.visibility;

    let (_regular_impl_generics, regular_type_generics, regular_where_clause) =
        if let Some(generics) = view_struct.get_regular_generics() {
            let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
            (Some(impl_generics), Some(type_generics), Some(where_clause))
        } else {
            (None, None, None)
        };
    let struct_name = &view_struct.name;

    Ok(quote! {
        #(#ref_attributes)*
        #visibility struct #ref_struct_name #ref_type_generics #ref_where_clause {
            #(#immutable_struct_fields,)*
        }

        #(#mut_attributes)*
        #visibility struct #mut_struct_name #ref_type_generics #ref_where_clause {
            #(#mutable_struct_fields,)*
        }

        impl #ref_impl_generics #struct_name #regular_type_generics #regular_where_clause {
            pub fn as_ref(&'original self) -> #ref_struct_name #ref_type_generics {
                #ref_struct_name {
                    #(#immutable_struct_method_fields,)*
                }
            }

            pub fn as_mut(&'original mut self) -> #mut_struct_name #ref_type_generics {
                #mut_struct_name {
                    #(#mutable_struct_method_fields,)*
                }
            }
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
        .insert(0, syn::parse_quote!('original));
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

            pub fn #as_ref_method(&'original self) -> #ref_return_type {
                #ref_body
            }

            pub fn #as_mut_method(&'original mut self) -> #mut_return_type {
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
        // Need to rebind lifetime to the original struct
        let final_deref = if builder_field.refs_need_original_lifetime {
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
