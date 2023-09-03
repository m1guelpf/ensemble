use inflector::Inflector;
use rbs::Value;
use serde::Serialize;
use std::{collections::HashMap, fmt::Debug};

use super::{find_related, Relationship, Status};
use crate::{builder::Builder, query::Error, value::serializing_for_db, Model};

/// ## A Many to Many relationship.
/// A many to many relationship is used to define relationships where a model is the parent of one or more child models, but can also be a child to multiple parent models.
/// For example, a user may be assigned the role of “Author” and “Editor”; however, those roles may also be assigned to other users as well. So, a user has many roles and a role has many users.
///
/// ## Example
///
/// ```rust
/// # use ensemble::{Model, relationships::BelongsToMany};
/// # #[derive(Debug, Model)]
/// # struct Role {
/// #   id: u64,
/// # }
/// #[derive(Debug, Model)]
/// struct User {
///   id: u64,
///   name: String,
///   roles: BelongsToMany<User, Role>
/// }
///
/// # async fn call() -> Result<(), ensemble::query::Error> {
/// let mut user = User::find(1).await?;
///
/// let roles: &Vec<Role> = user.roles().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default)]
pub struct BelongsToMany<Local: Model, Related: Model> {
    local_key: String,
    foreign_key: String,
    pivot_table: String,
    relation: Status<Vec<Related>>,
    _local: std::marker::PhantomData<Local>,
    /// The value of the local model's primary key.
    pub value: Related::PrimaryKey,
}

#[async_trait::async_trait]
impl<Local: Model, Related: Model> Relationship for BelongsToMany<Local, Related> {
    type Value = Vec<Related>;
    type Key = Related::PrimaryKey;
    type RelatedKey = (Option<String>, Option<String>, Option<String>);

    fn build(value: Self::Key, (pivot_table, foreign_key, local_key): Self::RelatedKey) -> Self {
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
            local_key,
            foreign_key,
            pivot_table,
            relation: Status::initial(),
            _local: std::marker::PhantomData,
        }
    }

    fn query(&self) -> Builder {
        Related::query()
            .from(Related::TABLE_NAME)
            .join(
                &self.pivot_table,
                &format!("{}.{}", Related::TABLE_NAME, Related::PRIMARY_KEY),
                "=",
                &format!("{}.{}", self.pivot_table, self.foreign_key),
            )
            .r#where(
                &format!("{}.{}", self.pivot_table, self.local_key),
                "=",
                self.value.clone(),
            )
    }

    async fn get(&mut self) -> Result<&mut Self::Value, Error> {
        if self.relation.is_none() {
            let relation = self.query().get().await?;

            self.relation = Status::Fetched(Some(relation));
        }

        Ok(self.relation.as_mut().unwrap())
    }

    fn is_loaded(&self) -> bool {
        self.relation.is_loaded()
    }

    fn eager_query(&self, related: Vec<Self::Key>) -> Builder {
        Related::query()
            .from(Related::TABLE_NAME)
            .join(
                &self.pivot_table,
                &format!("{}.{}", Related::TABLE_NAME, Related::PRIMARY_KEY),
                "=",
                &format!("{}.{}", self.pivot_table, self.foreign_key),
            )
            .r#where(
                &format!("{}.{}", self.pivot_table, self.local_key),
                "in",
                related,
            )
    }

    fn r#match(&mut self, related: &[HashMap<String, Value>]) -> Result<(), Error> {
        let related = find_related(related, &self.foreign_key, &self.value, false)?;

        if !related.is_empty() {
            self.relation = Status::Fetched(Some(related));
        }

        Ok(())
    }
}

impl<Local: Model, Related: Model> Debug for BelongsToMany<Local, Related> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.relation.fmt(f)
    }
}

impl<Local: Model, Related: Model> Serialize for BelongsToMany<Local, Related> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializing_for_db::<S>() {
            if self.value == Default::default() {
                return serializer.serialize_none();
            }

            return self.value.serialize(serializer);
        }

        self.relation.serialize(serializer)
    }
}

#[cfg(feature = "schema")]
impl<Local: Model, Related: Model + schemars::JsonSchema> schemars::JsonSchema
    for BelongsToMany<Local, Related>
{
    fn schema_name() -> String {
        <Option<Vec<Related>>>::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        gen.subschema_for::<Option<Vec<Related>>>()
    }
}
