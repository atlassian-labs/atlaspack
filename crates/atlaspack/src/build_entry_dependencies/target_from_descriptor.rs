use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::config_loader::ConfigFile;
use atlaspack_core::diagnostic_error;
use atlaspack_core::types::browsers::Browsers;
use atlaspack_core::types::engines::Engines;
use atlaspack_core::types::engines::EnginesBrowsers;
use atlaspack_core::types::AtlaspackOptions;
use atlaspack_core::types::CodeFrame;
use atlaspack_core::types::DiagnosticBuilder;
use atlaspack_core::types::Entry;
use atlaspack_core::types::Environment;
use atlaspack_core::types::EnvironmentContext;
use atlaspack_core::types::OutputFormat;
use atlaspack_core::types::SourceField;
use atlaspack_core::types::SourceMapField;
use atlaspack_core::types::Target;
use atlaspack_core::types::TargetDescriptor;
use atlaspack_core::types::TargetSourceMapOptions;
use atlaspack_resolver::IncludeNodeModules;

use super::package_json::ModuleFormat;
use super::package_json::PackageJson;
use crate::build_entry_dependencies::default_dist_dir::default_dist_dir;

pub fn target_from_descriptor(
  entry: &Entry,
  options: &AtlaspackOptions,
  dist: Option<PathBuf>,
  package_json: &ConfigFile<PackageJson>,
  target_descriptor: TargetDescriptor,
  target_name: &str,
) -> Result<Option<Target>, anyhow::Error> {
  if skip_target(entry, target_name, &target_descriptor.source) {
    return Ok(None);
  }

  let mut engines = target_descriptor
    .engines
    .clone()
    .or_else(|| package_json.contents.engines.clone())
    .unwrap_or_else(|| options.default_target_options.engines.clone());

  // TODO LOC
  let context = target_descriptor
    .context
    .unwrap_or_else(|| infer_environment_context(&package_json.contents, Some(engines.clone())));

  // Default browsers if it has not been set yet
  if engines.browsers.is_none()
    && matches!(
      context,
      EnvironmentContext::Browser
        | EnvironmentContext::ServiceWorker
        | EnvironmentContext::WebWorker
        | EnvironmentContext::ElectronRenderer
    )
  {
    engines.browsers = Some(EnginesBrowsers::default());
  }

  let dist_entry = target_descriptor
    .dist_entry
    .clone()
    .or_else(|| dist.as_ref().and_then(|d| d.file_name().map(PathBuf::from)));

  let inferred_output_format = infer_output_format(&dist_entry, package_json, &target_descriptor)?;

  let output_format = target_descriptor
    .output_format
    .or(options.default_target_options.output_format)
    .or(inferred_output_format)
    .unwrap_or(match target_name {
      "browser" => OutputFormat::CommonJS,
      "main" => OutputFormat::CommonJS,
      "module" => OutputFormat::EsModule,
      "types" => OutputFormat::CommonJS,
      _ => match context {
        EnvironmentContext::ElectronMain => OutputFormat::CommonJS,
        EnvironmentContext::ElectronRenderer => OutputFormat::CommonJS,
        EnvironmentContext::Node => OutputFormat::CommonJS,
        _ => OutputFormat::Global,
      },
    });

  if target_name == "main"
    && output_format == OutputFormat::EsModule
    && inferred_output_format.is_some_and(|f| f != OutputFormat::EsModule)
  {
    return Err(diagnostic_error!(DiagnosticBuilder::default()
      .code_frames(vec![CodeFrame::from(package_json)])
      .message("Output format \"esmodule\" cannot be used in the \"main\" target without a .mjs extension or \"type\": \"module\" field")));
  }

  let is_library = target_descriptor
    .is_library
    .unwrap_or_else(|| options.default_target_options.is_library.unwrap_or(false));

  let target_descriptor_engines = target_descriptor.engines.clone();

  tracing::debug!("Target descriptor engines: {:?}", target_descriptor_engines);

  Ok(Some(Target {
    dist_dir: match dist.as_ref() {
      None => options
        .default_target_options
        .dist_dir
        .clone()
        .unwrap_or_else(|| default_dist_dir(&package_json.path).join(target_name)),
      Some(target_dist) => {
        let package_dir = package_json
          .path
          .parent()
          .unwrap_or_else(|| &package_json.path);
        let dir = target_dist
          .parent()
          .map(|dir| dir.strip_prefix("./").ok().unwrap_or(dir))
          .and_then(|dir| {
            if dir == PathBuf::from("") {
              None
            } else {
              Some(dir)
            }
          });

        match dir {
          None => PathBuf::from(package_dir),
          Some(dir) => package_dir.join(dir),
        }
      }
    },
    dist_entry,
    env: Arc::new(Environment {
      context,
      engines,
      include_node_modules: target_descriptor
        .include_node_modules
        .unwrap_or_else(|| IncludeNodeModules::from(context)),
      is_library,
      loc: None, // TODO
      output_format,
      should_optimize: options.default_target_options.should_optimize.map_or_else(
        || target_descriptor.optimize.unwrap_or(!is_library),
        |o| o && target_descriptor.optimize.unwrap_or(!is_library),
      ),
      should_scope_hoist: (is_library
        || options
          .default_target_options
          .should_scope_hoist
          .unwrap_or(false))
        && (target_descriptor.scope_hoist.is_none()
          || target_descriptor.scope_hoist.is_some_and(|s| s)),
      source_map: match options.default_target_options.source_maps {
        false => None,
        true => match target_descriptor.source_map.as_ref() {
          None => Some(TargetSourceMapOptions::default()),
          Some(SourceMapField::Bool(source_maps)) => {
            source_maps.then(TargetSourceMapOptions::default)
          }
          Some(SourceMapField::Options(source_maps)) => Some(source_maps.clone()),
        },
      },
      ..Environment::default()
    }),
    loc: None, // TODO
    name: String::from(target_name),
    public_url: target_descriptor
      .public_url
      .clone()
      .unwrap_or(options.default_target_options.public_url.clone()),
    ..Target::default()
  }))
}

