use inflector::Inflector;
use itertools::{Either, Itertools};
use rbs::Value;
use std::{any::type_name, sync::mpsc};

use self::{
	column::{Column, Type},
	command::{Command, ForeignIndex},
};
use super::{migrator::MIGRATE_CONN, Error};
use crate::{
	connection::{self, Database},
	Model,
};

pub use column::Column;
pub use command::ForeignIndex;

mod column;
mod command;

/// A database schema.
pub struct Schema {}

pub(crate) enum Schemable {
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
		let (table, columns, commands) = Self::get_schema(table_name.to_string(), callback)?;
		let mut conn_lock = MIGRATE_CONN.try_lock().map_err(|_| Error::Lock)?;
		let mut conn = conn_lock.take().ok_or(Error::Lock)?;

		#[cfg(not(feature = "mysql"))]
		let db_config = String::new();
		#[cfg(feature = "mysql")]
		let db_config = format!(
			"ENGINE=InnoDB DEFAULT CHARSET={} COLLATE={}",
			table.charset, table.collation
		);

		let sql = format!(
			"CREATE TABLE {table_name} ({columns}) {db_config}; {commands}",
			columns = columns
				.iter()
				.map(Column::to_sql)
				.chain(commands.iter().filter_map(|cmd| cmd.inline_sql.clone()))
				.join(", "),
			db_config = db_config,
			commands = commands
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

	/// Alters a table.
	///
	/// # Errors
	///
	/// Returns an error if the table cannot be altered, or if a connection to the database cannot be established.
	pub async fn table<F>(table_name: &str, callback: F) -> Result<(), Error>
	where
		F: FnOnce(&mut Table) + Send,
	{
		let (_, columns, commands) = Self::get_schema(table_name.to_string(), callback)?;
		let mut conn_lock = MIGRATE_CONN.try_lock().map_err(|_| Error::Lock)?;
		let mut conn = conn_lock.take().ok_or(Error::Lock)?;

		let sql = format!(
			"ALTER TABLE {} {};",
			table_name,
			match connection::which_db() {
				Database::MySQL => format!(
					"{}",
					columns
						.iter()
						.map(|c| format!("ADD {}", c.to_sql()))
						.join(", ")
				),
				Database::PostgreSQL => {
					format!(
						"{}",
						columns
							.iter()
							.map(|c| format!("ADD COLUMN {}", c.to_sql()))
							.join(", ")
					)
				},
			}
		);

		tracing::debug!(sql = sql.as_str(), "Running ALTER TABLE SQL query");
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
			"DROP TABLE ?".to_string(),
			vec![Value::String(table_name.to_string())],
		);

		tracing::debug!(sql = sql, bindings = ?bindings, "Running DROP TABLE SQL query");
		let query_result = conn.exec(sql, bindings).await;

		conn_lock.replace(conn);
		drop(conn_lock);

		match query_result {
			Ok(_) => Ok(()),
			Err(e) => Err(Error::Database(e.to_string())),
		}
	}

	/// Drops a table if it exists.
	///
	/// # Errors
	///
	/// Returns an error if the table cannot be dropped, or if a connection to the database cannot be established.
	pub async fn drop_if_exists(table_name: &str) -> Result<(), Error> {
		let mut conn_lock = MIGRATE_CONN.try_lock().map_err(|_| Error::Lock)?;
		let mut conn = conn_lock.take().ok_or(Error::Lock)?;

		let (sql, bindings) = (
			"DROP TABLE IF EXISTS ?".to_string(),
			vec![Value::String(table_name.to_string())],
		);

		tracing::debug!(sql = sql.as_str(), bindings = ?bindings, "Running DROP TABLE IF EXISTS SQL query");
		let query_result = conn.exec(&sql, bindings).await;

		conn_lock.replace(conn);
		drop(conn_lock);

		match query_result {
			Ok(_) => Ok(()),
			Err(e) => Err(Error::Database(e.to_string())),
		}
	}

	/// Renames a table.
	///
	/// # Errors
	///
	/// Returns an error if the table cannot be renamed, or if a connection to the database cannot be established.
	pub async fn rename(old_name: &str, new_name: &str) -> Result<(), Error> {
		let mut conn_lock = MIGRATE_CONN.try_lock().map_err(|_| Error::Lock)?;
		let mut conn = conn_lock.take().ok_or(Error::Lock)?;

		let (sql, bindings) = (
			match connection::which_db() {
				Database::MySQL => "RENAME TABLE ? TO ?".to_string(),
				Database::PostgreSQL => "ALTER TABLE ? RENAME TO ?".to_string(),
			},
			vec![
				Value::String(old_name.to_string()),
				Value::String(new_name.to_string()),
			],
		);

		tracing::debug!(sql = sql.as_str(), bindings = ?bindings, "Running RENAME TABLE SQL query");
		let query_result = conn.exec(&sql, bindings).await;

		conn_lock.replace(conn);
		drop(conn_lock);

		match query_result {
			Ok(_) => Ok(()),
			Err(e) => Err(Error::Database(e.to_string())),
		}
	}

