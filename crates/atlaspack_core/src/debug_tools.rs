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
    let mut tools = Self::default();

    if let Ok(env_value) = std::env::var("ATLASPACK_DEBUG_TOOLS") {
      for tool in env_value.split(',') {
        let tool = tool.trim();

        match tool {
          "all" => {
            tools.asset_file_names_in_output = true;
            tools.simple_cli_reporter = true;
            tools.bundle_stats = true;
            tools.scope_hoisting_stats = true;
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
  fn test_from_env_empty() {
    unsafe {
      std::env::remove_var("ATLASPACK_DEBUG_TOOLS");
    }
    let tools = DebugTools::from_env();
    assert!(!tools.asset_file_names_in_output);
  }

  #[test]
  fn test_from_env_single_tool() {
    unsafe {
      std::env::set_var("ATLASPACK_DEBUG_TOOLS", "asset-file-names-in-output");
    }
    let tools = DebugTools::from_env();
    assert!(tools.asset_file_names_in_output);
    assert!(!tools.simple_cli_reporter);
    unsafe {
      std::env::remove_var("ATLASPACK_DEBUG_TOOLS");
    }
  }

  #[test]
  fn test_from_env_multiple_tools() {
    unsafe {
      std::env::set_var(
        "ATLASPACK_DEBUG_TOOLS",
        "asset-file-names-in-output,bundle-stats",
      );
    }
    let tools = DebugTools::from_env();
    assert!(tools.asset_file_names_in_output);
    assert!(tools.bundle_stats);
    assert!(!tools.simple_cli_reporter);
    unsafe {
      std::env::remove_var("ATLASPACK_DEBUG_TOOLS");
    }
  }

  #[test]
  fn test_from_env_all() {
    unsafe {
      std::env::set_var("ATLASPACK_DEBUG_TOOLS", "all");
    }
    let tools = DebugTools::from_env();
    assert!(tools.asset_file_names_in_output);
    assert!(tools.simple_cli_reporter);
    assert!(tools.bundle_stats);
    assert!(tools.scope_hoisting_stats);
    unsafe {
      std::env::remove_var("ATLASPACK_DEBUG_TOOLS");
    }
  }

  #[test]
  fn test_from_env_with_spaces() {
    unsafe {
      std::env::set_var(
        "ATLASPACK_DEBUG_TOOLS",
        " asset-file-names-in-output , bundle-stats ",
      );
    }
    let tools = DebugTools::from_env();
    assert!(tools.asset_file_names_in_output);
    assert!(tools.bundle_stats);
    unsafe {
      std::env::remove_var("ATLASPACK_DEBUG_TOOLS");
    }
  }
}
