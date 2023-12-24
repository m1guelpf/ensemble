use serde::{Deserialize, Serialize};
use sha256::{digest, Sha256Digest};
use std::{fmt::Debug, ops::Deref};

use crate::value::deserializing_from_db;

/// A wrapper around a value that has been hashed with SHA-256.
#[derive(Clone, Eq, Default)]
pub struct Hashed<T: Sha256Digest> {
	hash: String,
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
	/// # assert_eq!(hashed, "hello world")
	/// ```
	pub fn new(value: T) -> Self {
		Self {
			hash: digest(value),
			_marker: std::marker::PhantomData,
		}
	}
}

impl<T: Sha256Digest> Deref for Hashed<T> {
	type Target = String;

	fn deref(&self) -> &Self::Target {
		&self.hash
	}
}

impl<T: Sha256Digest> From<T> for Hashed<T> {
	fn from(value: T) -> Self {
		Self::new(value)
	}
}

impl<T: Sha256Digest> From<Hashed<T>> for String {
	fn from(val: Hashed<T>) -> Self {
		val.hash
	}
}

impl<T: Sha256Digest> Debug for Hashed<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.hash.fmt(f)
	}
}

impl<T: Sha256Digest> PartialEq for Hashed<T> {
	fn eq(&self, other: &Self) -> bool {
		self.hash == other.hash
	}
}

impl<T: Sha256Digest> PartialEq<String> for Hashed<T> {
	fn eq(&self, other: &String) -> bool {
		self.hash == digest(other)
	}
}

impl<T: Sha256Digest> PartialEq<&str> for Hashed<T> {
	fn eq(&self, other: &&str) -> bool {
		self.hash == digest(*other)
	}
}

impl<T: Sha256Digest> Serialize for Hashed<T> {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		self.hash.serialize(serializer)
	}
}

impl<'de, T: Sha256Digest> Deserialize<'de> for Hashed<T> {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let value = String::deserialize(deserializer)?;

		if deserializing_from_db::<D>() {
			Ok(Self {
				hash: value,
				_marker: std::marker::PhantomData,
			})
		} else {
			Ok(Self {
				hash: digest(value),
				_marker: std::marker::PhantomData,
			})
		}
	}
}

#[cfg(feature = "schema")]
impl<T: Sha256Digest> schemars::JsonSchema for Hashed<T> {
	fn schema_name() -> String {
		String::schema_name()
	}

	fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
		gen.subschema_for::<String>()
	}
}
