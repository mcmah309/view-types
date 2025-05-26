use std::{
    cell::OnceCell,
    collections::{HashMap, HashSet},
};
use syn::{
    Error, Expr, Field, GenericArgument, GenericParam, Generics, Ident, ItemStruct, Type,
    Visibility,
};

use crate::parse::{ViewStructFieldKind, Views};

pub(crate) struct Builder<'a> {
    pub view_structs: Vec<ViewStructBuilder<'a>>,
    // todo
    // /// view structs that are subsets of other view structs, thus can be converted between
    // sub_structs: HashMap<usize,usize>,
}

// /// Check if target view is a subset of source view
// fn is_view_subset(target_view: &ResolvedViewStruct, source_view: &ResolvedViewStruct) -> bool {
//     let source_field_names: HashSet<String> = source_view.builder_fields
//         .iter()
//         .map(|f| f.source_view_field_name.to_string())
//         .collect();

//     target_view.builder_fields.iter().all(|target_field| {
//         source_field_names.contains(&target_field.source_view_field_name.to_string())
//     })
// }

#[derive(Debug)]
pub(crate) struct ViewStructBuilder<'a> {
    pub name: &'a Ident,
    original_generics: &'a Option<syn::Generics>,
    pub builder_fields: Vec<BuilderViewField<'a>>,
    pub attributes: &'a Vec<syn::Attribute>,
    pub visibility: &'a Option<Visibility>,
    /// Generics that are added to the view struct *Ref and *Mut
    ref_generics: Option<syn::Generics>,
}

impl<'a> ViewStructBuilder<'a> {
    pub fn new(
        name: &'a Ident,
        original_generics: &'a Option<syn::Generics>,
        builder_fields: Vec<BuilderViewField<'a>>,
        attributes: &'a Vec<syn::Attribute>,
        visibility: &'a Option<Visibility>,
    ) -> Self {
        Self {
            name,
            original_generics,
            builder_fields,
            attributes,
            visibility,
            ref_generics: None,
        }
    }

    pub fn add_ref_generic(&mut self, generic: GenericParam) {
        if let Some(built_generics) = &mut self.ref_generics {
            built_generics.params.insert(0, generic);
            return;
        }
        if let Some(original_generics) = &self.original_generics {
            let mut new_generics = original_generics.clone();
            new_generics.params.insert(0, generic);
            self.ref_generics = Some(new_generics);
        } else {
            let mut generics = Generics::default();
            generics.params.push(generic);
            self.ref_generics = Some(generics);
        }
    }

    pub fn get_ref_generics(&self) -> Option<&syn::Generics> {
        if let Some(generics) = &self.ref_generics {
            return Some(generics);
        } else if let Some(original_generics) = &self.original_generics {
            return Some(original_generics);
        } else {
            None
        }
    }

    pub fn get_regular_generics(&self) -> &Option<syn::Generics> {
        &self.original_generics
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BuilderViewField<'a> {
    pub original_struct_field: &'a Field,
    pub this_struct_field_type: &'a syn::Type,
    pub pattern_to_match: &'a Option<syn::Path>,
    pub transformation: &'a Option<Expr>,
}

impl<'a> BuilderViewField<'a> {
    pub fn new(
        original_struct_field: &'a Field,
        pattern_to_match: &'a Option<syn::Path>,
        transformation: &'a Option<Expr>,
    ) -> syn::Result<BuilderViewField<'a>> {
        let this_struct_field_type = if pattern_to_match.is_some() {
            get_inner_type_for_pattern_match(&original_struct_field.ty)?
        } else {
            &original_struct_field.ty
        };
        Ok(BuilderViewField {
            original_struct_field,
            this_struct_field_type,
            pattern_to_match,
            transformation,
        })
    }
}

/// Resolves the references to fragments and fields
pub(crate) fn resolve<'a>(
    original_struct: &'a syn::ItemStruct,
    view_spec: &'a Views,
) -> syn::Result<Builder<'a>> {
    validate_original_struct(original_struct)?;
    validate_unique_fields(view_spec)?;

    let original_struct_fields = extract_original_fields(&original_struct)?;

    let builder_view_structs = resolve_field_references(view_spec, &original_struct_fields)?;
    Ok(Builder {
        view_structs: builder_view_structs,
    })
}

