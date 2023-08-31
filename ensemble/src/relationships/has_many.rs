use inflector::Inflector;
use rbs::Value;
use serde::Serialize;
use std::{collections::HashMap, fmt::Debug};

use super::{find_related, Relationship};
use crate::{builder::Builder, query::Error, Model};

/// ## A One to Many relationship.
/// A one-to-many relationship is used to define relationships where a single model is the parent to one or more child models.
/// For example, a blog post may have an infinite number of comments.
///
/// To define this relationship, we will place a comments field on the Post model. The comments field should be of type `HasMany<Post, Comment>`.
///
/// ## Example
///
/// ```rust
/// # use ensemble::{Model, relationships::HasMany};
/// # #[derive(Debug, Model)]
/// # struct Comment {
/// #   id: u64,
/// # }
/// #[derive(Debug, Model)]
/// struct Post {
///   id: u64,
///   title: String,
///   content: String,
///   comments: HasMany<Post, Comment>
/// }
///
/// # async fn call() -> Result<(), ensemble::query::Error> {
/// let mut post = Post::find(1).await?;
///
/// let comments: &Vec<Comment> = post.comments().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default)]
pub struct HasMany<Local: Model, Related: Model> {
    foreign_key: String,
    relation: Option<Vec<Related>>,
    /// The value of the local model's primary key.
    pub value: Local::PrimaryKey,
}

#[async_trait::async_trait]
impl<Local: Model, Related: Model> Relationship for HasMany<Local, Related> {
    type Value = Vec<Related>;
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
    }

    fn query(&self) -> Builder {
        Related::query()
            .r#where(
                &format!("{}.{}", Related::TABLE_NAME, self.foreign_key),
                "=",
                self.value.clone(),
            )
            .where_not_null(&format!("{}.{}", Related::TABLE_NAME, self.foreign_key))
    }

    /// Get the related models.
    async fn get(&mut self) -> Result<&Self::Value, Error> {
        if self.relation.is_none() {
            let relation = self.query().get().await?;

            self.relation = Some(relation);
        }

        Ok(self.relation.as_ref().unwrap())
    }

    fn r#match(&mut self, related: &[HashMap<String, Value>]) -> Result<(), Error> {
        let related = find_related(related, &self.foreign_key, &self.value, false)?;

        if !related.is_empty() {
            self.relation = Some(related);
        }

        Ok(())
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
