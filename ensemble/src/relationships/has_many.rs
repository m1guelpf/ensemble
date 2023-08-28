use inflector::Inflector;
use serde::Serialize;
use std::fmt::Debug;

use super::Relationship;
use crate::{query::Error, Model};

/// A has many relationship.
#[derive(Clone, Default)]
pub struct HasMany<Local: Model, Related: Model> {
    foreign_key: String,
    value: Local::PrimaryKey,
    relation: Option<Vec<Related>>,
}

#[async_trait::async_trait]
impl<Local: Model, Related: Model> Relationship for HasMany<Local, Related> {
    type Value = Vec<Related>;
    type Key = Local::PrimaryKey;
    type ForeignKey = Option<String>;

    fn build(
        value: Self::Key,
        relation: Option<Self::Value>,
        foreign_key: Self::ForeignKey,
    ) -> Self {
        let foreign_key = foreign_key.unwrap_or_else(|| {
            format!("{}_{}", Local::NAME.to_snake_case(), Local::PRIMARY_KEY).to_snake_case()
        });

        Self {
            value,
            relation,
            foreign_key: format!("{}.{foreign_key}", Related::TABLE_NAME),
        }
    }

    /// Get the related models.
    async fn get(&mut self) -> Result<&Self::Value, Error> {
        if self.relation.is_none() {
            let relation = Related::query()
                .r#where(&self.foreign_key, "=", rbs::to_value(self.value.clone())?)
                .where_not_null(&self.foreign_key)
                .get()
                .await?;

            self.relation = Some(relation);
        }

        Ok(self.relation.as_ref().unwrap())
    }
}

impl<Local: Model, Related: Model> Debug for HasMany<Local, Related> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.relation.fmt(f)
    }
}

impl<Local: Model, Related: Model> Serialize for HasMany<Local, Related> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}
