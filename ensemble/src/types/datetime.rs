use rbs::Value;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};
use std::{
	fmt::{Debug, Display, Formatter},
	ops::{Add, Deref, DerefMut, Sub},
	str::FromStr,
	time::{Duration, SystemTime},
};

/// A date and time value, used for storing timestamps in the database.
#[derive(Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
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

	#[must_use]
	pub fn from_system_time(s: SystemTime, offset: i32) -> Self {
		Self(fastdate::DateTime::from_system_time(s, offset))
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

impl Default for DateTime {
	fn default() -> Self {
		Self(fastdate::DateTime::from_timestamp(0))
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

#[cfg(test)]
mod test {
	use super::*;
	use std::str::FromStr;

	#[test]
	fn test_ser_de() {
		let dt = DateTime::now();
		let v = serde_json::to_value(&dt).unwrap();
		let new_dt: DateTime = serde_json::from_value(v).unwrap();
		assert_eq!(new_dt, dt);
	}

	#[test]
	fn test_de() {
		let dt = DateTime::from_str("2023-10-21T00:15:00.9233333+08:00").unwrap();

		let v = serde_json::to_value(&dt).unwrap();
		let new_dt: DateTime = serde_json::from_value(v).unwrap();
		assert_eq!(new_dt, dt);
	}

	#[test]
	fn test_de2() {
		let dt = vec![DateTime::from_str("2023-10-21T00:15:00.9233333+08:00").unwrap()];
		let v = serde_json::to_value(&dt).unwrap();

		let new_dt: Vec<DateTime> = serde_json::from_value(v).unwrap();
		assert_eq!(new_dt, dt);
	}

	#[test]
	fn test_de3() {
		let dt = vec![DateTime::from_str("2023-10-21T00:15:00.9233333+08:00").unwrap()];
		let v = rbs::to_value!(&dt);
		let new_dt: Vec<DateTime> = rbs::from_value(v).unwrap();
		assert_eq!(new_dt, dt);
	}

	#[test]
	fn test_de4() {
		let dt = DateTime::from_str("2023-10-21T00:15:00.9233333+08:00").unwrap();
		let v = rbs::to_value!(&dt.unix_timestamp_millis());
		let new_dt: DateTime = rbs::from_value(v).unwrap();
		assert_eq!(
			new_dt,
			DateTime::from_str("2023-10-20T16:15:00.923Z").unwrap()
		);
	}

	#[test]
	fn test_de5() {
		let dt = DateTime::from_str("2023-10-21T00:15:00.9233333+08:00").unwrap();
		let v = serde_json::to_value(dt.unix_timestamp_millis()).unwrap();
		let new_dt: DateTime = serde_json::from_value(v).unwrap();
		assert_eq!(
			new_dt,
			DateTime::from_str("2023-10-20T16:15:00.923Z").unwrap()
		);
	}

	#[test]
	fn test_default() {
		let dt = DateTime::default();

		assert_eq!(dt.to_string(), "DateTime(1970-01-01T00:00:00Z)");
	}

	#[test]
	fn test_format() {
		let dt = DateTime::default();
		let s = dt.format("YYYY-MM-DD/hh/mm/ss");

		assert_eq!(s, "1970-1-1/0/0/0");
	}
}
