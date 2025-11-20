use atlaspack_core::plugin::PluginOptions;
use atlaspack_core::types::{
  Asset, BuildMode, Dependency, DependencyBuilder, DependencyKind, Priority, SpecifierType,
};
use indexmap::IndexMap;
use swc_core::atoms::Atom;

/// Determines whether an asset should be excluded from React refresh wrapping
pub fn should_exclude_from_react_refresh(
  asset: &Asset,
  options: &PluginOptions,
  dependencies: &IndexMap<Atom, Dependency>,
) -> bool {
  !asset.is_source
    || options.hmr_options.is_none()
    || !asset.env.context.is_browser()
    || asset.env.is_library
    || asset.env.context.is_worker()
    || asset.env.context.is_worklet()
    || options.mode != BuildMode::Development
    || !has_react_dependency(dependencies)
}

/// Checks if the dependencies include React or React-related packages
/// This matches the logic from the original ReactRefreshWrapTransformer
fn has_react_dependency(dependencies: &IndexMap<Atom, Dependency>) -> bool {
  dependencies.values().any(|dep| {
    matches!(
      dep.specifier.as_str(),
      "react"
        | "react/jsx-runtime"
        | "react/jsx-dev-runtime"
        | "@emotion/react"
        | "@emotion/react/jsx-runtime"
        | "@emotion/react/jsx-dev-runtime"
    )
  })
}

type SourceMapOffset = (u32, u32);

pub fn wrap_with_react_refresh(
  asset: &Asset,
  code: &Vec<u8>,
  options: &PluginOptions,
  dependencies: &IndexMap<Atom, Dependency>,
  hmr_improvements: bool,
) -> Option<(Vec<u8>, Dependency, SourceMapOffset)> {
  if should_exclude_from_react_refresh(asset, options, dependencies) {
    return None;
  }

  let helper_filename = if hmr_improvements {
    "helpers-new.js"
  } else {
    "helpers.js"
  };

  let wrapper_path = format!(
    "@atlaspack/transformer-react-refresh-wrap/lib/helpers/{}",
    helper_filename
  );

  let name = format!(
    "$parcel$ReactRefreshHelpers${}",
    &asset.id[asset.id.len().saturating_sub(4)..]
  );
  let code_str = String::from_utf8_lossy(code);
  let wrapper_path_json = format!("\"{}\"", wrapper_path);
  let init_line = if hmr_improvements {
    format!("{}.init();\n", name)
  } else {
    String::new()
  };

  let wrapped_code = format!(
    r#"var {name} = require({wrapper_path_json});
{init_line}var prevRefreshReg = window.$RefreshReg$;
var prevRefreshSig = window.$RefreshSig$;
{name}.prelude(module);

try {{
{code_str}
  {name}.postlude(module);
}} finally {{
  window.$RefreshReg$ = prevRefreshReg;
  window.$RefreshSig$ = prevRefreshSig;
}}"#
  );

  // Create the dependency for the React refresh helpers
  let dependency = DependencyBuilder::default()
    .specifier(wrapper_path.clone())
    .specifier_type(SpecifierType::Esm)
    .kind(DependencyKind::Require)
    .priority(Priority::Sync)
    .env(asset.env.clone())
    .build();

  // Calculate the number of lines added by the wrapper
  // The original JS transformer used offsetLines(1, 6) but let's be more precise
  let offset_lines = if hmr_improvements { 7 } else { 6 };
  let start_column = 0; // Original code starts at column 0 after the "try {" line

  Some((
    wrapped_code.into_bytes(),
    dependency,
    (start_column, offset_lines),
  ))
}

#[cfg(test)]
mod tests {
  use std::path::PathBuf;
  use std::sync::Arc;

  use atlaspack_core::plugin::{HmrOptions, PluginOptions};
  use atlaspack_core::types::{
    Asset, BuildMode, Environment, EnvironmentContext, FileType, OutputFormat, SourceType,
  };
  use indexmap::IndexMap;
  use swc_core::atoms::Atom;

  use super::*;

  fn create_test_asset(id: &str, is_source: bool) -> Asset {
    Asset {
      id: id.to_string(),
      file_path: PathBuf::from(format!("{}.js", id)),
      file_type: FileType::Js,
      is_source,
      env: Arc::new(Environment {
        context: EnvironmentContext::Browser,
        source_type: SourceType::Module,
        output_format: OutputFormat::EsModule,
        is_library: false,
        ..Default::default()
      }),
      ..Default::default()
    }
  }

  fn create_test_options(mode: BuildMode, hmr_enabled: bool) -> PluginOptions {
    PluginOptions {
      mode,
      hmr_options: if hmr_enabled {
        Some(HmrOptions {
          port: Some(1234),
          host: Some("localhost".to_string()),
        })
      } else {
        None
      },
      ..Default::default()
    }
  }

  fn create_empty_dependencies() -> IndexMap<Atom, Dependency> {
    IndexMap::new()
  }

