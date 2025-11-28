use anyhow::{Context, Result, anyhow};
use atlaspack_js_swc_core::{
  Config, emit, parse, utils::ErrorBuffer, utils::error_buffer_to_diagnostics,
};
use serde::Serialize;
use swc_atlaskit_tokens::{
  design_system_tokens_visitor, token_map::get_or_load_token_map_from_json,
};
use swc_core::{
  common::{
    FileName, SourceMap,
    errors::{self, Handler},
    source_map::SourceMapGenConfig,
    sync::Lrc,
  },
  ecma::ast::{Module, ModuleItem, Program},
};

#[derive(Clone)]
pub struct TokensPluginOptions {
  pub token_data_path: String,
  pub should_use_auto_fallback: bool,
  pub should_force_auto_fallback: bool,
  pub force_auto_fallback_exemptions: Vec<String>,
  pub default_theme: String,
}

#[derive(Clone)]
pub struct TokensConfig {
  pub filename: String,
  pub project_root: String,
  pub is_source: bool,
  pub source_maps: bool,
  pub tokens_options: TokensPluginOptions,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokensPluginResult {
  pub code: String,
  pub map: Option<String>,
}

// Exclude macro expansions from source maps.
struct SourceMapConfig;
impl SourceMapGenConfig for SourceMapConfig {
  fn file_name_to_source(&self, f: &FileName) -> String {
    f.to_string()
  }

  fn skip(&self, f: &FileName) -> bool {
    matches!(f, FileName::MacroExpansion | FileName::Internal(..))
  }
}

/// Process tokens in a single piece of code - designed to be called from somewhere that orchestrates it
pub fn process_tokens_sync(code: &str, config: &TokensConfig) -> Result<TokensPluginResult> {
  if code.trim().is_empty() {
    return Err(anyhow!("Empty code input"));
  }

  let swc_config = Config {
    is_type_script: true,
    is_jsx: true,
    decorators: false,
    ..Default::default()
  };

  let error_buffer = ErrorBuffer::default();
  let handler = Handler::with_emitter(true, false, Box::new(error_buffer.clone()));
  errors::HANDLER.set(&handler, || {
    let source_map = Lrc::new(SourceMap::default());

    // Parse and handle parsing errors
    let (module, comments) = match parse(
      code,
      &config.project_root,
      &config.filename,
      &source_map,
      &swc_config,
    ) {
      Ok(result) => result,
      Err(_parsing_errors) => {
        let diagnostics = error_buffer_to_diagnostics(&error_buffer, &source_map);
        let error_msg = diagnostics
          .iter()
          .map(|d| &d.message)
          .cloned()
          .collect::<Vec<_>>()
          .join("\n");
        return Err(anyhow!("Parse error: {}", error_msg));
      }
    };

    let module = match module {
      Program::Module(module) => Program::Module(module),
      Program::Script(script) => Program::Module(Module {
        span: script.span,
        shebang: None,
        body: script.body.into_iter().map(ModuleItem::Stmt).collect(),
      }),
    };

    let token_map = get_or_load_token_map_from_json(Some(&config.tokens_options.token_data_path))
      .with_context(|| {
      format!(
        "Failed to load token map from: {}",
        config.tokens_options.token_data_path
      )
    })?;

    let mut passes = design_system_tokens_visitor(
      comments.clone(),
      config.tokens_options.should_use_auto_fallback,
      config.tokens_options.should_force_auto_fallback,
      config.tokens_options.force_auto_fallback_exemptions.clone(),
      config.tokens_options.default_theme.clone(),
      !config.is_source,
      token_map.as_ref().map(|t| t.as_ref()),
    );
    let module = module.apply(&mut passes);
    let module_result = module
      .module()
      .ok_or_else(|| anyhow!("Failed to get transformed module"))?;
    let (code_bytes, line_pos_buffer) = emit(
      source_map.clone(),
      comments,
      &module_result,
      config.source_maps,
      Some(false), // Preserve Unicode characters in tokens
    )
    .with_context(|| "Failed to emit transformed code")?;

    let code =
      String::from_utf8(code_bytes).with_context(|| "Failed to convert emitted code to UTF-8")?;
    let map_json = if config.source_maps && !line_pos_buffer.is_empty() {
      let mut output_map_buffer = vec![];
      if source_map
        .build_source_map_with_config(&line_pos_buffer, None, SourceMapConfig)
        .to_writer(&mut output_map_buffer)
        .is_ok()
      {
        Some(String::from_utf8(output_map_buffer).unwrap_or_default())
      } else {
        None
      }
    } else {
      None
    };

    Ok(TokensPluginResult {
      code,
      map: map_json,
    })
  })
}
