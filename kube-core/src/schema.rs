//! Utilities for managing [`CustomResourceDefinition`] schemas
//!
//! [`CustomResourceDefinition`]: `k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition`

use std::collections::btree_map::Entry;

// Used in docs
#[allow(unused_imports)] use schemars::gen::SchemaSettings;

use schemars::schema::Metadata;
use schemars::{
    schema::{ObjectValidation, Schema, SchemaObject},
    visit::Visitor,
};

/// schemars [`Visitor`] that rewrites a [`Schema`] to conform to Kubernetes' "structural schema" rules
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
                // If we provide descriptions for enum variants it will produce invalid CRDs like in the following example
                //
                // #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
                // #[serde(rename_all = "camelCase")]
                // enum ComplexEnum {
                //     /// First variant with an int
                //     VariantOne { int: i32 },
                //     /// Second variant which doesn't has an attribute
                //     VariantTwo {}
                // }
                //
                // This produces the following invalid CRD (snippet)
                // "complexEnum": {
                //     "type": "object",
                //     "properties": {
                //         "variantOne": {
                //             "type": "object",
                //             "properties": {
                //                 "int": {
                //                     "type": "integer",
                //                     "format": "int32"
                //                 }
                //             },
                //             "required": ["int"],
                //         },
                //         "VariantTwo": {
                //             "type": "object",
                //         }
                //     },
                //     "oneOf": [
                //     {
                //         "required": ["variantOne"],
                //         "description": "First variant with an int"
                //     },
                //     {
                //         "required": ["variantTwo"],
                //         "description": "Second variant which doesn't has an attribute"
                //     }
                //     ],
                //     "description": "This is a complex enum"
                // }
                //
                // The correct solution requires to move the description from the oneOf section to to the complexEnum.properties
                // The corrected solution looks like follows
                // "complexEnum": {
                //     "type": "object",
                //     "properties": {
                //         "variantOne": {
                //             "type": "object",
                //             "properties": {
                //                 "int": {
                //                     "type": "integer",
                //                     "format": "int32"
                //                 }
                //             },
                //             "required": ["int"],
                //             "description": "First variant with an int"
                //         },
                //         "variantTwo": {
                //             "type": "object",
                //             "description": "Second variant which doesn't has an attribute"
                //         }
                //     },
                //     "oneOf": [
                //     {
                //         "required": ["variantOne"],
                //     },
                //     {
                //         "required": ["variantTwo"],
                //     }
                //     ],
                //     "description": "This is a complex enum"
                // }
                //
                // The following lines move the descriptions to the correct places
                if let Schema::Object(SchemaObject {
                    metadata: Some(variant_metadata),
                    object: Some(variant_obj),
                    ..
                }) = variant
                {
                    if variant_obj.properties.len() == 1 {
                        if let Some(description) = std::mem::take(&mut variant_metadata.description) {
                            if let Some(Schema::Object(variant_object)) =
                                variant_obj.properties.values_mut().next()
                            {
                                let metadata = variant_object.metadata.get_or_insert_with(|| Box::new(Metadata::default()));
                                metadata.description = Some(description.to_string());
                            }
                        }
                    }
                }

                if let Schema::Object(SchemaObject {
                    instance_type: variant_type,
                    object: Some(variant_obj),
                    ..
                }) = variant
                {
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
    }
}
