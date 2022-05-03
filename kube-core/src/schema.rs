//! Utilities for managing [`CustomResourceDefinition`] schemas
//!
//! [`CustomResourceDefinition`]: `k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition`

use std::collections::btree_map::Entry;

// Used in docs
#[allow(unused_imports)] use schemars::gen::SchemaSettings;

use schemars::{
    schema::{Metadata, ObjectValidation, Schema, SchemaObject},
    visit::Visitor,
};

/// schemars [`Visitor`] that rewrites a [`Schema`] to conform to Kubernetes' "structural schema" rules
///
/// The following two transformations are applied
///  * Rewrite enums from `oneOf` to `object`s with multiple variants ([schemars#84](https://github.com/GREsau/schemars/issues/84))
///  * Rewrite `additionalProperties` from `#[serde(flatten)]` to `x-kubernetes-preserve-unknown-fields` ([kube-rs#844](https://github.com/kube-rs/kube-rs/issues/844))
///
/// This is used automatically by `kube::derive`'s `#[derive(CustomResource)]`,
/// but it can also be used manually with [`SchemaSettings::with_visitor`].
///
/// # Panics
///
/// The [`Visitor`] functions may panic if the transform could not be applied. For example,
/// there must not be any overlapping properties between `oneOf` branches.
#[derive(Debug, Clone)]
pub struct StructuralSchemaRewriter;

impl Visitor for StructuralSchemaRewriter {
    fn visit_schema_object(&mut self, schema: &mut schemars::schema::SchemaObject) {
        schemars::visit::visit_schema_object(self, schema);
        if let Some(one_of) = schema
            .subschemas
            .as_mut()
            .and_then(|subschemas| subschemas.one_of.as_mut())
        {
            let common_obj = schema
                .object
                .get_or_insert_with(|| Box::new(ObjectValidation::default()));
            for variant in one_of {
                if let Schema::Object(SchemaObject {
                    instance_type: variant_type,
                    object: Some(variant_obj),
                    metadata: variant_metadata,
                    ..
                }) = variant
                {
                    if let Some(variant_metadata) = variant_metadata {
                        // Move enum variant description from oneOf clause to its corresponding property
                        if let Some(description) = std::mem::take(&mut variant_metadata.description) {
                            if let Some(Schema::Object(variant_object)) =
                                only_item(variant_obj.properties.values_mut())
                            {
                                let metadata = variant_object
                                    .metadata
                                    .get_or_insert_with(|| Box::new(Metadata::default()));
                                metadata.description = Some(description);
                            }
                        }
                    }

                    // Move all properties
                    let variant_properties = std::mem::take(&mut variant_obj.properties);
                    for (property_name, property) in variant_properties {
                        match common_obj.properties.entry(property_name) {
                            Entry::Occupied(entry) => panic!(
                                "property {:?} is already defined in another enum variant",
                                entry.key()
                            ),
                            Entry::Vacant(entry) => {
                                entry.insert(property);
                            }
                        }
                    }

                    // Kubernetes doesn't allow variants to set additionalProperties
                    variant_obj.additional_properties = None;

                    // Try to merge metadata
                    match (&mut schema.instance_type, variant_type.take()) {
                        (_, None) => {}
                        (common_type @ None, variant_type) => {
                            *common_type = variant_type;
                        }
                        (Some(common_type), Some(variant_type)) => {
                            if *common_type != variant_type {
                                panic!(
                                    "variant defined type {:?}, conflicting with existing type {:?}",
                                    variant_type, common_type
                                );
                            }
                        }
                    }
                }
            }
        }
        // check for maps without with properties (i.e. flattnened maps)
        // and allow these to persist dynamically
        if let Some(object) = &mut schema.object {
            if !object.properties.is_empty()
                && object.additional_properties.as_deref() == Some(&Schema::Bool(true))
            {
                object.additional_properties = None;
                schema
                    .extensions
                    .insert("x-kubernetes-preserve-unknown-fields".into(), true.into());
            }
        }
    }
}

fn only_item<I: Iterator>(mut i: I) -> Option<I::Item> {
    let item = i.next()?;
    if i.next().is_some() {
        return None;
    }
    Some(item)
}
