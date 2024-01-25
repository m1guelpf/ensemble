use quaint::{error::Error, pooled::Quaint};
use std::sync::OnceLock;

pub use quaint::pooled::PooledConnection as Connection;

static DB_POOL: OnceLock<Quaint> = OnceLock::new();

#[derive(Debug, thiserror::Error)]
pub enum SetupError {
	#[error("There was an error while setting up the database pool.")]
	Pool(#[from] Error),

	#[cfg(any(feature = "mysql", feature = "postgres"))]
	#[error("The database pool has already been initialized.")]
	AlreadyInitialized,
}

/// Sets up the database pool.
///
/// # Errors
///
/// Returns an error if the database pool has already been initialized, or if the provided database URL is invalid.
#[cfg(any(feature = "mysql", feature = "postgres"))]
pub fn setup(database_url: &str) -> Result<(), SetupError> {
	let pool = Quaint::builder(database_url)?.build();

	tracing::info!(
		database_url = database_url,
		"Setting up {} database pool...",
		pool.connection_info().sql_family().as_str()
	);

	DB_POOL
		.set(pool)
		.map_err(|_| SetupError::AlreadyInitialized)?;

	Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
	#[error("The database pool has not been initialized.")]
	NotInitialized,

	#[error("An error occurred while connecting to the database.")]
	Connection(#[from] Error),
}

/// Returns a connection to the database. Used internally by `ensemble` models.
///
/// # Errors
///
/// Returns an error if the database pool has not been initialized, or if an error occurs while connecting to the database.
pub async fn get() -> Result<Connection, ConnectError> {
	match DB_POOL.get() {
		None => Err(ConnectError::NotInitialized),
		Some(pool) => Ok(pool.check_out().await?),
	}
}
