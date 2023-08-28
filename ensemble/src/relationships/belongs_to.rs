use inflector::Inflector;
use serde::Serialize;
use std::fmt::Debug;

use super::Relationship;
use crate::{query::Error, Model};

#[derive(Clone, Default)]
pub struct BelongsTo<Local: Model, Related: Model> {
    foreign_key: String,
    relation: Option<Related>,
    value: Related::PrimaryKey,
    _local: std::marker::PhantomData<Local>,
}

#[async_trait::async_trait]
impl<Local: Model, Related: Model> Relationship for BelongsTo<Local, Related> {
    type Value = Related;
    type Key = Related::PrimaryKey;
    type ForeignKey = Option<String>;

    fn build(
        value: Self::Key,
        relation: Option<Self::Value>,
        foreign_key: Self::ForeignKey,
    ) -> Self {
        let foreign_key = foreign_key.unwrap_or_else(|| {
            format!("{}_{}", Related::NAME.to_snake_case(), Related::PRIMARY_KEY).to_snake_case()
        });

        Self {
            value,
            relation,
            _local: std::marker::PhantomData,
            foreign_key: format!("{}.{}", Local::TABLE_NAME, foreign_key),
        }
    }

    /// Get the related model.
    async fn get(&mut self) -> Result<&Self::Value, Error> {
        if self.relation.is_none() {
            let relation = Related::query()
                .r#where(&self.foreign_key, "=", rbs::to_value(self.value.clone())?)
                .first()
                .await?
                .ok_or(Error::NotFound)?;

            self.relation = Some(relation);
        }

        Ok(self.relation.as_ref().unwrap())
    }
}

impl<Local: Model, Related: Model> Debug for BelongsTo<Local, Related> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.relation.fmt(f)
    }
}

impl<Local: Model, Related: Model> Serialize for BelongsTo<Local, Related> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}
