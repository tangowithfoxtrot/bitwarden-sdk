use std::{
    borrow::Cow,
    fmt::{self, Formatter},
};

use generic_array::{ArrayLength, GenericArray};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::CryptoError;

/// Wrapper for sensitive values which makes a best effort to enforce zeroization of the inner value
/// on drop. The inner value exposes a [`Sensitive::expose`] method which returns a reference to the
/// inner value. Care must be taken to avoid accidentally exposing the inner value through copying
/// or cloning.
///
/// Internally [`Sensitive`] contains a [`Box`] which ensures the value is placed on the heap. It
/// implements the [`Drop`] trait which calls `zeroize` on the inner value.
#[derive(PartialEq, Clone, Zeroize, ZeroizeOnDrop)]
pub struct Sensitive<V: Zeroize> {
    pub(super) value: Box<V>,
}

/// Important: This type does not protect against reallocations made by the Vec.
/// This means that if you insert any elements past the capacity, the data will be copied to a
/// new allocation and the old allocation will not be zeroized.
/// To avoid this, use Vec::with_capacity to preallocate the capacity you need.
pub type SensitiveVec = Sensitive<Vec<u8>>;

/// Important: This type does not protect against reallocations made by the String.
/// This means that if you insert any characters past the capacity, the data will be copied to a
/// new allocation and the old allocation will not be zeroized.
/// To avoid this, use String::with_capacity to preallocate the capacity you need.
pub type SensitiveString = Sensitive<String>;

impl<V: Zeroize> Sensitive<V> {
    /// Create a new [`Sensitive`] value. In an attempt to avoid accidentally placing this on the
    /// stack it only accepts a [`Box`] value. The rust compiler should be able to optimize away the
    /// initial stack allocation presuming the value is not used before being boxed.
    #[inline(always)]
    pub fn new(value: Box<V>) -> Self {
        Self { value }
    }

    /// Expose the inner value. By exposing the inner value, you take responsibility for ensuring
    /// that any copy of the value is zeroized.
    #[inline(always)]
    pub fn expose(&self) -> &V {
        &self.value
    }

    /// Expose the inner value mutable. By exposing the inner value, you take responsibility for
    /// ensuring that any copy of the value is zeroized.
    #[inline(always)]
    pub fn expose_mut(&mut self) -> &mut V {
        &mut self.value
    }
}

/// Helper to convert a `Sensitive<[u8, N]>` to a `SensitiveVec`.
impl<const N: usize> From<Sensitive<[u8; N]>> for SensitiveVec {
    fn from(sensitive: Sensitive<[u8; N]>) -> Self {
        SensitiveVec::new(Box::new(sensitive.value.to_vec()))
    }
}

/// Helper to convert a `&SensitiveVec` to a `Sensitive<[u8, N]>`.
impl<const N: usize> TryFrom<&SensitiveVec> for Sensitive<[u8; N]> {
    type Error = CryptoError;

    fn try_from(v: &SensitiveVec) -> Result<Self, CryptoError> {
        Ok(Sensitive::new(Box::new(
            TryInto::<[u8; N]>::try_into(v.expose().as_slice())
                .map_err(|_| CryptoError::InvalidKey)?,
        )))
    }
}

/// Helper to convert a `Sensitive<Vec<u8>>` to a `Sensitive<String>`, care is taken to ensure any
/// intermediate copies are zeroed to avoid leaking sensitive data.
impl TryFrom<SensitiveVec> for SensitiveString {
    type Error = CryptoError;

    fn try_from(mut v: SensitiveVec) -> Result<Self, CryptoError> {
        let value = std::mem::take(&mut v.value);

        let rtn = String::from_utf8(*value).map_err(|_| CryptoError::InvalidUtf8String);
        rtn.map(|v| Sensitive::new(Box::new(v)))
    }
}

impl From<SensitiveString> for SensitiveVec {
    fn from(mut s: SensitiveString) -> Self {
        let value = std::mem::take(&mut s.value);
        Sensitive::new(Box::new(value.into_bytes()))
    }
}

impl<N: ArrayLength<u8>> From<Sensitive<GenericArray<u8, N>>> for SensitiveVec {
    fn from(val: Sensitive<GenericArray<u8, N>>) -> Self {
        SensitiveVec::new(Box::new(val.value.to_vec()))
    }
}

