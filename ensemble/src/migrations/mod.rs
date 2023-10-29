use async_trait::async_trait;
use std::fmt::Debug;

use crate::connection::ConnectError;

#[cfg(any(feature = "mysql", feature = "postgres"))]
pub use {migrator::Migrator, schema::Schema};

#[cfg(any(feature = "mysql", feature = "postgres"))]
mod migrator;

#[cfg(any(feature = "mysql", feature = "postgres"))]
/// The migration schema.
pub mod schema;

/// Errors that can occur while running migrations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error occurred while connecting to the database.
    #[error("Failed to connect to database.")]
    Connection(#[from] ConnectError),

    /// An error occurred while running a migration.
    #[error("{0}")]
    Database(String),

    /// The migration could not be found.
    #[error("Could not locate the {0} migration.")]
    NotFound(String),

    /// There was an internal error with the migrations system.
    #[error("Failed to receive column in schema.")]
    SendColumn,

    /// One of the migrations locked the connection.
    #[error("Failed to obtain connection")]
    Lock,

    /// The migration data could not be decoded.
    #[error("Failed to deserialize migration data.")]
    Decode(#[from] rbs::Error),
}

/// Accepts a list of structs that implement the [`Migration`] trait, and runs them.
#[macro_export]
macro_rules! migrate {
    ($($migration:ty),*) => {
        async move {
            let mut migrator = $crate::migrations::Migrator::new().await?;

            $(
                migrator.register(stringify!($migration).to_string(), Box::new(<$migration>::default()));
            )*

            migrator.run().await
        }
    };
}

#[async_trait]
/// A trait for defining migrations.
pub trait Migration: Sync + Send {
    /// Runs the migration.
    ///
    /// # Errors
    ///
    /// Returns an error if the migration fails, or if a connection to the database cannot be established.
    async fn up(&self) -> Result<(), Error>;

    /// Reverts the migration.
    ///
    /// # Errors
    ///
    /// Returns an error if the migration fails, or if a connection to the database cannot be established.
    async fn down(&self) -> Result<(), Error>;
}
