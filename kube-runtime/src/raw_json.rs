//! Utilities for dynamic objects represented in raw JSON.

use std::{borrow::Cow, convert::Infallible, fmt, num::NonZeroU32, ops::Index};

use kube_client::discovery::ApiResource;
use serde_json::value::RawValue;

use crate::reflector;

/// The offsets of a string value within a JSON buffer.
#[derive(Clone, Copy)]
pub struct StrOffset {
    /// The starting offset.
    ///
    /// This value is always positive since a string must start with a `"` first.
    ///
    /// `buffer[start]` may be out-of-bounds if `start == end`.
    start: NonZeroU32,
    /// The exclusive ending offset.
    ///
    /// This value is always positive since a string must start with a `"` first.
    end: NonZeroU32,
}

impl StrOffset {
    /// Converts a substring slice into string offsets within the buffer.
    ///
    /// This function is purely arithmetic and O(1).
    #[allow(clippy::missing_panics_doc)] // cannot panic
    #[must_use]
    pub fn new(buffer: &str, substr: &str) -> Option<Self> {
        if substr.is_empty() {
            return Some(Self {
                start: NonZeroU32::new(1).expect("1 != 0"),
                end: NonZeroU32::new(1).expect("1 != 0"),
            });
        }

        let start = substr.as_ptr() as isize - buffer.as_ptr() as isize;
        let end = usize::try_from(start).ok()?.checked_add(substr.len())?;

        let start = NonZeroU32::new(u32::try_from(start).ok()?)?;
        let end = NonZeroU32::new(u32::try_from(end).ok()?)?;
        Some(Self { start, end })
    }
}

impl Index<StrOffset> for str {
    type Output = str;

    fn index(&self, index: StrOffset) -> &str {
        &self[index.start.get() as usize..index.end.get() as usize]
    }
}

/// A dynamic object represented in raw JSON that can be used in [`reflector`](crate::reflector).
///
/// Offsets of certain required fields are cached for efficient interaction with caches.
pub struct RawJson<X> {
    ref_fields: ObjectRefFields,
    /// Extra data to cache from the object.
    pub extra: X,
    /// The raw JSON data.
    pub buffer: Box<RawValue>,
}

struct ObjectRefFields {
    namespace: Option<StrOffset>,
    name: StrOffset,
    resource_version: StrOffset,
    uid: StrOffset,
}

/// Extra data to store within a [`RawJson`] object.
pub trait Extra: Sized + 'static {
    /// Extra data to parse from the raw root.
    type Root<'a>: Sized;
    /// Extra data to parse from the raw meta.
    type Meta<'a>: Sized;

    /// The type of error returned when [`new`] fails.
    type Error: fmt::Display;

    /// Constructs this type from the two parsed parts.
    ///
    /// # Errors
    /// Returns `Err` if the `root` or `meta` contains invalid data.
    fn new(buffer: &str, root: Self::Root<'_>, meta: Self::Meta<'_>) -> Result<Self, Self::Error>;
}

#[allow(clippy::missing_fields_in_debug)] // All fields are actually included
impl<X: fmt::Debug> fmt::Debug for RawJson<X> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawJson")
            .field("namespace", &self.namespace())
            .field("name", &self.name())
            .field("resource_version", &self.resource_version())
            .field("uid", &self.uid())
            .field("extra", &self.extra)
            .field("buffer", &&*self.buffer)
            .finish()
    }
}

impl<X> RawJson<X> {
    /// Namespace of this object.
    pub fn namespace(&self) -> Option<&str> {
        self.ref_fields.namespace.map(|offset| &self.buffer.get()[offset])
    }

    /// Name of this object.
    pub fn name(&self) -> &str {
        &self.buffer.get()[self.ref_fields.name]
    }

    /// Resource version of this object.
    pub fn resource_version(&self) -> &str {
        &self.buffer.get()[self.ref_fields.resource_version]
    }

    /// UID of this object.
    pub fn uid(&self) -> &str {
        &self.buffer.get()[self.ref_fields.uid]
    }
}

impl<X> serde::Serialize for RawJson<X> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.buffer.serialize(serializer)
    }
}

