use std::{collections::HashMap, path::PathBuf, sync::LazyLock};

use serde_json5::from_str;

use crate::atlaspack_rc::AtlaspackRcFile;

static BUILTIN_CONFIGS: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
  HashMap::from([(
    "@atlaspack/config-default".into(),
    include_str!("../../../packages/configs/default/index.json").into(),
  )])
});

pub fn get_builtin_config(config: &str) -> Option<AtlaspackRcFile> {
  let builtin = BUILTIN_CONFIGS.get(config)?;
  let raw_config = String::from(builtin);
  let contents =
    from_str(&raw_config).unwrap_or_else(|_| panic!("Invalid builtin config: {}", config));

  Some(AtlaspackRcFile {
    contents,
    raw: raw_config,
    path: PathBuf::from(config),
  })
}
