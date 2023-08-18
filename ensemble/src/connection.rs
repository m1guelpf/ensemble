use mysql_async::{Conn, Opts, Pool, UrlError};
use std::sync::OnceLock;

static DB_POOL: OnceLock<Pool> = OnceLock::new();

#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error("The provided database URL is invalid.")]
    UrlError(#[from] UrlError),

    #[error("The database pool has already been initialized.")]
    AlreadyInitialized,
}

/// Sets up the database pool.
///
/// # Errors
///
/// Returns an error if the database pool has already been initialized, or if the provided database URL is invalid.
pub fn setup(database_url: &str) -> Result<(), SetupError> {
    let opts = Opts::from_url(database_url)?;

    let pool = Pool::new(opts);

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
    Disconnected(#[from] mysql_async::Error),
}

/// Returns a connection to the database. Used internally by `ensemble` models.
///
/// # Errors
///
/// Returns an error if the database pool has not been initialized, or if an error occurs while connecting to the database.
pub async fn get() -> Result<Conn, ConnectError> {
    match DB_POOL.get() {
        None => Err(ConnectError::NotInitialized),
        Some(pool) => Ok(pool.get_conn().await?),
    }
}
