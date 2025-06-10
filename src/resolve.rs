use std::collections::{HashMap, HashSet};
use syn::{
    Attribute, Error, Expr, Field, GenericArgument, Generics, Ident, ItemStruct, Lifetime, Type,
    Visibility,
};

use crate::parse::{ViewStructFieldKind, Views};

pub(crate) struct Builder<'a> {
    pub view_structs: Vec<ViewStructBuilder<'a>>,
    pub enum_attributes: Vec<Attribute>,
}

#[derive(Debug)]
pub(crate) struct ViewStructBuilder<'a> {
    pub name: &'a Ident,
    original_generics: &'a Option<syn::Generics>,
    pub builder_fields: Vec<BuilderViewField<'a>>,
    pub attributes: &'a Vec<syn::Attribute>,
    pub visibility: &'a Option<Visibility>,
    /// Generics that are added to the view struct *Ref and *Mut
    ref_generics: Option<syn::Generics>,
    /// Generics that are used in the regular view struct
    regular_generics: Option<syn::Generics>,
    pub ref_attributes: &'a Vec<Attribute>,
    pub mut_attributes: &'a Vec<Attribute>,
}

impl<'a> ViewStructBuilder<'a> {
    pub fn new(
        name: &'a Ident,
        original_generics: &'a Option<syn::Generics>,
        builder_fields: Vec<BuilderViewField<'a>>,
        attributes: &'a Vec<syn::Attribute>,
        visibility: &'a Option<Visibility>,
        ref_attributes: &'a Vec<Attribute>,
        mut_attributes: &'a Vec<Attribute>,
    ) -> Self {
        Self {
            name,
            original_generics,
            builder_fields,
            attributes,
            visibility,
            ref_generics: None,
            regular_generics: None,
            ref_attributes,
            mut_attributes,
        }
    }

