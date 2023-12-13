use std::{
	fmt::{self, Debug},
	vec::IntoIter,
};

use rbs::{value::map::ValueMap, Value};
use serde::{
	de::{self, IntoDeserializer, Unexpected, Visitor},
	forward_to_deserialize_any, Deserialize, Deserializer,
};

#[inline]
pub fn deserialize_value<'de, T: Deserialize<'de>>(val: rbs::Value) -> Result<T, rbs::Error> {
	Deserialize::deserialize(ValueDeserializer(val))
}

#[repr(transparent)]
struct ValueDeserializer(rbs::Value);

trait ValueBase<'de>: Deserializer<'de, Error = rbs::Error> {
	type Item: ValueBase<'de>;
	type MapDeserializer: Deserializer<'de>;
	type Iter: ExactSizeIterator<Item = Self::Item>;
	type MapIter: Iterator<Item = (Self::Item, Self::Item)>;

	fn is_null(&self) -> bool;
	fn unexpected(&self) -> Unexpected<'_>;

	fn into_iter(self) -> Result<Self::Iter, Self::Item>;
	fn into_map_iter(self) -> Result<Self::MapIter, Self::Item>;
}

impl<'de> ValueBase<'de> for Value {
	type Item = ValueDeserializer;
	type Iter = IntoIter<Self::Item>;
	type MapIter = IntoIter<(Self::Item, Self::Item)>;
	type MapDeserializer = MapDeserializer<Self::MapIter, Self::Item>;

	#[inline]
	fn is_null(&self) -> bool {
		matches!(self, Self::Null)
	}

	#[inline]
	fn into_iter(self) -> Result<Self::Iter, Self::Item> {
		match self {
			Self::Array(v) => Ok(v
				.into_iter()
				.map(ValueDeserializer)
				.collect::<Vec<_>>()
				.into_iter()),
			other => Err(other.into()),
		}
	}

	#[inline]
	fn into_map_iter(self) -> Result<Self::MapIter, Self::Item> {
		match self {
			Self::Map(v) => Ok(v
				.0
				.into_iter()
				.map(|(k, v)| (ValueDeserializer(k), ValueDeserializer(v)))
				.collect::<Vec<_>>()
				.into_iter()),
			other => Err(other.into()),
		}
	}

	#[cold]
	fn unexpected(&self) -> Unexpected<'_> {
		match *self {
			Self::Null => Unexpected::Unit,
			Self::Map(..) => Unexpected::Map,
			Self::F64(v) => Unexpected::Float(v),
			Self::Bool(v) => Unexpected::Bool(v),
			Self::I64(v) => Unexpected::Signed(v),
			Self::U64(v) => Unexpected::Unsigned(v),
			Self::Ext(..) | Self::Array(..) => Unexpected::Seq,
			Self::F32(v) => Unexpected::Float(f64::from(v)),
			Self::I32(v) => Unexpected::Signed(i64::from(v)),
			Self::Binary(ref v) => Unexpected::Bytes(v),
			Self::U32(v) => Unexpected::Unsigned(u64::from(v)),
			Self::String(ref v) => Unexpected::Bytes(v.as_bytes()),
		}
	}
}

impl<'de> ValueBase<'de> for ValueDeserializer {
	type Item = Self;
	type Iter = IntoIter<Self::Item>;
	type MapIter = IntoIter<(Self::Item, Self::Item)>;
	type MapDeserializer = MapDeserializer<Self::MapIter, Self::Item>;

	#[inline]
	fn is_null(&self) -> bool {
		self.0.is_null()
	}

	#[inline]
	fn into_iter(self) -> Result<Self::Iter, Self::Item> {
		match self.0 {
			Value::Array(v) => Ok(v
				.into_iter()
				.map(ValueDeserializer)
				.collect::<Vec<_>>()
				.into_iter()),
			other => Err(other.into()),
		}
	}

	#[inline]
	fn into_map_iter(self) -> Result<Self::MapIter, Self::Item> {
		match self.0 {
			Value::Map(v) => Ok(v
				.0
				.into_iter()
				.map(|(k, v)| (Self(k), Self(v)))
				.collect::<Vec<_>>()
				.into_iter()),
			other => Err(other.into()),
		}
	}

