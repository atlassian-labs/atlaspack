use std::path::{Path, PathBuf};
use std::sync::Arc;

use atlaspack_core::diagnostic;
use atlaspack_sourcemap::SourceMapError;
use indexmap::IndexMap;
use std::collections::{BTreeMap, HashMap};
use swc_core::atoms::Atom;

use atlaspack_core::plugin::{PluginOptions, TransformResult};
use atlaspack_core::types::engines::EnvironmentFeature;
use atlaspack_core::types::{
  Asset, BundleBehavior, Code, CodeFrame, CodeHighlight, Dependency, DependencyBuilder,
  DependencyKind, Diagnostic, DiagnosticBuilder, Environment, EnvironmentContext, File, FileType,
  IncludeNodeModules, OutputFormat, Priority, SourceLocation, SourceMap, SourceType, SpecifierType,
  Symbol,
};

use crate::js_transformer::conversion::dependency_kind::{
  convert_priority, convert_specifier_type,
};
use crate::js_transformer::conversion::loc::convert_loc;
use crate::js_transformer::conversion::symbol::{
  transformer_collect_imported_symbol_to_symbol, transformer_exported_symbol_into_symbol,
  transformer_imported_symbol_to_symbol,
};
use crate::react_refresh;

mod dependency_kind;
mod loc;
/// Conversions from SWC symbol types into [`Symbol`]
mod symbol;

