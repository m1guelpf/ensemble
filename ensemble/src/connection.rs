use rbatis::{
    rbdc::{
        deadpool::managed::{Object, PoolError},
        pool::ManagerPorxy,
    },
    RBatis,
};
#[cfg(feature = "mysql")]
use rbdc_mysql::driver::MysqlDriver;
#[cfg(feature = "postgres")]
use rbdc_pg::driver::PgDriver;
#[cfg(feature = "sqlite")]
use rbdc_sqlite::driver::SqliteDriver;
use std::sync::OnceLock;

static DB_POOL: OnceLock<RBatis> = OnceLock::new();

#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error("The provided database URL is invalid.")]
    UrlError(#[from] rbatis::Error),

    #[error("The database pool has already been initialized.")]
    AlreadyInitialized,
}

/// Sets up the database pool.
///
/// # Errors
///
/// Returns an error if the database pool has already been initialized, or if the provided database URL is invalid.
pub async fn setup(database_url: &str) -> Result<(), SetupError> {
    let rb = RBatis::new();
    #[cfg(feature = "mysql")]
    rb.link(MysqlDriver {}, database_url).await?;
    #[cfg(feature = "sqlite")]
    rb.link(SqliteDriver {}, database_url).await?;
    #[cfg(feature = "postgres")]
    rb.link(PgDriver {}, database_url).await?;

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
    Disconnected(#[from] rbatis::Error),

    #[error("An error occurred while getting a connection from the database pool.")]
    Pool(#[from] PoolError<rbatis::Error>),
}

/// Returns a connection to the database. Used internally by `ensemble` models.
///
/// # Errors
///
/// Returns an error if the database pool has not been initialized, or if an error occurs while connecting to the database.
pub async fn get() -> Result<Object<ManagerPorxy>, ConnectError> {
    match DB_POOL.get() {
        None => Err(ConnectError::NotInitialized),
        Some(rb) => Ok(rb.get_pool()?.get().await?),
    }
}
