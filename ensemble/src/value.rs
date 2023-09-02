use crate::Model;
use rbs::Value;

pub(crate) fn from<M: Model>(value: Value) -> Result<M, rbs::Error> {
    rbs::from_value::<M>(value)
}

#[allow(clippy::module_name_repetitions)]
pub fn to_value<T: serde::Serialize>(value: T) -> Value {
    let value = rbs::to_value(value).unwrap_or_default();

    match std::any::type_name::<T>() {
        "uuid::Uuid" => Value::Ext("Uuid", Box::new(value)),
        _ => value,
    }
}
