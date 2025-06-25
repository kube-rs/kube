//! Utilities for managing [`CustomResourceDefinition`] schemas
//!
//! [`CustomResourceDefinition`]: `k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition`

// Used in docs
#[allow(unused_imports)] use schemars::generate::SchemaSettings;

use schemars::{transform::Transform, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::{btree_map::Entry, BTreeMap, BTreeSet}, ops::Deref};

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

/// A JSON Schema.
#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum Schema {
    /// A trivial boolean JSON Schema.
    ///
    /// The schema `true` matches everything (always passes validation), whereas the schema `false`
    /// matches nothing (always fails validation).
    Bool(bool),
    /// A JSON Schema object.
    Object(SchemaObject),
}

impl Schema {
    /// Creates a new `$ref` schema.
    ///
    /// The given reference string should be a URI reference. This will usually be a JSON Pointer
    /// in [URI Fragment representation](https://tools.ietf.org/html/rfc6901#section-6).
    pub fn new_ref(reference: String) -> Self {
        SchemaObject::new_ref(reference).into()
    }

    /// Returns `true` if `self` is a `$ref` schema.
    ///
    /// If `self` is a [`SchemaObject`] with `Some` [`reference`](struct.SchemaObject.html#structfield.reference) set, this returns `true`.
    /// Otherwise, returns `false`.
    pub fn is_ref(&self) -> bool {
        match self {
            Schema::Object(o) => o.is_ref(),
            _ => false,
        }
    }

    /// Converts the given schema (if it is a boolean schema) into an equivalent schema object.
    ///
    /// If the given schema is already a schema object, this has no effect.
    ///
    /// # Example
    /// ```
    /// use kube::core::schema::{Schema, SchemaObject};
    ///
    /// let bool_schema = Schema::Bool(true);
    ///
    /// assert_eq!(bool_schema.into_object(), SchemaObject::default());
    /// ```
    pub fn into_object(self) -> SchemaObject {
        match self {
            Schema::Object(o) => o,
            Schema::Bool(true) => SchemaObject::default(),
            Schema::Bool(false) => SchemaObject {
                subschemas: Some(Box::new(SubschemaValidation {
                    not: Some(Schema::Object(Default::default()).into()),
                    ..Default::default()
                })),
                ..Default::default()
            },
        }
    }
}

impl From<SchemaObject> for Schema {
    fn from(o: SchemaObject) -> Self {
        Schema::Object(o)
    }
}

impl From<bool> for Schema {
    fn from(b: bool) -> Self {
        Schema::Bool(b)
    }
}


/// A JSON Schema object.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct SchemaObject {
    /// Properties which annotate the [`SchemaObject`] which typically have no effect when an object is being validated against the schema.
    #[serde(flatten, deserialize_with = "skip_if_default")]
    pub metadata: Option<Box<Metadata>>,
    /// The `type` keyword.
    ///
    /// See [JSON Schema Validation 6.1.1. "type"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.1.1)
    /// and [JSON Schema 4.2.1. Instance Data Model](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-4.2.1).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub instance_type: Option<SingleOrVec<InstanceType>>,
    /// The `format` keyword.
    ///
    /// See [JSON Schema Validation 7. A Vocabulary for Semantic Content With "format"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-7).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// The `enum` keyword.
    ///
    /// See [JSON Schema Validation 6.1.2. "enum"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.1.2)
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<Value>>,
    /// The `const` keyword.
    ///
    /// See [JSON Schema Validation 6.1.3. "const"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.1.3)
    #[serde(
        rename = "const",
        skip_serializing_if = "Option::is_none",
        deserialize_with = "allow_null"
    )]
    pub const_value: Option<Value>,
    /// Properties of the [`SchemaObject`] which define validation assertions in terms of other schemas.
    #[serde(flatten, deserialize_with = "skip_if_default")]
    pub subschemas: Option<Box<SubschemaValidation>>,
    /// Properties of the [`SchemaObject`] which define validation assertions for numbers.
    #[serde(flatten, deserialize_with = "skip_if_default")]
    pub number: Option<Box<NumberValidation>>,
    /// Properties of the [`SchemaObject`] which define validation assertions for strings.
    #[serde(flatten, deserialize_with = "skip_if_default")]
    pub string: Option<Box<StringValidation>>,
    /// Properties of the [`SchemaObject`] which define validation assertions for arrays.
    #[serde(flatten, deserialize_with = "skip_if_default")]
    pub array: Option<Box<ArrayValidation>>,
    /// Properties of the [`SchemaObject`] which define validation assertions for objects.
    #[serde(flatten, deserialize_with = "skip_if_default")]
    pub object: Option<Box<ObjectValidation>>,
    /// The `$ref` keyword.
    ///
    /// See [JSON Schema 8.2.4.1. Direct References with "$ref"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-8.2.4.1).
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Arbitrary extra properties which are not part of the JSON Schema specification, or which `schemars` does not support.
    #[serde(flatten)]
    pub extensions: BTreeMap<String, Value>,
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