	#[cold]
	fn unexpected(&self) -> Unexpected<'_> {
		match self.0 {
			Value::Null => Unexpected::Unit,
			Value::Map(..) => Unexpected::Map,
			Value::F64(v) => Unexpected::Float(v),
			Value::I64(v) => Unexpected::Signed(v),
			Value::Bool(v) => Unexpected::Bool(v),
			Value::U64(v) => Unexpected::Unsigned(v),
			Value::Ext(..) | Value::Array(..) => Unexpected::Seq,
			Value::F32(v) => Unexpected::Float(f64::from(v)),
			Value::I32(v) => Unexpected::Signed(i64::from(v)),
			Value::Binary(ref v) => Unexpected::Bytes(v),
			Value::U32(v) => Unexpected::Unsigned(u64::from(v)),
			Value::String(ref v) => Unexpected::Bytes(v.as_bytes()),
		}
	}
}

impl From<Value> for ValueDeserializer {
	#[inline]
	fn from(value: Value) -> Self {
		Self(value)
	}
}

impl From<ValueDeserializer> for Value {
	#[inline]
	fn from(value: ValueDeserializer) -> Self {
		value.0
	}
}

impl<'de> Deserialize<'de> for ValueDeserializer {
	#[inline]
	#[allow(clippy::too_many_lines)]
	fn deserialize<D>(de: D) -> Result<Self, D::Error>
	where
		D: de::Deserializer<'de>,
	{
		struct ValueVisitor;

		impl<'de> serde::de::Visitor<'de> for ValueVisitor {
			type Value = ValueDeserializer;

			#[cold]
			fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
				"any valid MessagePack value".fmt(fmt)
			}

			#[inline]
			fn visit_some<D>(self, de: D) -> Result<Self::Value, D::Error>
			where
				D: de::Deserializer<'de>,
			{
				Deserialize::deserialize(de)
			}

			#[inline]
			fn visit_none<E>(self) -> Result<Self::Value, E> {
				Ok(Value::Null.into())
			}

			#[inline]
			fn visit_unit<E>(self) -> Result<Self::Value, E> {
				Ok(Value::Null.into())
			}

			#[inline]
			fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
				Ok(Value::Bool(value).into())
			}

			fn visit_u32<E: de::Error>(self, v: u32) -> Result<Self::Value, E> {
				Ok(Value::U32(v).into())
			}

			#[inline]
			fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
				Ok(Value::U64(value).into())
			}

			fn visit_i32<E: de::Error>(self, v: i32) -> Result<Self::Value, E> {
				Ok(Value::I32(v).into())
			}

			#[inline]
			fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
				Ok(Value::I64(value).into())
			}

			#[inline]
			fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E> {
				Ok(Value::F32(value).into())
			}

			#[inline]
			fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
				Ok(Value::F64(value).into())
			}

			#[inline]
			fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
				Ok(Value::String(value).into())
			}

			#[inline]
			fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
				self.visit_string(String::from(value))
			}

			#[inline]
			fn visit_seq<V: de::SeqAccess<'de>>(
				self,
				mut visitor: V,
			) -> Result<Self::Value, V::Error> {
				let mut vec = {
					visitor
						.size_hint()
						.map_or_else(Vec::new, Vec::with_capacity)
				};
				while let Some(elem) = visitor.next_element::<ValueDeserializer>()? {
					vec.push(elem.into());
				}
				Ok(Value::Array(vec).into())
			}

			#[inline]
			fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
				Ok(Value::Binary(v.to_owned()).into())
			}

			#[inline]
			fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
				Ok(Value::Binary(v).into())
			}

			#[inline]
			fn visit_map<V: de::MapAccess<'de>>(
				self,
				mut visitor: V,
			) -> Result<Self::Value, V::Error> {
				let mut pairs = {
					visitor
						.size_hint()
						.map_or_else(Vec::new, Vec::with_capacity)
				};
				while let Some(key) = visitor.next_key::<ValueDeserializer>()? {
					let val = visitor.next_value::<ValueDeserializer>()?;
					pairs.push((key.into(), val.into()));
				}

				Ok(Value::Map(ValueMap(pairs)).into())
			}

			fn visit_newtype_struct<D: Deserializer<'de>>(
				self,
				deserializer: D,
			) -> Result<Self::Value, D::Error> {
				deserializer.deserialize_newtype_struct("", self)
			}
		}

		de.deserialize_any(ValueVisitor)
	}
}

impl<'de> Deserializer<'de> for ValueDeserializer {
	type Error = rbs::Error;

	fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.into() {
			Value::Null => visitor.visit_unit(),
			Value::I32(v) => visitor.visit_i32(v),
			Value::I64(v) => visitor.visit_i64(v),
			Value::U32(v) => visitor.visit_u32(v),
			Value::U64(v) => visitor.visit_u64(v),
			Value::F32(v) => visitor.visit_f32(v),
			Value::F64(v) => visitor.visit_f64(v),
			Value::Bool(v) => visitor.visit_bool(v),
			Value::String(v) => visitor.visit_string(v),
			Value::Binary(v) => visitor.visit_byte_buf(v),
			Value::Array(v) => {
				let len = v.len();
				let mut de = SeqDeserializer {
					iter: v.into_iter().map(ValueDeserializer),
				};
				let seq = visitor.visit_seq(&mut de)?;
				if de.iter.len() == 0 {
					Ok(seq)
				} else {
					Err(de::Error::invalid_length(len, &"fewer elements in array"))
				}
			},
			Value::Map(v) => {
				let len = v.len();
				let mut de = MapDeserializer {
					val: None,
					iter: v.0.into_iter().map(|(k, v)| (Self(k), Self(v))),
				};
				let map = visitor.visit_map(&mut de)?;
				if de.iter.len() == 0 {
					Ok(map)
				} else {
					Err(de::Error::invalid_length(len, &"fewer elements in map"))
				}
			},
			Value::Ext(_tag, data) => Deserializer::deserialize_any(Self(*data), visitor),
		}
	}

	#[inline]
	fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		if self.0.is_null() {
			visitor.visit_none()
		} else {
			visitor.visit_some(self)
		}
	}

	#[inline]
	fn deserialize_enum<V>(
		self,
		_name: &str,
		_variants: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.0 {
			Value::String(variant) => visitor.visit_enum(variant.into_deserializer()),
			Value::Array(iter) => {
				let mut iter = iter.into_iter();
				if !(iter.len() == 1 || iter.len() == 2) {
					return Err(de::Error::invalid_length(
						iter.len(),
						&"array with one or two elements",
					));
				}

				let id = match iter.next() {
					Some(id) => deserialize_value(id)?,
					None => {
						return Err(de::Error::invalid_value(
							Unexpected::Seq,
							&"array with one or two elements",
						));
					},
				};

				visitor.visit_enum(EnumDeserializer {
					id,
					value: iter.next(),
				})
			},
			other => Err(de::Error::invalid_type(
				other.unexpected(),
				&"string, array, map or int",
			)),
		}
	}

	#[inline]
	fn deserialize_newtype_struct<V>(
		self,
		_name: &'static str,
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_newtype_struct(self)
	}

	#[inline]
	fn deserialize_unit_struct<V>(
		self,
		_name: &'static str,
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		match self.0 {
			Value::Array(iter) => {
				let iter = iter.into_iter();

				if iter.len() == 0 {
					visitor.visit_unit()
				} else {
					Err(de::Error::invalid_type(Unexpected::Seq, &"empty array"))
				}
			},
			other => Err(de::Error::invalid_type(other.unexpected(), &"empty array")),
		}
	}

	forward_to_deserialize_any! {
		bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string unit seq
		bytes byte_buf map tuple_struct struct
		identifier tuple ignored_any
	}
}

struct SeqDeserializer<I> {
	iter: I,
}

impl<'de, I, U> de::SeqAccess<'de> for SeqDeserializer<I>
where
	I: Iterator<Item = U>,
	U: Deserializer<'de, Error = rbs::Error>,
{
	type Error = rbs::Error;

	fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
	where
		T: de::DeserializeSeed<'de>,
	{
		self.iter
			.next()
			.map_or_else(|| Ok(None), |val| seed.deserialize(val).map(Some))
	}
}

impl<'de, I, U> Deserializer<'de> for SeqDeserializer<I>
where
	I: ExactSizeIterator<Item = U>,
	U: Deserializer<'de, Error = rbs::Error>,
{
	type Error = rbs::Error;

	#[inline]
	fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		let len = self.iter.len();
		if len == 0 {
			visitor.visit_unit()
		} else {
			let value = visitor.visit_seq(&mut self)?;

			if self.iter.len() == 0 {
				Ok(value)
			} else {
				Err(de::Error::invalid_length(len, &"fewer elements in array"))
			}
		}
	}

	forward_to_deserialize_any! {
		bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string unit option
		seq bytes byte_buf map unit_struct newtype_struct
		tuple_struct struct identifier tuple enum ignored_any
	}
}

struct MapDeserializer<I, U> {
	iter: I,
	val: Option<U>,
}

