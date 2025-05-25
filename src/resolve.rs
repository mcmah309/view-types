use std::collections::{HashMap, HashSet};
use syn::{Error, Expr, Field, Ident};

use crate::parse::{ViewStructFieldKind, Views};

#[derive(Debug)]
pub(crate) struct ResolvedViewStruct<'a> {
    pub name: &'a Ident,
    pub generics: &'a Option<syn::Generics>,
    pub resolved_fields: Vec<ResolvedViewField<'a>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedViewField<'a> {
    /// `..field_name` or `field_name`
    pub source_view_field_name: &'a Ident,
    /// The filed in the original struct.
    /// If from a spread fragment, `field.ident` equals `source_view_field_name` otherwise it equals the entry
    /// in the fragment
    pub field: &'a Field,
    /// e.g. `std::option::Option::Some` in `std::option::Option::Some(field)`
    pub pattern_to_match: &'a Option<syn::Path>,
    /// e.g. `transfrom(field)` in `Some(field) = transfrom(field)`
    pub transformation: &'a Option<Expr>,
}

pub(crate) fn resolve<'a>(
    original_struct: &'a syn::ItemStruct,
    view_spec: &'a Views,
) -> syn::Result<Vec<ResolvedViewStruct<'a>>> {
    validate_unique_fields(view_spec)?;

    let original_struct_fields = extract_original_fields(&original_struct)?;

    let resolved_view_structs = resolve_field_references(view_spec, &original_struct_fields)?;
    Ok(resolved_view_structs)
}

fn validate_unique_fields(view_spec: &Views) -> syn::Result<()> {
    let mut fragment_names = HashSet::new();
    let mut view_struct_names = HashSet::new();

    for fragment in &view_spec.fragments {
        if !fragment_names.insert(fragment.name.to_string()) {
            return Err(Error::new(
                fragment.name.span(),
                format!("Duplicate fragment name found: '{}'", fragment.name),
            ));
        }
        let mut fields = HashSet::new();
        for field in &fragment.fields {
            if !fields.insert(field.field_name.to_string()) {
                return Err(Error::new(
                    field.field_name.span(),
                    format!(
                        "Duplicate field name '{}' in fragment '{}'",
                        field.field_name, fragment.name
                    ),
                ));
            }
        }
    }

    for view_struct in &view_spec.view_structs {
        if !view_struct_names.insert(view_struct.name.to_string()) {
            return Err(Error::new(
                view_struct.name.span(),
                format!("Duplicate view struct name found: '{}'", view_struct.name),
            ));
        }
        let mut spread_fields = HashSet::new();
        let mut regular_fields = HashSet::new();
        for item in &view_struct.items {
            match item {
                ViewStructFieldKind::FragmentSpread(fragment_name) => {
                    if !spread_fields.insert(fragment_name.to_string()) {
                        return Err(Error::new(
                            fragment_name.span(),
                            format!(
                                "Duplicate fragment spread '{}' in view struct '{}'",
                                fragment_name, view_struct.name
                            ),
                        ));
                    }
                }
                ViewStructFieldKind::Field(field_item) => {
                    if !regular_fields.insert(field_item.field_name.to_string()) {
                        return Err(Error::new(
                            field_item.field_name.span(),
                            format!(
                                "Duplicate field '{}' in view struct '{}'",
                                field_item.field_name, view_struct.name
                            ),
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Extract field map from the original struct
fn extract_original_fields(
    original_struct: &syn::ItemStruct,
) -> syn::Result<HashMap<String, &Field>> {
    let fields = match &original_struct.fields {
        syn::Fields::Named(fields) => fields,
        _ => {
            return Err(Error::new_spanned(
                original_struct,
                "Only structs with named fields are supported",
            ));
        }
    };

    let mut field_map = HashMap::new();
    for field in &fields.named {
        if let Some(field_name) = &field.ident {
            field_map.insert(field_name.to_string(), field);
        }
    }

    Ok(field_map)
}

/// Validate that all fragment fields exist the original struct and
/// that all in the view struct fields are existing fragments or existing fields in the original struct
fn resolve_field_references<'a, 'b>(
    view_spec: &'a Views,
    original_fields: &'b HashMap<String, &'a Field>,
) -> syn::Result<Vec<ResolvedViewStruct<'a>>> {
    // fragment name to original field
    let mut resolved_fragments: HashMap<String, Vec<ResolvedViewField<'a>>> = HashMap::new();
    for fragment in &view_spec.fragments {
        let fragment_name = fragment.name.to_string();
        if resolved_fragments.contains_key(&fragment_name) {
            return Err(Error::new(
                fragment.name.span(),
                format!("Duplicate fragment name found: '{}'", fragment_name),
            ));
        }
        let mut binding = resolved_fragments
            .entry(fragment_name)
            .insert_entry(Vec::new());
        let mut resolved_fragment_fields = binding.get_mut();
        for fragment_field_item in &fragment.fields {
            let fragment_field_name = fragment_field_item.field_name.to_string();
            if let Some(original_field) = original_fields.get(&fragment_field_name) {
                resolved_fragment_fields.push(ResolvedViewField {
                    source_view_field_name: &fragment_field_item.field_name,
                    field: original_field,
                    pattern_to_match: &fragment_field_item.pattern_to_match,
                    transformation: &fragment_field_item.transformation,
                });
            } else {
                return Err(Error::new(
                    fragment_field_item.field_name.span(),
                    format!(
                        "Field '{}' not found in original struct",
                        fragment_field_name
                    ),
                ));
            }
        }
    }

    let mut resolved_view_structs = Vec::new();

    for view_struct in &view_spec.view_structs {
        let mut resolved_fields: Vec<ResolvedViewField<'a>> = Vec::new();
        for field_kind in &view_struct.items {
            match field_kind {
                ViewStructFieldKind::FragmentSpread(fragment_name) => {
                    let fragment_name_string = fragment_name.to_string();
                    let fragment_resolved_fields = resolved_fragments
                        .get(&fragment_name_string)
                        .ok_or_else(|| {
                            Error::new(
                                fragment_name.span(),
                                format!("Fragment '{}' not found", fragment_name_string),
                            )
                        })?;
                    for fragment_resolved_field in fragment_resolved_fields {
                        resolved_fields.push(fragment_resolved_field.clone());
                    }
                }
                ViewStructFieldKind::Field(field_item) => {
                    let field_name = field_item.field_name.to_string();
                    if let Some(original_field) = original_fields.get(&field_name) {
                        resolved_fields.push(ResolvedViewField {
                            source_view_field_name: &field_item.field_name,
                            field: original_field,
                            pattern_to_match: &field_item.pattern_to_match,
                            transformation: &field_item.transformation,
                        });
                    } else {
                        return Err(Error::new(
                            field_item.field_name.span(),
                            format!("Field '{}' not found in original struct", field_name),
                        ));
                    }
                }
            };
        }
        resolved_view_structs.push(ResolvedViewStruct {
            name: &view_struct.name,
            generics: &view_struct.generics,
            resolved_fields,
        })
    }

    Ok(resolved_view_structs)
}