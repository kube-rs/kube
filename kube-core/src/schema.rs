//! Utilities for managing [`CustomResourceDefinition`] schemas
//!
//! [`CustomResourceDefinition`]: `k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition`

// Used in docs
#[allow(unused_imports)] use schemars::generate::SchemaSettings;

use schemars::{
    JsonSchema,
    transform::{Transform, transform_subschemas},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

/// schemars [`Visitor`] that rewrites a [`Schema`] to conform to Kubernetes' "structural schema" rules
///
/// The following two transformations are applied
///  * Rewrite enums from `oneOf` to `object`s with multiple variants ([schemars#84](https://github.com/GREsau/schemars/issues/84))
///  * Rewrite untagged enums from `anyOf` to `object`s with multiple variants ([kube#1028](https://github.com/kube-rs/kube/pull/1028))
///  * Rewrite `additionalProperties` from `#[serde(flatten)]` to `x-kubernetes-preserve-unknown-fields` ([kube#844](https://github.com/kube-rs/kube/issues/844))
///
/// This is used automatically by `kube::derive`'s `#[derive(CustomResource)]`,
/// but it can also be used manually with [`SchemaSettings::with_transform`].
///
/// # Panics
///
/// The [`Visitor`] functions may panic if the transform could not be applied. For example,
/// there must not be any overlapping properties between `oneOf` branches.
#[derive(Debug, Clone)]
pub struct StructuralSchemaRewriter;

/// Recursively restructures JSON Schema objects so that the Option<Enum> object
/// is returned per k8s CRD schema expectations.
///
/// In kube 2.x the schema output behavior for `Option<Enum>` types changed.
///
/// Previously given an enum like:
///
/// ```rust
/// enum LogLevel {
///     Debug,
///     Info,
///     Error,
/// }
/// ```
///
/// The following would be generated for Optional<LogLevel>:
///
/// ```json
/// { "enum": ["Debug", "Info", "Error"], "type": "string", "nullable": true }
/// ```
///
/// Now, schemars generates `anyOf` for `Option<LogLevel>` like:
///
/// ```json
/// {
///   "anyOf": [
///     { "enum": ["Debug", "Info", "Error"], "type": "string" },
///     { "enum": [null], "nullable": true }
///   ]
/// }
/// ```
///
/// This transform implementation prevents this specific case from happening.
#[derive(Debug, Clone, Default)]
pub struct OptionalEnum;

/// Recursively restructures JSON Schema objects so that the `Option<T>` object
/// where `T` uses `x-kubernetes-int-or-string` is returned per k8s CRD schema expectations.
///
/// In kube 2.x with k8s-openapi 0.26.x, the schema output behavior for `Option<Quantity>`
/// and similar `x-kubernetes-int-or-string` types changed.
///
/// Previously given an optional Quantity field:
///
/// ```json
/// { "nullable": true, "type": "string" }
/// ```
///
/// Now, schemars generates `anyOf` for `Option<Quantity>` like:
///
/// ```json
/// {
///   "anyOf": [
///     { "x-kubernetes-int-or-string": true },
///     { "enum": [null], "nullable": true }
///   ]
/// }
/// ```
///
/// This transform converts it to:
///
/// ```json
/// { "x-kubernetes-int-or-string": true, "nullable": true }
/// ```
#[derive(Debug, Clone, Default)]
pub struct OptionalIntOrString;

/// A JSON Schema.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[serde(untagged)]
enum Schema {
    /// A trivial boolean JSON Schema.
    ///
    /// The schema `true` matches everything (always passes validation), whereas the schema `false`
    /// matches nothing (always fails validation).
    Bool(bool),
    /// A JSON Schema object.
    Object(SchemaObject),
}

/// A JSON Schema object.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
struct SchemaObject {
    /// Properties which annotate the [`SchemaObject`] which typically have no effect when an object is being validated against the schema.
    #[serde(flatten, deserialize_with = "skip_if_default")]
    metadata: Option<Box<Metadata>>,
    /// The `type` keyword.
    ///
    /// See [JSON Schema Validation 6.1.1. "type"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.1.1)
    /// and [JSON Schema 4.2.1. Instance Data Model](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-4.2.1).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    instance_type: Option<SingleOrVec<InstanceType>>,
    /// The `format` keyword.
    ///
    /// See [JSON Schema Validation 7. A Vocabulary for Semantic Content With "format"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-7).
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    /// The `enum` keyword.
    ///
    /// See [JSON Schema Validation 6.1.2. "enum"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.1.2)
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    enum_values: Option<Vec<Value>>,
    /// Properties of the [`SchemaObject`] which define validation assertions in terms of other schemas.
    #[serde(flatten, deserialize_with = "skip_if_default")]
    subschemas: Option<Box<SubschemaValidation>>,
    /// Properties of the [`SchemaObject`] which define validation assertions for arrays.
    #[serde(flatten, deserialize_with = "skip_if_default")]
    array: Option<Box<ArrayValidation>>,
    /// Properties of the [`SchemaObject`] which define validation assertions for objects.
    #[serde(flatten, deserialize_with = "skip_if_default")]
    object: Option<Box<ObjectValidation>>,
    /// Arbitrary extra properties which are not part of the JSON Schema specification, or which `schemars` does not support.
    #[serde(flatten)]
    extensions: BTreeMap<String, Value>,
    /// Arbitrary data.
    #[serde(flatten)]
    other: Value,
}

// Deserializing "null" to `Option<Value>` directly results in `None`,
// this function instead makes it deserialize to `Some(Value::Null)`.
fn allow_null<'de, D>(de: D) -> Result<Option<Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Value::deserialize(de).map(Option::Some)
}

