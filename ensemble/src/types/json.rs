use rbs::Value;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use std::{
    ops::{Deref, DerefMut},
    str::FromStr,
};

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[repr(transparent)]
pub struct Json<T: DeserializeOwned = serde_json::Value>(pub T);

impl FromStr for Json {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(serde_json::from_str(s)?))
    }
}

impl<T: Serialize + DeserializeOwned> Serialize for Json<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::Error;
        if std::any::type_name::<S::Error>() == std::any::type_name::<rbs::Error>() {
            serializer.serialize_newtype_struct(
                "Json",
                &serde_json::to_string(&self.0).map_err(|e| Error::custom(e.to_string()))?,
            )
        } else {
            self.0.serialize(serializer)
        }
    }
}

impl<'de, T: Serialize + DeserializeOwned> Deserialize<'de> for Json<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        if std::any::type_name::<D::Error>() == std::any::type_name::<rbs::Error>() {
            let mut v = Value::deserialize(deserializer)?;
            if let Value::Ext(_ty, buf) = v {
                v = *buf;
            }

            let js;
            if let Value::Binary(buf) = v {
                js = String::from_utf8(buf).map_err(|e| Error::custom(e.to_string()))?;
            } else if let Value::String(buf) = v {
                js = buf;
            } else {
                js = v.to_string();
            }

            Ok(Self(
                serde_json::from_str(&js).map_err(|e| Error::custom(e.to_string()))?,
            ))
        } else {
            Ok(Self(T::deserialize(deserializer)?))
        }
    }
}

impl<T: Serialize + DeserializeOwned> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Serialize + DeserializeOwned> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "schema")]
impl<T: Serialize + DeserializeOwned + JsonSchema> schemars::JsonSchema for Json<T> {
    fn schema_name() -> String {
        T::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        T::json_schema(gen)
    }
}
