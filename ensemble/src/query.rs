use itertools::Itertools;
use rbs::Value;
use serde::Serialize;
use std::{
	collections::{HashMap, HashSet},
	fmt::Display,
};

use crate::{
	connection::{self, Database},
	value, Error, Model,
};

/// The Query Builder.
#[derive(Debug)]
pub struct Builder {
	table: String,
	join: Vec<Join>,
	order: Vec<Order>,
	limit: Option<usize>,
	offset: Option<usize>,
	r#where: Vec<WhereClause>,
	eager_load: HashSet<String>,
}

impl Builder {
	pub(crate) fn new(table: String) -> Self {
		Self {
			table,
			limit: None,
			offset: None,
			join: vec![],
			order: vec![],
			r#where: vec![],
			eager_load: HashSet::new(),
		}
	}

	/// Execute a raw SQL query and return the results.
	///
	/// # Safety
	///
	/// This method is unsafe because it allows for arbitrary SQL to be executed, which can lead to SQL injection.
	/// It is recommended to build queries using the methods provided by the query builder instead.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async unsafe fn raw_sql(sql: &str, bindings: Vec<Value>) -> Result<Vec<Value>, Error> {
		let mut conn = connection::get().await?;

		conn.get_values(sql, bindings)
			.await
			.map_err(|e| Error::Database(e.to_string()))
	}

	/// Set the table which the query is targeting.
	#[must_use]
	pub fn from(mut self, table: &str) -> Self {
		self.table = table.to_string();
		self
	}

	/// Apply the given callback to the builder if the provided condition is true.
	#[must_use]
	pub fn when(mut self, condition: bool, r#fn: impl FnOnce(Self) -> Self) -> Self {
		if condition {
			self = r#fn(self);
		}

		self
	}

	/// Apply the given callback to the builder if the provided [`Option`] is `Some`.
	#[must_use]
	pub fn when_some<T>(mut self, value: Option<T>, r#fn: impl FnOnce(Self, T) -> Self) -> Self {
		if let Some(value) = value {
			self = r#fn(self, value);
		}

		self
	}

	/// Add a basic where clause to the query.
	///
	/// # Panics
	///
	/// Panics if the provided value cannot be serialized.
	#[must_use]
	pub fn r#where<T, Op>(mut self, column: &str, operator: Op, value: T) -> Self
	where
		Op: Into<Operator>,
		T: serde::Serialize,
	{
		self.r#where.push(WhereClause::Simple(Where {
			boolean: Boolean::And,
			operator: operator.into(),
			column: Columns::escape(column),
			value: Some(value::for_db(value).unwrap()),
		}));

		self
	}

	/// Set the "limit" value of the query.
	#[must_use]
	pub const fn limit(mut self, take: usize) -> Self {
		self.limit = Some(take);
		self
	}

	/// Set the "offset" value of the query.
	#[must_use]
	pub const fn offset(mut self, skip: usize) -> Self {
		self.offset = Some(skip);
		self
	}

	/// Set the relationships that should be eager loaded.
	#[must_use]
	pub fn with<T: Into<EagerLoad>>(mut self, relations: T) -> Self {
		self.eager_load.extend(relations.into().list());

		self
	}

	/// Add an "or where" clause to the query.
	///
	/// # Panics
	///
	/// Panics if this is the first where clause.
	#[must_use]
	pub fn or_where<T, Op>(mut self, column: &str, op: Op, value: T) -> Self
	where
		T: Into<Value>,
		Op: Into<Operator>,
	{
		assert!(
			!self.r#where.is_empty(),
			"Cannot use or_where without a where clause."
		);

		self.r#where.push(WhereClause::Simple(Where {
			operator: op.into(),
			boolean: Boolean::Or,
			value: Some(value.into()),
			column: Columns::escape(column),
		}));

		self
	}

	/// Add a "where not null" clause to the query.
	#[must_use]
	pub fn where_not_null(mut self, column: &str) -> Self {
		self.r#where.push(WhereClause::Simple(Where {
			value: None,
			boolean: Boolean::And,
			operator: Operator::NotNull,
			column: Columns::escape(column),
		}));

		self
	}

	// Add a "where in" clause to the query.
	#[must_use]
	pub fn where_in<T>(mut self, column: &str, values: Vec<T>) -> Self
	where
		T: Into<Value>,
	{
		self.r#where.push(WhereClause::Simple(Where {
			boolean: Boolean::And,
			operator: Operator::In,
			column: Columns::escape(column),
			value: Some(Value::Array(values.into_iter().map(Into::into).collect())),
		}));

		self
	}

	/// Add a "where is null" clause to the query.
	#[must_use]
	pub fn where_null(mut self, column: &str) -> Self {
		self.r#where.push(WhereClause::Simple(Where {
			value: None,
			boolean: Boolean::And,
			operator: Operator::IsNull,
			column: Columns::escape(column),
		}));

		self
	}

	/// Add an inner join to the query.
	#[must_use]
	pub fn join<Op: Into<Operator>>(
		mut self,
		column: &str,
		first: &str,
		op: Op,
		second: &str,
	) -> Self {
		self.join.push(Join {
			operator: op.into(),
			r#type: JoinType::Inner,
			first: first.to_string(),
			second: second.to_string(),
			column: Columns::escape(column),
		});

		self
	}

	/// Add an "order by" clause to the query.
	#[must_use]
	pub fn order_by<Dir: Into<Direction>>(mut self, column: &str, direction: Dir) -> Self {
		self.order.push(Order {
			direction: direction.into(),
			column: Columns::escape(column),
		});

		self
	}

	/// Logically group a set of where clauses.
	#[must_use]
	pub fn where_group(mut self, r#fn: impl FnOnce(Self) -> Self) -> Self {
		let builder = r#fn(Self::new(self.table.clone()));

		self.r#where
			.push(WhereClause::Group(builder.r#where, Boolean::And));

		self
	}

	/// Get the SQL representation of the query.
	#[must_use]
	pub fn to_sql(&self, r#type: Type) -> String {
		let mut sql = match r#type {
			Type::Update => String::new(), // handled in update()
			Type::Delete => format!("DELETE FROM {}", self.table),
			Type::Select => format!("SELECT * FROM {}", self.table),
			Type::Count => format!("SELECT COUNT(*) FROM {}", self.table),
		};

		if !self.join.is_empty() {
			for join in &self.join {
				sql.push_str(&format!(
					" {} {} ON {} {} {}",
					join.r#type, join.column, join.first, join.operator, join.second
				));
			}
		}

		if !self.r#where.is_empty() {
			sql.push_str(" WHERE ");

			for (i, where_clause) in self.r#where.iter().enumerate() {
				sql.push_str(&where_clause.to_sql(i != 0));
			}
		}

		if !self.order.is_empty() {
			sql.push_str(" ORDER BY ");

			sql.push_str(
				&self
					.order
					.iter()
					.map(|order| format!("{} {}", order.column, order.direction))
					.join(", "),
			);
		}

		if let Some(take) = self.limit {
			sql.push_str(&format!(" LIMIT {take}"));
		}

		if let Some(skip) = self.offset {
			sql.push_str(&format!(" OFFSET {skip}"));
		}

		sql
	}

	/// Get the current query value bindings.
	#[must_use]
	pub fn get_bindings(&self) -> Vec<Value> {
		self.r#where
			.iter()
			.flat_map(WhereClause::get_bindings)
			.collect()
	}

	/// Retrieve the number of records that match the query constraints.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn count(self) -> Result<u64, Error> {
		let mut conn = connection::get().await?;

		let values = conn
			.get_values(&self.to_sql(Type::Count), self.get_bindings())
			.await
			.map_err(|e| Error::Database(e.to_string()))?;

		values.first().and_then(Value::as_u64).ok_or_else(|| {
			Error::Serialization(rbs::value::ext::Error::Syntax(
				"Failed to parse count value".to_string(),
			))
		})
	}

	/// Execute the query and return the first result.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn first<M: Model>(mut self) -> Result<Option<M>, Error> {
		self.limit = Some(1);
		let values = self.get::<M>().await?;

		Ok(values.into_iter().next())
	}

	/// Execute the query and return the results.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn get<M: Model>(self) -> Result<Vec<M>, Error> {
		let mut models = self
			._get()
			.await?
			.into_iter()
			.map(value::from::<M>)
			.collect::<Result<Vec<M>, rbs::Error>>()?;

		if models.is_empty() || self.eager_load.is_empty() {
			return Ok(models);
		}

		let model = M::default();
		for relation in self.eager_load {
			tracing::trace!(
				"Eager loading {relation} relation for {} models",
				models.len()
			);

			let rows = model
				.eager_load(&relation, models.iter().collect::<Vec<&M>>().as_slice())
				.get_rows()
				.await?;

			for model in &mut models {
				model.fill_relation(&relation, &rows)?;
			}
		}

		Ok(models)
	}

	/// Execute the query and return the results as a vector of rows.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub(crate) async fn get_rows(&self) -> Result<Vec<HashMap<String, Value>>, Error> {
		let values = self
			._get()
			.await?
			.into_iter()
			.map(|v| {
				let Value::Map(map) = v else { unreachable!() };

				map.into_iter()
					.map(|(k, v)| (k.into_string().unwrap_or_else(|| unreachable!()), v))
					.collect()
			})
			.collect();

		Ok(values)
	}

	/// Insert a new record into the database. Returns the ID of the inserted record, if applicable.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn insert<Id: for<'de> serde::Deserialize<'de>, T: Into<Columns> + Send>(
		&self,
		columns: T,
	) -> Result<Id, Error> {
		if self.limit.is_some()
			|| !self.join.is_empty()
			|| !self.order.is_empty()
			|| !self.r#where.is_empty()
		{
			return Err(Error::InvalidQuery);
		}

		let mut conn = connection::get().await?;
		let values: Vec<(String, Value)> = columns.into().0;

		let (sql, bindings) = (
			format!(
				"INSERT INTO {} ({}) VALUES ({})",
				self.table,
				values.iter().map(|(column, _)| column).join(", "),
				values.iter().map(|_| "?").join(", ")
			),
			values.into_iter().map(|(_, value)| value).collect(),
		);

		tracing::debug!(sql = sql.as_str(), bindings = ?bindings, "Executing INSERT SQL query");

		let result = conn
			.exec(&sql, bindings)
			.await
			.map_err(|e| Error::Database(e.to_string()))?;

		Ok(rbs::from_value(result.last_insert_id)?)
	}

	/// Increment a column's value by a given amount. Returns the number of affected rows.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn increment(self, column: &str, amount: u64) -> Result<u64, Error> {
		let mut conn = connection::get().await?;
		let (sql, mut bindings) = (
			format!(
				"UPDATE {} SET {} = {} + ? {}",
				self.table,
				Columns::escape(column),
				Columns::escape(column),
				self.to_sql(Type::Update)
			),
			self.get_bindings(),
		);
		bindings.insert(0, amount.into());

		tracing::debug!(sql = sql.as_str(), bindings = ?bindings, "Executing UPDATE SQL query for increment");

		conn.exec(&sql, bindings)
			.await
			.map_err(|e| Error::Database(e.to_string()))
			.map(|r| r.rows_affected)
	}

	/// Update records in the database. Returns the number of affected rows.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn update<T: Into<Columns> + Send>(self, values: T) -> Result<u64, Error> {
		let mut conn = connection::get().await?;
		let values: Vec<(String, Value)> = values.into().0;

		let (sql, bindings) = (
			format!(
				"UPDATE {} SET {} {}",
				self.table,
				values
					.iter()
					.map(|(column, _)| format!("{column} = ?"))
					.join(", "),
				self.to_sql(Type::Update)
			),
			values
				.iter()
				.map(|(_, value)| value.clone())
				.chain(self.get_bindings())
				.collect(),
		);

		tracing::debug!(sql = sql.as_str(), bindings = ?bindings, "Executing UPDATE SQL query");

		conn.exec(&sql, bindings)
			.await
			.map_err(|e| Error::Database(e.to_string()))
			.map(|r| r.rows_affected)
	}

	/// Delete records from the database. Returns the number of affected rows.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn delete(self) -> Result<u64, Error> {
		let mut conn = connection::get().await?;
		let (sql, bindings) = (self.to_sql(Type::Delete), self.get_bindings());

		tracing::debug!(sql = sql.as_str(), bindings = ?bindings, "Executing DELETE SQL query");

		conn.exec(&sql, bindings)
			.await
			.map_err(|e| Error::Database(e.to_string()))
			.map(|r| r.rows_affected)
	}

	/// Run a truncate statement on the table. Returns the number of affected rows.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn truncate(self) -> Result<u64, Error> {
		let mut conn = connection::get().await?;
		let sql = format!("TRUNCATE TABLE {}", self.table);

		tracing::debug!(sql = sql.as_str(), "Executing TRUNCATE SQL query");

		conn.exec(&sql, vec![])
			.await
			.map_err(|e| Error::Database(e.to_string()))
			.map(|r| r.rows_affected)
	}
}

