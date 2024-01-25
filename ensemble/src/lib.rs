//! A Laravel-inspired ORM for Rust
#![doc = include_str!("../docs/getting-started.md")]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::inconsistent_struct_constructor)]

#[doc(hidden)]
pub use async_trait::async_trait;
use connection::ConnectError;
#[doc(hidden)]
pub use inflector::Inflector;
use quaint::ast::Comparable;
#[doc(hidden)]
pub use quaint::Value;
#[doc(hidden)]
pub use serde;
#[doc(hidden)]
#[cfg(feature = "json")]
pub use serde_json;

use query::{Builder, EagerLoad};
use serde::{de::DeserializeOwned, Serialize};
use std::{
	collections::HashMap,
	fmt::{Debug, Display},
	future::Future,
	sync::Arc,
};

mod connection;
// pub mod migrations;
pub mod query;
// pub mod relationships;
// pub mod types;
// pub mod value;
#[cfg(any(feature = "mysql", feature = "postgres"))]
pub use connection::setup;
pub use ensemble_derive::Model;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error(transparent)]
	Connection(#[from] ConnectError),

	#[cfg(feature = "validator")]
	#[error(transparent)]
	Validation(#[from] validator::ValidationErrors),

	#[error(transparent)]
	Database(#[from] quaint::error::Error),

	#[error("The {0} field is required.")]
	Required(&'static str),

	#[error("Failed to serialize model.")]
	Serialization(#[from] rbs::value::ext::Error),

	#[error("The model could not be found.")]
	NotFound,

	#[error("The unique constraint was violated.")]
	UniqueViolation,

	#[error("The query is invalid.")]
	InvalidQuery,
}

pub trait Model: DeserializeOwned + Serialize + Sized + Send + Sync + Debug + Default {
	/// The type of the primary key for the model.
	type PrimaryKey: Display
		+ for<'a> Into<quaint::Value<'a>>
		+ DeserializeOwned
		+ Serialize
		+ PartialEq
		+ Default
		+ Clone
		+ Send
		+ Sync;

	/// The name of the model.
	const NAME: &'static str;

	/// The name of the table for the model
	const TABLE_NAME: &'static str;

	/// The name of the primary key field for the model.
	const PRIMARY_KEY: &'static str;

	/// Returns the value of the model's primary key.
	fn primary_key(&self) -> &Self::PrimaryKey;

	/// Get all of the models from the database.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	#[must_use]
	fn all() -> impl Future<Output = Result<Vec<Self>, Error>> + Send {
		async { Self::query().get().await }
	}

	/// Find a model by its primary key.
	///
	/// # Errors
	///
	/// Returns an error if the model cannot be found, or if a connection to the database cannot be established.
	fn find(key: Self::PrimaryKey) -> impl Future<Output = Result<Self, Error>> + Send;

	/// Insert a new model into the database.
	///
	/// # Errors
	///
	/// Returns an error if the model cannot be inserted, or if a connection to the database cannot be established.
	fn create(self) -> impl Future<Output = Result<Self, Error>> + Send;

	/// Update the model in the database.
	///
	/// # Errors
	///
	/// Returns an error if the model cannot be updated, or if a connection to the database cannot be established.
	fn save(&mut self) -> impl Future<Output = Result<(), Error>> + Send;

	/// Delete the model from the database.
	///
	/// # Errors
	///
	/// Returns an error if the model cannot be deleted, or if a connection to the database cannot be established.
	#[allow(unused_mut)]
	fn delete(mut self) -> impl Future<Output = Result<(), Error>> + Send {
		async move {
			Self::query()
				.r#where(Self::PRIMARY_KEY.equals(self.primary_key().clone()))
				.delete()
				.await?;

			Ok(())
		}
	}

	/// Reload a fresh model instance from the database.
	///
	/// # Errors
	/// Returns an error if the model cannot be retrieved, or if a connection to the database cannot be established.
	fn fresh(&self) -> impl Future<Output = Result<Self, Error>> + Send;

	/// Begin querying the model.
	#[must_use]
	fn query<'a>() -> Builder<'a> {
		Builder::new(Self::TABLE_NAME.to_string())
	}

	/// Begin querying a model with eager loading.
	fn with<'a, T: Into<EagerLoad>>(eager_load: T) -> Builder<'a> {
		Self::query().with(eager_load)
	}

	/// Load a relationship for the model.
	fn load<T: Into<EagerLoad> + Send>(
		&mut self,
		relation: T,
	) -> impl Future<Output = Result<(), Error>> + Send {
		async move {
			for relation in relation.into().list() {
				let query = self.eager_load(&relation, std::iter::once(&*self));
				let rows = query.get_rows().await?.clone();

				self.fill_relation(&relation, Arc::new(rows))?;
			}

			Ok(())
		}
	}

	fn increment(
		&mut self,
		column: &str,
		amount: i64,
	) -> impl Future<Output = Result<(), Error>> + Send {
		async move {
			let rows_affected = Self::query()
				.r#where(Self::PRIMARY_KEY.equals(self.primary_key().clone()))
				.increment(column, amount)
				.await?;

			if rows_affected != 1 {
				return Err(Error::UniqueViolation);
			}

			Ok(())
		}
	}

	/// Convert the model to a JSON value.
	///
	/// # Panics
	///
	/// Panics if the model cannot be converted to JSON. Since Ensemble manually implement Serialize, this should never happen.
	#[cfg(feature = "json")]
	fn json(&self) -> serde_json::Value {
		serde_json::to_value(self).unwrap()
	}

	/// Eager load a relationship for a set of models.
	/// This method is used internally by Ensemble, and should not be called directly.
	#[doc(hidden)]
	fn eager_load<'a>(&self, relation: &str, related: impl Iterator<Item = &'a Self>) -> Builder
	where
		Self: 'a;

	/// Fill a relationship for a set of models.
	/// This method is used internally by Ensemble, and should not be called directly.
	#[doc(hidden)]
	fn fill_relation(
		&mut self,
		relation: &str,
		related: Arc<Vec<HashMap<String, quaint::Value>>>,
	) -> Result<(), Error>;
}

pub trait Collection {
	/// Eager load a relationship for a collection of models.
	///
	/// # Errors
	///
	/// Returns an error if any of the models fail to load, or if a connection to the database cannot be established.
	fn load<T>(&mut self, relation: T) -> impl Future<Output = Result<(), Error>> + Send
	where
		T: Into<EagerLoad> + Send + Sync + Clone;

	/// Convert the collection to a JSON value.
	///
	/// # Panics
	///
	/// Panics if the collection cannot be converted to JSON. Since models manually implement Serialize, this should never happen.
	#[cfg(feature = "json")]
	fn json(&self) -> serde_json::Value;
}

impl<T: Model> Collection for &mut Vec<T> {
	async fn load<U>(&mut self, relation: U) -> Result<(), Error>
	where
		U: Into<EagerLoad> + Send + Sync + Clone,
	{
		for model in self.iter_mut() {
			model.load(relation.clone()).await?;
		}

		Ok(())
	}

	#[cfg(feature = "json")]
	fn json(&self) -> serde_json::Value {
		serde_json::to_value(self).unwrap()
	}
}