macro_rules! get_or_insert_default_fn {
    ($name:ident, $ret:ty) => {
        get_or_insert_default_fn!(
            concat!(
                "Returns a mutable reference to this schema's [`",
                stringify!($ret),
                "`](#structfield.",
                stringify!($name),
                "), creating it if it was `None`."
            ),
            $name,
            $ret
        );
    };
    ($doc:expr, $name:ident, $ret:ty) => {
        #[doc = $doc]
        pub fn $name(&mut self) -> &mut $ret {
            self.$name.get_or_insert_with(Default::default)
        }
    };
}

impl SchemaObject {
    /// Creates a new `$ref` schema.
    ///
    /// The given reference string should be a URI reference. This will usually be a JSON Pointer
    /// in [URI Fragment representation](https://tools.ietf.org/html/rfc6901#section-6).
    pub fn new_ref(reference: String) -> Self {
        SchemaObject {
            reference: Some(reference),
            ..Default::default()
        }
    }

    /// Returns `true` if `self` is a `$ref` schema.
    ///
    /// If `self` has `Some` [`reference`](struct.SchemaObject.html#structfield.reference) set, this returns `true`.
    /// Otherwise, returns `false`.
    pub fn is_ref(&self) -> bool {
        self.reference.is_some()
    }

    /// Returns `true` if `self` accepts values of the given type, according to the [`instance_type`](struct.SchemaObject.html#structfield.instance_type) field.
    ///
    /// This is a basic check that always returns `true` if no `instance_type` is specified on the schema,
    /// and does not check any subschemas. Because of this, both `{}` and  `{"not": {}}` accept any type according
    /// to this method.
    pub fn has_type(&self, ty: InstanceType) -> bool {
        self.instance_type
            .as_ref()
            .map_or(true, |x| x.contains(&ty))
    }

    get_or_insert_default_fn!(metadata, Metadata);
    get_or_insert_default_fn!(subschemas, SubschemaValidation);
    get_or_insert_default_fn!(number, NumberValidation);
    get_or_insert_default_fn!(string, StringValidation);
    get_or_insert_default_fn!(array, ArrayValidation);
    get_or_insert_default_fn!(object, ObjectValidation);
}

impl From<Schema> for SchemaObject {
    fn from(schema: Schema) -> Self {
        schema.into_object()
    }
}

/// Properties which annotate a [`SchemaObject`] which typically have no effect when an object is being validated against the schema.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct Metadata {
    /// The `$id` keyword.
    ///
    /// See [JSON Schema 8.2.2. The "$id" Keyword](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-8.2.2).
    #[serde(rename = "$id", skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The `title` keyword.
    ///
    /// See [JSON Schema Validation 9.1. "title" and "description"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-9.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The `description` keyword.
    ///
    /// See [JSON Schema Validation 9.1. "title" and "description"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-9.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The `default` keyword.
    ///
    /// See [JSON Schema Validation 9.2. "default"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-9.2).
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "allow_null"
    )]
    pub default: Option<Value>,
    /// The `deprecated` keyword.
    ///
    /// See [JSON Schema Validation 9.3. "deprecated"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-9.3).
    #[serde(skip_serializing_if = "is_false")]
    pub deprecated: bool,
    /// The `readOnly` keyword.
    ///
    /// See [JSON Schema Validation 9.4. "readOnly" and "writeOnly"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-9.4).
    #[serde(skip_serializing_if = "is_false")]
    pub read_only: bool,
    /// The `writeOnly` keyword.
    ///
    /// See [JSON Schema Validation 9.4. "readOnly" and "writeOnly"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-9.4).
    #[serde(skip_serializing_if = "is_false")]
    pub write_only: bool,
    /// The `examples` keyword.
    ///
    /// See [JSON Schema Validation 9.5. "examples"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-9.5).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<Value>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !b
}

