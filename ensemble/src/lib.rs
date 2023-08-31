//! A Laravel-inspired ORM for Rust
#![doc = include_str!("../docs/getting-started.md")]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::inconsistent_struct_constructor)]

#[doc(hidden)]
pub use async_trait::async_trait;
#[doc(hidden)]
pub use inflector::Inflector;
#[doc(hidden)]
pub use rbs;
#[doc(hidden)]
pub use serde;
#[doc(hidden)]
#[cfg(feature = "json")]
pub use serde_json;

use builder::{Builder, EagerLoad};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

pub mod builder;
mod connection;
pub mod migrations;
pub mod query;
pub mod relationships;
pub mod types;
mod value;
#[cfg(any(feature = "mysql", feature = "postgres"))]
pub use connection::setup;
#[cfg(any(feature = "mysql", feature = "postgres"))]
pub use connection::get as get_connection;
pub use rbs::to_value;
pub use ensemble_derive::Model;

#[async_trait]
pub trait Model: DeserializeOwned + Serialize + Sized + Send + Sync + Debug + Default {
    /// The type of the primary key for the model.
    type PrimaryKey: Display
        + DeserializeOwned
        + Serialize
        + Send
        + Sync
        + Clone
        + PartialEq
        + Default;

    /// The name of the model.
    const NAME: &'static str;

    /// The name of the table for the model
    const TABLE_NAME: &'static str;

    /// The name of the primary key field for the model.
    const PRIMARY_KEY: &'static str;

    /// Returns the names of the fields for the model.
    fn keys() -> Vec<&'static str>;

    /// Returns the value of the model's primary key.
    fn primary_key(&self) -> &Self::PrimaryKey;

    /// Get all of the models from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails, or if a connection to the database cannot be established.
    async fn all() -> Result<Vec<Self>, query::Error> {
        Self::query().get().await
    }

    /// Find a model by its primary key.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be found, or if a connection to the database cannot be established.
    async fn find(key: Self::PrimaryKey) -> Result<Self, query::Error>;

    /// Insert a new model into the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be inserted, or if a connection to the database cannot be established.
    async fn create(self) -> Result<Self, query::Error>;

    /// Update the model in the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be updated, or if a connection to the database cannot be established.
    async fn save(&mut self) -> Result<(), query::Error>;

    /// Delete the model from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be deleted, or if a connection to the database cannot be established.
    async fn delete(mut self) -> Result<(), query::Error> {
        let rows_affected = Self::query()
            .r#where(Self::PRIMARY_KEY, "=", rbs::to_value!(self.primary_key()))
            .delete()
            .await?;

        if rows_affected != 1 {
            return Err(query::Error::UniqueViolation);
        }

        Ok(())
    }

    /// Reload a fresh model instance from the database.
    ///
    /// # Errors
    /// Returns an error if the model cannot be retrieved, or if a connection to the database cannot be established.
    async fn fresh(&self) -> Result<Self, query::Error>;

    /// Begin querying the model.
    #[must_use]
    fn query() -> Builder {
        Builder::new(Self::TABLE_NAME.to_string())
    }

    /// Begin querying a model with eager loading.
    fn with<T: Into<EagerLoad>>(eager_load: T) -> Builder {
        Self::query().with(eager_load)
    }

    async fn load<T: Into<EagerLoad> + Send>(&mut self, relation: T) -> Result<(), query::Error> {
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
    ) -> Result<(), query::Error>;
}
