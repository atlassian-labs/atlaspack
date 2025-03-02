use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::config_loader::ConfigLoader;
use atlaspack_core::diagnostic_error;
use atlaspack_core::types::engines::Engines;
use atlaspack_core::types::engines::EnginesBrowsers;
use atlaspack_core::types::*;
use atlaspack_filesystem::FileSystemRef;

use super::default_dist_dir::default_dist_dir;
use super::fallback_output_format::fallback_output_format;
use super::load_config::load_config;
use super::package_json::BuiltInTargetDescriptor;
use super::target::builtin_browser_target;
use super::target::builtin_main_target;
use super::target::builtin_module_target;
use super::target::builtin_types_target;
use super::target::CustomTarget;
use super::target_from_descriptor::target_from_descriptor;
use crate::build_targets::target_from_descriptor::infer_environment_context;

pub fn resolve_package_targets(
  entry: &Entry,
  config_loader: &ConfigLoader,
  options: &AtlaspackOptions,
  file_system: &FileSystemRef,
) -> Result<Vec<Option<Target>>, anyhow::Error> {
  let config = load_config(config_loader, options, file_system)?;
  let mut targets: Vec<Option<Target>> = Vec::new();

  let builtin_targets = [
    builtin_browser_target(
      config.contents.targets.browser.clone(),
      config.contents.browser.clone(),
      config.contents.name.clone(),
    ),
    builtin_main_target(
      config.contents.targets.main.clone(),
      config.contents.main.clone(),
    ),
    builtin_module_target(
      config.contents.targets.module.clone(),
      config.contents.module.clone(),
    ),
    builtin_types_target(
      config.contents.targets.types.clone(),
      config.contents.types.clone(),
    ),
  ];

  let mut target_filter = None;

  if let Some(target_options) = &options.targets {
    match target_options {
      Targets::Filter(target_list) => {
        target_filter = Some(target_list);
      }
      Targets::CustomTarget(custom_targets) => {
        for (name, descriptor) in custom_targets.iter() {
          targets.push(target_from_descriptor(
            entry,
            options,
            None,
            &config,
            descriptor.clone(),
            name,
          )?);
        }

        // Custom targets have been passed in so let's just use those
        return Ok(targets);
      }
    }
  }

  for builtin_target in builtin_targets {
    if builtin_target.dist.is_none() {
      continue;
    }

    match builtin_target.descriptor {
      BuiltInTargetDescriptor::Disabled(_disabled) => continue,
      BuiltInTargetDescriptor::TargetDescriptor(builtin_target_descriptor) => {
        targets.push(target_from_descriptor(
          entry,
          options,
          builtin_target.dist,
          &config,
          builtin_target_descriptor,
          &builtin_target.name,
        )?);
      }
    }
  }

  let custom_targets = config
    .contents
    .targets
    .custom_targets
    .iter()
    .map(|(name, descriptor)| CustomTarget {
      descriptor,
      name: name.clone(),
    })
    .filter(|CustomTarget { name, .. }| {
      target_filter
        .as_ref()
        .is_none_or(|targets| targets.iter().any(|target_name| target_name == name))
    });

  for custom_target in custom_targets {
    let mut dist = None;
    if let Some(value) = config.contents.fields.get(&custom_target.name) {
      match value {
        serde_json::Value::String(str) => {
          dist = Some(PathBuf::from(str));
        }
        _ => {
          return Err(diagnostic_error!(DiagnosticBuilder::default()
            .code_frames(vec![CodeFrame::from(&config)])
            .message(format!("Invalid path for target {}", custom_target.name))));
        }
      }
    }

    targets.push(target_from_descriptor(
      entry,
      options,
      dist,
      &config,
      custom_target.descriptor.clone(),
      &custom_target.name,
    )?);
  }

  if targets.is_empty() {
    let context = infer_environment_context(&config.contents, None);

    let is_library = options.default_target_options.is_library.unwrap_or(false);
    let config_engines = config
      .contents
      .engines
      .unwrap_or_else(|| get_default_engines_for_context(options, context));

    tracing::debug!("Package JSON engines: {:?}", config_engines);

    targets.push(Some(Target {
      dist_dir: options
        .default_target_options
        .dist_dir
        .clone()
        .unwrap_or_else(|| default_dist_dir(&config.path)),
      dist_entry: None,
      env: Arc::new(Environment {
        context,
        engines: config_engines,
        include_node_modules: IncludeNodeModules::from(context),
        is_library,
        loc: None,
        output_format: options
          .default_target_options
          .output_format
          .unwrap_or_else(|| fallback_output_format(context)),
        should_optimize: options
          .default_target_options
          .should_optimize
          .unwrap_or_else(|| options.mode == BuildMode::Production && !is_library),
        should_scope_hoist: options
          .default_target_options
          .should_scope_hoist
          .unwrap_or_else(|| options.mode == BuildMode::Production && !is_library),
        source_map: options
          .default_target_options
          .source_maps
          .then(TargetSourceMapOptions::default),
        source_type: SourceType::Module,
      }),
      loc: None,
      name: String::from("default"),
      public_url: options.default_target_options.public_url.clone(),
      ..Target::default()
    }));
  }

  Ok(targets)
}

fn get_default_engines_for_context(
  options: &AtlaspackOptions,
  context: EnvironmentContext,
) -> Engines {
  let defaults = options.default_target_options.engines.clone();
  if context.is_browser() {
    Engines {
      browsers: defaults.browsers.or(Some(EnginesBrowsers::default())),
      ..Engines::default()
    }
  } else if context.is_node() {
    Engines {
      node: defaults.node,
      ..Engines::default()
    }
  } else {
    defaults
  }
}
