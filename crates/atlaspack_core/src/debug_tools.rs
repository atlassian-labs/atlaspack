/// Debug tools for Atlaspack developers
///
/// These tools provide extra utilities to make debugging Atlaspack issues more straightforward.
///
/// To enable a tool, set the `ATLASPACK_DEBUG_TOOLS` environment variable to a
/// comma-separated list of tool names. For example:
/// `ATLASPACK_DEBUG_TOOLS="asset-file-names-in-output,simple-cli-reporter"`
///
/// You can enable all tools by setting `ATLASPACK_DEBUG_TOOLS=all`.

#[derive(Debug, Clone, Default)]
pub struct DebugTools {
  pub asset_file_names_in_output: bool,
  pub simple_cli_reporter: bool,
  pub bundle_stats: bool,
  pub scope_hoisting_stats: bool,
  pub debug_prelude: bool,
}

impl DebugTools {
  pub fn from_env() -> Self {
    match std::env::var("ATLASPACK_DEBUG_TOOLS") {
      Ok(env_value) => Self::from_value(&env_value),
      Err(_) => Self::default(),
    }
  }

  pub fn from_value(env_value: &str) -> Self {
    let mut tools = Self::default();

    for tool in env_value.split(',') {
      let tool = tool.trim();

      match tool {
        "all" => {
          tools.asset_file_names_in_output = true;
          tools.simple_cli_reporter = true;
          tools.bundle_stats = true;
          tools.scope_hoisting_stats = true;
          tools.debug_prelude = true;
          break;
        }
        "asset-file-names-in-output" => tools.asset_file_names_in_output = true,
        "simple-cli-reporter" => tools.simple_cli_reporter = true,
        "bundle-stats" => tools.bundle_stats = true,
        "scope-hoisting-stats" => tools.scope_hoisting_stats = true,
        "debug-prelude" => tools.debug_prelude = true,
        "" => continue,
        _ => {
          eprintln!(
            "Warning: Unknown debug tool option: '{}'. Valid options are: asset-file-names-in-output, simple-cli-reporter, bundle-stats, scope-hoisting-stats, debug-prelude, all",
            tool
          );
        }
      }
    }

    tools
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_default_debug_tools() {
    let tools = DebugTools::default();
    assert!(!tools.asset_file_names_in_output);
    assert!(!tools.simple_cli_reporter);
    assert!(!tools.bundle_stats);
    assert!(!tools.scope_hoisting_stats);
  }

  #[test]
  fn test_from_value_empty() {
    let tools = DebugTools::from_value("");
    assert!(!tools.asset_file_names_in_output);
    assert!(!tools.simple_cli_reporter);
    assert!(!tools.bundle_stats);
    assert!(!tools.scope_hoisting_stats);
    assert!(!tools.debug_prelude);
  }

  #[test]
  fn test_from_value_single_tool() {
    let tools = DebugTools::from_value("asset-file-names-in-output");
    assert!(tools.asset_file_names_in_output);
    assert!(!tools.simple_cli_reporter);
  }

  #[test]
  fn test_from_value_multiple_tools() {
    let tools = DebugTools::from_value("asset-file-names-in-output,bundle-stats");
    assert!(tools.asset_file_names_in_output);
    assert!(tools.bundle_stats);
    assert!(!tools.simple_cli_reporter);
  }

  #[test]
  fn test_from_value_all() {
    let tools = DebugTools::from_value("all");
    assert!(tools.asset_file_names_in_output);
    assert!(tools.simple_cli_reporter);
    assert!(tools.bundle_stats);
    assert!(tools.scope_hoisting_stats);
    assert!(tools.debug_prelude);
  }

  #[test]
  fn test_from_value_with_spaces() {
    let tools = DebugTools::from_value(" asset-file-names-in-output , bundle-stats ");
    assert!(tools.asset_file_names_in_output);
    assert!(tools.bundle_stats);
  }
}
