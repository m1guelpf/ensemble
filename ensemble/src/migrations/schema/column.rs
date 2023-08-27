use std::{fmt::Display, sync::mpsc};

use ensemble_derive::Column;

use super::Schemable;

#[derive(Debug, Clone, Copy)]
pub enum Type {
    Uuid,
    Text,
    String,
    Boolean,
    Timestamp,
    BigInteger,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Uuid => "uuid",
                Self::Text => "text",
                Self::String => "varchar",
                Self::Boolean => "boolean",
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
    /// Automatically generate UUIDs for the column
    #[builder(type = Type::Uuid)]
    #[cfg(any(feature = "mysql", feature = "postgres"))]
    uuid: bool,
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
    tx: Option<mpsc::Sender<Schemable>>,
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

        #[cfg(any(feature = "mysql", feature = "postgres"))]
        if self.uuid {
            assert!(
                self.default.is_none(),
                "cannot set a default valud and automatically generate UUIDs at the same time"
            );

            #[cfg(feature = "mysql")]
            sql.push_str(" DEFAULT (UUID())");

            #[cfg(feature = "postgres")]
            sql.push_str(" DEFAULT (uuid_generate_v4())");
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
            tx.send(Schemable::Column(self.clone())).unwrap();
            drop(tx);
        }
    }
}
