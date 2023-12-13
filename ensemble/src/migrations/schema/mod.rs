use inflector::Inflector;
use itertools::{Either, Itertools};
use rbs::Value;
use std::{any::type_name, sync::mpsc};

use self::{
    column::{Column, Type},
    command::{Command, ForeignIndex},
};
use super::{migrator::MIGRATE_CONN, Error};
use crate::{connection, Model};

mod column;
mod command;

pub struct Schema {}

pub enum Schemable {
    Column(Column),
    Command(Command),
}

impl Schema {
    /// Creates a new table.
    ///
    /// # Errors
    ///
    /// Returns an error if the table cannot be created, or if a connection to the database cannot be established.
    #[allow(clippy::unused_async)]
    pub async fn create<F>(table_name: &str, callback: F) -> Result<(), Error>
    where
        F: FnOnce(&mut Table) + Send,
    {
        let (columns, commands) = Self::get_schema(table_name.to_string(), callback)?;
        let mut conn_lock = MIGRATE_CONN.try_lock().map_err(|_| Error::Lock)?;
        let mut conn = conn_lock.take().ok_or(Error::Lock)?;

        let sql = format!(
            "CREATE TABLE {} ({}) {}; {}",
            table_name,
            columns
                .iter()
                .map(Column::to_sql)
                .chain(commands.iter().map(|cmd| cmd.inline_sql.clone()))
                .join(", "),
            if connection::which_db().is_mysql() {
                "ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci"
            } else {
                ""
            },
            commands
                .iter()
                .filter_map(|cmd| cmd.post_sql.clone())
                .join("\n")
        );

        tracing::debug!(sql = sql.as_str(), "Running CREATE TABLE SQL query");
        let query_result = conn.exec(&sql, vec![]).await;

        conn_lock.replace(conn);
        drop(conn_lock);

        match query_result {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::Database(e.to_string())),
        }
    }

    /// Drops a table.
    ///
    /// # Errors
    ///
    /// Returns an error if the table cannot be dropped, or if a connection to the database cannot be established.
    pub async fn drop(table_name: &str) -> Result<(), Error> {
        let mut conn_lock = MIGRATE_CONN.try_lock().map_err(|_| Error::Lock)?;
        let mut conn = conn_lock.take().ok_or(Error::Lock)?;

        let (sql, bindings) = (
            format!("DROP TABLE ?"),
            vec![Value::String(table_name.to_string())],
        );

        tracing::debug!(sql = sql.as_str(), bindings = ?bindings, "Running DROP TABLE SQL query");
        let query_result = conn.exec(&sql, bindings).await;

        conn_lock.replace(conn);
        drop(conn_lock);

        match query_result {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::Database(e.to_string())),
        }
    }

    fn get_schema<F>(table_name: String, callback: F) -> Result<(Vec<Column>, Vec<Command>), Error>
    where
        F: FnOnce(&mut Table),
    {
        let (tx, rx) = mpsc::channel();
        let mut table = Table {
            name: table_name,
            sender: Some(tx),
        };

        let ret = std::thread::spawn(move || {
            let mut schema = vec![];

            while let Ok(part) = rx.recv() {
                schema.push(part);
            }

            schema
        });

        callback(&mut table);
        drop(table.sender.take());

        let schema = ret.join().map_err(|_| Error::SendColumn)?;

        Ok(schema
            .into_iter()
            .map(|part| match part {
                Schemable::Column(col) => Either::Left(col),
                Schemable::Command(cmd) => Either::Right(cmd),
            })
            .partition_map(|part| part))
    }
}

#[derive(Debug)]
pub struct Table {
    name: String,
    sender: Option<mpsc::Sender<Schemable>>,
}

impl Table {
    /// Creates a primary key incrementing integer column called `id`.
    pub fn id(&mut self) -> Column {
        let column = Column::new("id".to_string(), Type::BigInteger, self.sender.clone())
            .primary(true)
            .increments(true);

        #[cfg(feature = "mysql")]
        {
            return column.unsigned(true);
        }

        #[cfg(not(feature = "mysql"))]
        {
            column
        }
    }