pub(crate) fn convert_result(
  mut asset: Asset,
  transformer_config: &atlaspack_js_swc_core::Config,
  result: atlaspack_js_swc_core::TransformResult,
  options: &PluginOptions,
) -> Result<TransformResult, Vec<Diagnostic>> {
  let asset_file_path = asset.file_path.to_path_buf();
  let asset_environment = asset.env.clone();

  asset.interpreter = result.shebang;

  let (mut dependency_by_specifier, invalidate_on_file_change) = convert_dependencies(
    &options.project_root,
    transformer_config,
    result.dependencies,
    result.magic_comments,
    &asset,
  )?;

  if result.needs_esm_helpers {
    let dependency = make_esm_helpers_dependency(
      options,
      &asset_file_path,
      (*asset_environment).clone(),
      &asset.id,
    );
    dependency_by_specifier.insert(dependency.specifier.as_str().into(), dependency);
  }

  let mut asset_symbols = Vec::new();

  if let Some(hoist_result) = result.hoist_result {
    // Pre-allocate expected symbols
    asset_symbols.reserve(hoist_result.exported_symbols.len() + hoist_result.re_exports.len() + 1);

    // Dependency symbols are always present during scope hoisting, this is necessary for the
    // packaging phase and should be revisited later.
    for dependency in dependency_by_specifier.values_mut() {
      dependency.symbols = Some(Vec::new());
    }

    // Collect all exported variable names
    for symbol in &hoist_result.exported_symbols {
      let symbol =
        transformer_exported_symbol_into_symbol(&options.project_root, &asset_file_path, symbol);
      asset_symbols.push(symbol);
    }

    // Collect all imported symbols into each of the corresponding dependencies' symbols array
    for symbol in hoist_result.imported_symbols {
      if let Some(dependency) = dependency_by_specifier.get_mut(&symbol.source) {
        let symbol =
          transformer_imported_symbol_to_symbol(&options.project_root, &asset_file_path, &symbol);
        if let Some(symbols) = dependency.symbols.as_mut() {
          symbols.push(symbol);
        } else {
          dependency.symbols = Some(vec![symbol]);
        }
      }
    }

    for symbol in hoist_result.re_exports {
      if let Some(dependency) = dependency_by_specifier.get_mut(&symbol.source) {
        if is_re_export_all_symbol(&symbol) {
          let loc = Some(convert_loc(
            &options.project_root,
            asset_file_path.clone(),
            &symbol.loc,
          ));
          let symbol = make_export_all_symbol(loc);

          if let Some(symbols) = dependency.symbols.as_mut() {
            symbols.push(symbol);
          } else {
            dependency.symbols = Some(vec![symbol]);
          }
          // TODO: Why isn't this added to the asset.symbols array?
        } else {
          #[allow(clippy::op_ref)]
          let existing = if let Some(symbols) = dependency.symbols.as_ref() {
            symbols
              .as_slice()
              .iter()
              .find(|candidate| candidate.exported == &*symbol.imported)
          } else {
            None
          };

          // `re_export_fake_local_key` is a generated mangled identifier only for purposes of
          // keying this `Symbol`. It is not actually inserted onto the file.
          //
          // Unlike other symbols, we're generating the mangled name in here rather than in the
          // SWC transformer implementation.
          // TODO: Move this into the SWC transformer
          let re_export_fake_local_key = existing
            .map(|sym| sym.local.clone())
            .unwrap_or_else(|| format!("${}$re_export${}", asset.id, symbol.local));

          let dependency_symbol = Symbol {
            exported: symbol.imported.as_ref().into(),
            local: re_export_fake_local_key.clone(),
            loc: Some(convert_loc(
              &options.project_root,
              asset_file_path.clone(),
              &symbol.loc,
            )),
            is_weak: existing.map(|e| e.is_weak).unwrap_or(true),
            ..Symbol::default()
          };

          if let Some(symbols) = dependency.symbols.as_mut() {
            symbols.push(dependency_symbol);
          } else {
            dependency.symbols = Some(vec![dependency_symbol]);
          }

          asset_symbols.push(Symbol {
            exported: symbol.local.as_ref().into(),
            local: re_export_fake_local_key.clone(),
            loc: Some(convert_loc(
              &options.project_root,
              asset_file_path.clone(),
              &symbol.loc,
            )),
            is_weak: false,
            ..Symbol::default()
          });
        }
      }
    }

    for specifier in hoist_result.wrapped_requires {
      if let Some(dependency) = dependency_by_specifier.get_mut(&Atom::new(specifier)) {
        dependency.should_wrap = true;
      }
    }

    for (name, specifier) in hoist_result.dynamic_imports {
      if let Some(dependency) = dependency_by_specifier.get_mut(&specifier) {
        dependency.promise_symbol = Some(name.to_string());
      }
    }

    for name in hoist_result.self_references {
      // Do not create a self-reference for the `default` symbol unless we have seen an __esModule flag.
      if &*name == "default"
        && !asset_symbols
          .as_slice()
          .iter()
          .any(|s| &*s.exported == "__esModule")
      {
        continue;
      }

      let symbol = asset_symbols
        .iter_mut()
        .find(|s| s.exported.as_str() == name.as_str())
        .unwrap();

      symbol.self_referenced = true;
    }

    // Add * symbol if there are CJS exports, no imports/exports at all
    // (and the asset has side effects), or the asset is wrapped.
    // This allows accessing symbols that don't exist without errors in symbol propagation.
    if hoist_result.has_cjs_exports
      || (!hoist_result.is_esm
        && asset.side_effects
        && dependency_by_specifier.is_empty()
        && hoist_result.exported_symbols.is_empty())
      || (hoist_result.should_wrap && !asset_symbols.as_slice().iter().any(|s| s.exported == "*"))
    {
      if result.is_empty_or_empty_export {
        asset.empty_file_star_reexport = Some(true);
      }
      asset_symbols.push(make_export_star_symbol(&asset.id));
    }

    asset.has_cjs_exports = hoist_result.has_cjs_exports;
    asset.static_exports = hoist_result.static_cjs_exports;
    asset.should_wrap = hoist_result.should_wrap;
  } else {
    if let Some(symbol_result) = result.symbol_result {
      asset_symbols.reserve(symbol_result.exports.len() + 1);
      for sym in &symbol_result.exports {
        let (local, is_weak) = if let Some(dependency) = sym
          .source
          .as_ref()
          .and_then(|source| dependency_by_specifier.get_mut(source))
        {
          let local = format!("${}${}", dependency.id(), sym.local);
          let symbol = Symbol {
            exported: sym.local.as_ref().into(),
            local: local.clone(),
            loc: Some(convert_loc(
              &options.project_root,
              asset_file_path.clone(),
              &sym.loc,
            )),
            is_weak: true,
            ..Symbol::default()
          };

          if let Some(symbols) = dependency.symbols.as_mut() {
            symbols.push(symbol);
          } else {
            dependency.symbols = Some(vec![symbol]);
          }

          (local, true)
        } else {
          (format!("${}", sym.local), false)
        };

        asset_symbols.push(Symbol {
          exported: sym.exported.as_ref().into(),
          local,
          loc: Some(convert_loc(
            &options.project_root,
            asset_file_path.clone(),
            &sym.loc,
          )),
          is_weak,
          ..Symbol::default()
        });
      }

      for sym in symbol_result.imports {
        if let Some(dependency) = dependency_by_specifier.get_mut(&sym.source) {
          let symbol = transformer_collect_imported_symbol_to_symbol(
            &options.project_root,
            &asset_file_path,
            &sym,
          );
          if let Some(symbols) = dependency.symbols.as_mut() {
            symbols.push(symbol);
          } else {
            dependency.symbols = Some(vec![symbol]);
          }
        }
      }

      for sym in symbol_result.exports_all {
        if let Some(dependency) = dependency_by_specifier.get_mut(&sym.source) {
          let loc = Some(convert_loc(
            &options.project_root,
            asset_file_path.clone(),
            &sym.loc,
          ));
          let symbol = make_export_all_symbol(loc);
          if let Some(symbols) = dependency.symbols.as_mut() {
            symbols.push(symbol);
          } else {
            dependency.symbols = Some(vec![symbol]);
          }
        }
      }

      // Add * symbol if there are CJS exports, no imports/exports at all, or the asset is wrapped.
      // This allows accessing symbols that don't exist without errors in symbol propagation.
      if symbol_result.has_cjs_exports
        || (!symbol_result.is_esm
          && asset.side_effects
          && dependency_by_specifier.is_empty()
          && symbol_result.exports.is_empty())
        || (symbol_result.should_wrap
          && !asset_symbols.as_slice().iter().any(|s| s.exported == "*"))
      {
        asset_symbols.push(make_export_star_symbol(&asset.id));
      }
    } else {
      // If the asset is wrapped, add * as a fallback
      asset_symbols.push(make_export_star_symbol(&asset.id));
    }

    // For all other imports and requires, mark everything as imported (this covers both dynamic
    // imports and non-top-level requires)
    for dependency in dependency_by_specifier.values_mut() {
      let symbol = Symbol {
        exported: "*".into(),
        local: format!("{}$", dependency.id()), // TODO: coalesce with dep.placeholder
        loc: None,
        ..Default::default()
      };

      if let Some(symbols) = dependency.symbols.as_mut() {
        if symbols.is_empty() {
          symbols.push(symbol);
        }
      } else {
        dependency.symbols = Some(vec![symbol]);
      }
    }
  }

  if let Some(symbols) = asset.symbols.as_mut() {
    symbols.extend(asset_symbols);
  } else {
    asset.symbols = Some(asset_symbols);
  }

  asset.has_node_replacements = result.has_node_replacements;
  asset.is_constant_module = result.is_constant_module;

  if transformer_config.conditional_bundling {
    let mut converted = vec![];

    for condition in result.conditions.iter() {
      converted.push(serde_json::json!({
        "key": condition.key.to_string(),
        "ifTruePlaceholder": condition.if_true_placeholder,
        "ifFalsePlaceholder": condition.if_false_placeholder,
      }));
    }

    asset.conditions = result.conditions;
  }

  asset.file_type = FileType::Js;

  let mut final_code = result.code;

  // Apply React refresh wrapping if needed
  let react_refresh_map_offset = if transformer_config.react_refresh {
    if let Some((wrapped_code, refresh_dependency, source_map_offset)) =
      react_refresh::wrap_with_react_refresh(
        &asset,
        &final_code,
        options,
        &dependency_by_specifier,
        transformer_config.hmr_improvements,
      )
    {
      final_code = wrapped_code;

      dependency_by_specifier.insert(
        refresh_dependency.specifier.as_str().into(),
        refresh_dependency,
      );

      Some(source_map_offset)
    } else {
      None
    }
  } else {
    None
  };

  asset.code = Code::new(final_code);
  // Updating the file_type to JS will cause the asset id to be updated.
  // However, the packager needs to be aware of the original id when creating
  // symbols replacements in scope hoisting. That's why we store the id before
  // it get's updated on the meta object.
  asset.packaging_id = Some(asset.id.clone());

  if let Some(map) = result.map {
    let source_map_error_to_diagnostic = |error: SourceMapError| {
      vec![
        DiagnosticBuilder::default()
          .origin(Some("@atlaspack/transformer-js".into()))
          .message(format!(
            "Error building SourceMap for {}: {}",
            asset.file_path.display(),
            error
          ))
          .build()
          .unwrap(),
      ]
    };

    let mut source_map =
      SourceMap::from_json(&options.project_root, &map).map_err(source_map_error_to_diagnostic)?;

    if let Some(original_map) = asset.map {
      source_map
        .extends(&mut original_map.clone())
        .map_err(source_map_error_to_diagnostic)?
    }

    // Adjust source map if React refresh wrapping was applied
    if let Some((start_column, offset_lines)) = react_refresh_map_offset {
      source_map
        .offset_lines(start_column, offset_lines)
        .map_err(source_map_error_to_diagnostic)?
    }

    asset.map = Some(source_map);
  }

  Ok(TransformResult {
    asset,
    dependencies: dependency_by_specifier.into_values().collect(),
    // diagnostics: result.diagnostics,
    // used_env: result.used_env.into_iter().map(|v| v.to_string()).collect(),
    invalidate_on_file_change,
    ..Default::default()
  })
}

