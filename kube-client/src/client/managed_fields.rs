use crate::request_operation::{DELETE_COLLECTION, LIST, LIST_METADATA};
use http::{Request, header::ACCEPT};
use serde::{
    Deserialize,
    de::{self, DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor},
};
use std::{borrow::Cow, fmt};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy)]
pub(crate) struct DropManagedFields {
    list: bool,
}

/// Add the drop parameter while preserving the original representation as a fallback.
pub(crate) fn prepare(request: &mut Request<Vec<u8>>, policy: crate::api::ManagedFieldsPolicy) {
    // Preserve metadata-only negotiation, including its fallback representation.
    let accept = request
        .headers()
        .get(ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");
    let value = format!("{accept};drop=metadata.managedFields, {accept}");
    request
        .headers_mut()
        .insert(ACCEPT, value.parse().expect("valid generated Accept header"));
    if policy == crate::api::ManagedFieldsPolicy::ServerOnly {
        return;
    }
    let list = matches!(
        request.extensions().get::<&'static str>().copied(),
        Some(LIST | LIST_METADATA | DELETE_COLLECTION)
    );
    request.extensions_mut().insert(DropManagedFields { list });
}

pub(super) fn decode<T: DeserializeOwned>(
    text: &str,
    policy: Option<DropManagedFields>,
    watch: bool,
) -> Result<T, serde_json::Error> {
    let Some(policy) = policy else {
        return serde_json::from_str(text);
    };
    let path = if watch {
        // The object may precede the event type. Read just the discriminator
        // first, ignoring other values without constructing an intermediate tree.
        #[derive(Deserialize)]
        enum EventType {
            #[serde(rename = "ADDED")]
            Added,
            #[serde(rename = "MODIFIED")]
            Modified,
            #[serde(rename = "DELETED")]
            Deleted,
            #[serde(rename = "BOOKMARK")]
            Bookmark,
            #[serde(other)]
            Other,
        }
        #[derive(Deserialize)]
        struct Event {
            #[serde(rename = "type")]
            kind: EventType,
        }
        if matches!(serde_json::from_str::<Event>(text)?.kind, EventType::Other) {
            return serde_json::from_str(text);
        }
        Path::Watch
    } else if policy.list {
        Path::List
    } else {
        Path::Object
    };
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let result = T::deserialize(Filter {
        inner: &mut deserializer,
        path,
    })?;
    deserializer.end()?;
    Ok(result)
}

#[derive(Clone, Copy)]
enum Path {
    Watch,
    List,
    Items,
    Object,
    Metadata,
}

// Wrap only the resource-envelope paths. All other values go directly to the
// original deserializer, including nested user data and annotation strings.
struct Filter<T> {
    inner: T,
    path: Path,
}

macro_rules! forward_deserialize {
    ($($method:ident $(($($arg:ident: $ty:ty),*))?;)*) => {
        $(fn $method<V: Visitor<'de>>(self, $($($arg: $ty,)*)? visitor: V) -> Result<V::Value, D::Error> {
            self.inner.$method($($($arg,)*)? Filter { inner: visitor, path: self.path })
        })*
    };
}

