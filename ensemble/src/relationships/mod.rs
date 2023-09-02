//! Relationships between models.
#![doc = include_str!("../../docs/relationships.md")]

mod belongs_to;
mod belongs_to_many;
mod has_many;
mod has_one;

use std::collections::HashMap;

use crate::{
    builder::Builder,
    query::Error,
    value::{self, to_value},
    Model,
};

pub use belongs_to::BelongsTo;
pub use belongs_to_many::BelongsToMany;
pub use has_many::HasMany;
pub use has_one::HasOne;
use rbs::Value;

/// A relationship between two models.
#[async_trait::async_trait]
pub trait Relationship {
    /// The provided input for the relationship.
    type RelatedKey;

    /// The type of the primary key for the model.
    type Key;

    /// The return type of the relationship.
    type Value;

    /// Get the related model.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be retrieved, or if a connection to the database cannot be established.
    async fn get(&mut self) -> Result<&Self::Value, Error>;

    /// Get the query builder for the relationship.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails when building the query.
    fn query(&self) -> Builder;

    /// Get the query builder for eager loading the relationship. Not intended to be used directly.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails when building the query.
    #[doc(hidden)]
    fn eager_query(&self, related: Vec<Self::Key>) -> Builder;

    /// Match the eagerly loaded results to their parents. Not intended to be used directly.
    #[doc(hidden)]
    fn r#match(&mut self, related: &[HashMap<String, Value>]) -> Result<(), Error>;

    /// Create an instance of the relationship. Not intended to be used directly.
    #[doc(hidden)]
    fn build(value: Self::Key, related_key: Self::RelatedKey) -> Self;
}

fn find_related<M: Model, T: serde::Serialize>(
    related: &[HashMap<String, Value>],
    foreign_key: &str,
    value: T,
    wants_one: bool,
) -> Result<Vec<M>, Error> {
    let value = to_value(value);

    let related = related
        .iter()
        .filter(|model| {
            model
                .get(foreign_key)
                .is_some_and(|v| v.to_string() == value.to_string())
        })
        .take(if wants_one { 1 } else { usize::MAX })
        .map(|model| value::from::<M>(to_value(model)))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(related)
}
