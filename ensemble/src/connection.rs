use once_cell::sync::Lazy;
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
use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
};
use tokio::task::Id;

pub type Connection = Object<ManagerPorxy>;

static DB_POOL: OnceLock<RBatis> = OnceLock::new();
static PER_THREAD_QUERY_SETTINGS: Lazy<RwLock<HashMap<Id, QuerySettings>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

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
pub async fn setup(database_url: &str) -> Result<(), SetupError> {
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
    rb.link(MysqlDriver {}, database_url).await?;
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
pub async fn get() -> Result<Connection, ConnectError> {
    match DB_POOL.get() {
        None => Err(ConnectError::NotInitialized),
        Some(rb) => Ok(rb.get_pool()?.get().await?),
    }
}

#[cfg(any(feature = "mysql", feature = "postgres"))]
pub enum Database {
    MySQL,
    PostgreSQL,
}

#[cfg(any(feature = "mysql", feature = "postgres"))]
impl Database {
    pub fn is_mysql(&self) -> bool {
        matches!(self, Database::MySQL)
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Database::PostgreSQL)
    }
}

#[cfg(any(feature = "mysql", feature = "postgres"))]
pub const fn which_db() -> Database {
    #[cfg(all(feature = "mysql", feature = "postgres"))]
    panic!("Both the `mysql` and `postgres` features are enabled. Please enable only one of them.");

    if cfg!(feature = "mysql") {
        Database::MySQL
    } else {
        Database::PostgreSQL
    }
}

struct QuerySettings {
    pub(crate) pre_sql: Option<String>,
}

pub struct Guard {
    id: Id,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let mut settings = PER_THREAD_QUERY_SETTINGS.write().unwrap();

        settings.remove(&self.id);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Function called outside of a tokio thread.")]
    TokioThread,

    #[error("Failed to get lock.")]
    Lock,
}

pub fn before_query(pre_sql: &str) -> Result<Guard, SettingsError> {
    let id = tokio::task::try_id().ok_or(SettingsError::TokioThread)?;
    let mut settings = PER_THREAD_QUERY_SETTINGS
        .write()
        .map_err(|_| SettingsError::Lock)?;

    settings.insert(
        id,
        QuerySettings {
            pre_sql: Some(pre_sql.to_string()),
        },
    );
    drop(settings);

    Ok(Guard { id })
}
