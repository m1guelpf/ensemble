mod belongs_to;
mod belongs_to_many;
mod has_many;
mod has_one;

use crate::query::Error;

pub use belongs_to::BelongsTo;
pub use belongs_to_many::BelongsToMany;
pub use has_many::HasMany;
pub use has_one::HasOne;

/// A relationship between two models.
#[async_trait::async_trait]
pub trait Relationship {
    /// The provided input for the relationship.
    type ForeignKey;

    /// The type of the primary key for the model.
    type Key;
    /// The return type of the relationship.
    type Value;

    /// Get the related model.
    async fn get(&mut self) -> Result<&Self::Value, Error>;

    /// Create an instance of the relationship. Not intended to be used directly.
    #[doc(hidden)]
    fn build(
        value: Self::Key,
        relation: Option<Self::Value>,
        foreign_key: Self::ForeignKey,
    ) -> Self;
}
