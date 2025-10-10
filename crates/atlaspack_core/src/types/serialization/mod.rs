pub fn extract_bool(map: &mut serde_json::Map<String, serde_json::Value>, key: &str) -> bool {
  let val = map.get(key).and_then(|v| v.as_bool()).unwrap_or_default();
  map.remove(key);
  val
}

pub fn extract_string(
  map: &mut serde_json::Map<String, serde_json::Value>,
  key: &str,
) -> Option<String> {
  let val = map.get(key).and_then(|v| v.as_str().map(|s| s.to_string()));
  map.remove(key);
  val
}