    pub fn add_original_struct_lifetime_to_refs(&mut self) {
        if self.ref_generics.is_some() {
            return;
        }
        let new_lifetime = syn::parse_quote!('original);
        if let Some(original_generics) = &self.original_generics {
            let mut new_generics = original_generics.clone();
            new_generics.params.insert(0, new_lifetime);
            self.ref_generics = Some(new_generics);
        } else {
            let mut generics = Generics::default();
            generics.params.push(new_lifetime);
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

    pub fn get_regular_generics(&self) -> Option<&syn::Generics> {
        if let Some(generics) = &self.regular_generics {
            return Some(generics);
        }
        if let Some(original_generics) = &self.original_generics {
            return Some(original_generics);
        }
        None
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BuilderViewField<'a> {
    pub vis: &'a Visibility,
    pub name: &'a Ident,
    // pub original_struct_field_type: &'a syn::Type,
    /// view struct field type
    pub regular_struct_field_type: syn::Type,
    /// ref view struct field type
    pub ref_struct_field_type: syn::Type,
    /// ref view struct field type
    pub mut_struct_field_type: syn::Type,
    /// regular struct type without outer ref/mut and outer Option (possible inner ref/mut still there)
    pub stripped_type: syn::Type,
    pub is_stripped_type_ref: bool,
    pub is_ref: bool,
    pub is_mut: bool,
    pub is_option: bool,
    pub refs_need_original_lifetime: bool,
    pub pattern_to_match: &'a Option<syn::Path>,
    pub validation: &'a Option<Expr>,
}

impl<'a> BuilderViewField<'a> {
    pub fn new(
        original_struct_field: &'a Field,
        pattern_to_match: &'a Option<syn::Path>,
        explicit_type: &'a Option<syn::Type>,
        validation: &'a Option<Expr>,
    ) -> syn::Result<BuilderViewField<'a>> {
        let original_struct_field_type = &original_struct_field.ty;
        let regular_struct_field_type;
        let ref_struct_field_type;
        let mut_struct_field_type;
        let refs_need_original_lifetime;
        if let Some(pattern_to_match) = pattern_to_match {
            if let Some(explicit_type) = explicit_type {
                regular_struct_field_type = explicit_type.clone();
            } else {
                regular_struct_field_type = infer_inner_type_for_pattern_match(
                    original_struct_field_type,
                    pattern_to_match,
                )?
            }
        } else {
            if let Some(explicit_type) = explicit_type {
                regular_struct_field_type = explicit_type.clone();
            } else {
                regular_struct_field_type = original_struct_field_type.clone();
            }
        }
        let (is_ref, is_mut, type_changes) = determine_reference_types(&regular_struct_field_type);
        refs_need_original_lifetime = type_changes.is_some();
        if let Some((ref_type, mut_type)) = type_changes {
            ref_struct_field_type = ref_type;
            mut_struct_field_type = mut_type;
        } else {
            ref_struct_field_type = regular_struct_field_type.clone();
            mut_struct_field_type = regular_struct_field_type.clone();
        }
        let is_option = is_option(&ref_struct_field_type);
        let stripped_type = stripped_type(&regular_struct_field_type);
        let is_stripped_type_ref = match stripped_type {
            syn::Type::Reference(_) => true,
            _ => false,
        };

        Ok(BuilderViewField {
            vis: &original_struct_field.vis,
            name: &original_struct_field
                .ident
                .as_ref()
                .expect("Should not be a tuple struct"),
            // original_struct_field_type,
            regular_struct_field_type,
            ref_struct_field_type,
            mut_struct_field_type,
            stripped_type,
            is_stripped_type_ref,
            is_ref,
            is_mut,
            is_option,
            refs_need_original_lifetime,
            pattern_to_match,
            validation,
        })
    }
}

/// Resolves the references to fragments and fields
pub(crate) fn resolve<'a>(
    original_struct: &'a syn::ItemStruct,
    views: &'a Views,
    enum_attributes: Vec<Attribute>,
) -> syn::Result<Builder<'a>> {
    validate_original_struct(original_struct)?;
    validate_unique_fields(views)?;

    let original_struct_fields = extract_original_fields(&original_struct)?;

    let builder_view_structs = resolve_field_references(views, &original_struct_fields)?;

    Ok(Builder {
        view_structs: builder_view_structs,
        enum_attributes,
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
                    &fragment_field_item.explicit_type,
                    &fragment_field_item.validation,
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
                            &field_item.explicit_type,
                            &field_item.validation,
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

        let mut struct_builder = ViewStructBuilder::new(
            &view_struct.name,
            &view_struct.generics,
            builder_fields,
            &view_struct.attributes,
            &view_struct.visibility,
            &view_struct.ref_attributes,
            &view_struct.mut_attributes,
        );

        if struct_builder.builder_fields.iter().any(|e| e.is_ref) {
            struct_builder.add_original_struct_lifetime_to_refs();
        }

        builder_view_structs.push(struct_builder);
    }

    Ok(builder_view_structs)
}

/// Determines the correct reference types.
/// Outer references may need to change -
/// Mut lifetimes need to become `'original`, since otherwise it would imply the possibility of having two mutable references,
/// and `as_*_mut` methods would need `'original: *` (original to live at least as long as all inner lifetimes).
/// And for ref, all refs need to immutable, because the original struct will be borrowed as `&`.
/// # Returns
/// (is_ref, is_mut, (ref_ty, mut_ty))
/// * `is_ref` - whether the type is a reference type
/// * `is_mut` - whether the type is a mut reference type
/// * `(ref_ty, mut_ty)` - the new types if it is a reference type for `Ref` and `Mut` types
fn determine_reference_types(ty: &syn::Type) -> (bool, bool, Option<(syn::Type, syn::Type)>) {
    match ty {
        syn::Type::Reference(reference) => {
            if reference.mutability.is_some() {
                let lifetime: Lifetime = syn::parse_quote!('original);
                (
                    true,
                    true,
                    Some((
                        syn::Type::Reference(syn::TypeReference {
                            and_token: reference.and_token.clone(),
                            lifetime: Some(lifetime.clone()), // todo why can't this remain the same again?
                            mutability: None,
                            elem: Box::new(reference.elem.as_ref().clone()),
                        }),
                        (syn::Type::Reference(syn::TypeReference {
                            and_token: reference.and_token.clone(),
                            lifetime: Some(lifetime),
                            mutability: reference.mutability.clone(),
                            elem: Box::new(reference.elem.as_ref().clone()),
                        })),
                    )),
                )
            } else {
                (true, false, None)
            }
        }
        _ => (false, false, None),
    }
}

/// Strips the type of references and options.
fn stripped_type(mut ty: &syn::Type) -> syn::Type {
    if let syn::Type::Reference(type_reference) = ty {
        ty = &*type_reference.elem;
    }
    if let syn::Type::Path(type_path) = ty {
        if let Some(last_segment) = type_path.path.segments.last() {
            if last_segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    if let Some(GenericArgument::Type(inner_type)) = args.args.first() {
                        return inner_type.clone();
                    }
                }
            }
        }
    }

    ty.clone()
}

fn is_option(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            if let Some(last_segment) = type_path.path.segments.last() {
                return last_segment.ident == "Option";
            }
        }
        Type::Reference(type_reference) => {
            if let Type::Path(type_path) = type_reference.elem.as_ref() {
                if let Some(last_segment) = type_path.path.segments.last() {
                    return last_segment.ident == "Option";
                }
            }
        }
        _ => {}
    };
    false
}

