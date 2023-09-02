use std::{
    fmt::{Debug, Display, Formatter},
    ops::{Deref, DerefMut},
    str::FromStr,
};

use rbs::Value;
use serde::Deserializer;

#[derive(serde::Serialize, Clone, Eq, PartialEq, Hash, Default)]
#[repr(transparent)]
pub struct Uuid(uuid::Uuid);

impl<'de> serde::Deserialize<'de> for Uuid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self(uuid::Uuid::deserialize(deserializer)?))
    }
}

impl Display for Uuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Debug for Uuid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Uuid({})", self.0)
    }
}

impl From<Uuid> for Value {
    fn from(uuid: Uuid) -> Self {
        Self::Ext("Uuid", Box::new(Self::String(uuid.0.to_string())))
    }
}

impl FromStr for Uuid {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl Uuid {
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    #[must_use]
    pub const fn nil() -> Self {
        Self(uuid::Uuid::nil())
    }
}

impl Deref for Uuid {
    type Target = uuid::Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Uuid {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for Uuid {
    fn schema_name() -> String {
        uuid::Uuid::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        uuid::Uuid::json_schema(gen)
    }
}
