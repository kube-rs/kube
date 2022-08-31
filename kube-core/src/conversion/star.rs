use std::{any::Any, marker::PhantomData};

use serde::{de::DeserializeOwned, Serialize};

use crate::Resource;

use super::Conversion;

/// StarConversion is opinionated easy-to-use Conversion implementation.
///
/// # Semantics
/// StarConversion assumes both unversioned and all versioned representations are logically equivalent, and each version can be
/// converted to and from unversioned representation without loss of data. If you cannot satisfy this requirement,
/// you may need to implement `Conversion` directly instead to minimize data loss.
///
/// It then implements `Conversion` contract by converting input object to the unversioned representation at first,
/// and then converting this representation to the desired version.
pub struct StarConversion {
    rays: Vec<Box<dyn ErasedStarRay + Send + Sync + 'static>>,
}

impl StarConversion {
    /// Creates new builder.
    pub fn builder<U>() -> StarConversionBuilder<U> {
        StarConversionBuilder {
            marker: PhantomData,
            rays: Vec::new(),
        }
    }

    fn into_ur(&self, object: serde_json::Value) -> Result<Box<dyn Any>, String> {
        let api_version = object
            .get("apiVersion")
            .ok_or_else(|| ".apiVersion is missing".to_string())?;
        let api_version = api_version
            .as_str()
            .ok_or_else(|| ".apiVersion is not string".to_string())?;
        for ray in &self.rays {
            if ray.api_version() != api_version {
                continue;
            }
            return ray.into_ur(object);
        }
        Err("current apiVersion is unknown".to_string())
    }

    fn from_ur(&self, unversioned: Box<dyn Any>, api_version: &str) -> Result<serde_json::Value, String> {
        for ray in &self.rays {
            if ray.api_version() != api_version {
                continue;
            }
            return ray.from_ur(unversioned);
        }
        Err("desired apiVersion is unknown".to_string())
    }
}

impl Conversion for StarConversion {
    fn convert(
        &self,
        object: serde_json::Value,
        desired_api_version: &str,
    ) -> Result<serde_json::Value, String> {
        let kind = object.get("kind").unwrap_or(&serde_json::Value::Null).clone();
        let ur = self.into_ur(object)?;
        let mut converted = self.from_ur(ur, desired_api_version);

        if let Some(obj) = converted.as_mut().ok().and_then(|val| val.as_object_mut()) {
            obj.insert("apiVersion".to_string(), desired_api_version.into());
            obj.insert("kind".to_string(), kind);
        }

        converted
    }
}

/// Simple builder for the `StarConversion`.
/// `I` is type of the IR.
pub struct StarConversionBuilder<U> {
    marker: PhantomData<fn(U) -> U>,
    rays: Vec<Box<dyn ErasedStarRay + Send + Sync + 'static>>,
}

impl<U: 'static> StarConversionBuilder<U> {
    /// Registers new ray of the star.
    /// # Panics
    /// This method panics if another ray was added for the same api version.
    pub fn add_ray<V: Resource<DynamicType = ()>, R: StarRay<Unversioned = U, Versioned = V>>(
        self,
        ray: R,
    ) -> Self {
        self.add_ray_with_version(ray, V::api_version(&()).as_ref())
    }

    /// Registers new ray of the star. Unlike `add_ray`, this method does not deduce api version and
    /// takes it as a parameter.
    /// # Panics
    /// This method panics if another ray was added for the same api version.
    pub fn add_ray_with_version<V, R: StarRay<Unversioned = U, Versioned = V>>(
        mut self,
        ray: R,
        version: &str,
    ) -> Self {
        for other in &self.rays {
            if other.api_version() == version {
                panic!("Ray for the api version {} was already registered", version);
            }
        }

        let imp = ErasedStarRayImpl(ray, version.to_string());
        self.rays.push(Box::new(imp));
        self
    }

    /// Finalizes construction and returns `StarConversion` instance
    pub fn build(self) -> StarConversion {
        StarConversion { rays: self.rays }
    }
}

trait ErasedStarRay {
    fn into_ur(&self, versioned: serde_json::Value) -> Result<Box<dyn Any>, String>;
    fn from_ur(&self, unversioned: Box<dyn Any>) -> Result<serde_json::Value, String>;
    fn api_version(&self) -> &str;
}

struct ErasedStarRayImpl<T>(T, String);

