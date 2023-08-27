use std::{fmt::Display, sync::mpsc};

use ensemble_derive::Column;

use super::Schemable;

#[derive(Debug)]
pub struct Command {
    sql: String,
}

impl Command {
    pub fn to_sql(&self) -> String {
        self.sql.clone()
    }
}

#[derive(Debug, Clone, Column)]
#[allow(dead_code)]
pub struct ForeignIndex {
    #[builder(init)]
    column: String,
    name: Option<String>,
    #[builder(rename = "references")]
    foreign_column: Option<String>,
    #[builder(rename = "on")]
    table: String,
    #[builder(into)]
    on_delete: Option<OnAction>,
    #[builder(into)]
    on_update: Option<OnAction>,

    #[builder(init)]
    tx: Option<mpsc::Sender<Schemable>>,
}

impl ForeignIndex {
    fn to_sql(&self) -> String {
        let foreign_column = &self
            .foreign_column
            .as_ref()
            .expect("failed to build index: foreign column must be specified");

        let index_name = self.name.as_ref().map_or_else(
            || format!("{}_{}_foreign", self.table, self.column),
            ToString::to_string,
        );

        let mut sql = format!(
            "KEY {index_name} ({}), CONSTRAINT {index_name} FOREIGN KEY ({}) REFERENCES {}({foreign_column})", self.column, self.column, self.table,
        );

        if let Some(on_delete) = &self.on_delete {
            sql.push_str(&format!(" ON DELETE {on_delete}"));
        }

        if let Some(on_update) = &self.on_update {
            sql.push_str(&format!(" ON UPDATE {on_update}"));
        }

        sql
    }
}

// Incredibly cursed impl that basically recreates PHP's `__destruct` magic method.
// If you're mad about this, go use sqlx or something idk.
impl Drop for ForeignIndex {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            tx.send(Schemable::Command(Command { sql: self.to_sql() }))
                .unwrap();
            drop(tx);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OnAction {
    Restrict,
    Cascade,
    SetNull,
}

impl Display for OnAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cascade => write!(f, "CASCADE"),
            Self::SetNull => write!(f, "SET NULL"),
            Self::Restrict => write!(f, "RESTRICT"),
        }
    }
}

#[allow(clippy::fallible_impl_from)]
impl From<&str> for OnAction {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "CASCADE" => Self::Cascade,
            "SET NULL" => Self::SetNull,
            "RESTRICT" => Self::Restrict,
            _ => panic!("invalid action"),
        }
    }
}