fn infer_inner_type_for_pattern_match<'a>(
    ty: &'a Type,
    pattern_match: &syn::Path,
) -> syn::Result<Type> {
    let error = || {
        Err(syn::Error::new_spanned(
            pattern_match,
            "Anonymous pattern deconstructing is not implemented for this type. Add a type definition for the inner e.g. `EnumName::Branch(field: Type)`",
        ))
    };
    let is_ref;
    let ty2 = if let syn::Type::Reference(ref_ty) = ty {
        is_ref = true;
        &*ref_ty.elem
    } else {
        is_ref = false;
        ty
    };
    let inner_type: &syn::Type = if let syn::Type::Path(ty) = ty2 {
        let ty_last_segment = &ty.path.segments.last().unwrap();
        let ty_last_segment_name = ty_last_segment.ident.to_string();
        match ty_last_segment_name.as_str() {
            "Result" => {
                let arguments = &ty.path.segments.last().unwrap().arguments;
                match arguments {
                    syn::PathArguments::AngleBracketed(generic_arguments) => {
                        let mut args = generic_arguments.args.iter();
                        let ok = args.next().unwrap();
                        let Some(err) = args.next() else {
                            return error();
                        };
                        let is_ok = pattern_match
                            .segments
                            .last()
                            .unwrap()
                            .ident
                            .to_string()
                            .as_str()
                            == "Ok";
                        let type_to_use = if is_ok { ok } else { err };
                        match type_to_use {
                            GenericArgument::Type(inner_type) => inner_type,
                            _ => return error(),
                        }
                    }
                    _ => return error(),
                }
            }
            "Option" => {
                let arguments = &ty_last_segment.arguments;
                match arguments {
                    syn::PathArguments::AngleBracketed(generic_arguments) => {
                        let args = generic_arguments.args.iter();
                        let inner_generic_type = args.last().unwrap();
                        match inner_generic_type {
                            GenericArgument::Type(inner_type) => inner_type,
                            _ => return error(),
                        }
                    }
                    _ => return error(),
                }
            }
            _ => return error(),
        }
    } else {
        return error();
    };
    if is_ref {
        if let syn::Type::Reference(ref_ty) = ty {
            Ok(syn::Type::Reference(syn::TypeReference {
                and_token: ref_ty.and_token.clone(),
                lifetime: ref_ty.lifetime.clone(),
                mutability: None,
                elem: Box::new(inner_type.clone()),
            }))
        } else {
            unreachable!()
        }
    } else {
        Ok(inner_type.clone())
    }
}
