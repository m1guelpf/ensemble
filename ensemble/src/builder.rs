use itertools::Itertools;
use rbs::Value;
use std::fmt::Display;

use crate::{connection, query::Error, value, Model};

/// The Query Builder.
pub struct Builder {
    table: String,
    join: Vec<Join>,
    order: Vec<Order>,
    r#where: Vec<Where>,
    take: Option<usize>,
}

impl Builder {
    pub(crate) fn new(table: String) -> Self {
        Self {
            table,
            take: None,
            join: vec![],
            order: vec![],
            r#where: vec![],
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
        T: Into<Value>,
        Op: Into<Operator>,
    {
        self.r#where.push(Where {
            boolean: Boolean::And,
            operator: operator.into(),
            value: Some(value.into()),
            column: column.to_string(),
        });

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

        self.r#where.push(Where {
            operator: op.into(),
            boolean: Boolean::Or,
            value: Some(value.into()),
            column: column.to_string(),
        });

        self
    }

    /// Add a "where not null" clause to the query.
    #[must_use]
    pub fn where_not_null(mut self, column: &str) -> Self {
        self.r#where.push(Where {
            value: None,
            boolean: Boolean::And,
            column: column.to_string(),
            operator: Operator::NotNull,
        });

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

    /// Get the SQL representation of the query.
    #[must_use]
    pub fn to_sql(&self, r#type: QueryType) -> String {
        let mut sql = match r#type {
            QueryType::Select => format!("SELECT * FROM {}", self.table),
            QueryType::Update => String::new(), // handled in update()
            QueryType::Delete => format!("DELETE FROM {}", self.table),
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
                sql.push_str(&format!(
                    "{} {} {}",
                    where_clause.column,
                    where_clause.operator,
                    if where_clause.value.is_some() {
                        "?"
                    } else {
                        ""
                    }
                ));

                if i != self.r#where.len() - 1 {
                    sql.push_str(&format!(" {} ", where_clause.boolean));
                }
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

        if let Some(take) = self.take {
            sql.push_str(&format!(" LIMIT {take}"));
        }

        sql
    }

    /// Get the current query value bindings.
    #[must_use]
    pub fn get_bindings(&self) -> Vec<Value> {
        self.r#where
            .iter()
            .filter_map(|w| w.value.clone())
            .collect()
    }

    async fn run(&self) -> Result<Vec<Value>, Error> {
        let mut conn = connection::get().await?;

        conn.get_values(&self.to_sql(QueryType::Select), self.get_bindings())
            .await
            .map_err(|s| Error::Database(s.to_string()))
    }

    /// Execute the query and return the first result.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails, or if a connection to the database cannot be established.
    pub async fn first<M: Model>(mut self) -> Result<Option<M>, Error> {
        self.take = Some(1);
        let values = self.get::<M>().await?;

        Ok(values.into_iter().next())
    }

    /// Execute the query and return the results.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails, or if a connection to the database cannot be established.
    pub async fn get<M: Model>(self) -> Result<Vec<M>, Error> {
        let values = self.run().await?;

        Ok(values
            .into_iter()
            .map(value::from::<M>)
            .collect::<Result<Vec<M>, rbs::Error>>()?)
    }

    /// Update records in the database. Returns the number of affected rows.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails, or if a connection to the database cannot be established.
    pub async fn update(self, values: &[(&str, rbs::Value)]) -> Result<u64, Error> {
        let mut conn = connection::get().await?;
        let sql = self.to_sql(QueryType::Update);

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

/// A where clause.
#[derive(Debug)]
struct Where {
    column: String,
    boolean: Boolean,
    operator: Operator,
    value: Option<Value>,
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