impl SensitiveString {
    pub fn decode_base64<T: base64::Engine>(self, engine: T) -> Result<SensitiveVec, CryptoError> {
        // Prevent accidental copies by allocating the full size
        let len = base64::decoded_len_estimate(self.value.len());
        let mut value = SensitiveVec::new(Box::new(Vec::with_capacity(len)));

        engine
            .decode_vec(self.value.as_ref(), &mut value.value)
            .map_err(|_| CryptoError::InvalidKey)?;

        Ok(value)
    }
}

impl<T: Zeroize + AsRef<[u8]>> Sensitive<T> {
    pub fn encode_base64<E: base64::Engine>(self, engine: E) -> SensitiveString {
        use base64::engine::Config;

        let inner: &[u8] = self.value.as_ref().as_ref();

        // Prevent accidental copies by allocating the full size
        let padding = engine.config().encode_padding();
        let len = base64::encoded_len(inner.len(), padding).expect("Valid length");

        let mut value = SensitiveVec::new(Box::new(vec![0u8; len]));
        engine
            .encode_slice(inner, &mut value.value[..len])
            .expect("Valid base64 string length");

        value.try_into().expect("Valid base64 string")
    }
}

impl<V: Zeroize + Default> Default for Sensitive<V> {
    fn default() -> Self {
        Self::new(Box::default())
    }
}

impl<V: Zeroize + Serialize> fmt::Debug for Sensitive<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sensitive")
            .field("type", &std::any::type_name::<V>())
            .field("value", &"********")
            .finish()
    }
}

/// Unfortunately once we serialize a `SensitiveString` we can't control the future memory.
impl<V: Zeroize + Serialize> Serialize for Sensitive<V> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

impl<'de, V: Zeroize + Deserialize<'de>> Deserialize<'de> for Sensitive<V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::new(Box::new(V::deserialize(deserializer)?)))
    }
}

/// Transparently expose the inner value for serialization
impl<V: Zeroize + JsonSchema> JsonSchema for Sensitive<V> {
    fn schema_name() -> String {
        V::schema_name()
    }

    fn schema_id() -> Cow<'static, str> {
        V::schema_id()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        V::json_schema(gen)
    }
}

// We use a lot of `&str` and `&[u8]` in our tests, so we expose this helper
// to make it easier.
// IMPORTANT: This should not be used outside of test code
// Note that we can't just mark it with #[cfg(test)] because that only applies
// when testing this crate, not when testing other crates that depend on it.
// By at least limiting it to &'static reference we should be able to avoid accidental usages
impl<V: Zeroize> Sensitive<V> {
    pub fn test<T: ?Sized>(value: &'static T) -> Self
    where
        &'static T: Into<V>,
    {
        Self::new(Box::new(value.into()))
    }
}

#[cfg(test)]
mod tests {
    use schemars::schema_for;

    use super::*;

    #[test]
    fn test_debug() {
        let string = SensitiveString::test("test");
        assert_eq!(
            format!("{:?}", string),
            "Sensitive { type: \"alloc::string::String\", value: \"********\" }"
        );

        let vector = Sensitive::new(Box::new(vec![1, 2, 3]));
        assert_eq!(
            format!("{:?}", vector),
            "Sensitive { type: \"alloc::vec::Vec<i32>\", value: \"********\" }"
        );
    }

    #[test]
    fn test_schemars() {
        #[derive(JsonSchema)]
        struct TestStruct {
            #[allow(dead_code)]
            s: SensitiveString,
            #[allow(dead_code)]
            v: SensitiveVec,
        }

        let schema = schema_for!(TestStruct);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        let expected = r##"{
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "TestStruct",
            "type": "object",
            "required": ["s", "v"],
            "properties": {
                "s": {
                    "$ref": "#/definitions/String"
                },
                "v": {
                    "$ref": "#/definitions/Array_of_uint8"
                }
            },
            "definitions": {
                "Array_of_uint8": {
                    "type": "array",
                    "items": {
                        "type": "integer",
                        "format": "uint8",
                        "minimum": 0.0
                    }
                },
                "String": {
                    "type": "string"
                }
            }
        }"##;

        assert_eq!(
            json.parse::<serde_json::Value>().unwrap(),
            expected.parse::<serde_json::Value>().unwrap()
        );
    }
}