/// Properties of a [`SchemaObject`] which define validation assertions in terms of other schemas.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct SubschemaValidation {
    /// The `allOf` keyword.
    ///
    /// See [JSON Schema 9.2.1.1. "allOf"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.2.1.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<Schema>>,
    /// The `anyOf` keyword.
    ///
    /// See [JSON Schema 9.2.1.2. "anyOf"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.2.1.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<Schema>>,
    /// The `oneOf` keyword.
    ///
    /// See [JSON Schema 9.2.1.3. "oneOf"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.2.1.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_of: Option<Vec<Schema>>,
    /// The `not` keyword.
    ///
    /// See [JSON Schema 9.2.1.4. "not"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.2.1.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not: Option<Box<Schema>>,
    /// The `if` keyword.
    ///
    /// See [JSON Schema 9.2.2.1. "if"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.2.2.1).
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub if_schema: Option<Box<Schema>>,
    /// The `then` keyword.
    ///
    /// See [JSON Schema 9.2.2.2. "then"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.2.2.2).
    #[serde(rename = "then", skip_serializing_if = "Option::is_none")]
    pub then_schema: Option<Box<Schema>>,
    /// The `else` keyword.
    ///
    /// See [JSON Schema 9.2.2.3. "else"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.2.2.3).
    #[serde(rename = "else", skip_serializing_if = "Option::is_none")]
    pub else_schema: Option<Box<Schema>>,
}

/// Properties of a [`SchemaObject`] which define validation assertions for numbers.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct NumberValidation {
    /// The `multipleOf` keyword.
    ///
    /// See [JSON Schema Validation 6.2.1. "multipleOf"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.2.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_of: Option<f64>,
    /// The `maximum` keyword.
    ///
    /// See [JSON Schema Validation 6.2.2. "maximum"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.2.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    /// The `exclusiveMaximum` keyword.
    ///
    /// See [JSON Schema Validation 6.2.3. "exclusiveMaximum"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.2.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_maximum: Option<f64>,
    /// The `minimum` keyword.
    ///
    /// See [JSON Schema Validation 6.2.4. "minimum"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.2.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    /// The `exclusiveMinimum` keyword.
    ///
    /// See [JSON Schema Validation 6.2.5. "exclusiveMinimum"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.2.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclusive_minimum: Option<f64>,
}

/// Properties of a [`SchemaObject`] which define validation assertions for strings.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct StringValidation {
    /// The `maxLength` keyword.
    ///
    /// See [JSON Schema Validation 6.3.1. "maxLength"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.3.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    /// The `minLength` keyword.
    ///
    /// See [JSON Schema Validation 6.3.2. "minLength"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.3.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    /// The `pattern` keyword.
    ///
    /// See [JSON Schema Validation 6.3.3. "pattern"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.3.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

/// Properties of a [`SchemaObject`] which define validation assertions for arrays.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct ArrayValidation {
    /// The `items` keyword.
    ///
    /// See [JSON Schema 9.3.1.1. "items"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.1.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<SingleOrVec<Schema>>,
    /// The `additionalItems` keyword.
    ///
    /// See [JSON Schema 9.3.1.2. "additionalItems"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.1.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_items: Option<Box<Schema>>,
    /// The `maxItems` keyword.
    ///
    /// See [JSON Schema Validation 6.4.1. "maxItems"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.4.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_items: Option<u32>,
    /// The `minItems` keyword.
    ///
    /// See [JSON Schema Validation 6.4.2. "minItems"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.4.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_items: Option<u32>,
    /// The `uniqueItems` keyword.
    ///
    /// See [JSON Schema Validation 6.4.3. "uniqueItems"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.4.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_items: Option<bool>,
    /// The `contains` keyword.
    ///
    /// See [JSON Schema 9.3.1.4. "contains"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.1.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contains: Option<Box<Schema>>,
}

/// Properties of a [`SchemaObject`] which define validation assertions for objects.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct ObjectValidation {
    /// The `maxProperties` keyword.
    ///
    /// See [JSON Schema Validation 6.5.1. "maxProperties"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.5.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_properties: Option<u32>,
    /// The `minProperties` keyword.
    ///
    /// See [JSON Schema Validation 6.5.2. "minProperties"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.5.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_properties: Option<u32>,
    /// The `required` keyword.
    ///
    /// See [JSON Schema Validation 6.5.3. "required"](https://tools.ietf.org/html/draft-handrews-json-schema-validation-02#section-6.5.3).
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub required: BTreeSet<String>,
    /// The `properties` keyword.
    ///
    /// See [JSON Schema 9.3.2.1. "properties"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.1).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, Schema>,
    /// The `patternProperties` keyword.
    ///
    /// See [JSON Schema 9.3.2.2. "patternProperties"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.2).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub pattern_properties: BTreeMap<String, Schema>,
    /// The `additionalProperties` keyword.
    ///
    /// See [JSON Schema 9.3.2.3. "additionalProperties"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<Box<Schema>>,
    /// The `propertyNames` keyword.
    ///
    /// See [JSON Schema 9.3.2.5. "propertyNames"](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-9.3.2.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_names: Option<Box<Schema>>,
}