fn skip_if_default<'de, D, T>(deserializer: D) -> Result<Option<Box<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default + PartialEq,
{
    let value = T::deserialize(deserializer)?;
    if value == T::default() {
        Ok(None)
    } else {
        Ok(Some(Box::new(value)))
    }
}

/// Properties which annotate a [`SchemaObject`] which typically have no effect when an object is being validated against the schema.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
struct Metadata {
    /// The `description` keyword.
    ///
    /// See [JSON Schema Validation 9.1. "title" and "description"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-9.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    /// The `default` keyword.
    ///
    /// See [JSON Schema Validation 9.2. "default"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-9.2).
    #[serde(skip_serializing_if = "Option::is_none", deserialize_with = "allow_null")]
    default: Option<Value>,
}

/// Properties of a [`SchemaObject`] which define validation assertions in terms of other schemas.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
struct SubschemaValidation {
    /// The `anyOf` keyword.
    ///
    /// See [JSON Schema 9.2.1.2. "anyOf"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.2.1.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    any_of: Option<Vec<Schema>>,
    /// The `oneOf` keyword.
    ///
    /// See [JSON Schema 9.2.1.3. "oneOf"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.2.1.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    one_of: Option<Vec<Schema>>,
}

/// Properties of a [`SchemaObject`] which define validation assertions for arrays.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
struct ArrayValidation {
    /// The `items` keyword.
    ///
    /// See [JSON Schema 9.3.1.1. "items"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.1.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<SingleOrVec<Schema>>,
    /// The `additionalItems` keyword.
    ///
    /// See [JSON Schema 9.3.1.2. "additionalItems"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.1.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    additional_items: Option<Box<Schema>>,
    /// The `maxItems` keyword.
    ///
    /// See [JSON Schema Validation 6.4.1. "maxItems"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.4.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    max_items: Option<u32>,
    /// The `minItems` keyword.
    ///
    /// See [JSON Schema Validation 6.4.2. "minItems"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.4.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    min_items: Option<u32>,
    /// The `uniqueItems` keyword.
    ///
    /// See [JSON Schema Validation 6.4.3. "uniqueItems"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.4.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    unique_items: Option<bool>,
    /// The `contains` keyword.
    ///
    /// See [JSON Schema 9.3.1.4. "contains"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.1.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    contains: Option<Box<Schema>>,
}

/// Properties of a [`SchemaObject`] which define validation assertions for objects.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
struct ObjectValidation {
    /// The `maxProperties` keyword.
    ///
    /// See [JSON Schema Validation 6.5.1. "maxProperties"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.5.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    max_properties: Option<u32>,
    /// The `minProperties` keyword.
    ///
    /// See [JSON Schema Validation 6.5.2. "minProperties"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.5.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    min_properties: Option<u32>,
    /// The `required` keyword.
    ///
    /// See [JSON Schema Validation 6.5.3. "required"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.5.3).
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    required: BTreeSet<String>,
    /// The `properties` keyword.
    ///
    /// See [JSON Schema 9.3.2.1. "properties"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.1).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    properties: BTreeMap<String, Schema>,
    /// The `patternProperties` keyword.
    ///
    /// See [JSON Schema 9.3.2.2. "patternProperties"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.2).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pattern_properties: BTreeMap<String, Schema>,
    /// The `additionalProperties` keyword.
    ///
    /// See [JSON Schema 9.3.2.3. "additionalProperties"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    additional_properties: Option<Box<Schema>>,
    /// The `propertyNames` keyword.
    ///
    /// See [JSON Schema 9.3.2.5. "propertyNames"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    property_names: Option<Box<Schema>>,
}

