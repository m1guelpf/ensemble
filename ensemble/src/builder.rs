use std::fmt::Display;

use rbs::Value;

use crate::{connection, query::Error, value, Model};

pub struct Builder {
    table: String,
    r#where: Vec<Where>,
    take: Option<usize>,
}

impl Builder {
    pub(crate) fn new(table: String) -> Self {
        Self {
            table,
            take: None,
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

    /// Get the SQL representation of the query.
    #[must_use]
    pub fn to_sql(&self) -> String {
        let mut sql = format!("SELECT * FROM {}", self.table);

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

        conn.get_values(&self.to_sql(), self.get_bindings())
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
}

#[derive(Debug)]
struct Where {
    column: String,
    boolean: Boolean,
    operator: Operator,
    value: Option<Value>,
}

#[derive(Debug)]
pub enum Operator {
    In,
    Like,
    NotIn,
    Equals,
    IsNull,
    NotNull,
    Between,
    NotLike,
    LessThan,
    NotEquals,
    NotBetween,
    GreaterThan,
    LessOrEqual,
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
