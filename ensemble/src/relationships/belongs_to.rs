use inflector::Inflector;
use rbs::Value;
use serde::Serialize;
use std::{collections::HashMap, fmt::Debug};

use super::{find_related, Relationship};
use crate::{builder::Builder, query::Error, Model};

/// ## A Belongs To relationship.
/// A belongs to relationship is used to define relationships where a model is the child to a single models. For example, a website may belong to a user.
///
/// To define this relationship, we will place a user field on the Site model. The comments field should be of type `BelongsTo<Site, User>`.
///
/// ## Example
///
/// ```rust
/// # use ensemble::{Model, relationships::BelongsTo};
/// # #[derive(Debug, Model)]
/// # struct User {
/// #   id: u64,
/// # }
/// #[derive(Debug, Model)]
/// struct Site {
///   id: u64,
///   url: String,
///   user: BelongsTo<Site, User>
/// }
///
/// # async fn call() -> Result<(), ensemble::query::Error> {
/// let mut site = Site::find(1).await?;
///
/// site.user().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default)]
pub struct BelongsTo<Local: Model, Related: Model> {
    foreign_key: String,
    relation: Option<Related>,
    _local: std::marker::PhantomData<Local>,
    /// The value of the local model's related key.
    pub value: Related::PrimaryKey,
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
            foreign_key,
            _local: std::marker::PhantomData,
        }
    }

    fn eager_query(&self, related: Vec<Self::Key>) -> Builder {
        Related::query()
            .r#where(
                &format!("{}.{}", Local::TABLE_NAME, self.foreign_key),
                "in",
                related,
            )
            .limit(1)
    }

    fn query(&self) -> Builder {
        Related::query()
            .r#where(
                &format!("{}.{}", Local::TABLE_NAME, self.foreign_key),
                "=",
                self.value.clone(),
            )
            .limit(1)
    }

    /// Get the related model.
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