    /// Create a primary key UUID column called `id`.
    pub fn uuid(&mut self) -> Column {
        Column::new("id".to_string(), Type::Uuid, self.sender.clone())
            .uuid(true)
            .primary(true)
    }

    /// Create a new big integer (8-byte) column on the table.
    pub fn integer(&mut self, name: &str) -> Column {
        Column::new(name.to_string(), Type::BigInteger, self.sender.clone())
    }

    /// Create a new json column on the table.
    pub fn json(&mut self, name: &str) -> Column {
        Column::new(name.to_string(), Type::Json, self.sender.clone())
    }

    /// Create a new string column on the table.
    pub fn string(&mut self, name: &str) -> Column {
        Column::new(name.to_string(), Type::String(255), self.sender.clone())
    }

    /// Create a new boolean column on the table.
    pub fn boolean(&mut self, name: &str) -> Column {
        Column::new(name.to_string(), Type::Boolean, self.sender.clone())
    }

    /// Create a new text column on the table.
    pub fn text(&mut self, name: &str) -> Column {
        Column::new(name.to_string(), Type::Text, self.sender.clone())
    }

    /// Create a new timestamp column on the table.
    pub fn timestamp(&mut self, name: &str) -> Column {
        Column::new(name.to_string(), Type::Timestamp, self.sender.clone())
    }

    /// Specify a foreign key for the table.
    pub fn foreign(&mut self, column: &str) -> ForeignIndex {
        ForeignIndex::new(column.to_string(), self.name.clone(), self.sender.clone())
    }

    #[cfg(feature = "mysql")]
    /// Create a new enum column on the table.
    pub fn r#enum(&mut self, name: &str, values: &[&str]) -> Column {
        Column::new(
            name.to_string(),
            Type::Enum(
                name.to_string(),
                values.iter().map(ToString::to_string).collect(),
            ),
            self.sender.clone(),
        )
    }

    /// Create a foreign ID column for the given model.
    pub fn foreign_id_for<M: Model>(&mut self) -> ForeignIndex {
        let column = format!("{}_{}", M::NAME, M::PRIMARY_KEY).to_snake_case();

        if ["u64", "u32", "u16", "u8", "usize"].contains(&type_name::<M::PrimaryKey>()) {
            #[allow(unused_variables)]
            let column = Column::new(column.clone(), Type::BigInteger, self.sender.clone());

            #[cfg(feature = "mysql")]
            {
                column.unsigned(true);
            };
        } else {
            Column::new(column.clone(), Type::String(255), self.sender.clone());
        }

        let index = ForeignIndex::new(column, self.name.clone(), self.sender.clone());
        index.on(M::TABLE_NAME).references(M::PRIMARY_KEY)
    }

    /// Create a foreign ID column for the given model.
    pub fn foreign_id(&mut self, name: &str) -> ForeignIndex {
        #[allow(unused_variables)]
        let column = Column::new(name.to_string(), Type::BigInteger, self.sender.clone());

        #[cfg(feature = "mysql")]
        {
            column.unsigned(true);
        };

        let index = ForeignIndex::new(name.to_string(), self.name.clone(), self.sender.clone());

        // if the column name is of the form `resource_id`, we extract and set the table name and foreign column name
        if let Some((resource, column)) = name.split_once('_') {
            index.on(&resource.to_plural()).references(column)
        } else {
            index
        }
    }

    /// Create a foreign UUID column for the given model.
    pub fn foreign_uuid(&mut self, name: &str) -> ForeignIndex {
        Column::new(name.to_string(), Type::Uuid, self.sender.clone()).uuid(true);
        let index = ForeignIndex::new(name.to_string(), self.name.clone(), self.sender.clone());

        // if the column name is of the form `resource_id`, we extract and set the table name and foreign column name
        if let Some((resource, column)) = name.split_once('_') {
            index.on(&resource.to_plural()).references(column)
        } else {
            index
        }
    }

    /// Add nullable creation and update timestamps to the table.
    pub fn timestamps(&mut self) {
        self.timestamp("created_at")
            .nullable(true)
            .use_current(true);

        #[allow(unused_variables)]
        let updated_at = self.timestamp("updated_at").nullable(true);

        #[cfg(feature = "mysql")]
        {
            updated_at.use_current_on_update(true);
        }
    }
}