  fn create_dependencies_with_specifiers(specifiers: Vec<&str>) -> IndexMap<Atom, Dependency> {
    let mut deps = IndexMap::new();
    for specifier in specifiers {
      let dep = DependencyBuilder::default()
        .specifier(specifier.to_string())
        .specifier_type(SpecifierType::Esm)
        .kind(DependencyKind::Require)
        .priority(Priority::Sync)
        .env(Arc::new(Environment::default()))
        .build();
      deps.insert(Atom::new(specifier), dep);
    }
    deps
  }

  #[test]
  fn test_should_exclude_from_react_refresh_not_source() {
    let asset = create_test_asset("test1234abcd5678", false);
    let options = create_test_options(BuildMode::Development, true);
    let deps = create_dependencies_with_specifiers(vec!["react"]);

    assert!(should_exclude_from_react_refresh(&asset, &options, &deps));
  }

  #[test]
  fn test_should_exclude_from_react_refresh_no_hmr() {
    let asset = create_test_asset("test1234abcd5678", true);
    let options = create_test_options(BuildMode::Development, false);
    let deps = create_dependencies_with_specifiers(vec!["react"]);

    assert!(should_exclude_from_react_refresh(&asset, &options, &deps));
  }

  #[test]
  fn test_should_exclude_from_react_refresh_not_browser() {
    let mut asset = create_test_asset("test1234abcd5678", true);
    asset.env = Arc::new(Environment {
      context: EnvironmentContext::Node,
      ..(*asset.env).clone()
    });
    let options = create_test_options(BuildMode::Development, true);
    let deps = create_dependencies_with_specifiers(vec!["react"]);

    assert!(should_exclude_from_react_refresh(&asset, &options, &deps));
  }

  #[test]
  fn test_should_exclude_from_react_refresh_library() {
    let mut asset = create_test_asset("test1234abcd5678", true);
    asset.env = Arc::new(Environment {
      is_library: true,
      ..(*asset.env).clone()
    });
    let options = create_test_options(BuildMode::Development, true);
    let deps = create_dependencies_with_specifiers(vec!["react"]);

    assert!(should_exclude_from_react_refresh(&asset, &options, &deps));
  }

  #[test]
  fn test_should_exclude_from_react_refresh_web_worker() {
    let mut asset = create_test_asset("test1234abcd5678", true);
    asset.env = Arc::new(Environment {
      context: EnvironmentContext::WebWorker,
      ..(*asset.env).clone()
    });
    let options = create_test_options(BuildMode::Development, true);
    let deps = create_dependencies_with_specifiers(vec!["react"]);

    assert!(should_exclude_from_react_refresh(&asset, &options, &deps));
  }

  #[test]
  fn test_should_exclude_from_react_refresh_worklet() {
    let mut asset = create_test_asset("test1234abcd5678", true);
    asset.env = Arc::new(Environment {
      context: EnvironmentContext::Worklet,
      ..(*asset.env).clone()
    });
    let options = create_test_options(BuildMode::Development, true);
    let deps = create_dependencies_with_specifiers(vec!["react"]);

    assert!(should_exclude_from_react_refresh(&asset, &options, &deps));
  }

  #[test]
  fn test_should_exclude_from_react_refresh_production() {
    let asset = create_test_asset("test1234abcd5678", true);
    let options = create_test_options(BuildMode::Production, true);
    let deps = create_dependencies_with_specifiers(vec!["react"]);

    assert!(should_exclude_from_react_refresh(&asset, &options, &deps));
  }

  #[test]
  fn test_should_exclude_from_react_refresh_no_react_dependency() {
    let asset = create_test_asset("test1234abcd5678", true);
    let options = create_test_options(BuildMode::Development, true);
    let deps = create_dependencies_with_specifiers(vec!["lodash", "axios"]);

    assert!(should_exclude_from_react_refresh(&asset, &options, &deps));
  }

  #[test]
  fn test_should_not_exclude_from_react_refresh_valid() {
    let asset = create_test_asset("test1234abcd5678", true);
    let options = create_test_options(BuildMode::Development, true);
    let deps = create_dependencies_with_specifiers(vec!["react"]);

    assert!(!should_exclude_from_react_refresh(&asset, &options, &deps));
  }

  #[test]
  fn test_has_react_dependency_no_dependencies() {
    let deps = create_empty_dependencies();
    assert!(!has_react_dependency(&deps));
  }

  #[test]
  fn test_has_react_dependency_no_react() {
    let deps = create_dependencies_with_specifiers(vec!["lodash", "axios"]);
    assert!(!has_react_dependency(&deps));
  }

  #[test]
  fn test_has_react_dependency_react() {
    let deps = create_dependencies_with_specifiers(vec!["react"]);
    assert!(has_react_dependency(&deps));
  }

  #[test]
  fn test_has_react_dependency_react_jsx_runtime() {
    let deps = create_dependencies_with_specifiers(vec!["react/jsx-runtime"]);
    assert!(has_react_dependency(&deps));
  }

  #[test]
  fn test_has_react_dependency_emotion_react() {
    let deps = create_dependencies_with_specifiers(vec!["@emotion/react"]);
    assert!(has_react_dependency(&deps));
  }

  #[test]
  fn test_has_react_dependency_mixed() {
    let deps = create_dependencies_with_specifiers(vec!["lodash", "react", "axios"]);
    assert!(has_react_dependency(&deps));
  }

