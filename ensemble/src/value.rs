use rbs::Value;

use crate::Model;

pub fn from<M: Model>(value: Value) -> Result<M, rbs::Error> {
    rbs::from_value::<M>(value)
}
