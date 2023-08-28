use rbs::{to_value, Value};

use crate::Model;

pub fn into<M: Model>(model: &M) -> Vec<Value> {
    to_value!(model)
        .into_iter()
        .map(|(_, value)| value)
        .collect::<Vec<_>>()
}

pub fn from<M: Model>(value: Value) -> Result<M, rbs::Error> {
    rbs::from_value::<M>(value)
}