/// The possible types of values in JSON Schema documents.
///
/// See [JSON Schema 4.2.1. Instance Data Model](https://tools.ietf.org/html/draft-handrews-json-schema-02#section-4.2.1).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum InstanceType {
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
pub enum SingleOrVec<T> {
    /// Represents a single item.
    Single(Box<T>),
    /// Represents a vector of items.
    Vec(Vec<T>),
}

impl<T> From<T> for SingleOrVec<T> {
    fn from(single: T) -> Self {
        SingleOrVec::Single(Box::new(single))
    }
}

impl<T> From<Vec<T>> for SingleOrVec<T> {
    fn from(vec: Vec<T>) -> Self {
        SingleOrVec::Vec(vec)
    }
}

impl<T: PartialEq> SingleOrVec<T> {
    /// Returns `true` if `self` is either a `Single` equal to `x`, or a `Vec` containing `x`.
    ///
    /// # Examples
    ///
    /// ```
    /// use kube::core::schema::SingleOrVec;
    ///
    /// let s = SingleOrVec::from(10);
    /// assert!(s.contains(&10));
    /// assert!(!s.contains(&20));
    ///
    /// let v = SingleOrVec::from(vec![10, 20]);
    /// assert!(v.contains(&10));
    /// assert!(v.contains(&20));
    /// assert!(!v.contains(&30));
    /// ```
    pub fn contains(&self, x: &T) -> bool {
        match self {
            SingleOrVec::Single(s) => s.deref() == x,
            SingleOrVec::Vec(v) => v.contains(x),
        }
    }
}

impl Transform for StructuralSchemaRewriter {
    fn transform(&mut self, transform_schema: &mut schemars::Schema) {
        schemars::transform::transform_subschemas(self, transform_schema);

        let mut schema: SchemaObject = match serde_json::from_value(transform_schema.clone().to_value()).ok(){
            Some(schema) => schema,
            None => return,
        };

        if let Some(subschemas) = &mut schema.subschemas {
            if let Some(one_of) = subschemas.one_of.as_mut() {
                // Tagged enums are serialized using `one_of`
                hoist_subschema_properties(one_of, &mut schema.object, &mut schema.instance_type);

                // "Plain" enums are serialized using `one_of` if they have doc tags
                hoist_subschema_enum_values(one_of, &mut schema.enum_values, &mut schema.instance_type);

                if one_of.is_empty() {
                    subschemas.one_of = None;
                }
            }

            if let Some(any_of) = &mut subschemas.any_of {
                // Untagged enums are serialized using `any_of`
                hoist_subschema_properties(any_of, &mut schema.object, &mut schema.instance_type);
            }
        }

        // check for maps without with properties (i.e. flattened maps)
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

        // As of version 1.30 Kubernetes does not support setting `uniqueItems` to `true`,
        // so we need to remove this fields.
        // Users can still set `x-kubernetes-list-type=set` in case they want the apiserver
        // to do validation, but we can't make an assumption about the Set contents here.
        // See https://kubernetes.io/docs/reference/using-api/server-side-apply/ for details.
        if let Some(array) = &mut schema.array {
            array.unique_items = None;
        }

        if let Some(schema) = serde_json::to_value(schema).ok() {
            if let Some(transformed) = serde_json::from_value(schema).ok() {
                *transform_schema = transformed;
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

            if let Some(variant_metadata) = variant_metadata {
                // Move enum variant description from oneOf clause to its corresponding property
                if let Some(description) = std::mem::take(&mut variant_metadata.description) {
                    if let Some(Schema::Object(variant_object)) =
                        only_item(variant_obj.properties.values_mut())
                    {
                        let metadata = variant_object
                            .metadata
                            .get_or_insert_with(Box::<Metadata>::default);
                        metadata.description = Some(description);
                    }
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
                            panic!("Property {:?} has the schema {:?} but was already defined as {:?} in another subschema. The schemas for a property used in multiple subschemas must be identical",
                            entry.key(),
                            &property,
                            entry.get());
                        }
                    }
                }
            }

            // Kubernetes doesn't allow variants to set additionalProperties
            variant_obj.additional_properties = None;

            merge_metadata(instance_type, variant_type.take());
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