impl Builder {
	async fn _get(&self) -> Result<Vec<Value>, Error> {
		let mut conn = connection::get().await?;
		let (sql, bindings) = (self.to_sql(Type::Select), self.get_bindings());

		tracing::debug!(sql = sql.as_str(), bindings = ?bindings, "Executing SELECT SQL query");

		let values = conn
			.get_values(&sql, bindings)
			.await
			.map_err(|s| Error::Database(s.to_string()))?;

		Ok(values)
	}
}

pub enum EagerLoad {
	Single(String),
	Multiple(Vec<String>),
}

impl EagerLoad {
	#[must_use]
	pub fn list(self) -> Vec<String> {
		match self {
			Self::Single(value) => vec![value],
			Self::Multiple(value) => value,
		}
	}
}

impl From<&str> for EagerLoad {
	fn from(value: &str) -> Self {
		Self::Single(value.to_string())
	}
}

impl From<Vec<&str>> for EagerLoad {
	fn from(value: Vec<&str>) -> Self {
		Self::Multiple(value.iter().map(ToString::to_string).collect())
	}
}

pub struct Columns(Vec<(String, Value)>);

impl Columns {
	fn escape(column: &str) -> String {
		match connection::which_db() {
			Database::MySQL => format!("`{column}`"),
			Database::PostgreSQL => format!("\"{column}\""),
		}
	}
}

