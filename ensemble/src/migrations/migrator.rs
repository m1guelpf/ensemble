use std::collections::HashMap;

use rbs::{from_value, to_value};
use tokio::sync::Mutex;

use super::{Error, Migration};
use crate::connection::{self, Connection};

pub static MIGRATE_CONN: Mutex<Option<Connection>> = Mutex::const_new(None);

pub struct Migrator {
    batch: u64,
    connection: Connection,
    state: Vec<StoredMigration>,
    migrations: HashMap<String, Box<dyn Migration>>,
}

impl Migrator {
    /// Creates a new [`Migrator`].
    ///
    /// # Errors
    ///
    /// Returns an error if a connection to the database cannot be established, or if the migrations cannot be retrieved.
    pub async fn new() -> Result<Self, Error> {
        let mut conn = connection::get().await?;
        let state = Self::get_state(&mut conn).await?;
        let batch = state
            .iter()
            .map(|m| m.batch)
            .max()
            .unwrap_or_default()
            .saturating_add(1);

        tracing::debug!(
            batch = batch,
            state = ?state,
            "Loaded migration state from database."
        );

        Ok(Self {
            state,
            batch,
            connection: conn,
            migrations: HashMap::new(),
        })
    }

    pub fn register(&mut self, name: String, migration: Box<dyn Migration>) {
        tracing::trace!("Registered migration [{name}]");

        self.migrations.insert(name, migration);
    }

    /// Returns a list of migrations that have been run.
    #[must_use]
    pub fn status(&self) -> Vec<StoredMigration> {
        self.state.clone()
    }

    /// Returns a list of migrations that have not been run.
    #[must_use]
    pub fn pending(&self) -> HashMap<&str, &dyn Migration> {
        self.migrations
            .iter()
            .filter(|(name, _)| !self.state.iter().any(|m| &&m.migration == name))
            .map(|(name, migration)| (name.as_str(), migration.as_ref()))
            .collect()
    }

    /// Runs the migrations.
    ///
    /// # Errors
    ///
    /// Returns an error if the migrations fail, or if a connection to the database cannot be established.
    pub async fn run(mut self) -> Result<(), Error> {
        for (name, migration) in &self.migrations {
            if self.state.iter().any(|m| &m.migration == name) {
                tracing::trace!("Skipping migration [{name}], since it's already been run.");
                continue;
            }

            self.connection
                .exec("begin", vec![])
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

            MIGRATE_CONN
                .try_lock()
                .map_err(|_| Error::Lock)?
                .replace(self.connection);

            migration.up().await?;

            self.connection = MIGRATE_CONN
                .try_lock()
                .map_err(|_| Error::Lock)?
                .take()
                .ok_or(Error::Lock)?;

            self.connection
                .exec(
                    "insert into migrations (migration, batch) values (?, ?)",
                    vec![to_value!(&name), to_value!(&self.batch)],
                )
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

            self.connection
                .exec("commit", vec![])
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

            self.state.push(StoredMigration {
                id: 0,
                batch: self.batch,
                migration: name.to_string(),
            });

            tracing::info!("Successfully ran migration [{name}].");
        }

        Ok(())
    }

    /// Rolls back all of the migrations.
    ///
    /// # Errors
    ///
    /// Returns an error if the migrations fail, or if a connection to the database cannot be established.
    pub async fn rollback(mut self, batches: u64) -> Result<(), Error> {
        let migrations = self
            .state
            .into_iter()
            .filter(|m| m.batch >= self.batch.saturating_sub(batches))
            .rev();

        for record in migrations {
            let migration = self
                .migrations
                .get(&record.migration)
                .ok_or_else(|| Error::NotFound(record.migration.clone()))?;

            self.connection
                .exec("begin", vec![])
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

            MIGRATE_CONN
                .try_lock()
                .map_err(|_| Error::Lock)?
                .replace(self.connection);

            migration.down().await?;

            self.connection = MIGRATE_CONN
                .try_lock()
                .map_err(|_| Error::Lock)?
                .take()
                .ok_or(Error::Lock)?;

            self.connection
                .exec(
                    "delete from migrations where id = ?",
                    vec![to_value!(&record.id)],
                )
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

            self.connection
                .exec("commit", vec![])
                .await
                .map_err(|e| Error::Database(e.to_string()))?;

            tracing::info!("Successfully rolled back migration [{}].", record.migration);
        }

        Ok(())
    }

    async fn get_state(conn: &mut Connection) -> Result<Vec<StoredMigration>, Error> {
        let sql = migrations_table_query();

        tracing::debug!(sql = sql, "Running CREATE TABLE IF NOT EXISTS SQL query");

        conn.exec(sql, vec![])
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(conn
            .get_values("select * from migrations", vec![])
            .await
            .map_err(|e| Error::Database(e.to_string()))?
            .into_iter()
            .map(from_value)
            .collect::<Result<Vec<_>, _>>()?)
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct StoredMigration {
    id: u64,
    batch: u64,
    migration: String,
}

fn migrations_table_query() -> &'static str {
    #[cfg(feature = "mysql")]
    return "create table if not exists migrations (
        id int unsigned not null auto_increment primary key,
        migration varchar(255) not null,
        batch int not null
    )";

    #[cfg(feature = "postgres")]
    return "create table if not exists migrations (
        id serial primary key,
        migration varchar(255) not null,
        batch int not null
    )";

    #[cfg(all(not(feature = "mysql"), not(feature = "postgres")))]
    panic!("either the `mysql` or `postgres` feature must be enabled to use migrations.");
}
