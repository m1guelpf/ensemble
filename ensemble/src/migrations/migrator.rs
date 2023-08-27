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

        Ok(Self {
            state,
            batch,
            connection: conn,
            migrations: HashMap::new(),
        })
    }

    pub fn register(&mut self, name: String, migration: Box<dyn Migration>) {
        self.migrations.insert(name, migration);
    }

    /// Runs the migrations.
    ///
    /// # Errors
    ///
    /// Returns an error if the migrations fail, or if a connection to the database cannot be established.
    pub async fn run(mut self) -> Result<(), Error> {
        for (name, migration) in &self.migrations {
            if self.state.iter().any(|m| &m.migration == name) {
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
        }

        Ok(())
    }

    async fn get_state(conn: &mut Connection) -> Result<Vec<StoredMigration>, Error> {
        conn.exec(
            "create table if not exists migrations (
                id int unsigned not null auto_increment primary key,
                migration varchar(255) not null,
                batch int not null
            )",
            vec![],
        )
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

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct StoredMigration {
    id: u64,
    batch: u64,
    migration: String,
}
