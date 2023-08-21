use std::{fmt::Display, rc::Rc};

use ensemble_derive::Column;
use rbs::to_value;
use std::sync::mpsc;

use crate::connection;

use super::{migrator::MIGRATE_CONN, Error};

pub struct Schema {}

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
        let columns = Self::get_schema(callback)?;
        let mut conn_lock = MIGRATE_CONN.try_lock().map_err(|_| Error::Lock)?;
        let mut conn = conn_lock.take().ok_or(Error::Lock)?;

        conn.exec(
            &format!(
                "CREATE TABLE {} ({}) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci",
                table_name,
                columns
                    .iter()
                    .map(Column::to_sql)
                    .collect::<Rc<_>>()
                    .join(", ")
            ),
            vec![],
        )
        .await
        .map_err(|e| Error::Database(e.to_string()))?;

        conn_lock.replace(conn);
        drop(conn_lock);

        Ok(())
    }

    /// Drops a table.
    ///
    /// # Errors
    ///
    /// Returns an error if the table cannot be dropped, or if a connection to the database cannot be established.
    pub async fn drop(table_name: &str) -> Result<(), Error> {
        let mut conn = connection::get().await?;

        conn.exec(&format!("DROP TABLE ?"), vec![to_value!(table_name)])
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    fn get_schema<F>(callback: F) -> Result<Vec<Column>, Error>
    where
        F: FnOnce(&mut Table),
    {
        let (tx, rx) = mpsc::channel();
        let mut table = Table { sender: Some(tx) };

        let ret = std::thread::spawn(move || {
            let mut columns = vec![];

            while let Ok(column) = rx.recv() {
                columns.push(column);
            }

            columns
        });

        callback(&mut table);
        drop(table.sender.take());

        ret.join().map_err(|_| Error::SendColumn)
    }
}

#[derive(Debug)]
pub struct Table {
    sender: Option<mpsc::Sender<Column>>,
}

impl Table {
    pub fn id(&mut self) -> Column {
        Column::new("id".to_string(), Type::BigInteger, self.sender.clone())
            .primary(true)
            .unsigned(true)
            .increments(true)
    }

    pub fn string(&mut self, name: &str) -> Column {
        Column::new(name.to_string(), Type::String, self.sender.clone()).length(Some(255))
    }

    pub fn timestamp(&mut self, name: &str) -> Column {
        Column::new(name.to_string(), Type::Timestamp, self.sender.clone())
    }

    pub fn timestamps(&mut self) {
        self.timestamp("created_at")
            .nullable(true)
            .use_current(true);

        self.timestamp("updated_at")
            .nullable(true)
            .use_current_on_update(true);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Type {
    String,
    Timestamp,
    BigInteger,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::String => "varchar",
                Self::BigInteger => "bigint",
                Self::Timestamp => "timestamp",
            }
        )
    }
}

/// A column in a table.
#[derive(Debug, Clone, Column)]
#[allow(clippy::struct_excessive_bools, dead_code)]
pub struct Column {
    /// The name of the column.
    #[builder(init)]
    name: String,
    /// The type of the column.
    #[builder(init)]
    r#type: Type,
    /// Place the column "after" another column
    after: Option<String>,
    /// The column's length (for string types)
    #[builder(type = Type::String)]
    length: Option<u32>,
    /// Set INTEGER columns as auto-increment (primary key)
    #[builder(rename = "increments", type = Type::BigInteger, needs = [primary, unique])]
    auto_increment: bool,
    /// Add a comment to the column
    comment: Option<String>,
    /// Specify a "default" value for the column
    default: Option<rbs::Value>,
    /// Add an index
    index: Option<String>,
    /// Allow NULL values to be inserted into the column
    nullable: bool,
    /// Specify a collation for the column
    #[builder(type = Type::String)]
    collation: Option<String>,
    /// Add a primary index
    primary: bool,
    /// Add a unique index
    unique: bool,
    /// Set the INTEGER column as UNSIGNED
    #[builder(type = Type::BigInteger)]
    unsigned: bool,
    /// Set the TIMESTAMP column to use CURRENT_TIMESTAMP as default value
    #[builder(type = Type::Timestamp)]
    use_current: bool,
    /// Set the TIMESTAMP column to use CURRENT_TIMESTAMP when updating
    #[builder(type = Type::Timestamp)]
    use_current_on_update: bool,

    /// The channel to send the column to when it is dropped.
    #[builder(init)]
    tx: Option<mpsc::Sender<Column>>,
}

impl Column {
    pub(crate) fn to_sql(&self) -> String {
        let mut sql = format!("{} {}", self.name, self.r#type);

        if let Some(length) = self.length {
            sql.push_str(&format!("({length})"));
        }

        if self.unsigned {
            sql.push_str(" unsigned");
        }

        if self.nullable {
            sql.push_str(" NULL");
        } else {
            sql.push_str(" NOT NULL");
        }

        if let Some(after) = &self.after {
            sql.push_str(&format!(" AFTER {after}"));
        }

        if let Some(comment) = &self.comment {
            sql.push_str(&format!(" COMMENT {comment}"));
        }

        if let Some(default) = &self.default {
            sql.push_str(&format!(" DEFAULT {default}"));
        }

        if self.auto_increment {
            sql.push_str(" AUTO_INCREMENT");
        }

        if let Some(index) = &self.index {
            sql.push_str(&format!(" INDEX {index}"));
        }

        if self.primary {
            sql.push_str(" PRIMARY KEY");
        }

        if self.unique {
            sql.push_str(" UNIQUE");
        }

        if self.use_current {
            sql.push_str(" DEFAULT CURRENT_TIMESTAMP");
        }

        if self.use_current_on_update {
            sql.push_str(" ON UPDATE CURRENT_TIMESTAMP");
        }

        sql
    }
}

// Incredibly cursed impl that basically recreates PHP's `__destruct` magic method.
// If you're mad about this, go use sqlx or something idk.
impl Drop for Column {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            tx.send(self.clone()).unwrap();
            drop(tx);
        }
    }
}
