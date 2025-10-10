pub fn extract_val_default<T>(map: &mut serde_json::Map<String, serde_json::Value>, key: &str) -> T
where
  T: serde::de::DeserializeOwned + Default,
{
  extract_val(map, key).unwrap_or_default()
}

pub fn extract_val<T>(map: &mut serde_json::Map<String, serde_json::Value>, key: &str) -> Option<T>
where
  T: serde::de::DeserializeOwned,
{
  let val = map.get(key).and_then(|v| T::deserialize(v).ok());
  map.remove(key);
  val
}
