use async_trait::async_trait;
use std::fmt::Debug;

use crate::connection::ConnectError;

pub use migrator::Migrator;
pub use schema::Schema;

mod migrator;
mod schema;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to connect to database.")]
    Connection(#[from] ConnectError),

    #[error("{0}")]
    Database(String),

    #[error("Could not locate the {0} migration.")]
    NotFound(String),

    #[error("Failed to receive column in schema.")]
    SendColumn,

    #[error("Failed to obtain connection")]
    Lock,

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
    async fn down(&self) -> Result<(), Error> {
        Ok(())
    }
}
