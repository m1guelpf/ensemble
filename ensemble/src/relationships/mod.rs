//! Relationships between models.
#![doc = include_str!("../../docs/relationships.md")]

mod belongs_to;
mod belongs_to_many;
mod has_many;
mod has_one;

use std::{
	collections::HashMap,
	future::Future,
	ops::{Deref, DerefMut},
};

use crate::{query::Builder, value, Error, Model};

pub use belongs_to::BelongsTo;
pub use belongs_to_many::BelongsToMany;
pub use has_many::HasMany;
pub use has_one::HasOne;
use rbs::Value;

/// A relationship between two models.
pub trait Relationship {
	/// The provided input for the relationship.
	type RelatedKey;

	/// The type of the primary key for the model.
	type Key;

	/// The return type of the relationship.
	type Value;

	/// Get the related model.
	///
	/// # Errors
	///
	/// Returns an error if the model cannot be retrieved, or if a connection to the database cannot be established.
	fn get(&mut self) -> impl Future<Output = Result<&mut Self::Value, Error>> + Send;

	/// Whether the relationship has been loaded.
	fn is_loaded(&self) -> bool;

	/// Get the query builder for the relationship.
	///
	/// # Errors
	///
	/// Returns an error if serialization fails when building the query.
	fn query(&self) -> Builder;

	#[doc(hidden)]
	/// Get the query builder for eager loading the relationship. Not intended to be used directly.
	fn eager_query(&self, related: Vec<Self::Key>) -> Builder;

	#[doc(hidden)]
	/// Match the eagerly loaded results to their parents. Not intended to be used directly.
	fn r#match(&mut self, related: &[HashMap<String, Value>]) -> Result<(), Error>;

	#[doc(hidden)]
	/// Create an instance of the relationship. Not intended to be used directly.
	fn build(value: Self::Key, related_key: Self::RelatedKey) -> Self;
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Status<T> {
	Initial(Option<T>),
	Fetched(Option<T>),
}

impl<T> Status<T> {
	const fn initial() -> Self {
		Self::Initial(None)
	}

	const fn is_loaded(&self) -> bool {
		match self {
			Self::Initial(_) => false,
			Self::Fetched(_) => true,
		}
	}
}

impl<T> Default for Status<T> {
	fn default() -> Self {
		Self::initial()
	}
}

impl<T> Deref for Status<T> {
	type Target = Option<T>;

	fn deref(&self) -> &Self::Target {
		match self {
			Self::Initial(value) | Self::Fetched(value) => value,
		}
	}
}

impl<T> DerefMut for Status<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		match self {
			Self::Initial(value) | Self::Fetched(value) => value,
		}
	}
}

impl<T: serde::Serialize> serde::Serialize for Status<T> {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		match self {
			Self::Initial(_) | Self::Fetched(None) => serializer.serialize_none(),
			Self::Fetched(Some(ref value)) => value.serialize(serializer),
		}
	}
}

fn find_related<M: Model, T: serde::Serialize>(
	related: &[HashMap<String, Value>],
	foreign_key: &str,
	value: T,
	wants_one: bool,
) -> Result<Vec<M>, Error> {
	let value = value::for_db(value)?;

	let related = related
		.iter()
		.filter(|model| {
			model
				.get(foreign_key)
				.is_some_and(|v| v.to_string() == value.to_string())
		})
		.take(if wants_one { 1 } else { usize::MAX })
		.map(|model| value::from::<M>(value::for_db(model).unwrap()))
		.collect::<Result<Vec<_>, _>>()?;

	Ok(related)
}
