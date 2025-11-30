use crate::types::Metadata;

/// Return the runtime helper identifier used for joining class names.
///
/// Mirrors the Babel implementation by switching to the compressed helper when
/// a class name compression map has been provided through the plugin options.
pub fn get_runtime_class_name_library(meta: &Metadata) -> &'static str {
  let state = meta.state();

  if state
    .opts
    .class_name_compression_map
    .as_ref()
    .map(|map| !map.is_empty())
    .unwrap_or(false)
  {
    "ac"
  } else {
    "ax"
  }
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;
  use std::collections::BTreeMap;
  use std::rc::Rc;

  use swc_core::common::sync::Lrc;
  use swc_core::common::SourceMap;

  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};

  use super::get_runtime_class_name_library;

  fn create_metadata(options: PluginOptions) -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm, Vec::new());
    let state = Rc::new(RefCell::new(TransformState::new(file, options)));
    Metadata::new(state)
  }

  #[test]
  fn defaults_to_ax_helper() {
    let meta = create_metadata(PluginOptions::default());
    assert_eq!(get_runtime_class_name_library(&meta), "ax");
  }

  #[test]
  fn uses_ac_when_compression_map_present() {
    let mut options = PluginOptions::default();
    let mut map = BTreeMap::new();
    map.insert("a".into(), "b".into());
    options.class_name_compression_map = Some(map);

    let meta = create_metadata(options);
    assert_eq!(get_runtime_class_name_library(&meta), "ac");
  }
}