#[allow(clippy::fallible_impl_from)]
impl From<Value> for Columns {
	fn from(value: Value) -> Self {
		match value {
			Value::Map(map) => Self(
				map.into_iter()
					.map(|(column, value)| (Self::escape(&column.into_string().unwrap()), value))
					.collect(),
			),
			_ => panic!("The provided value is not a map."),
		}
	}
}

impl<T: Serialize> From<Vec<(&str, T)>> for Columns {
	fn from(values: Vec<(&str, T)>) -> Self {
		Self(
			values
				.iter()
				.map(|(column, value)| (Self::escape(column), value::for_db(value).unwrap()))
				.collect(),
		)
	}
}
impl<T: Serialize> From<&[(&str, T)]> for Columns {
	fn from(values: &[(&str, T)]) -> Self {
		Self(
			values
				.iter()
				.map(|(column, value)| (Self::escape(column), value::for_db(value).unwrap()))
				.collect(),
		)
	}
}

/// Available sort directions.
#[derive(Debug)]
pub enum Direction {
	Ascending,
	Descending,
}

impl Display for Direction {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Ascending => write!(f, "ASC"),
			Self::Descending => write!(f, "DESC"),
		}
	}
}

impl From<String> for Direction {
	fn from(value: String) -> Self {
		value.as_str().into()
	}
}