impl<'de, D: de::Deserializer<'de>> de::Deserializer<'de> for Filter<D> {
    type Error = D::Error;

    forward_deserialize! {
        deserialize_any;
        deserialize_bool;
        deserialize_i8;
        deserialize_i16;
        deserialize_i32;
        deserialize_i64;
        deserialize_i128;
        deserialize_u8;
        deserialize_u16;
        deserialize_u32;
        deserialize_u64;
        deserialize_u128;
        deserialize_f32;
        deserialize_f64;
        deserialize_char;
        deserialize_str;
        deserialize_string;
        deserialize_bytes;
        deserialize_byte_buf;
        deserialize_option;
        deserialize_unit;
        deserialize_unit_struct(name: &'static str);
        deserialize_newtype_struct(name: &'static str);
        deserialize_seq;
        deserialize_tuple(len: usize);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_map;
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_enum(name: &'static str, variants: &'static [&'static str]);
        deserialize_identifier;
        deserialize_ignored_any;
    }

    fn is_human_readable(&self) -> bool {
        self.inner.is_human_readable()
    }
}

macro_rules! forward_visit {
    ($($method:ident($ty:ty);)*) => {
        $(fn $method<E: de::Error>(self, value: $ty) -> Result<Self::Value, E> {
            self.inner.$method(value)
        })*
    };
}

impl<'de, V: Visitor<'de>> Visitor<'de> for Filter<V> {
    type Value = V::Value;

    forward_visit! {
        visit_bool(bool);
        visit_i8(i8);
        visit_i16(i16);
        visit_i32(i32);
        visit_i64(i64);
        visit_i128(i128);
        visit_u8(u8);
        visit_u16(u16);
        visit_u32(u32);
        visit_u64(u64);
        visit_u128(u128);
        visit_f32(f32);
        visit_f64(f64);
        visit_char(char);
        visit_str(&str);
        visit_borrowed_str(&'de str);
        visit_string(String);
        visit_bytes(&[u8]);
        visit_borrowed_bytes(&'de [u8]);
        visit_byte_buf(Vec<u8>);
    }

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.inner.expecting(formatter)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        self.inner.visit_unit()
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        self.inner.visit_none()
    }

    fn visit_some<D: de::Deserializer<'de>>(self, inner: D) -> Result<Self::Value, D::Error> {
        self.inner.visit_some(Filter {
            inner,
            path: self.path,
        })
    }

    fn visit_newtype_struct<D: de::Deserializer<'de>>(self, inner: D) -> Result<Self::Value, D::Error> {
        self.inner.visit_newtype_struct(Filter {
            inner,
            path: self.path,
        })
    }

    fn visit_enum<A: de::EnumAccess<'de>>(self, access: A) -> Result<Self::Value, A::Error> {
        self.inner.visit_enum(access)
    }

    fn visit_map<M: MapAccess<'de>>(self, inner: M) -> Result<Self::Value, M::Error> {
        self.inner.visit_map(FilteredMap {
            inner,
            path: self.path,
            next: None,
        })
    }

    fn visit_seq<S: SeqAccess<'de>>(self, inner: S) -> Result<Self::Value, S::Error> {
        if matches!(self.path, Path::Items) {
            self.inner.visit_seq(Filter {
                inner,
                path: Path::Object,
            })
        } else {
            self.inner.visit_seq(inner)
        }
    }
}

impl<'de, S: DeserializeSeed<'de>> DeserializeSeed<'de> for Filter<S> {
    type Value = S::Value;

    fn deserialize<D: de::Deserializer<'de>>(self, inner: D) -> Result<Self::Value, D::Error> {
        self.inner.deserialize(Filter {
            inner,
            path: self.path,
        })
    }
}

impl<'de, S: SeqAccess<'de>> SeqAccess<'de> for Filter<S> {
    type Error = S::Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, inner: T) -> Result<Option<T::Value>, S::Error> {
        self.inner.next_element_seed(Filter {
            inner,
            path: self.path,
        })
    }

    fn size_hint(&self) -> Option<usize> {
        self.inner.size_hint()
    }
}

struct FilteredMap<M> {
    inner: M,
    path: Path,
    next: Option<Path>,
}

impl<'de, M: MapAccess<'de>> MapAccess<'de> for FilteredMap<M> {
    type Error = M::Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, M::Error> {
        while let Some(key) = self.inner.next_key_seed(Key)? {
            self.next = match (self.path, key.as_ref()) {
                (Path::Watch, "object") => Some(Path::Object),
                (Path::List, "items") => Some(Path::Items),
                (Path::Object, "metadata") => Some(Path::Metadata),
                (Path::Metadata, "managedFields") => {
                    self.inner.next_value::<de::IgnoredAny>()?;
                    continue;
                }
                _ => None,
            };
            return match key {
                Cow::Borrowed(key) => seed.deserialize(de::value::BorrowedStrDeserializer::new(key)),
                Cow::Owned(key) => seed.deserialize(de::value::StringDeserializer::new(key)),
            }
            .map(Some);
        }
        Ok(None)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, M::Error> {
        match self.next.take() {
            Some(path) => self.inner.next_value_seed(Filter { inner: seed, path }),
            None => self.inner.next_value_seed(seed),
        }
    }
}

// Borrow ordinary keys, allocating only for escaped key spellings.
struct Key;

impl<'de> DeserializeSeed<'de> for Key {
    type Value = Cow<'de, str>;

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for Key {
    type Value = Cow<'de, str>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a JSON key")
    }

    fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(Cow::Borrowed(value))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Cow::Owned(value.to_owned()))
    }
}