  #[test]
  fn test_wrap_with_react_refresh_excluded() {
    let asset = create_test_asset("excluded12345678", false); // not source
    let options = create_test_options(BuildMode::Development, true);
    let deps = create_dependencies_with_specifiers(vec!["react"]);
    let code = b"console.log('test');".to_vec();

    let result = wrap_with_react_refresh(&asset, &code, &options, &deps, false);

    assert!(result.is_none());
  }

  #[test]
  fn test_wrap_with_react_refresh_basic() {
    let asset = create_test_asset("abcd1234efgh5678", true);
    let options = create_test_options(BuildMode::Development, true);
    let deps = create_dependencies_with_specifiers(vec!["react"]);
    let code = b"console.log('test');".to_vec();

    let result = wrap_with_react_refresh(&asset, &code, &options, &deps, false);

    assert!(result.is_some());
    let (result_code, dependency, source_map_offset) = result.unwrap();

    let result_str = String::from_utf8(result_code).unwrap();

    let expected = r#"var $parcel$ReactRefreshHelpers$5678 = require("@atlaspack/transformer-react-refresh-wrap/lib/helpers/helpers.js");
var prevRefreshReg = window.$RefreshReg$;
var prevRefreshSig = window.$RefreshSig$;
$parcel$ReactRefreshHelpers$5678.prelude(module);

try {
console.log('test');
  $parcel$ReactRefreshHelpers$5678.postlude(module);
} finally {
  window.$RefreshReg$ = prevRefreshReg;
  window.$RefreshSig$ = prevRefreshSig;
}"#;

    assert_eq!(result_str, expected);

    // Check that dependency is created
    assert_eq!(
      dependency.specifier,
      "@atlaspack/transformer-react-refresh-wrap/lib/helpers/helpers.js"
    );
    assert_eq!(dependency.specifier_type, SpecifierType::Esm);
    assert_eq!(dependency.kind, Some(DependencyKind::Require));
    assert_eq!(dependency.priority, Priority::Sync);

    // Check source map offset
    assert_eq!(source_map_offset, (0, 6));
  }

  #[test]
  fn test_wrap_with_react_refresh_hmr_improvements() {
    let asset = create_test_asset("xyz789abc123def4", true);
    let options = create_test_options(BuildMode::Development, true);
    let deps = create_dependencies_with_specifiers(vec!["react"]);
    let code = b"const App = () => <div />;".to_vec();

    let result = wrap_with_react_refresh(
      &asset, &code, &options, &deps, true, // hmr_improvements = true
    );

    assert!(result.is_some());
    let (result_code, dependency, source_map_offset) = result.unwrap();

    let result_str = String::from_utf8(result_code).unwrap();

    let expected = r#"var $parcel$ReactRefreshHelpers$def4 = require("@atlaspack/transformer-react-refresh-wrap/lib/helpers/helpers-new.js");
$parcel$ReactRefreshHelpers$def4.init();
var prevRefreshReg = window.$RefreshReg$;
var prevRefreshSig = window.$RefreshSig$;
$parcel$ReactRefreshHelpers$def4.prelude(module);

try {
const App = () => <div />;
  $parcel$ReactRefreshHelpers$def4.postlude(module);
} finally {
  window.$RefreshReg$ = prevRefreshReg;
  window.$RefreshSig$ = prevRefreshSig;
}"#;

    assert_eq!(result_str, expected);

    // Check that dependency points to helpers-new.js
    assert_eq!(
      dependency.specifier,
      "@atlaspack/transformer-react-refresh-wrap/lib/helpers/helpers-new.js"
    );

    // Check source map offset (should be 7 for HMR improvements)
    assert_eq!(source_map_offset, (0, 7));
  }

  #[test]
  fn test_wrap_with_react_refresh_special_characters_in_code() {
    let asset = create_test_asset("special123chars7", true);
    let options = create_test_options(BuildMode::Development, true);
    let deps = create_dependencies_with_specifiers(vec!["react"]);
    let code = b"const str = \"Hello \\\"world\\\"\"; // Comment with 'quotes'".to_vec();

    let result = wrap_with_react_refresh(&asset, &code, &options, &deps, false);

    assert!(result.is_some());
    let (result_code, _, source_map_offset) = result.unwrap();

    let result_str = String::from_utf8(result_code).unwrap();

    let expected = r#"var $parcel$ReactRefreshHelpers$ars7 = require("@atlaspack/transformer-react-refresh-wrap/lib/helpers/helpers.js");
var prevRefreshReg = window.$RefreshReg$;
var prevRefreshSig = window.$RefreshSig$;
$parcel$ReactRefreshHelpers$ars7.prelude(module);

try {
const str = "Hello \"world\""; // Comment with 'quotes'
  $parcel$ReactRefreshHelpers$ars7.postlude(module);
} finally {
  window.$RefreshReg$ = prevRefreshReg;
  window.$RefreshSig$ = prevRefreshSig;
}"#;

    assert_eq!(result_str, expected);

    // Check source map offset
    assert_eq!(source_map_offset, (0, 6));
  }
}