impl<'de, I, U> de::MapAccess<'de> for MapDeserializer<I, U>
where
	I: Iterator<Item = (U, U)>,
	U: ValueBase<'de>,
{
	type Error = rbs::Error;

	fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
	where
		T: de::DeserializeSeed<'de>,
	{
		match self.iter.next() {
			Some((key, val)) => {
				self.val = Some(val);
				seed.deserialize(key).map(Some)
			},
			None => Ok(None),
		}
	}

	fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Self::Error>
	where
		T: de::DeserializeSeed<'de>,
	{
		Option::take(&mut self.val).map_or_else(
			|| Err(de::Error::custom("value is missing")),
			|val| seed.deserialize(val),
		)
	}
}

impl<'de, I, U> Deserializer<'de> for MapDeserializer<I, U>
where
	U: ValueBase<'de>,
	I: Iterator<Item = (U, U)>,
{
	type Error = rbs::Error;

	#[inline]
	fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		visitor.visit_map(self)
	}

	forward_to_deserialize_any! {
		bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string unit option
		seq bytes byte_buf map unit_struct newtype_struct
		tuple_struct struct identifier tuple enum ignored_any
	}
}

struct EnumDeserializer<U> {
	id: u32,
	value: Option<U>,
}

impl<'de, U: ValueBase<'de>> de::EnumAccess<'de> for EnumDeserializer<U> {
	type Error = rbs::Error;
	type Variant = VariantDeserializer<U>;

	fn variant_seed<V: de::DeserializeSeed<'de>>(
		self,
		seed: V,
	) -> Result<(V::Value, Self::Variant), Self::Error> {
		let variant = self.id.into_deserializer();
		let visitor = VariantDeserializer { value: self.value };
		seed.deserialize(variant).map(|v| (v, visitor))
	}
}

struct VariantDeserializer<U> {
	value: Option<U>,
}

impl<'de, U: ValueBase<'de>> de::VariantAccess<'de> for VariantDeserializer<U> {
	type Error = rbs::Error;

	fn unit_variant(self) -> Result<(), Self::Error> {
		// Can accept only [u32].
		self.value.map_or(Ok(()), |v| match v.into_iter() {
			Ok(ref v) if v.len() == 0 => Ok(()),
			Ok(..) => Err(de::Error::invalid_value(Unexpected::Seq, &"empty array")),
			Err(v) => Err(de::Error::invalid_value(v.unexpected(), &"empty array")),
		})
	}

	fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
	where
		T: de::DeserializeSeed<'de>,
	{
		// Can accept both [u32, T...] and [u32, [T]] cases.
		match self.value {
			Some(v) => match v.into_iter() {
				Ok(mut iter) => {
					if iter.len() > 1 {
						seed.deserialize(SeqDeserializer { iter })
					} else {
						let val = match iter.next() {
							Some(val) => seed.deserialize(val),
							None => {
								return Err(de::Error::invalid_value(
									Unexpected::Seq,
									&"array with one element",
								));
							},
						};

						if iter.next().is_some() {
							Err(de::Error::invalid_value(
								Unexpected::Seq,
								&"array with one element",
							))
						} else {
							val
						}
					}
				},
				Err(v) => seed.deserialize(v),
			},
			None => Err(de::Error::invalid_type(
				Unexpected::UnitVariant,
				&"newtype variant",
			)),
		}
	}

	fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		// Can accept [u32, [T...]].
		self.value.map_or_else(
			|| {
				Err(de::Error::invalid_type(
					Unexpected::UnitVariant,
					&"tuple variant",
				))
			},
			|v| match v.into_iter() {
				Ok(v) => Deserializer::deserialize_any(SeqDeserializer { iter: v }, visitor),
				Err(v) => Err(de::Error::invalid_type(v.unexpected(), &"tuple variant")),
			},
		)
	}

	fn struct_variant<V>(
		self,
		_fields: &'static [&'static str],
		visitor: V,
	) -> Result<V::Value, Self::Error>
	where
		V: Visitor<'de>,
	{
		self.value.map_or_else(
			|| {
				Err(de::Error::invalid_type(
					Unexpected::UnitVariant,
					&"struct variant",
				))
			},
			|v| match v.into_iter() {
				Ok(iter) => Deserializer::deserialize_any(SeqDeserializer { iter }, visitor),
				Err(v) => match v.into_map_iter() {
					Ok(iter) => {
						Deserializer::deserialize_any(MapDeserializer { iter, val: None }, visitor)
					},
					Err(v) => Err(de::Error::invalid_type(v.unexpected(), &"struct variant")),
				},
			},
		)
	}
}