impl<T: StarRay> ErasedStarRay for ErasedStarRayImpl<T> {
    fn into_ur(&self, versioned: serde_json::Value) -> Result<Box<dyn Any>, String> {
        let versioned =
            serde_json::from_value(versioned).map_err(|err| format!("Failed to parse object: {}", err))?;
        let unversioned = self.0.into_unversioned(versioned)?;
        Ok(Box::new(unversioned))
    }

    fn from_ur(&self, unversioned: Box<dyn Any>) -> Result<serde_json::Value, String> {
        // StarConversionBuilder enforces at compile-time that downcast will work.
        let unversioned = unversioned.downcast().expect("invalid input");
        let versioned = self.0.from_unversioned(*unversioned)?;
        serde_json::to_value(versioned).map_err(|err| format!("Failed to serialize object: {}", err))
    }

    fn api_version(&self) -> &str {
        &self.1
    }
}

/// Helper trait for the `StarConversion`.
/// # Errors
/// While the signature allows returning errors, it is discouraged.
/// Ideally, conversion should always succeed.
pub trait StarRay: Send + Sync + 'static {
    /// Type of the internal representation.
    type Unversioned: 'static;
    /// Version that can be converted to/from IR.
    type Versioned: Serialize + DeserializeOwned;
    /// Converts versioned resource into unversioned representation.
    fn into_unversioned(&self, versioned: Self::Versioned) -> Result<Self::Unversioned, String>;
    /// Converts unversioned representation into versioned resource. Note that you don't have
    /// to return correct `kind` and `apiVersion` - they will be written by the StarConverter.
    fn from_unversioned(&self, unversioned: Self::Unversioned) -> Result<Self::Versioned, String>;
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use crate::{conversion::Conversion, TypeMeta};

    use super::{StarConversion, StarRay};

    #[derive(Serialize, Deserialize)]
    struct V1 {
        #[serde(flatten)]
        types: Option<TypeMeta>,
        field_v1: i32,
    }
    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    struct V2 {
        #[serde(flatten)]
        types: Option<TypeMeta>,
        field_v2: i32,
    }

    struct Unversioned {
        field: i32,
    }

    struct Ray1;
    impl StarRay for Ray1 {
        type Unversioned = Unversioned;
        type Versioned = V1;

        fn into_unversioned(&self, versioned: Self::Versioned) -> Result<Self::Unversioned, String> {
            return Ok(Unversioned {
                field: versioned.field_v1,
            });
        }

        fn from_unversioned(&self, unversioned: Self::Unversioned) -> Result<Self::Versioned, String> {
            return Ok(V1 {
                field_v1: unversioned.field,
                types: None,
            });
        }
    }

    struct Ray2;
    impl StarRay for Ray2 {
        type Unversioned = Unversioned;
        type Versioned = V2;

        fn into_unversioned(&self, versioned: Self::Versioned) -> Result<Self::Unversioned, String> {
            return Ok(Unversioned {
                field: versioned.field_v2,
            });
        }

        fn from_unversioned(&self, unversioned: Self::Unversioned) -> Result<Self::Versioned, String> {
            return Ok(V2 {
                field_v2: unversioned.field,
                types: None,
            });
        }
    }

    #[test]
    fn test_star_conversion_works() {
        let converter = StarConversion::builder()
            .add_ray_with_version(Ray1, "foo/v1")
            .add_ray_with_version(Ray2, "foo/v2")
            .build();
        let value_v1 = V1 {
            field_v1: 5,
            types: Some(TypeMeta {
                api_version: "foo/v1".to_string(),
                kind: "Foo".to_string(),
            }),
        };
        let value_v1 = serde_json::to_value(value_v1).unwrap();
        let value_v2 = converter.convert(value_v1, "foo/v2").unwrap();
        let value_v2: V2 = serde_json::from_value(value_v2).unwrap();
        assert_eq!(value_v2, V2 {
            types: Some(TypeMeta {
                api_version: "foo/v2".to_string(),
                kind: "Foo".to_string()
            }),
            field_v2: 5
        });
    }

    #[test]
    #[should_panic]
    fn test_star_conversion_panics_when_versions_are_duplicated() {
        StarConversion::builder()
            .add_ray_with_version(Ray1, "foo/v1")
            .add_ray_with_version(Ray2, "foo/v1");
    }
}