/// Returns true if this `ImportedSymbol` corresponds to a statement such as:
///
/// ```skip
/// export * from 'other';
/// ```
///
/// See [`HoistResult::re_exports`]
pub(crate) fn is_re_export_all_symbol(symbol: &atlaspack_js_swc_core::ImportedSymbol) -> bool {
  symbol.local == "*" && symbol.imported == "*"
}

/// Convert the SWC transformer dependency descriptors into the core `Dependency` type.
///
/// Collect the dependencies by their local scope-hoisting names that the transformer has output
/// onto the file. This returns a map of mangled JS name (that the transformer generated) to the
/// dependency value.
///
/// This will be used to find dependencies corresponding to imported symbols' `local` mangled names.
#[allow(clippy::type_complexity)]
pub(crate) fn convert_dependencies(
  project_root: &Path,
  transformer_config: &atlaspack_js_swc_core::Config,
  dependencies: Vec<atlaspack_js_swc_core::DependencyDescriptor>,
  magic_comments: HashMap<String, String>,
  asset: &Asset,
) -> Result<(IndexMap<Atom, Dependency>, Vec<PathBuf>), Vec<Diagnostic>> {
  let mut dependency_by_specifier = IndexMap::new();
  let mut invalidate_on_file_change = Vec::new();
  for transformer_dependency in dependencies {
    let placeholder = transformer_dependency
      .placeholder
      .as_ref()
      .map(|d| d.as_str().into())
      .unwrap_or_else(|| transformer_dependency.specifier.clone());

    let result = convert_dependency(
      project_root,
      transformer_config,
      asset,
      transformer_dependency,
    )?;

    match result {
      DependencyConversionResult::Dependency(mut dependency) => {
        if let Some(chunk_name) = magic_comments.get(&dependency.specifier) {
          dependency.chunk_name_magic_comment = Some(chunk_name.clone());
        }
        dependency_by_specifier.insert(placeholder, dependency);
      }
      DependencyConversionResult::InvalidateOnFileChange(file_path) => {
        invalidate_on_file_change.push(file_path);
      }
    }
  }
  Ok((dependency_by_specifier, invalidate_on_file_change))
}

