use ensemble_derive::Column;
use itertools::Itertools;
use rbs::Value;
use std::{fmt::Display, sync::mpsc};

use super::Schemable;
use crate::connection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Json,
    Uuid,
    Text,
    Boolean,
    Timestamp,
    BigInteger,
    String(u32),
    Enum(Vec<String>),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => f.write_str("json"),
            Self::Uuid => f.write_str("uuid"),
            Self::Text => f.write_str("text"),
            Self::Boolean => f.write_str("boolean"),
            Self::BigInteger => f.write_str("bigint"),
            Self::Timestamp => f.write_str("timestamp"),
            Self::String(size) => {
                let value = format!("varchar({size})");
                f.write_str(&value)
            }
            Self::Enum(values) => {
                let value = format!(
                    "enum({})",
                    values
                        .iter()
                        .map(|v| format!("'{}'", v.replace('\'', "\\'")))
                        .join(", ")
                );
                f.write_str(&value)
            }
        }
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
    /// Set INTEGER columns as auto-increment (primary key)
    #[builder(rename = "increments", type = Type::BigInteger, needs = [primary, unique])]
    auto_increment: bool,
    /// Automatically generate UUIDs for the column
    #[builder(type = Type::Uuid)]
    uuid: bool,
    /// Add a comment to the column
    comment: Option<String>,
    /// Specify a "default" value for the column
    #[builder(skip)]
    default: Option<rbs::Value>,
    /// Add an index
    index: Option<String>,
    /// Allow NULL values to be inserted into the column
    nullable: bool,
    /// Add a primary index
    primary: bool,
    /// Add a unique index
    unique: bool,
    /// Set the INTEGER column as UNSIGNED
    #[cfg(feature = "mysql")]
    #[builder(type = Type::BigInteger)]
    unsigned: bool,
    /// Set the TIMESTAMP column to use CURRENT_TIMESTAMP as default value
    #[builder(type = Type::Timestamp)]
    use_current: bool,
    /// Set the TIMESTAMP column to use CURRENT_TIMESTAMP when updating
    #[cfg(feature = "mysql")]
    #[builder(type = Type::Timestamp)]
    use_current_on_update: bool,

    /// The channel to send the column to when it is dropped.
    #[builder(init)]
    tx: Option<mpsc::Sender<Schemable>>,
}

impl Column {
    /// Specify a "default" value for the column
    pub fn default<T: serde::Serialize>(mut self, default: T) -> Self {
        let value = if self.r#type == Type::Json {
            Value::String(serde_json::to_string(&default).unwrap())
        } else {
            rbs::to_value!(default)
        };

        if let Type::Enum(values) = &self.r#type {
            assert!(
                values.contains(&value.as_str().unwrap_or_default().to_string()),
                "default value must be one of the enum values"
            );
        }

        self.default = Some(value);

        self
    }

    pub(crate) fn to_sql(&self) -> String {
        let db_type = if connection::which_db().is_postgres()
            && self.r#type == Type::BigInteger
            && self.auto_increment
        {
            "bigserial".to_string()
        } else {
            self.r#type.to_string()
        };

        let mut sql = format!("{} {db_type}", self.name);

        #[cfg(feature = "mysql")]
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
            if let Type::Enum(values) = &self.r#type {
                assert!(
                    values.contains(&default.to_string()),
                    "default value must be one of the enum values"
                );
            }

            sql.push_str(&format!(" DEFAULT {default}"));
        }

        if self.uuid {
            assert!(
                self.default.is_none(),
                "cannot set a default valud and automatically generate UUIDs at the same time"
            );

            #[cfg(feature = "mysql")]
            sql.push_str(" DEFAULT (UUID())");

            #[cfg(feature = "postgres")]
            sql.push_str(" DEFAULT (gen_random_uuid())");
        }

        if self.auto_increment {
            #[cfg(feature = "mysql")]
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
            #[cfg(feature = "mysql")]
            sql.push_str(" DEFAULT CURRENT_TIMESTAMP");

            #[cfg(feature = "postgres")]
            sql.push_str(" DEFAULT now()");
        }

        #[cfg(feature = "mysql")]
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
