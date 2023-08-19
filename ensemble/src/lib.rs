#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

#[doc(hidden)]
pub use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

mod connection;
pub mod query;
pub use connection::setup;
pub use ensemble_derive::Model;

#[async_trait]
pub trait Model: DeserializeOwned + Sized {
    /// The type of the primary key for the model.
    type PrimaryKey: Serialize + Send;

    /// The name of the table for the model
    const TABLE_NAME: &'static str;

    /// The name of the primary key field for the model.
    const PRIMARY_KEY: &'static str;

    /// Returns the names of the fields for the model.
    fn keys() -> Vec<&'static str>;

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
}
