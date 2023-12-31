use rbatis::{rbdc::db::Connection as RbdcConnection, RBatis};
#[cfg(feature = "mysql")]
use rbdc_mysql::{driver::MysqlDriver, options::MySqlConnectOptions};
#[cfg(feature = "postgres")]
use rbdc_pg::{driver::PgDriver, options::PgConnectOptions};
use std::sync::OnceLock;
#[cfg(any(feature = "mysql", feature = "postgres"))]
use {rbatis::DefaultPool, std::str::FromStr};

pub type Connection = Box<dyn RbdcConnection>;

static DB_POOL: OnceLock<RBatis> = OnceLock::new();

#[derive(Debug, thiserror::Error)]
pub enum SetupError {
	#[error("The provided database URL is invalid.")]
	UrlError(#[from] rbatis::Error),

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
	let rb = RBatis::new();

	#[cfg(feature = "mysql")]
	tracing::info!(
		database_url = database_url,
		"Setting up MySQL database pool..."
	);
	#[cfg(feature = "postgres")]
	tracing::info!(
		database_url = database_url,
		"Setting up PostgreSQL database pool..."
	);

	#[cfg(feature = "mysql")]
	rb.init_option::<MysqlDriver, MySqlConnectOptions, DefaultPool>(
		MysqlDriver {},
		MySqlConnectOptions::from_str(database_url)?,
	)?;
	#[cfg(feature = "postgres")]
	rb.init_option::<PgDriver, PgConnectOptions, DefaultPool>(
		PgDriver {},
		PgConnectOptions::from_str(database_url)?,
	)?;

	DB_POOL
		.set(rb)
		.map_err(|_| SetupError::AlreadyInitialized)?;

	Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
	#[error("The database pool has not been initialized.")]
	NotInitialized,

	#[error("An error occurred while connecting to the database.")]
	Connection(#[from] rbatis::Error),
}

/// Returns a connection to the database. Used internally by `ensemble` models.
///
/// # Errors
///
/// Returns an error if the database pool has not been initialized, or if an error occurs while connecting to the database.
pub async fn get() -> Result<Connection, ConnectError> {
	match DB_POOL.get() {
		None => Err(ConnectError::NotInitialized),
		Some(rb) => Ok(rb.get_pool()?.get().await?),
	}
}

pub enum Database {
	MySQL,
	PostgreSQL,
}

impl Database {
	pub const fn is_mysql(&self) -> bool {
		matches!(self, Self::MySQL)
	}

	pub const fn is_postgres(&self) -> bool {
		matches!(self, Self::PostgreSQL)
	}
}

pub const fn which_db() -> Database {
	#[cfg(all(not(feature = "mysql"), not(feature = "postgres")))]
	panic!("Either the `mysql` or `postgres` feature must be enabled to use `ensemble`.");

	#[cfg(all(feature = "mysql", feature = "postgres"))]
	panic!("Both the `mysql` and `postgres` features are enabled. Please enable only one of them.");

	if cfg!(feature = "mysql") {
		Database::MySQL
	} else {
		Database::PostgreSQL
	}
}
