use serde::Serialize;

use self::{de::deserialize_value, ser::fast_serialize};
use crate::Model;

mod de;
mod ser;

/// Serialize a model for the database.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn for_db<T: Serialize>(value: T) -> Result<rbs::Value, rbs::Error> {
    fast_serialize(value)
}

/// Deserialize a model from the database.
///
/// # Errors
///
/// Returns an error if deserialization fails.
pub(crate) fn from<M: Model>(value: rbs::Value) -> Result<M, rbs::Error> {
    deserialize_value::<M>(value)
}
