#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

#[doc(hidden)]
pub use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Display;

mod connection;
pub mod query;
pub use connection::setup;
pub use ensemble_derive::Model;

#[async_trait]
pub trait Model: DeserializeOwned + Serialize + Sized + Send + Sync {
    /// The type of the primary key for the model.
    type PrimaryKey: Display + Serialize + Send;

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
    async fn all() -> Result<Vec<Self>, query::Error>;

    /// Find a model by its primary key.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be found, or if a connection to the database cannot be established.
    async fn find(id: Self::PrimaryKey) -> Result<Self, query::Error>;

    async fn create(self) -> Result<Self, query::Error>;

    async fn save(&mut self) -> Result<(), query::Error>;

    async fn delete(mut self) -> Result<(), query::Error>;
}