	fn get_schema<F>(
		table_name: String,
		callback: F,
	) -> Result<(Table, Vec<Column>, Vec<Command>), Error>
	where
		F: FnOnce(&mut Table),
	{
		let (tx, rx) = mpsc::channel();
		let mut table = Table {
			name: table_name,
			sender: Some(tx),
			#[cfg(feature = "mysql")]
			charset: "utf8mb4".to_string(),
			#[cfg(feature = "mysql")]
			collation: "utf8mb4_unicode_ci".to_string(),
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

		let (columns, commands) = schema
			.into_iter()
			.map(|part| match part {
				Schemable::Column(col) => Either::Left(col),
				Schemable::Command(cmd) => Either::Right(cmd),
			})
			.partition_map(|part| part);

		Ok((table, columns, commands))
	}
}

/// A database table.
#[derive(Debug)]
pub struct Table {
	name: String,
	sender: Option<mpsc::Sender<Schemable>>,
	#[cfg(feature = "mysql")]
	/// The character set of the table.
	pub charset: String,
	#[cfg(feature = "mysql")]
	/// The collation of the table.
	pub collation: String,
}

impl Table {
	/// Creates a primary auto-incrementing `UNSIGNED BIGINT` equivalent column called `id`.
	pub fn id(&mut self) -> Column {
		let column = Column::new("id".to_string(), Type::BigInteger, self.sender.clone())
			.primary(true)
			.increments(true);

		#[cfg(feature = "mysql")]
		{
			column.unsigned(true)
		}

		#[cfg(not(feature = "mysql"))]
		{
			column
		}
	}

	/// Create a `UUID` column.
	pub fn uuid(&mut self, name: &str) -> Column {
		Column::new(name.to_string(), Type::Uuid, self.sender.clone()).uuid(true)
	}

	/// Create a new big integer (8-byte) column.
	pub fn integer(&mut self, name: &str) -> Column {
		Column::new(name.to_string(), Type::BigInteger, self.sender.clone())
	}

	/// Create a new JSON column.
	pub fn json(&mut self, name: &str) -> Column {
		Column::new(name.to_string(), Type::Json, self.sender.clone())
	}

	/// Create a `VARCHAR(255)` equivalent column.
	pub fn string(&mut self, name: &str) -> Column {
		Column::new(name.to_string(), Type::String(255), self.sender.clone())
	}

	/// Create a `BOOLEAN` equivalent column.
	pub fn boolean(&mut self, name: &str) -> Column {
		Column::new(name.to_string(), Type::Boolean, self.sender.clone())
	}

	/// Create a `TEXT` equivalent column.
	pub fn text(&mut self, name: &str) -> Column {
		Column::new(name.to_string(), Type::Text, self.sender.clone())
	}

	/// Create a `TIMESTAMP` equivalent column.
	pub fn timestamp(&mut self, name: &str) -> Column {
		Column::new(name.to_string(), Type::Timestamp, self.sender.clone())
	}

	/// Specify a foreign key for the table.
	pub fn foreign(&mut self, column: &str) -> ForeignIndex {
		ForeignIndex::new(column.to_string(), self.name.clone(), self.sender.clone())
	}

	/// create an `ENUM` equivalent column with the given valid values.
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

	/// Creates a foreign key column for the given model.
	/// The type of the column will be `BIGINT` for numeric primary keys, and `VARCHAR(255)` otherwise. Use `foreign` to specify a custom column type.
	/// The foreign key will point to the primary column and table of the given model.
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

	/// Create an `UNSIGNED BIGINT` equivalent column and a foreign key for it.
	/// Ensemble will attempt to infer the foreign table and reference column from the column name if the column name is of the form `resource_id`.
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

	/// Drop a column from the table.
	/// This will also drop any foreign keys referencing the column.
	pub fn drop_column(&mut self, name: &str) {
		self.sender
			.as_ref()
			.unwrap()
			.send(Command::from_sql(format!("DROP COLUMN {name}")).into())
			.unwrap();
	}

	/// Create a `UUID` equivalent column and add a foreign key for it.
	/// Ensemble will attempt to infer the foreign table and reference column from the column name if the column name is of the form `resource_id`.
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
