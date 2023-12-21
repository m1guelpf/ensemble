//! A Laravel-inspired ORM for Rust
#![doc = include_str!("../docs/getting-started.md")]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::inconsistent_struct_constructor)]

#[doc(hidden)]
pub use async_trait::async_trait;
use connection::ConnectError;
#[doc(hidden)]
pub use inflector::Inflector;
#[doc(hidden)]
pub use rbs;
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
};

mod connection;
pub mod migrations;
pub mod query;
pub mod relationships;
pub mod types;
pub mod value;
#[cfg(any(feature = "mysql", feature = "postgres"))]
pub use connection::{before_query, setup};
pub use ensemble_derive::Model;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Connection(#[from] ConnectError),

    #[cfg(feature = "validator")]
    #[error(transparent)]
    Validation(#[from] validator::ValidationErrors),

    #[error("{0}")]
    Database(String),

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

#[async_trait]
pub trait Model: DeserializeOwned + Serialize + Sized + Send + Sync + Debug + Default {
    /// The type of the primary key for the model.
    type PrimaryKey: Display
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
    async fn all() -> Result<Vec<Self>, Error> {
        Self::query().get().await
    }

    /// Find a model by its primary key.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be found, or if a connection to the database cannot be established.
    async fn find(key: Self::PrimaryKey) -> Result<Self, Error>;

    /// Insert a new model into the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be inserted, or if a connection to the database cannot be established.
    async fn create(self) -> Result<Self, Error>;

    /// Update the model in the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be updated, or if a connection to the database cannot be established.
    async fn save(&mut self) -> Result<(), Error>;

    /// Delete the model from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be deleted, or if a connection to the database cannot be established.
    async fn delete(mut self) -> Result<(), Error> {
        let rows_affected = Self::query()
            .r#where(
                Self::PRIMARY_KEY,
                "=",
                value::for_db(self.primary_key()).unwrap(),
            )
            .delete()
            .await?;

        if rows_affected != 1 {
            return Err(Error::UniqueViolation);
        }

        Ok(())
    }

    /// Reload a fresh model instance from the database.
    ///
    /// # Errors
    /// Returns an error if the model cannot be retrieved, or if a connection to the database cannot be established.
    async fn fresh(&self) -> Result<Self, Error>;

    /// Begin querying the model.
    #[must_use]
    fn query() -> Builder {
        Builder::new(Self::TABLE_NAME.to_string())
    }

    /// Begin querying a model with eager loading.
    fn with<T: Into<EagerLoad>>(eager_load: T) -> Builder {
        Self::query().with(eager_load)
    }

    async fn load<T: Into<EagerLoad> + Send>(&mut self, relation: T) -> Result<(), Error> {
        for relation in relation.into().list() {
            let rows = self.eager_load(&relation, &[&self]).get_rows().await?;

            self.fill_relation(&relation, &rows)?;
        }

        Ok(())
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
    fn eager_load(&self, relation: &str, related: &[&Self]) -> Builder;

    /// Fill a relationship for a set of models.
    /// This method is used internally by Ensemble, and should not be called directly.
    #[doc(hidden)]
    fn fill_relation(
        &mut self,
        relation: &str,
        related: &[HashMap<String, rbs::Value>],
    ) -> Result<(), Error>;
}

#[async_trait]
pub trait Collection {
    /// Eager load a relationship for a collection of models.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the models fail to load, or if a connection to the database cannot be established.
    async fn load<T>(&mut self, relation: T) -> Result<(), Error>
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

#[async_trait]
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