#[allow(clippy::fallible_impl_from)]
impl From<&str> for Direction {
	fn from(value: &str) -> Self {
		match value.to_uppercase().as_str() {
			"ASC" | "ASCENDING" => Self::Ascending,
			"DESC" | "DESCENDING" => Self::Descending,

			_ => panic!("Invalid direction {value}"),
		}
	}
}

/// An order clause.
#[derive(Debug)]
struct Order {
	column: String,
	direction: Direction,
}

/// Available join types.
#[derive(Debug)]
enum JoinType {
	/// The `INNER JOIN` type.
	Inner,
}

impl Display for JoinType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Inner => write!(f, "INNER JOIN"),
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub enum Type {
	Count,
	Select,
	Update,
	Delete,
}

/// A join clause.
#[derive(Debug)]
struct Join {
	column: String,
	first: String,
	second: String,
	r#type: JoinType,
	operator: Operator,
}

#[derive(Debug)]
enum WhereClause {
	Simple(Where),
	Group(Vec<WhereClause>, Boolean),
}

impl WhereClause {
	fn to_sql(&self, add_boolean: bool) -> String {
		match self {
			Self::Simple(where_clause) => where_clause.to_sql(add_boolean),
			Self::Group(where_clauses, boolean) => {
				let mut sql = String::new();

				for (i, where_clause) in where_clauses.iter().enumerate() {
					sql.push_str(&where_clause.to_sql(i != 0));
				}

				if add_boolean {
					format!(" {boolean} ({sql})")
				} else {
					format!("({sql})")
				}
			},
		}
	}

