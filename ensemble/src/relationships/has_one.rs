use inflector::Inflector;
use rbs::Value;
use serde::Serialize;
use std::{collections::HashMap, fmt::Debug};

use super::{find_related, Relationship};
use crate::{builder::Builder, query::Error, Model};

/// ## A One to One relationship.
/// A one-to-one relationship is a very basic type of database relationship. For example, a User model might be associated with one Phone model.
///
/// To define this relationship, we will place a phone field on the User model. The phone field should be of type `HasOne<User, Phone>`.
///
/// ## Example
///
/// ```rust
/// # use ensemble::{Model, relationships::HasOne};
/// # #[derive(Debug, Model)]
/// # struct Phone {
/// #   id: u64,
/// # }
/// #[derive(Debug, Model)]
/// struct User {
///   id: u64,
///   name: String,
///   phone: HasOne<User, Phone>
/// }
///
/// # async fn call() -> Result<(), ensemble::query::Error> {
/// let mut user = User::find(1).await?;
///
/// let phone: &Phone = user.phone().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default)]
pub struct HasOne<Local: Model, Related: Model> {
    foreign_key: String,
    relation: Option<Related>,
    /// The value of the local model's primary key.
    pub value: Local::PrimaryKey,
}

#[async_trait::async_trait]
impl<Local: Model, Related: Model> Relationship for HasOne<Local, Related> {
    type Value = Related;
    type Key = Local::PrimaryKey;
    type RelatedKey = Option<String>;

    fn build(value: Self::Key, foreign_key: Self::RelatedKey) -> Self {
        let foreign_key = foreign_key.unwrap_or_else(|| {
            format!("{}_{}", Local::NAME.to_snake_case(), Local::PRIMARY_KEY).to_snake_case()
        });

        Self {
            value,
            foreign_key,
            relation: None,
        }
    }

    fn eager_query(&self, related: Vec<Self::Key>) -> Builder {
        Related::query()
            .r#where(
                &format!("{}.{}", Related::TABLE_NAME, self.foreign_key),
                "in",
                related,
            )
            .where_not_null(&format!("{}.{}", Related::TABLE_NAME, self.foreign_key))
            .limit(1)
    }

    fn query(&self) -> Builder {
        Related::query()
            .r#where(
                &format!("{}.{}", Related::TABLE_NAME, self.foreign_key),
                "=",
                self.value.clone(),
            )
            .where_not_null(&format!("{}.{}", Related::TABLE_NAME, self.foreign_key))
            .limit(1)
    }

    /// Get the related models.
    async fn get(&mut self) -> Result<&Self::Value, Error> {
        if self.relation.is_none() {
            let relation = self.query().first().await?.ok_or(Error::NotFound)?;

            self.relation = Some(relation);
        }

        Ok(self.relation.as_ref().unwrap())
    }

    fn r#match(&mut self, related: &[HashMap<String, Value>]) -> Result<(), Error> {
        let related = find_related(related, &self.foreign_key, &self.value, true)?;

        self.relation = related.into_iter().next();

        Ok(())
    }
}

impl<Local: Model, Related: Model> Debug for HasOne<Local, Related> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.relation.fmt(f)
    }
}

impl<Local: Model, Related: Model> Serialize for HasOne<Local, Related> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}
