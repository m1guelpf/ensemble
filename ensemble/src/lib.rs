#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

pub use async_trait::async_trait;
use connection::ConnectError;
pub use ensemble_derive::Model;

pub mod connection;
pub use connection::setup;

#[derive(Debug, thiserror::Error)]
pub enum FindError {
    #[error(transparent)]
    Connection(#[from] ConnectError),
}

#[async_trait]
pub trait Model {
    /// The type of the primary key for the model.
    type PrimaryKey;

    /// Returns the names of the fields for the model.
    fn keys() -> Vec<&'static str>;

    /// Returns the name of the table for the model.
    fn table_name() -> &'static str;

    /// Returns the name of the primary key field for the model.
    fn primary_key() -> &'static str;

    /// Find a model by its primary key.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be found, or if a connection to the database cannot be established.
    async fn find(id: Self::PrimaryKey) -> Result<Self, FindError>
    where
        Self: std::marker::Sized;
}
