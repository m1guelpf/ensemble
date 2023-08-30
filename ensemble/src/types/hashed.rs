use serde::{Deserialize, Serialize};
use sha256::{digest, Sha256Digest};
use std::{fmt::Debug, ops::Deref};

/// A wrapper around a value that has been hashed with SHA-256.
#[derive(Clone, Eq, Default)]
pub struct Hashed<T: Sha256Digest> {
    value: String,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Sha256Digest> Hashed<T> {
    /// Create a new `Hashed` value from the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use ensemble::types::Hashed;
    /// let hashed = Hashed::new("hello world");
    /// # assert_eq!(hashed, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9")
    /// ```
    pub fn new(value: T) -> Self {
        Self {
            value: digest(value),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: Sha256Digest> Deref for Hashed<T> {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Sha256Digest> From<T> for Hashed<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Sha256Digest> From<Hashed<T>> for String {
    fn from(val: Hashed<T>) -> Self {
        val.value
    }
}

impl<T: Sha256Digest> Debug for Hashed<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl<T: Sha256Digest> PartialEq for Hashed<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Sha256Digest> PartialEq<String> for Hashed<T> {
    fn eq(&self, other: &String) -> bool {
        self.value == digest(other)
    }
}

impl<T: Sha256Digest> PartialEq<&str> for Hashed<T> {
    fn eq(&self, other: &&str) -> bool {
        self.value == digest(*other)
    }
}

impl<T: Sha256Digest> Serialize for Hashed<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

impl<'de, T: Sha256Digest> Deserialize<'de> for Hashed<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self {
            _marker: std::marker::PhantomData,
            value: String::deserialize(deserializer)?,
        })
    }
}

#[cfg(feature = "validator")]
impl<T: Sha256Digest> validator::HasLen for &Hashed<T> {
    fn length(&self) -> u64 {
        self.value.len() as u64
    }
}
