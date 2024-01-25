use quaint::{
	ast::*,
	connector::{Queryable, ResultRow},
	Value,
};
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
};

use crate::{
	connection::{self},
	Error, Model,
};

/// The Query Builder.
#[derive(Debug)]
pub struct Builder<'a> {
	table: String,
	order: Ordering<'a>,
	join: Vec<Join<'a>>,
	limit: Option<usize>,
	offset: Option<usize>,
	eager_load: HashSet<String>,
	conditions: Option<ConditionTree<'a>>,
}

impl<'a> Builder<'a> {
	pub(crate) fn new(table: String) -> Self {
		Self {
			table,
			limit: None,
			offset: None,
			join: vec![],
			conditions: None,
			order: Ordering::default(),
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
	pub async unsafe fn raw_sql(
		sql: &str,
		bindings: &[Value<'_>],
	) -> Result<impl Iterator<Item = ResultRow>, Error> {
		let conn = connection::get().await?;

		Ok(conn.query_raw(sql, bindings).await?.into_iter())
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
	pub fn r#where(mut self, condition: Compare<'a>) -> Self {
		self.conditions = Some(match self.conditions {
			None => condition.into(),
			Some(previous) => previous.and(condition),
		});

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
	pub fn or_where(mut self, condition: Compare<'a>) -> Self {
		let Some(previous) = self.conditions else {
			panic!("Cannot use or_where without a where clause.");
		};

		self.conditions = Some(previous.or(condition));

		self
	}

	/// Add an inner join to the query.
	///
	/// # Example
	///
	/// ```
	/// # use ensemble::query::Builder;
	/// # let query = Builder::new("users".to_string());
	/// query.join("articles", col!("articles", "user_id").equals(col!("users", "id")))
	/// ```
	#[must_use]
	pub fn join<T: Into<ConditionTree<'a>>>(mut self, table: &'a str, condition: T) -> Self {
		self.join.push(Join::Inner(table.on(condition)));

		self
	}

	/// Add a left join to the query.
	#[must_use]
	pub fn left_join<T: Into<ConditionTree<'a>>>(mut self, table: &'a str, condition: T) -> Self {
		self.join.push(Join::Left(table.on(condition)));

		self
	}

	/// Add a right join to the query.
	#[must_use]
	pub fn right_join<T: Into<ConditionTree<'a>>>(mut self, table: &'a str, condition: T) -> Self {
		self.join.push(Join::Right(table.on(condition)));

		self
	}

	/// Add a full join to the query.
	#[must_use]
	pub fn full_join<T: Into<ConditionTree<'a>>>(mut self, table: &'a str, condition: T) -> Self {
		self.join.push(Join::Full(table.on(condition)));

		self
	}

	/// Add an "order by" clause to the query.
	#[must_use]
	pub fn order_by<T: IntoOrderDefinition<'a>>(mut self, ordering: T) -> Self {
		self.order = self.order.append(ordering.into_order_definition());

