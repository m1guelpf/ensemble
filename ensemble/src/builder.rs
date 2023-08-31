use itertools::Itertools;
use rbs::{to_value, Value};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use crate::{connection, query::Error, value, Model};

/// The Query Builder.
#[derive(Debug)]
pub struct Builder {
    table: String,
    join: Vec<Join>,
    order: Vec<Order>,
    r#where: Vec<WhereClause>,
    eager_load: HashSet<String>,
    pub(crate) limit: Option<usize>,
}

impl Builder {
    pub(crate) fn new(table: String) -> Self {
        Self {
            table,
            limit: None,
            join: vec![],
            order: vec![],
            r#where: vec![],
            eager_load: HashSet::new(),
        }
    }

    /// Set the table which the query is targeting.
    #[must_use]
    pub fn from(mut self, table: &str) -> Self {
        self.table = table.to_string();
        self
    }

    /// Add a basic where clause to the query.
    #[must_use]
    pub fn r#where<T, Op>(mut self, column: &str, operator: Op, value: T) -> Self
    where
        Op: Into<Operator>,
        T: serde::Serialize,
    {
        self.r#where.push(WhereClause::Simple(Where {
            boolean: Boolean::And,
            operator: operator.into(),
            column: column.to_string(),
            value: Some(to_value!(value)),
        }));

        self
    }

    /// Set the "limit" value of the query.
    #[must_use]
    pub const fn limit(mut self, take: usize) -> Self {
        self.limit = Some(take);
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
            column: column.to_string(),
        }));

        self
    }

    /// Add a "where not null" clause to the query.
    #[must_use]
    pub fn where_not_null(mut self, column: &str) -> Self {
        self.r#where.push(WhereClause::Simple(Where {
            value: None,
            boolean: Boolean::And,
            column: column.to_string(),
            operator: Operator::NotNull,
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
            first: first.to_string(),
            column: column.to_string(),
            r#type: JoinType::Inner,
            second: second.to_string(),
        });

        self
    }

    /// Add an "order by" clause to the query.
    #[must_use]
    pub fn order_by<Dir: Into<Direction>>(mut self, column: &str, direction: Dir) -> Self {
        self.order.push(Order {
            column: column.to_string(),
            direction: direction.into(),
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
    pub fn to_sql(&self, r#type: QueryType) -> String {
        let mut sql = match r#type {
            QueryType::Update => String::new(), // handled in update()
            QueryType::Delete => format!("DELETE FROM {}", self.table),
            QueryType::Select => format!("SELECT * FROM {}", self.table),
            QueryType::Count => format!("SELECT COUNT(*) FROM {}", self.table),
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
                sql.push_str(&where_clause.to_sql(i != self.r#where.len() - 1));
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
            .get_values(&self.to_sql(QueryType::Count), self.get_bindings())
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

        let result = conn
            .exec(
                &format!(
                    "INSERT INTO {} ({}) VALUES ({})",
                    self.table,
                    values.iter().map(|(column, _)| column).join(", "),
                    values.iter().map(|_| "?").join(", ")
                ),
                values.into_iter().map(|(_, value)| value).collect(),
            )
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(rbs::from_value(result.last_insert_id)?)
    }

    /// Update records in the database. Returns the number of affected rows.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails, or if a connection to the database cannot be established.
    pub async fn update<T: Into<Columns> + Send>(self, values: T) -> Result<u64, Error> {
        let mut conn = connection::get().await?;
        let sql = self.to_sql(QueryType::Update);
        let values: Vec<(String, Value)> = values.into().0;

        conn.exec(
            &format!(
                "UPDATE {} SET {} {sql}",
                self.table,
                values
                    .iter()
                    .map(|(column, _)| format!("{} = ?", column))
                    .join(", "),
            ),
            values
                .iter()
                .map(|(_, value)| value.clone())
                .chain(self.get_bindings())
                .collect(),
        )
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

        conn.exec(&self.to_sql(QueryType::Delete), self.get_bindings())
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

        conn.exec(&format!("TRUNCATE TABLE {}", self.table), vec![])
            .await
            .map_err(|e| Error::Database(e.to_string()))
            .map(|r| r.rows_affected)
    }
}

impl Builder {
    async fn _get(&self) -> Result<Vec<Value>, Error> {
        let mut conn = connection::get().await?;
        let (sql, bindings) = (self.to_sql(QueryType::Select), self.get_bindings());

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

#[allow(clippy::fallible_impl_from)]
impl From<Value> for Columns {
    fn from(value: Value) -> Self {
        match value {
            Value::Map(map) => Self(
                map.into_iter()
                    .map(|(column, value)| (column.into_string().unwrap(), value))
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
                .map(|(column, value)| ((*column).to_string(), to_value!(value)))
                .collect(),
        )
    }
}
impl<T: Serialize> From<&[(&str, T)]> for Columns {
    fn from(values: &[(&str, T)]) -> Self {
        Self(
            values
                .iter()
                .map(|(column, value)| ((*column).to_string(), to_value!(value)))
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
pub enum QueryType {
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
                    sql.push_str(&format!("({})", where_clause.to_sql(false)));

                    if i != where_clauses.len() - 1 {
                        sql.push_str(" AND ");
                    }
                }

                if add_boolean {
                    format!("{boolean} {sql}")
                } else {
                    sql
                }
            }
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
            }
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
            format!("{sql} {} ", self.boolean)
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

#[derive(Debug)]
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
