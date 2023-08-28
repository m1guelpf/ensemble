use inflector::Inflector;
use serde::Serialize;
use std::fmt::Debug;

use super::Relationship;
use crate::{query::Error, Model};

#[derive(Clone, Default)]
pub struct BelongsToMany<Local: Model, Related: Model> {
    local_key: String,
    foreign_key: String,
    pivot_table: String,
    value: Related::PrimaryKey,
    relation: Option<Vec<Related>>,
    _local: std::marker::PhantomData<Local>,
}

#[async_trait::async_trait]
impl<Local: Model, Related: Model> Relationship for BelongsToMany<Local, Related> {
    type Value = Vec<Related>;
    type Key = Related::PrimaryKey;
    type ForeignKey = (Option<String>, Option<String>, Option<String>);

    fn build(
        value: Self::Key,
        relation: Option<Self::Value>,
        (pivot_table, foreign_key, local_key): Self::ForeignKey,
    ) -> Self {
        let pivot_table = pivot_table.unwrap_or_else(|| {
            let mut names = [Local::NAME.to_string(), Related::NAME.to_string()];
            names.sort();
            names.join("_").to_snake_case()
        });

        let foreign_key = foreign_key.unwrap_or_else(|| {
            format!("{}_{}", Related::NAME.to_snake_case(), Related::PRIMARY_KEY).to_snake_case()
        });

        let local_key = local_key.unwrap_or_else(|| {
            format!("{}_{}", Local::NAME.to_snake_case(), Local::PRIMARY_KEY).to_snake_case()
        });

        Self {
            value,
            relation,
            _local: std::marker::PhantomData,
            pivot_table: pivot_table.clone(),
            local_key: format!("{pivot_table}.{local_key}"),
            foreign_key: format!("{pivot_table}.{foreign_key}"),
        }
    }

    /// Get the related model.
    async fn get(&mut self) -> Result<&Self::Value, Error> {
        if self.relation.is_none() {
            let relation = Related::query()
                .from(Related::TABLE_NAME)
                .join(
                    &self.pivot_table,
                    &format!("{}.{}", Related::TABLE_NAME, Related::PRIMARY_KEY),
                    "=",
                    &self.foreign_key,
                )
                .r#where(&self.local_key, "=", rbs::to_value(self.value.clone())?)
                .get()
                .await?;

            self.relation = Some(relation);
        }

        Ok(self.relation.as_ref().unwrap())
    }
}

impl<Local: Model, Related: Model> Debug for BelongsToMany<Local, Related> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.relation.fmt(f)
    }
}

impl<Local: Model, Related: Model> Serialize for BelongsToMany<Local, Related> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}