/// "Export star" symbol is added as a placeholder for assets that may have symbols that aren't
/// explicitly listed. This is used to avoid errors if a symbol that hasn't been statically
/// analyzed is accessed.
fn make_export_star_symbol(asset_id: &str) -> Symbol {
  Symbol {
    exported: "*".into(),
    local: format!("${asset_id}$exports"),
    loc: None,
    ..Default::default()
  }
}

fn make_esm_helpers_dependency(
  options: &PluginOptions,
  #[allow(clippy::ptr_arg)] asset_file_path: &PathBuf,
  asset_environment: Environment,
  asset_id: &str,
) -> Dependency {
  DependencyBuilder::default()
    .source_asset_id(asset_id.to_string())
    .specifier("@atlaspack/transformer-js/src/esmodule-helpers.js".to_string())
    .specifier_type(SpecifierType::Esm)
    .source_path(asset_file_path.clone())
    .env(Arc::new(Environment {
      include_node_modules: IncludeNodeModules::Map(BTreeMap::from([(
        "@atlaspack/transformer-js".into(),
        true,
      )])),
      ..asset_environment.clone()
    }))
    .resolve_from(options.core_path.as_path().into())
    .priority(Priority::default())
    .build()
}

/// This will replace the hoist result symbols that `is_re_export_all` returns true for as well
/// as the `symbol_result.exports_all` symbols.
///
/// These correspond to `export * from './dep';` statements.
fn make_export_all_symbol(loc: Option<SourceLocation>) -> Symbol {
  Symbol {
    exported: "*".into(),
    local: "*".into(),
    loc,
    is_weak: true,
    ..Default::default()
  }
}

