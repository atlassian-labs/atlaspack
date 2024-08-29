use serde::Serializer;
use serde::{de, Deserialize, Deserializer};
use std::str::FromStr;

pub fn de_from_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
  T: FromStr,
  D: Deserializer<'de>,
{
  let s = String::deserialize(deserializer)?;
  let Ok(value) = T::from_str(&s) else {
    return Err(de::Error::custom(format!("Unable to deserialize {}", s)));
  };
  Ok(value) //.map_err(de::Error::custom)
}

pub fn se_to_string<S, T>(t: &T, serializer: S) -> Result<S::Ok, S::Error>
where
  T: ToString,
  S: Serializer,
{
  serializer.serialize_str(&T::to_string(t))
}