fn skip_target(entry: &Entry, target_name: &str, source: &Option<SourceField>) -> bool {
  // We skip targets if they have a descriptor.source that does not match the current
  // exclusiveTarget. They will be handled by a separate resolvePackageTargets call from their
  // Entry point but with exclusiveTarget set.
  match entry.target.as_ref() {
    None => source.is_some(),
    Some(exclusive_target) => target_name != exclusive_target,
  }
}

pub fn infer_environment_context(
  package_json: &PackageJson,
  engines_descriptor: Option<Engines>,
) -> EnvironmentContext {
  // If there is a separate `browser` target, or an `engines.node` field but no browser
  // targets, then the target refers to node, otherwise browser.
  if package_json.browser.is_some() || package_json.targets.browser.is_some() {
    let is_node = |engines: &Engines| {
      engines.node.is_some()
        && engines
          .browsers
          .as_ref()
          .is_none_or(|browsers| Browsers::from(browsers).is_empty())
    };

    if engines_descriptor.as_ref().is_some_and(is_node)
      || package_json.engines.as_ref().is_some_and(is_node)
    {
      return EnvironmentContext::Node;
    } else {
      return EnvironmentContext::Browser;
    }
  }

  if engines_descriptor
    .as_ref()
    .is_some_and(|e| e.node.is_some())
    || package_json
      .engines
      .as_ref()
      .is_some_and(|e| e.node.is_some())
  {
    return EnvironmentContext::Node;
  }

  EnvironmentContext::Browser
}

fn infer_output_format(
  dist_entry: &Option<PathBuf>,
  package_json: &ConfigFile<PackageJson>,
  target: &TargetDescriptor,
) -> Result<Option<OutputFormat>, anyhow::Error> {
  let ext = dist_entry
    .as_ref()
    .and_then(|e| e.extension())
    .unwrap_or_default()
    .to_str();

  let inferred_output_format = match ext {
    Some("cjs") => Some(OutputFormat::CommonJS),
    Some("mjs") => Some(OutputFormat::EsModule),
    Some("js") => package_json
      .contents
      .module_format
      .as_ref()
      .map(|format| match format {
        ModuleFormat::CommonJS => OutputFormat::CommonJS,
        ModuleFormat::Module => OutputFormat::EsModule,
      }),
    _ => None,
  };

  if let Some(inferred_output_format) = inferred_output_format {
    if let Some(output_format) = target.output_format {
      if output_format != inferred_output_format {
        return Err(diagnostic_error!(DiagnosticBuilder::default()
          .code_frames(vec![CodeFrame::from(package_json)])
          .message(format!(
            "Declared output format {output_format} does not match expected output format {inferred_output_format}",
          ))));
      }
    }
  }

  Ok(inferred_output_format)
}