/// The possible types of values in JSON Schema documents.
///
/// See [JSON Schema 4.2.1. Instance Data Model](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-4.2.1).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum InstanceType {
    /// Represents the JSON null type.
    Null,
    /// Represents the JSON boolean type.
    Boolean,
    /// Represents the JSON object type.
    Object,
    /// Represents the JSON array type.
    Array,
    /// Represents the JSON number type (floating point).
    Number,
    /// Represents the JSON string type.
    String,
    /// Represents the JSON integer type.
    Integer,
}

/// A type which can be serialized as a single item, or multiple items.
///
/// In some contexts, a `Single` may be semantically distinct from a `Vec` containing only item.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
#[serde(untagged)]
enum SingleOrVec<T> {
    /// Represents a single item.
    Single(Box<T>),
    /// Represents a vector of items.
    Vec(Vec<T>),
}

impl Transform for StructuralSchemaRewriter {
    fn transform(&mut self, transform_schema: &mut schemars::Schema) {
        schemars::transform::transform_subschemas(self, transform_schema);

        let mut schema: SchemaObject = match serde_json::from_value(transform_schema.clone().to_value()).ok()
        {
            Some(schema) => schema,
            None => return,
        };

        if let Some(subschemas) = &mut schema.subschemas {
            if let Some(one_of) = subschemas.one_of.as_mut() {
                // Tagged enums are serialized using `one_of`
                hoist_subschema_properties(one_of, &mut schema.object, &mut schema.instance_type, true);

                // "Plain" enums are serialized using `one_of` if they have doc tags
                hoist_subschema_enum_values(one_of, &mut schema.enum_values, &mut schema.instance_type);

                if one_of.is_empty() {
                    subschemas.one_of = None;
                }
            }

            if let Some(any_of) = &mut subschemas.any_of {
                // Untagged enums are serialized using `any_of`
                // Variant descriptions are not pushed into properties (because they are not for the field).
                hoist_subschema_properties(any_of, &mut schema.object, &mut schema.instance_type, false);
            }
        }

        if let Some(object) = &mut schema.object
            && !object.properties.is_empty()
        {
            // check for maps without with properties (i.e. flattened maps)
            // and allow these to persist dynamically
            match object.additional_properties.as_deref() {
                Some(&Schema::Bool(true)) => {
                    object.additional_properties = None;
                    schema
                        .extensions
                        .insert("x-kubernetes-preserve-unknown-fields".into(), true.into());
                }
                Some(&Schema::Bool(false)) => {
                    object.additional_properties = None;
                }
                _ => {}
            }
        }

        // As of version 1.30 Kubernetes does not support setting `uniqueItems` to `true`,
        // so we need to remove this fields.
        // Users can still set `x-kubernetes-list-type=set` in case they want the apiserver
        // to do validation, but we can't make an assumption about the Set contents here.
        // See https://kubernetes.io/docs/reference/using-api/server-side-apply/ for details.
        if let Some(array) = &mut schema.array {
            array.unique_items = None;
        }

        if let Ok(schema) = serde_json::to_value(schema)
            && let Ok(transformed) = serde_json::from_value(schema)
        {
            *transform_schema = transformed;
        }
    }
}

impl Transform for OptionalEnum {
    fn transform(&mut self, schema: &mut schemars::Schema) {
        transform_subschemas(self, schema);

        let Some(obj) = schema.as_object_mut() else {
            return;
        };

        let arr = obj
            .get("anyOf")
            .iter()
            .flat_map(|any_of| any_of.as_array())
            .last()
            .cloned()
            .unwrap_or_default();

        let [first, second] = arr.as_slice() else {
            return;
        };
        let (Some(first), Some(second)) = (first.as_object(), second.as_object()) else {
            return;
        };

        // Check if this is an Option<T> pattern:
        // anyOf with two elements where second is { "enum": [null], "nullable": true }
        if !first.contains_key("nullable")
            && second.get("enum") == Some(&json!([null]))
            && second.get("nullable") == Some(&json!(true))
        {
            // Remove anyOf and hoist first element's properties to root
            obj.remove("anyOf");
            obj.append(&mut first.clone());
            obj.insert("nullable".to_string(), Value::Bool(true));
        }
    }
}