impl<'de, X: Extra> serde::Deserialize<'de> for RawJson<X>
where
    for<'de2> X::Meta<'de2>: serde::Deserialize<'de2>,
    for<'de2> X::Root<'de2>: serde::Deserialize<'de2>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Obj<'a, MetaX, RootX> {
            #[serde(borrow)]
            metadata: Metadata<'a, MetaX>,

            #[serde(flatten)]
            root_extra: RootX,
        }
        #[derive(serde::Deserialize)]
        struct Metadata<'a, MetaX> {
            namespace: Option<&'a str>,
            name: &'a str,
            #[serde(rename = "resourceVersion")]
            resource_version: &'a str,
            uid: &'a str,

            #[serde(flatten)]
            meta_extra: MetaX,
        }

        fn convert<'de, D: serde::Deserializer<'de>>(
            buffer: &RawValue,
            substr: &str,
        ) -> Result<StrOffset, D::Error> {
            StrOffset::new(buffer.get(), substr)
                .ok_or_else(|| <D::Error as serde::de::Error>::custom("field not a substring of buffer"))
        }

        let buffer: Box<RawValue> = <_>::deserialize(deserializer)?;

        let (ref_fields, extra) = {
            let Obj::<'_, X::Meta<'_>, X::Root<'_>> { metadata, root_extra } =
                Obj::deserialize(&*buffer).map_err(<D::Error as serde::de::Error>::custom)?;

            let namespace = match metadata.namespace {
                None => None,
                Some(substr) => Some(convert::<D>(&buffer, substr)?),
            };
            let name = convert::<D>(&buffer, metadata.name)?;
            let resource_version = convert::<D>(&buffer, metadata.resource_version)?;
            let uid = convert::<D>(&buffer, metadata.uid)?;

            (
                ObjectRefFields {
                    namespace,
                    name,
                    resource_version,
                    uid,
                },
                X::new(buffer.get(), root_extra, metadata.meta_extra)
                    .map_err(<D::Error as serde::de::Error>::custom)?,
            )
        };

        Ok(Self {
            ref_fields,
            extra,
            buffer,
        })
    }
}

impl<X> reflector::Lookup for RawJson<X> {
    type DynamicType = ApiResource;

    fn kind(dyntype: &Self::DynamicType) -> Cow<'_, str> {
        dyntype.kind.as_str().into()
    }

    fn version(dyntype: &Self::DynamicType) -> Cow<'_, str> {
        dyntype.version.as_str().into()
    }

    fn group(dyntype: &Self::DynamicType) -> Cow<'_, str> {
        dyntype.group.as_str().into()
    }

    fn plural(dyntype: &Self::DynamicType) -> Cow<'_, str> {
        dyntype.plural.as_str().into()
    }

    fn name(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(RawJson::name(self)))
    }

    fn namespace(&self) -> Option<Cow<'_, str>> {
        RawJson::namespace(self).map(Cow::Borrowed)
    }

    fn resource_version(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(RawJson::resource_version(self)))
    }

    fn uid(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(RawJson::uid(self)))
    }
}

/// The base [`Extra`] implementor that does not store any extra data.
impl Extra for () {
    type Error = Infallible;
    type Meta<'a> = ();
    type Root<'a> = ();

    fn new(_: &str, (): Self::Root<'_>, (): Self::Meta<'_>) -> Result<Self, Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::raw_json::{Extra, RawJson, StrOffset};

    struct AppLabelExtra(StrOffset);
    #[derive(serde::Deserialize)]
    struct AppLabelMeta<'a> {
        #[serde(borrow)]
        labels: AppLabels<'a>,
    }
    #[derive(serde::Deserialize)]
    struct AppLabels<'a> {
        app: &'a str,
    }

    impl Extra for AppLabelExtra {
        type Error = &'static str;
        type Meta<'a> = AppLabelMeta<'a>;
        type Root<'a> = ();

        fn new(buffer: &str, (): Self::Root<'_>, meta: Self::Meta<'_>) -> Result<Self, &'static str> {
            Ok(Self(
                StrOffset::new(buffer, meta.labels.app).ok_or("field not a substring of buffer")?,
            ))
        }
    }

    #[test]
    fn test_deser_configmap() {
        let object = r#"{
            "apiVersion": "v1",
            "kind": "ConfigMap",
            "metadata": {
                "namespace": "default",
                "name": "foo",
                "resourceVersion": "1",
                "uid": "00000000-0000-0000-0000-000000000000",
                "labels": {"app": "bar"}
            },
            "items": {}
        }"#;

        let raw: RawJson<AppLabelExtra> = serde_json::from_str(object).unwrap();
        assert_eq!(raw.namespace(), Some("default"));
        assert_eq!(raw.name(), "foo");
        assert_eq!(raw.resource_version(), "1");
        assert_eq!(raw.uid(), "00000000-0000-0000-0000-000000000000");
        assert_eq!(&raw.buffer.get()[raw.extra.0], "bar");
        assert_eq!(raw.buffer.get(), object);
    }
}
