use inflector::Inflector;
use rbs::Value;
use serde::Serialize;
use std::{collections::HashMap, fmt::Debug};

use super::{find_related, Relationship, Status};
use crate::{query::Builder, value::serializing_for_db, Error, Model};

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
/// # async fn call() -> Result<(), ensemble::Error> {
/// let mut user = User::find(1).await?;
///
/// let phone: &Phone = user.phone().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default)]
pub struct HasOne<Local: Model, Related: Model> {
	foreign_key: String,
	relation: Status<Related>,
	/// The value of the local model's primary key.
	pub value: Local::PrimaryKey,
}

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
			relation: Status::initial(),
		}
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

	async fn get(&mut self) -> Result<&mut Self::Value, Error> {
		if self.relation.is_none() {
			let relation = self.query().first().await?.ok_or(Error::NotFound)?;

			self.relation = Status::Fetched(Some(relation));
		}

		Ok(self.relation.as_mut().unwrap())
	}

	fn is_loaded(&self) -> bool {
		self.relation.is_loaded()
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

	fn r#match(&mut self, related: &[HashMap<String, Value>]) -> Result<(), Error> {
		let related = find_related(related, &self.foreign_key, &self.value, true)?;

		self.relation = Status::Fetched(related.into_iter().next());

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
	for HasOne<Local, Related>
{
	fn schema_name() -> String {
		<Option<Related>>::schema_name()
	}

	fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
		gen.subschema_for::<Option<Related>>()
	}
}
