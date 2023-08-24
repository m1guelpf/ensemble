#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

#[doc(hidden)]
pub use async_trait::async_trait;
use builder::Builder;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Display;

pub mod builder;
mod connection;
pub mod migrations;
pub mod query;
pub mod types;
mod value;
pub use connection::setup;
pub use ensemble_derive::Model;

#[async_trait]
pub trait Model: DeserializeOwned + Serialize + Sized + Send + Sync {
    /// The type of the primary key for the model.
    type PrimaryKey: Display + DeserializeOwned + Serialize + Send + Sync;

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
        query::all().await
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
    async fn save(&mut self) -> Result<(), query::Error> {
        query::save(self).await
    }

    /// Delete the model from the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be deleted, or if a connection to the database cannot be established.
    async fn delete(mut self) -> Result<(), query::Error> {
        query::delete(&self).await
    }

    /// Reload a fresh model instance from the database.
    ///
    /// # Errors
    /// Returns an error if the model cannot be retrieved, or if a connection to the database cannot be established.
    async fn fresh(&self) -> Result<Self, query::Error> {
        query::find(self.primary_key()).await
    }

    #[must_use]
    fn query() -> Builder {
        Builder::new(Self::TABLE_NAME.to_string())
    }
}