#[allow(clippy::large_enum_variant)]
enum DependencyConversionResult {
  Dependency(Dependency),
  /// Only for [`atlaspack_js_swc_core::DependencyKind::File`] dependencies, the output will not be a
  /// [`Dependency`] but just an invalidation.
  InvalidateOnFileChange(PathBuf),
}

/// Convert dependency from the transformer `atlaspack_js_swc_core::DependencyDescriptor` into a
/// `DependencyConversionResult`.
fn convert_dependency(
  project_root: &Path,
  transformer_config: &atlaspack_js_swc_core::Config,
  asset: &Asset,
  transformer_dependency: atlaspack_js_swc_core::DependencyDescriptor,
) -> Result<DependencyConversionResult, Vec<Diagnostic>> {
  let loc = convert_loc(
    project_root,
    asset.file_path.clone(),
    &transformer_dependency.loc,
  );
  let dependency = DependencyBuilder::default()
    .bundle_behavior(match transformer_dependency.kind {
      DependencyKind::Url => Some(BundleBehavior::Isolated),
      _ => None,
    })
    .env(asset.env.clone())
    .loc(loc.clone())
    .priority(convert_priority(&transformer_dependency))
    .source_asset_id(asset.id.to_string())
    .source_asset_type(asset.file_type.clone())
    .source_path(asset.file_path.clone())
    .specifier(transformer_dependency.specifier.as_ref().to_string())
    .specifier_type(convert_specifier_type(&transformer_dependency))
    .placeholder_option(transformer_dependency.placeholder.clone());

  let source_type = convert_source_type(&transformer_dependency.source_type);
  match transformer_dependency.kind {
    // For all of web-worker, service-worker, worklet and URL we should probably set BundleBehaviour
    // to "isolated". At the moment though it is set to None on all but worklet.
    //
    // `output_format` here corresponds to `{ type: '...' }` on the `new Worker` or
    // `serviceWorker.register` calls
    //
    // ```skip
    // let worker = new Worker(
    //  new URL("./dependency", import.meta.url),
    //  {type: 'module'} // <- output format
    // );
    // ```
    DependencyKind::WebWorker => {
      // Use native ES module output if the worker was created with `type: 'module'` and all targets
      // support native module workers. Only do this if parent asset output format is also esmodule so that
      // assets can be shared between workers and the main thread in the global output format.
      let mut output_format = asset.env.output_format;
      if output_format == OutputFormat::EsModule
        && matches!(
          transformer_dependency.source_type,
          Some(atlaspack_js_swc_core::SourceType::Module)
        )
        && transformer_config.supports_module_workers
      {
        output_format = OutputFormat::EsModule;
      } else if output_format != OutputFormat::CommonJS {
        output_format = OutputFormat::Global;
      }

      let dependency = dependency
        .env(Arc::new(Environment {
          context: EnvironmentContext::WebWorker,
          engines: asset.env.engines.clone(),
          include_node_modules: asset.env.include_node_modules.clone(),
          loc: asset.env.loc.clone(),
          output_format,
          source_map: asset.env.source_map.clone(),
          source_type,
          custom_env: asset.env.custom_env.clone(),
          ..*asset.env.clone()
        }))
        .is_webworker(true)
        .kind(DependencyKind::WebWorker)
        .build();

      Ok(DependencyConversionResult::Dependency(dependency))
    }
    DependencyKind::ServiceWorker => {
      let dependency = dependency
        .env(Arc::new(Environment {
          context: EnvironmentContext::ServiceWorker,
          engines: asset.env.engines.clone(),
          include_node_modules: asset.env.include_node_modules.clone(),
          loc: asset.env.loc.clone(),
          output_format: OutputFormat::Global,
          source_map: asset.env.source_map.clone(),
          source_type,
          custom_env: asset.env.custom_env.clone(),
          ..*asset.env.clone()
        }))
        .needs_stable_name(true)
        .kind(DependencyKind::ServiceWorker)
        .build();

      Ok(DependencyConversionResult::Dependency(dependency))
    }
    DependencyKind::Worklet => {
      let dependency = dependency
        .env(Arc::new(Environment {
          context: EnvironmentContext::Worklet,
          engines: asset.env.engines.clone(),
          include_node_modules: asset.env.include_node_modules.clone(),
          loc: asset.env.loc.clone(),
          output_format: OutputFormat::EsModule,
          source_map: asset.env.source_map.clone(),
          source_type: SourceType::Module,
          custom_env: asset.env.custom_env.clone(),
          ..*asset.env.clone()
        }))
        .kind(DependencyKind::Worklet)
        .build();

      Ok(DependencyConversionResult::Dependency(dependency))
    }
    DependencyKind::Url => {
      let dependency = dependency
        .bundle_behavior(Some(BundleBehavior::Isolated))
        .kind(DependencyKind::Url)
        .build();

      Ok(DependencyConversionResult::Dependency(dependency))
    }
    // File dependencies need no handling and should just register an invalidation request.
    //
    // This is a bit non-uniform, and we might want to just consolidate dependencies as also being
    // non-module file dependencies.
    DependencyKind::File => Ok(DependencyConversionResult::InvalidateOnFileChange(
      PathBuf::from(transformer_dependency.specifier.to_string()),
    )),
    _ => {
      let mut env = asset.env.clone();
      let dependency = dependency.kind(transformer_dependency.kind.clone());

      let mut import_attributes = BTreeMap::new();

      if let Some(attributes) = transformer_dependency.attributes {
        for attr in ["preload", "prefetch"] {
          let attr_atom = Into::<Atom>::into(attr);
          if attributes.contains_key(&attr_atom) {
            let attr_key = Into::<String>::into(attr);
            import_attributes.insert(attr_key.to_string(), true);
          }
        }
      }

      let dependency = dependency.import_attributes(import_attributes);

      if transformer_dependency.kind == DependencyKind::DynamicImport {
        // https://html.spec.whatwg.org/multipage/webappapis.html#hostimportmoduledynamically(referencingscriptormodule,-modulerequest,-promisecapability)
        if matches!(
          env.context,
          EnvironmentContext::Worklet | EnvironmentContext::ServiceWorker
        ) {
          let diagnostic = diagnostic!(
            DiagnosticBuilder::default()
              .code_frames(vec![CodeFrame {
                code_highlights: vec![CodeHighlight::from(loc)],
                ..CodeFrame::from(File {
                  contents: asset.code.to_string(),
                  path: asset.file_path.clone()
                })
              }])
              .hints(vec![String::from("Try using a static `import`")])
              .message(format!(
                "import() is not allowed in {}.",
                match env.context {
                  EnvironmentContext::Worklet => "worklets",
                  EnvironmentContext::ServiceWorker => "service workers",
                  _ => unreachable!(),
                }
              ))
          );

          // environment_diagnostic(&mut diagnostic, &asset, false);
          return Err(vec![diagnostic]);
        }

        // If all the target engines support dynamic import natively,
        // we can output native ESM if scope hoisting is enabled.
        // Only do this for scripts, rather than modules in the global
        // output format so that assets can be shared between the bundles.
        let mut output_format = env.output_format;
        if env.source_type == SourceType::Script
          && asset.env.should_scope_hoist
          && env.engines.supports(EnvironmentFeature::DynamicImport)
        {
          output_format = OutputFormat::EsModule;
        }

        if env.source_type != SourceType::Module || env.output_format != output_format {
          env = Arc::new(Environment {
            engines: env.engines.clone(),
            include_node_modules: env.include_node_modules.clone(),
            loc: env.loc.clone(),
            output_format,
            source_map: env.source_map.clone(),
            source_type: SourceType::Module,
            custom_env: env.custom_env.clone(),
            ..*env
          });
        }
      }

      let dependency = dependency
        .env(env)
        .is_optional(transformer_dependency.is_optional)
        .is_esm(matches!(
          transformer_dependency.kind,
          DependencyKind::Import | DependencyKind::Export
        ))
        .build();

      Ok(DependencyConversionResult::Dependency(dependency))
    }
  }
}

fn convert_source_type(source_type: &Option<atlaspack_js_swc_core::SourceType>) -> SourceType {
  if matches!(source_type, Some(atlaspack_js_swc_core::SourceType::Module)) {
    SourceType::Module
  } else {
    SourceType::Script
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_js_swc_core::test_utils::run_swc_core_transform;

  #[test]
  fn test_is_re_export_all_symbol() {
    let source = r#"
export * from 'other';
    "#;
    let swc_output = run_swc_core_transform(source);
    let export = &swc_output.hoist_result.unwrap().re_exports[0];
    assert!(is_re_export_all_symbol(export));
  }
}
