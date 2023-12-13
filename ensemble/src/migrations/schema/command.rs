use std::{fmt::Display, sync::mpsc};

use ensemble_derive::Column;

use crate::connection::{self, Database};

use super::Schemable;

#[derive(Debug)]
pub struct Command {
	pub(crate) inline_sql: String,
	pub(crate) post_sql: Option<String>,
}

/// A foreign key constraint.
#[derive(Debug, Clone, Column)]
#[allow(dead_code)]
pub struct ForeignIndex {
	#[builder(init)]
	column: String,
	#[builder(init)]
	origin_table: String,
	/// The name of the foreign index.
	name: Option<String>,
	/// The name of the column in the foreign table.
	#[builder(rename = "references")]
	foreign_column: Option<String>,
	/// The name of the foreign table.
	#[builder(rename = "on")]
	table: String,
	/// The action to take when the foreign row is deleted.
	#[builder(into)]
	on_delete: Option<OnAction>,
	/// The action to take when the foreign row is updated.
	#[builder(into)]
	on_update: Option<OnAction>,

	#[builder(init)]
	tx: Option<mpsc::Sender<Schemable>>,
}

impl ForeignIndex {
	fn to_sql(&self) -> (String, Option<String>) {
		let foreign_column = &self
			.foreign_column
			.as_ref()
			.expect("failed to build index: foreign column must be specified");

		let index_name = self.name.as_ref().map_or_else(
			|| format!("{}_{}_foreign", self.origin_table, self.column),
			ToString::to_string,
		);

		let mut sql = match connection::which_db() {
            Database::MySQL => format!(
                "KEY {index_name} ({}), CONSTRAINT {index_name} FOREIGN KEY ({}) REFERENCES {}({foreign_column})", self.column, self.column, self.table,
            ),
            Database::PostgreSQL => format!(
                "FOREIGN KEY ({}) REFERENCES {}({foreign_column})",
                self.column, self.table,
            )
        };

		if let Some(on_delete) = &self.on_delete {
			sql.push_str(&format!(" ON DELETE {on_delete}"));
		}

		if let Some(on_update) = &self.on_update {
			sql.push_str(&format!(" ON UPDATE {on_update}"));
		}

		match connection::which_db() {
			Database::MySQL => (sql, None),
			Database::PostgreSQL => (
				sql,
				Some(format!(
					"CREATE INDEX {index_name} ON {}({});",
					self.origin_table, self.column
				)),
			),
		}
	}
}

// Incredibly cursed impl that basically recreates PHP's `__destruct` magic method.
// If you're mad about this, go use sqlx or something idk.
impl Drop for ForeignIndex {
	fn drop(&mut self) {
		if let Some(tx) = self.tx.take() {
			let (inline_sql, post_sql) = self.to_sql();

			tx.send(Schemable::Command(Command {
				inline_sql,
				post_sql,
			}))
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
