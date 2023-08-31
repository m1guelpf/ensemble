use rbs::Value;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Deref, DerefMut, Sub};
use std::str::FromStr;
use std::time::Duration;

/// A date and time value, used for storing timestamps in the database.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct DateTime(pub fastdate::DateTime);

impl Display for DateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DateTime({})", self.0)
    }
}

impl Serialize for DateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("DateTime", &self.0)
    }
}

impl Debug for DateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DateTime({})", self.0)
    }
}

impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = Value::deserialize(deserializer)?;

        match v {
            Value::I32(u) => Ok(Self(fastdate::DateTime::from_timestamp_millis(i64::from(
                u,
            )))),
            Value::U32(u) => Ok(Self(fastdate::DateTime::from_timestamp_millis(i64::from(
                u,
            )))),
            Value::I64(u) => Ok(Self(fastdate::DateTime::from_timestamp_millis(u))),
            Value::U64(u) => Ok(Self(fastdate::DateTime::from_timestamp_millis(
                i64::try_from(u).map_err(|e| D::Error::custom(e.to_string()))?,
            ))),
            Value::String(s) => Ok({
                Self(
                    fastdate::DateTime::from_str(&s)
                        .map_err(|e| D::Error::custom(e.to_string()))?,
                )
            }),
            _ => Err(D::Error::custom(
                &format!("unsupported type DateTime({v})",),
            )),
        }
    }
}

impl Deref for DateTime {
    type Target = fastdate::DateTime;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DateTime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DateTime {
    #[must_use]
    pub fn now() -> Self {
        Self(fastdate::DateTime::now())
    }

    #[must_use]
    pub fn utc() -> Self {
        Self(fastdate::DateTime::utc())
    }

    #[must_use]
    pub fn set_micro(mut self, micro: u32) -> Self {
        self.0 = self.0.set_micro(micro);
        self
    }

    #[must_use]
    pub fn set_sec(mut self, sec: u8) -> Self {
        self.0 = self.0.set_sec(sec);
        self
    }

    #[must_use]
    pub fn set_min(mut self, min: u8) -> Self {
        self.0 = self.0.set_min(min);
        self
    }

    #[must_use]
    pub fn set_hour(mut self, hour: u8) -> Self {
        self.0 = self.0.set_hour(hour);
        self
    }

    #[must_use]
    pub fn set_day(mut self, day: u8) -> Self {
        self.0 = self.0.set_day(day);
        self
    }

    #[must_use]
    pub fn set_mon(mut self, mon: u8) -> Self {
        self.0 = self.0.set_mon(mon);
        self
    }

    #[must_use]
    pub fn set_year(mut self, year: u16) -> Self {
        self.0 = self.0.set_year(year);
        self
    }

    #[must_use]
    pub fn from_timestamp(sec: i64) -> Self {
        Self(fastdate::DateTime::from_timestamp(sec))
    }

    #[must_use]
    pub fn from_timestamp_millis(ms: i64) -> Self {
        Self(fastdate::DateTime::from_timestamp_millis(ms))
    }

    #[must_use]
    pub fn from_timestamp_nano(nano: i128) -> Self {
        Self(fastdate::DateTime::from_timestamp_nano(nano))
    }
}

impl Sub for DateTime {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl Add<Duration> for DateTime {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0.add(rhs))
    }
}

impl Sub<Duration> for DateTime {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self::Output {
        Self(self.0.sub(rhs))
    }
}

impl FromStr for DateTime {
    type Err = rbs::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            fastdate::DateTime::from_str(s).map_err(|e| rbs::Error::Syntax(e.to_string()))?,
        ))
    }
}

impl From<DateTime> for Value {
    fn from(arg: DateTime) -> Self {
        Self::Ext("DateTime", Box::new(Self::String(arg.0.to_string())))
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for DateTime {
    fn is_referenceable() -> bool {
        false
    }

    fn schema_name() -> String {
        String::from("date-time")
    }

    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::SchemaObject {
            instance_type: Some(schemars::schema::InstanceType::String.into()),
            format: Some("date-time".to_owned()),
            ..Default::default()
        }
        .into()
    }
}
