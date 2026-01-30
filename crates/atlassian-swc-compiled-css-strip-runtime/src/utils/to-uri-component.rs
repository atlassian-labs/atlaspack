/// Escapes a CSS rule so it can be safely appended to a query parameter.
pub fn to_uri_component(rule: &str) -> String {
  let encoded = urlencoding::encode(rule);
  encoded.replace('!', "%21")
}
