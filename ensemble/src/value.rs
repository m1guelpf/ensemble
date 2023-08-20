use rbs::{to_value, Value};

use crate::Model;

pub fn into<M: Model>(model: &M) -> Vec<Value> {
    to_value!(model)
        .into_iter()
        .map(into_value)
        .collect::<Vec<_>>()
}

fn into_value((_, value): (Value, Value)) -> Value {
    value
}