impl Transform for OptionalIntOrString {
    fn transform(&mut self, schema: &mut schemars::Schema) {
        transform_subschemas(self, schema);

        let Some(obj) = schema.as_object_mut() else {
            return;
        };

        // Get required fields list
        let required: BTreeSet<String> = obj
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // Get mutable properties
        let Some(properties) = obj.get_mut("properties").and_then(|v| v.as_object_mut()) else {
            return;
        };

        // For each property that is NOT required and has x-kubernetes-int-or-string,
        // add nullable: true if not already present
        for (name, prop_schema) in properties.iter_mut() {
            if required.contains(name) {
                continue;
            }

            let Some(prop_obj) = prop_schema.as_object_mut() else {
                continue;
            };

            if prop_obj.get("x-kubernetes-int-or-string") == Some(&json!(true))
                && !prop_obj.contains_key("nullable")
            {
                prop_obj.insert("nullable".to_string(), Value::Bool(true));
            }
        }
    }
}

/// Bring all plain enum values up to the root schema,
/// since Kubernetes doesn't allow subschemas to define enum options.
///
/// (Enum here means a list of hard-coded values, not a tagged union.)
fn hoist_subschema_enum_values(
    subschemas: &mut Vec<Schema>,
    common_enum_values: &mut Option<Vec<serde_json::Value>>,
    instance_type: &mut Option<SingleOrVec<InstanceType>>,
) {
    subschemas.retain(|variant| {
        if let Schema::Object(SchemaObject {
            instance_type: variant_type,
            enum_values: Some(variant_enum_values),
            ..
        }) = variant
        {
            if let Some(variant_type) = variant_type {
                match instance_type {
                    None => *instance_type = Some(variant_type.clone()),
                    Some(tpe) => {
                        if tpe != variant_type {
                            panic!("Enum variant set {variant_enum_values:?} has type {variant_type:?} but was already defined as {instance_type:?}. The instance type must be equal for all subschema variants.")
                        }
                    }
                }
            }
            common_enum_values
                .get_or_insert_with(Vec::new)
                .extend(variant_enum_values.iter().cloned());
            false
        } else {
            true
        }
    })
}

/// Bring all property definitions from subschemas up to the root schema,
/// since Kubernetes doesn't allow subschemas to define properties.
fn hoist_subschema_properties(
    subschemas: &mut Vec<Schema>,
    common_obj: &mut Option<Box<ObjectValidation>>,
    instance_type: &mut Option<SingleOrVec<InstanceType>>,
    push_description_to_property: bool,
) {
    for variant in subschemas {
        if let Schema::Object(SchemaObject {
            instance_type: variant_type,
            object: Some(variant_obj),
            metadata: variant_metadata,
            ..
        }) = variant
        {
            let common_obj = common_obj.get_or_insert_with(Box::<ObjectValidation>::default);

            // Move enum variant description from oneOf clause to its corresponding property
            if let Some(variant_metadata) = variant_metadata
                && let Some(description) = std::mem::take(&mut variant_metadata.description)
                && let Some(Schema::Object(variant_object)) = only_item(variant_obj.properties.values_mut())
            {
                let metadata = variant_object
                    .metadata
                    .get_or_insert_with(Box::<Metadata>::default);
                if push_description_to_property {
                    metadata.description = Some(description);
                }
            }

            // Move all properties
            let variant_properties = std::mem::take(&mut variant_obj.properties);
            for (property_name, property) in variant_properties {
                match common_obj.properties.entry(property_name) {
                    Entry::Vacant(entry) => {
                        entry.insert(property);
                    }
                    Entry::Occupied(entry) => {
                        if &property != entry.get() {
                            panic!(
                                "Property {:?} has the schema {:?} but was already defined as {:?} in another subschema. The schemas for a property used in multiple subschemas must be identical",
                                entry.key(),
                                &property,
                                entry.get()
                            );
                        }
                    }
                }
            }

            // Kubernetes doesn't allow variants to set additionalProperties
            variant_obj.additional_properties = None;

            merge_metadata(instance_type, variant_type.take());
        }
        // Removes the type/description from oneOf and anyOf subschemas
        else if let Schema::Object(SchemaObject {
            metadata: variant_metadata,
            instance_type: variant_type,
            enum_values: None,
            subschemas: None,
            array: None,
            object: None,
            ..
        }) = variant
        {
            std::mem::take(&mut *variant_type);
            std::mem::take(&mut *variant_metadata);
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

fn merge_metadata(
    instance_type: &mut Option<SingleOrVec<InstanceType>>,
    variant_type: Option<SingleOrVec<InstanceType>>,
) {
    match (instance_type, variant_type) {
        (_, None) => {}
        (common_type @ None, variant_type) => {
            *common_type = variant_type;
        }
        (Some(common_type), Some(variant_type)) => {
            if *common_type != variant_type {
                panic!(
                    "variant defined type {variant_type:?}, conflicting with existing type {common_type:?}"
                );
            }
        }
    }
}