/// Validate that the original struct is suitable for view generation
fn validate_original_struct(original_struct: &ItemStruct) -> syn::Result<()> {
    match &original_struct.fields {
        syn::Fields::Named(_) => Ok(()),
        syn::Fields::Unnamed(_) => Err(syn::Error::new_spanned(
            original_struct,
            "Views macro only supports structs with named fields (not tuple structs)",
        )),
        syn::Fields::Unit => Err(syn::Error::new_spanned(
            original_struct,
            "Views macro only supports structs with named fields (not unit structs)",
        )),
    }
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
) -> syn::Result<Vec<ViewStructBuilder<'a>>> {
    // fragment name to original field
    let mut builder_fragments: HashMap<String, Vec<BuilderViewField<'a>>> = HashMap::new();
    for fragment in &view_spec.fragments {
        let fragment_name = fragment.name.to_string();
        if builder_fragments.contains_key(&fragment_name) {
            return Err(Error::new(
                fragment.name.span(),
                format!("Duplicate fragment name found: '{}'", fragment_name),
            ));
        }
        let mut binding = builder_fragments
            .entry(fragment_name)
            .insert_entry(Vec::new());
        let builder_fragment_fields = binding.get_mut();
        for fragment_field_item in &fragment.fields {
            let fragment_field_name = fragment_field_item.field_name.to_string();
            if let Some(original_field) = original_fields.get(&fragment_field_name) {
                builder_fragment_fields.push(BuilderViewField::new(
                    original_field,
                    &fragment_field_item.pattern_to_match,
                    &fragment_field_item.transformation,
                )?);
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

    let mut builder_view_structs = Vec::new();

    for view_struct in &view_spec.view_structs {
        let mut builder_fields: Vec<BuilderViewField<'a>> = Vec::new();
        for field_kind in &view_struct.items {
            match field_kind {
                ViewStructFieldKind::FragmentSpread(fragment_name) => {
                    let fragment_name_string = fragment_name.to_string();
                    let fragment_builder_fields = builder_fragments
                        .get(&fragment_name_string)
                        .ok_or_else(|| {
                            Error::new(
                                fragment_name.span(),
                                format!("Fragment '{}' not found", fragment_name_string),
                            )
                        })?;
                    for fragment_builder_field in fragment_builder_fields {
                        builder_fields.push(fragment_builder_field.clone());
                    }
                }
                ViewStructFieldKind::Field(field_item) => {
                    let field_name = field_item.field_name.to_string();
                    if let Some(original_field) = original_fields.get(&field_name) {
                        builder_fields.push(BuilderViewField::new(
                            original_field,
                            &field_item.pattern_to_match,
                            &field_item.transformation,
                        )?);
                    } else {
                        return Err(Error::new(
                            field_item.field_name.span(),
                            format!("Field '{}' not found in original struct", field_name),
                        ));
                    }
                }
            };
        }

        builder_view_structs.push(ViewStructBuilder::new(
            &view_struct.name,
            &view_struct.generics,
            builder_fields,
            &view_struct.attributes,
            &view_struct.visibility,
        ));
    }

    Ok(builder_view_structs)
}

fn get_inner_type_for_pattern_match(ty: &Type) -> syn::Result<&Type> {
    let error = || {
        Err(syn::Error::new_spanned(
            // todo: how to handle this for regular deconstruction since we don't know the type to use?
            ty,
            "Pattern deconstructing is only implemented for single generic types. Otherwise the type being mapped to is ambagious.",
        ))
    };
    match ty {
        syn::Type::Path(ty) => {
            let arguments = &ty.path.segments.last().unwrap().arguments;
            match arguments {
                syn::PathArguments::AngleBracketed(generic_arguments) => {
                    let mut args = generic_arguments.args.iter();
                    let inner_generic_arg = args.next().unwrap();
                    if args.len() != 0 {
                        return error();
                    }
                    match inner_generic_arg {
                        GenericArgument::Type(inner_type) => return Ok(inner_type),
                        _ => return error(),
                    };
                }
                _ => return error(),
            }
        }
        _ => return error(),
    };
}
