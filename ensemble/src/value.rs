use rbs::{value::map::ValueMap, Value};
use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct,
    },
    Serialize,
};
use std::collections::HashMap;

use crate::Model;

pub(crate) fn from<M: Model>(value: Value) -> Result<M, rbs::Error> {
    rbs::from_value::<M>(value)
}

/// Serialize a model for the database.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn for_db<T: Serialize>(value: T) -> Result<Value, rbs::Error> {
    value.serialize(Serializer)
}

struct Serializer;

impl serde::Serializer for Serializer {
    type Ok = rbs::Value;
    type Error = rbs::Error;

    type SerializeSeq = SerializeVec;
    type SerializeMap = MapSerializer;
    type SerializeTuple = SerializeVec;
    type SerializeStruct = StructSerializer;
    type SerializeTupleStruct = SerializeVec;
    type SerializeStructVariant = StructSerializer;
    type SerializeTupleVariant = SerializeTupleVariant;

    #[inline]
    fn serialize_bool(self, val: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Bool(val))
    }

    #[inline]
    fn serialize_i8(self, val: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::I32(i32::from(val)))
    }

    #[inline]
    fn serialize_i16(self, val: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::I32(i32::from(val)))
    }

    #[inline]
    fn serialize_i32(self, val: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::I32(val))
    }

    #[inline]
    fn serialize_i64(self, val: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::I64(val))
    }

    #[inline]
    fn serialize_u8(self, val: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::U32(u32::from(val)))
    }

    #[inline]
    fn serialize_u16(self, val: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::U32(u32::from(val)))
    }

    #[inline]
    fn serialize_u32(self, val: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::U32(val))
    }

    #[inline]
    fn serialize_u64(self, val: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::U64(val))
    }

    #[inline]
    fn serialize_f32(self, val: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::F32(val))
    }

    #[inline]
    fn serialize_f64(self, val: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::F64(val))
    }

    #[inline]
    fn serialize_char(self, val: char) -> Result<Self::Ok, Self::Error> {
        let mut buf = String::new();
        buf.push(val);
        self.serialize_str(&buf)
    }

    #[inline]
    fn serialize_str(self, val: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::String(val.into()))
    }

    #[inline]
    fn serialize_bytes(self, val: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Binary(val.into()))
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _idx: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Ext(name, Box::new(value.serialize(self)?)))
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        _idx: u32,
        variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(rbs::Error::Syntax(format!(
            "Ensemble does not support enums with values: {name}::{variant}",
        )))
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let se = SerializeVec {
            vec: Vec::with_capacity(len.unwrap_or(0)),
        };
        Ok(se)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(rbs::Error::Syntax(
            "Ensemble does not support tuples here.".to_string(),
        ))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(rbs::Error::Syntax(format!(
            "Ensemble does not support structs here: {name}"
        )))
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _idx: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(rbs::Error::Syntax(format!(
            "Ensemble does not support enums with values: {name}::{variant}",
        )))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let se = MapSerializer {
            next_key: None,
            map: HashMap::with_capacity(len.unwrap_or(0)),
        };
        Ok(se)
    }

    #[inline]
    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        let se = StructSerializer {
            map: Vec::with_capacity(len),
        };
        Ok(se)
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        name: &'static str,
        _idx: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(rbs::Error::Syntax(format!(
            "Ensemble does not support enums with values: {name}::{variant}",
        )))
    }
}

struct SerializeVec {
    vec: Vec<Value>,
}

impl SerializeSeq for SerializeVec {
    type Ok = Value;
    type Error = rbs::Error;

    #[inline]
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.vec.push(value.serialize(Serializer)?);
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Value, Self::Error> {
        Ok(Value::Array(self.vec))
    }
}

impl SerializeTuple for SerializeVec {
    type Ok = Value;
    type Error = rbs::Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error> {
        unreachable!("Ensemble does not support tuples here.")
    }

    fn end(self) -> Result<Value, Self::Error> {
        unreachable!("Ensemble does not support tuples here.")
    }
}

impl SerializeTupleStruct for SerializeVec {
    type Ok = Value;
    type Error = rbs::Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error> {
        unreachable!("Ensemble does not support structs here.")
    }

    fn end(self) -> Result<Value, Self::Error> {
        unreachable!("Ensemble does not support structs here.")
    }
}

struct SerializeTupleVariant {}