		self
	}

	/// Retrieve the number of records that match the query constraints.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn count(self) -> Result<u64, Error> {
		let conn = connection::get().await?;

		let values = conn
			.select(Select::from(&self).value(count(asterisk()).alias("count")))
			.await?;

		values
			.into_single()?
			.get("count")
			.and_then(|v| v.as_integer())
			.map(|i| i as u64)
			.ok_or(Error::InvalidQuery)
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
		let eager_load = self.eager_load.clone();

		let conn = connection::get().await?;
		let mut models: Vec<M> = quaint::serde::from_rows(conn.select(Select::from(&self)).await?)?;

		if models.is_empty() || eager_load.is_empty() {
			return Ok(models);
		}

		let model = M::default();
		for relation in eager_load {
			tracing::trace!(
				"Eager loading {relation} relation for {} models",
				models.len()
			);

			let query = model.eager_load(&relation, models.iter());
			let rows = Arc::new(query.get_rows().await?);

			for model in &mut models {
				model.fill_relation(&relation, rows.clone())?;
			}
		}

		Ok(models)
	}

	/// Execute the query and return the results as a vector of rows.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub(crate) async fn get_rows(&self) -> Result<Vec<HashMap<String, Value<'static>>>, Error> {
		let conn = connection::get().await?;
		let values = conn.select(Select::from(self.to_owned())).await?;

		Ok(values.into())
	}

	/// Insert a new record into the database. Returns the ID of the inserted record, if applicable.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn insert<Id: From<Value<'a>>, T: Into<Columns<'a>> + Send>(
		&self,
		columns: T,
	) -> Result<Option<u64>, Error> {
		if self.limit.is_some()
			|| !self.join.is_empty()
			|| !self.order.is_empty()
			|| self.conditions.is_some()
		{
			return Err(Error::InvalidQuery);
		}

		let columns: Vec<(String, Value)> = columns.into().0;
		let mut insert = Insert::single_into(&self.table);

		for column in columns {
			insert = insert.value(column.0, column.1);
		}

		let conn = connection::get().await?;
		let result = conn.insert(insert.into()).await?;

		Ok(result.last_insert_id())
	}

	/// Increment a column's value by a given amount. Returns the number of affected rows.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn increment(self, column: &str, amount: i64) -> Result<u64, Error> {
		let query = Update::from(&self).set(
			column,
			SqlOp::Add(Column::from(column).into(), amount.into()),
		);
		let mut conn = connection::get().await?;

		Ok(conn.update(query).await?)
	}

	/// Update records in the database. Returns the number of affected rows.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn update<T: Into<Columns<'a>> + Send>(self, values: T) -> Result<u64, Error> {
		if !self.join.is_empty()
			|| !self.order.is_empty()
			|| self.offset.is_some()
			|| self.limit.is_some()
		{
			return Err(Error::InvalidQuery);
		}

		let mut query = Update::from(&self);
		let values: Vec<(String, Value)> = values.into().0;

		for (column, value) in values {
			query = query.set(column, value);
		}

		let conn = connection::get().await?;

		Ok(conn.update(query).await?)
	}

	/// Delete records from the database.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn delete(self) -> Result<(), Error> {
		if !self.join.is_empty()
			|| !self.order.is_empty()
			|| self.offset.is_some()
			|| self.limit.is_some()
		{
			return Err(Error::InvalidQuery);
		}

		let query = Delete::from(&self);
		let conn = connection::get().await?;

		conn.delete(query).await?;

		Ok(())
	}

	/// Run a truncate statement on the table. Returns the number of affected rows.
	///
	/// # Errors
	///
	/// Returns an error if the query fails, or if a connection to the database cannot be established.
	pub async fn truncate(self) -> Result<u64, Error> {
		let conn = connection::get().await?;

		Ok(conn
			.execute_raw("TRUNCATE TABLE ?", &[self.table.into()])
			.await?)
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

pub struct Columns<'a>(Vec<(String, Value<'a>)>);

impl<'a, T: Into<Value<'a>>> From<Vec<(&str, T)>> for Columns<'a> {
	fn from(values: Vec<(&str, T)>) -> Self {
		Self(
			values
				.into_iter()
				.map(|(column, value)| (column.to_string(), value.into()))
				.collect(),
		)
	}
}

impl<'a> From<&Builder<'a>> for Select<'a> {
	fn from(value: &Builder<'a>) -> Self {
		let mut select = Self::from_table(value.table.clone());

		if let Some(conditions) = value.conditions.clone() {
			select = select.so_that(conditions);
		}

		for join in value.join.clone() {
			select = match join {
				Join::Full(join) => select.full_join(join),
				Join::Left(join) => select.left_join(join),
				Join::Inner(join) => select.inner_join(join),
				Join::Right(join) => select.right_join(join),
			}
		}

		for ordering in value.order.0.clone() {
			select = select.order_by(ordering);
		}

		if let Some(limit) = value.limit {
			select = select.limit(limit);
		}

		if let Some(offset) = value.offset {
			select = select.offset(offset);
		}

		select
	}
}

impl<'a> From<&Builder<'a>> for Update<'a> {
	fn from(value: &Builder<'a>) -> Self {
		let mut update = Self::table(value.table.clone());

		if let Some(conditions) = value.conditions.clone() {
			update = update.so_that(conditions);
		}

		update
	}
}

impl<'a> From<&Builder<'a>> for Delete<'a> {
	fn from(value: &Builder<'a>) -> Self {
		let mut delete = Self::from_table(value.table.clone());

		if let Some(conditions) = value.conditions.clone() {
			delete = delete.so_that(conditions);
		}

		delete
	}
}