	fn get_bindings(&self) -> Vec<Value> {
		match self {
			Self::Simple(where_clause) => where_clause
				.value
				.clone()
				.into_iter()
				.flat_map(|v| match v {
					Value::Array(array) => array,
					_ => vec![v],
				})
				.collect(),
			Self::Group(where_clauses, _) => {
				where_clauses.iter().flat_map(Self::get_bindings).collect()
			},
		}
	}
}

/// A where clause.
#[derive(Debug)]
struct Where {
	column: String,
	boolean: Boolean,
	operator: Operator,
	value: Option<Value>,
}

impl Where {
	fn to_sql(&self, add_boolean: bool) -> String {
		let sql = format!(
			"{} {} {}",
			self.column,
			self.operator,
			self.value.as_ref().map_or_else(String::new, |value| {
				value.as_array().map_or_else(
					|| "?".to_string(),
					|value| format!("({})", value.iter().map(|_| "?").join(", ")),
				)
			})
		);

		if add_boolean {
			format!(" {} {sql} ", self.boolean)
		} else {
			sql
		}
	}
}

/// Available operators for where clauses.
#[derive(Debug)]
pub enum Operator {
	/// The `IN` operator.
	In,
	/// The `LIKE` operator.
	Like,
	/// The `NOT IN` operator.
	NotIn,
	/// The `=` operator.
	Equals,
	/// The `IS NULL` operator.
	IsNull,
	/// The `IS NOT NULL` operator.
	NotNull,
	/// The `BETWEEN` operator.
	Between,
	/// The `NOT LIKE` operator.
	NotLike,
	/// The `<` operator.
	LessThan,
	/// The `<>` operator.
	NotEquals,
	/// The `NOT BETWEEN` operator.
	NotBetween,
	/// The `>` operator.
	GreaterThan,
	/// The `<=` operator.
	LessOrEqual,
	/// The `>=` operator.
	GreaterOrEqual,
}

impl Display for Operator {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::In => "IN",
				Self::Equals => "=",
				Self::Like => "LIKE",
				Self::LessThan => "<",
				Self::NotIn => "NOT IN",
				Self::NotEquals => "<>",
				Self::GreaterThan => ">",
				Self::LessOrEqual => "<=",
				Self::IsNull => "IS NULL",
				Self::Between => "BETWEEN",
				Self::NotLike => "NOT LIKE",
				Self::GreaterOrEqual => ">=",
				Self::NotNull => "IS NOT NULL",
				Self::NotBetween => "NOT BETWEEN",
			}
		)
	}
}

impl From<String> for Operator {
	fn from(value: String) -> Self {
		value.as_str().into()
	}
}
impl From<char> for Operator {
	fn from(value: char) -> Self {
		value.to_string().into()
	}
}

#[allow(clippy::fallible_impl_from)]
impl From<&str> for Operator {
	fn from(value: &str) -> Self {
		match value.to_uppercase().as_str() {
			"IN" => Self::In,
			"=" => Self::Equals,
			"LIKE" => Self::Like,
			"<" => Self::LessThan,
			"NOT IN" => Self::NotIn,
			"!=" => Self::NotEquals,
			">" => Self::GreaterThan,
			"<=" => Self::LessOrEqual,
			"BETWEEN" => Self::Between,
			"NOT LIKE" => Self::NotLike,
			">=" => Self::GreaterOrEqual,
			"NOT BETWEEN" => Self::NotBetween,

			_ => panic!("Invalid operator {value}"),
		}
	}
}

#[derive(Debug, Clone, Copy)]
enum Boolean {
	And,
	Or,
}

impl Display for Boolean {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Or => write!(f, "OR"),
			Self::And => write!(f, "AND"),
		}
	}
}

impl AsRef<Self> for Builder {
	fn as_ref(&self) -> &Self {
		self
	}
}