impl serde::ser::SerializeTupleVariant for SerializeTupleVariant {
    type Ok = Value;
    type Error = rbs::Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error> {
        unreachable!("Ensemble does not support enums with values.")
    }

    fn end(self) -> Result<Value, Self::Error> {
        unreachable!("Ensemble does not support enums with values.")
    }
}

struct MapSerializer {
    map: HashMap<String, Value>,
    next_key: Option<Value>,
}

impl SerializeMap for MapSerializer {
    type Ok = Value;
    type Error = rbs::Error;

    #[inline]
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.next_key = Some(key.serialize(Serializer)?);
        Ok(())
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        let key = self
            .next_key
            .take()
            .expect("`serialize_value` called before `serialize_key`");

        if let Value::String(key) = key {
            self.map.insert(key, value.serialize(Serializer)?);
            Ok(())
        } else {
            Err(rbs::Error::Syntax(
                "Ensemble only supports string keys.".to_string(),
            ))
        }
    }

    #[inline]
    fn end(self) -> Result<Value, Self::Error> {
        Ok(Value::Ext(
            "Json",
            Box::new(Value::String(serde_json::to_string(&self.map).map_err(
                |e| rbs::Error::Syntax(format!("Failed to serialize into JSON: {}", e)),
            )?)),
        ))
    }
}

struct StructSerializer {
    map: Vec<(Value, Value)>,
}

impl SerializeStruct for StructSerializer {
    type Ok = Value;
    type Error = rbs::Error;

    #[inline]
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.map.push((
            Value::String(key.to_string()),
            (value).serialize(Serializer)?,
        ));
        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Map(ValueMap(self.map)))
    }
}

impl SerializeStructVariant for StructSerializer {
    type Ok = Value;
    type Error = rbs::Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        _key: &'static str,
        _value: &T,
    ) -> Result<(), Self::Error> {
        unreachable!("Ensemble does not support enums with values.")
    }

    fn end(self) -> Result<Value, Self::Error> {
        unreachable!("Ensemble does not support enums with values.")
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{DateTime, Hashed, Uuid};

    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Test {
        a: i32,
        b: String,
        c: Vec<u8>,
    }

    #[test]
    fn test_serialize() {
        let test = Test {
            a: 1,
            b: "test".to_string(),
            c: vec![1, 2, 3],
        };

        assert_eq!(
            for_db(test).unwrap(),
            rbs::to_value! {
                "a" : 1,
                "b" : "test",
                "c" : [1u32, 2u32, 3u32],
            }
        );
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Status {
        Ok,
        Error,
        ThirdThing,
    }

    #[test]
    fn test_serialize_enum() {
        assert_eq!(for_db(Status::Ok).unwrap(), rbs::to_value!("Ok"));
        assert_eq!(for_db(Status::Error).unwrap(), rbs::to_value!("Error"));
        assert_eq!(
            for_db(Status::ThirdThing).unwrap(),
            rbs::to_value!("ThirdThing")
        );
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum StatusV2 {
        Ok,
        Error,
        ThirdThing,
    }

    #[test]
    fn test_serialize_enum_with_custom_config() {
        assert_eq!(for_db(StatusV2::Ok).unwrap(), rbs::to_value!("ok"));
        assert_eq!(for_db(StatusV2::Error).unwrap(), rbs::to_value!("error"));
        assert_eq!(
            for_db(StatusV2::ThirdThing).unwrap(),
            rbs::to_value!("third_thing")
        );
    }

    #[test]
    fn properly_serializes_datetime() {
        let datetime = DateTime::now();

        assert_eq!(
            for_db(&datetime).unwrap(),
            Value::Ext("DateTime", Box::new(rbs::to_value!(datetime.0)))
        );
    }

    #[test]
    fn properly_serializes_uuid() {
        let uuid = Uuid::new();

        assert_eq!(
            for_db(&uuid).unwrap(),
            Value::Ext("Uuid", Box::new(Value::String(uuid.to_string())))
        );
    }

    #[test]
    fn properly_serializes_hashed() {
        let hashed = Hashed::new("hello-world");

        assert_eq!(for_db(&hashed).unwrap(), Value::String(hashed.to_string()));
    }

    #[test]
    fn properly_serializes_json() {
        let json = json!({
            "hello": "world",
            "foo": "bar",
        });

        assert_eq!(
            for_db(&json).unwrap(),
            Value::Ext("Json", Box::new(Value::String(json.to_string())))
        );
    }
}
