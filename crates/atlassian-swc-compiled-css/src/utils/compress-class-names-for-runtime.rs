use std::collections::BTreeMap;

/// Compress class names based on the optional compression map provided via the plugin options.
///
/// Mirrors `utils/compress-class-names-for-runtime.ts` by rewriting class names into the
/// `_aaaa_a` format expected by the runtime helper when a mapping exists. When the map is missing
/// or a class name does not have a compressed representation the original class name is returned.
pub fn compress_class_names_for_runtime(
  class_names: &[String],
  class_name_compression_map: Option<&BTreeMap<String, String>>,
) -> Vec<String> {
  match class_name_compression_map {
    Some(map) if !map.is_empty() => class_names
      .iter()
      .map(|class_name| compress_class_name(class_name, map))
      .collect(),
    _ => class_names.to_vec(),
  }
}

fn compress_class_name(class_name: &str, map: &BTreeMap<String, String>) -> String {
  if class_name.len() <= 1 {
    return class_name.to_string();
  }

  let suffix = &class_name[1..];
  match map.get(suffix) {
    Some(compressed) => {
      let end = class_name.len().min(5);
      let group = if end > 1 { &class_name[1..end] } else { "" };
      format!("_{}_{}", group, compressed)
    }
    None => class_name.to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::compress_class_names_for_runtime;
  use std::collections::BTreeMap;

  #[test]
  fn returns_original_names_when_map_absent() {
    let class_names = vec!["_abcdabcd".to_string(), "_efghefgh".to_string()];
    let compressed = compress_class_names_for_runtime(&class_names, None);

    assert_eq!(compressed, class_names);
  }

  #[test]
  fn compresses_when_mapping_present() {
    let class_names = vec!["_abcdabcd".to_string(), "_efghefgh".to_string()];
    let mut map = BTreeMap::new();
    map.insert("abcdabcd".to_string(), "a".to_string());
    map.insert("efghefgh".to_string(), "b".to_string());

    let compressed = compress_class_names_for_runtime(&class_names, Some(&map));

    assert_eq!(
      compressed,
      vec!["_abcd_a".to_string(), "_efgh_b".to_string()]
    );
  }

  #[test]
  fn ignores_classes_without_mapping() {
    let class_names = vec!["_abcdabcd".to_string(), "_zzzzzzzz".to_string()];
    let mut map = BTreeMap::new();
    map.insert("abcdabcd".to_string(), "a".to_string());

    let compressed = compress_class_names_for_runtime(&class_names, Some(&map));

    assert_eq!(
      compressed,
      vec!["_abcd_a".to_string(), "_zzzzzzzz".to_string()]
    );
  }
}
