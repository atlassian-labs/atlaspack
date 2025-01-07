use std::collections::BTreeMap;
use std::hash::Hash;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_core::config_loader::ConfigFile;
use atlaspack_core::diagnostic_error;
use atlaspack_core::types::browsers::Browsers;
use atlaspack_core::types::engines::Engines;
use atlaspack_core::types::engines::EnginesBrowsers;
use atlaspack_core::types::BuildMode;
use atlaspack_core::types::CodeFrame;
use atlaspack_core::types::DefaultTargetOptions;
use atlaspack_core::types::Diagnostic;
use atlaspack_core::types::DiagnosticBuilder;
use atlaspack_core::types::Environment;
use atlaspack_core::types::EnvironmentContext;
use atlaspack_core::types::ErrorKind;
use atlaspack_core::types::OutputFormat;
use atlaspack_core::types::SourceField;
use atlaspack_core::types::SourceMapField;
use atlaspack_core::types::SourceType;
use atlaspack_core::types::Target;
use atlaspack_core::types::TargetDescriptor;
use atlaspack_core::types::TargetSourceMapOptions;
use atlaspack_core::types::Targets;
use atlaspack_resolver::IncludeNodeModules;
use package_json::BrowserField;
use package_json::BrowsersList;
use package_json::BuiltInTargetDescriptor;
use package_json::ModuleFormat;
use package_json::PackageJson;

use crate::request_tracker::Request;
use crate::request_tracker::ResultAndInvalidations;
use crate::request_tracker::RunRequestContext;
use crate::request_tracker::RunRequestError;

use super::entry_request::Entry;
use super::RequestResult;

mod package_json;

/// Infers how and where source code is outputted
///
/// Targets will be generated from the project package.json file and input Atlaspack options.
///
#[derive(Debug, Hash)]
pub struct TargetRequest {
  pub default_target_options: DefaultTargetOptions,
  pub entry: Entry,
  pub env: Option<BTreeMap<String, String>>,
  pub mode: BuildMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TargetRequestOutput {
  pub entry: PathBuf,
  pub targets: Vec<Target>,
}

struct BuiltInTarget<'a> {
  descriptor: BuiltInTargetDescriptor,
  dist: Option<PathBuf>,
  name: &'a str,
}

struct CustomTarget<'a> {
  descriptor: &'a TargetDescriptor,
  name: &'a str,
}

impl TargetRequest {
  fn builtin_browser_target(
    &self,
    descriptor: Option<BuiltInTargetDescriptor>,
    dist: Option<BrowserField>,
    name: Option<String>,
  ) -> BuiltInTarget {
    BuiltInTarget {
      descriptor: descriptor
        .map(|d| {
          merge_builtin_descriptors(d, builtin_target_descriptor(EnvironmentContext::Browser))
        })
        .unwrap_or_else(|| {
          BuiltInTargetDescriptor::TargetDescriptor(builtin_target_descriptor(
            EnvironmentContext::Browser,
          ))
        }),
      dist: dist.and_then(|browser| match browser {
        BrowserField::EntryPoint(entrypoint) => Some(entrypoint.clone()),
        BrowserField::ReplacementBySpecifier(replacements) => {
          let name = name?;
          let replacements = replacements.get(&name)?;
          let path = replacements.as_str()?;
          Some(path.into())
        }
      }),
      name: "browser",
    }
  }

  fn builtin_main_target(
    &self,
    descriptor: Option<BuiltInTargetDescriptor>,
    dist: Option<PathBuf>,
  ) -> BuiltInTarget {
    BuiltInTarget {
      descriptor: descriptor
        .map(|d| merge_builtin_descriptors(d, builtin_target_descriptor(EnvironmentContext::Node)))
        .unwrap_or_else(|| {
          BuiltInTargetDescriptor::TargetDescriptor(builtin_target_descriptor(
            EnvironmentContext::Node,
          ))
        }),
      dist,
      name: "main",
    }
  }

  fn builtin_module_target(
    &self,
    descriptor: Option<BuiltInTargetDescriptor>,
    dist: Option<PathBuf>,
  ) -> BuiltInTarget {
    BuiltInTarget {
      descriptor: descriptor
        .map(|d| merge_builtin_descriptors(d, builtin_target_descriptor(EnvironmentContext::Node)))
        .unwrap_or_else(|| {
          BuiltInTargetDescriptor::TargetDescriptor(builtin_target_descriptor(
            EnvironmentContext::Node,
          ))
        }),
      dist,
      name: "module",
    }
  }

  fn builtin_types_target(
    &self,
    descriptor: Option<BuiltInTargetDescriptor>,
    dist: Option<PathBuf>,
  ) -> BuiltInTarget {
    BuiltInTarget {
      descriptor: descriptor
        .map(|d| merge_builtin_descriptors(d, builtin_target_descriptor(EnvironmentContext::Node)))
        .unwrap_or_else(|| {
          BuiltInTargetDescriptor::TargetDescriptor(builtin_target_descriptor(
            EnvironmentContext::Node,
          ))
        }),
      dist,
      name: "types",
    }
  }

  fn infer_environment_context(
    &self,
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
    &self,
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

  fn load_config(
    &self,
    request_context: &RunRequestContext,
  ) -> Result<ConfigFile<PackageJson>, anyhow::Error> {
    // TODO Invalidations
    let mut config = match request_context.config().load_package_json::<PackageJson>() {
      Err(err) => {
        let diagnostic = err.downcast_ref::<Diagnostic>();

        if diagnostic.is_some_and(|d| d.kind != ErrorKind::NotFound) {
          return Err(err);
        }

        ConfigFile {
          contents: PackageJson::default(),
          path: PathBuf::default(),
          raw: String::default(),
        }
      }
      Ok(pkg) => pkg,
    };

    if let Some(engines) = config.contents.engines.as_ref() {
      if let Some(browsers) = &engines.browsers {
        if !Browsers::from(browsers).is_empty() {
          return Ok(config);
        }
      }
    }

    let env = self
      .env
      .as_ref()
      .and_then(|env| env.get("BROWSERSLIST_ENV").or_else(|| env.get("NODE_ENV")))
      .map(|e| e.to_owned())
      .unwrap_or_else(|| self.mode.to_string());

    let browsers = match config.contents.browserslist.clone() {
      None => {
        let browserslistrc_path = &request_context
          .config()
          .project_root
          .join(".browserslistrc");

        // Loading .browserslistrc
        if request_context
          .file_system()
          .is_file(browserslistrc_path.as_path())
        {
          let browserslistrc = request_context
            .file_system()
            .read_to_string(browserslistrc_path)?;

          Some(EnginesBrowsers::from_browserslistrc(&browserslistrc)?)
        } else {
          None
        }
      }
      Some(browserslist) => Some(EnginesBrowsers::new(match browserslist {
        BrowsersList::Browser(browser) => vec![browser],
        BrowsersList::Browsers(browsers) => browsers,
        BrowsersList::BrowsersByEnv(browsers_by_env) => {
          browsers_by_env.get(&env).cloned().unwrap_or_default()
        }
      })),
    };

    if let Some(browserslist) = browsers {
      config.contents.engines = Some(Engines {
        browsers: Some(browserslist),
        ..config.contents.engines.unwrap_or_default()
      });
    }

    Ok(config)
  }

  fn resolve_package_targets(
    &self,
    request_context: RunRequestContext,
  ) -> Result<Vec<Option<Target>>, anyhow::Error> {
    let config = self.load_config(&request_context)?;
    let mut targets: Vec<Option<Target>> = Vec::new();

    let builtin_targets = [
      self.builtin_browser_target(
        config.contents.targets.browser.clone(),
        config.contents.browser.clone(),
        config.contents.name.clone(),
      ),
      self.builtin_main_target(
        config.contents.targets.main.clone(),
        config.contents.main.clone(),
      ),
      self.builtin_module_target(
        config.contents.targets.module.clone(),
        config.contents.module.clone(),
      ),
      self.builtin_types_target(
        config.contents.targets.types.clone(),
        config.contents.types.clone(),
      ),
    ];

    let mut target_filter = None;

    if let Some(target_options) = &request_context.options.targets {
      match target_options {
        Targets::Filter(target_list) => {
          target_filter = Some(target_list);
        }
        Targets::CustomTarget(custom_targets) => {
          for (name, descriptor) in custom_targets.iter() {
            targets.push(self.target_from_descriptor(None, &config, descriptor.clone(), name)?);
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
          targets.push(self.target_from_descriptor(
            builtin_target.dist,
            &config,
            builtin_target_descriptor,
            builtin_target.name,
          )?);
        }
      }
    }

    let custom_targets = config
      .contents
      .targets
      .custom_targets
      .iter()
      .map(|(name, descriptor)| CustomTarget { descriptor, name })
      .filter(|CustomTarget { name, .. }| {
        target_filter
          .as_ref()
          .is_none_or(|targets| targets.iter().any(|target_name| target_name == name))
      });

    for custom_target in custom_targets {
      let mut dist = None;
      if let Some(value) = config.contents.fields.get(custom_target.name) {
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

      targets.push(self.target_from_descriptor(
        dist,
        &config,
        custom_target.descriptor.clone(),
        custom_target.name,
      )?);
    }

    if targets.is_empty() {
      let context = self.infer_environment_context(&config.contents, None);

      let is_library = self.default_target_options.is_library.unwrap_or(false);
      let config_engines = config
        .contents
        .engines
        .unwrap_or_else(|| self.get_default_engines_for_context(context));

      tracing::debug!("Package JSON engines: {:?}", config_engines);

      targets.push(Some(Target {
        dist_dir: self
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
          output_format: self
            .default_target_options
            .output_format
            .unwrap_or_else(|| fallback_output_format(context)),
          should_optimize: self
            .default_target_options
            .should_optimize
            .unwrap_or_else(|| self.mode == BuildMode::Production && !is_library),
          should_scope_hoist: self
            .default_target_options
            .should_scope_hoist
            .unwrap_or_else(|| self.mode == BuildMode::Production && !is_library),
          source_map: self
            .default_target_options
            .source_maps
            .then(TargetSourceMapOptions::default),
          source_type: SourceType::Module,
        }),
        loc: None,
        name: String::from("default"),
        public_url: self.default_target_options.public_url.clone(),
        ..Target::default()
      }));
    }

    Ok(targets)
  }

  fn get_default_engines_for_context(&self, context: EnvironmentContext) -> Engines {
    let defaults = self.default_target_options.engines.clone();
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

  fn skip_target(&self, target_name: &str, source: &Option<SourceField>) -> bool {
    // We skip targets if they have a descriptor.source that does not match the current
    // exclusiveTarget. They will be handled by a separate resolvePackageTargets call from their
    // Entry point but with exclusiveTarget set.
    match self.entry.target.as_ref() {
      None => source.is_some(),
      Some(exclusive_target) => target_name != exclusive_target,
    }
  }

  fn target_from_descriptor(
    &self,
    dist: Option<PathBuf>,
    package_json: &ConfigFile<PackageJson>,
    target_descriptor: TargetDescriptor,
    target_name: &str,
  ) -> Result<Option<Target>, anyhow::Error> {
    if self.skip_target(target_name, &target_descriptor.source) {
      return Ok(None);
    }

    let mut engines = target_descriptor
      .engines
      .clone()
      .or_else(|| package_json.contents.engines.clone())
      .unwrap_or_else(|| self.default_target_options.engines.clone());

    // TODO LOC
    let context = target_descriptor.context.unwrap_or_else(|| {
      self.infer_environment_context(&package_json.contents, Some(engines.clone()))
    });

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

    let inferred_output_format =
      self.infer_output_format(&dist_entry, package_json, &target_descriptor)?;

    let output_format = target_descriptor
      .output_format
      .or(self.default_target_options.output_format)
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
      .unwrap_or_else(|| self.default_target_options.is_library.unwrap_or(false));

    let target_descriptor_engines = target_descriptor.engines.clone();

    tracing::debug!("Target descriptor engines: {:?}", target_descriptor_engines);

    Ok(Some(Target {
      dist_dir: match dist.as_ref() {
        None => self
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
        should_optimize: self.default_target_options.should_optimize.map_or_else(
          || target_descriptor.optimize.unwrap_or(!is_library),
          |o| o && target_descriptor.optimize.unwrap_or(!is_library),
        ),
        should_scope_hoist: (is_library
          || self
            .default_target_options
            .should_scope_hoist
            .unwrap_or(false))
          && (target_descriptor.scope_hoist.is_none()
            || target_descriptor.scope_hoist.is_some_and(|s| s)),
        source_map: match self.default_target_options.source_maps {
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
        .unwrap_or(self.default_target_options.public_url.clone()),
      ..Target::default()
    }))
  }
}

fn merge_builtin_descriptors(
  descriptor: BuiltInTargetDescriptor,
  default_descriptor: TargetDescriptor,
) -> BuiltInTargetDescriptor {
  if let BuiltInTargetDescriptor::TargetDescriptor(descriptor) = descriptor {
    return BuiltInTargetDescriptor::TargetDescriptor(TargetDescriptor {
      context: descriptor.context.or(default_descriptor.context),
      dist_dir: descriptor.dist_dir.or(default_descriptor.dist_dir),
      dist_entry: descriptor.dist_entry.or(default_descriptor.dist_entry),
      engines: descriptor.engines.or(default_descriptor.engines),
      include_node_modules: descriptor
        .include_node_modules
        .or(default_descriptor.include_node_modules),
      is_library: descriptor.is_library.or(default_descriptor.is_library),
      optimize: descriptor.optimize.or(default_descriptor.optimize),
      output_format: descriptor
        .output_format
        .or(default_descriptor.output_format),
      public_url: descriptor.public_url.or(default_descriptor.public_url),
      scope_hoist: descriptor.scope_hoist.or(default_descriptor.scope_hoist),
      source: descriptor.source.or(default_descriptor.source),
      source_map: descriptor.source_map.or(default_descriptor.source_map),
    });
  }

  descriptor
}

fn builtin_target_descriptor(context: EnvironmentContext) -> TargetDescriptor {
  TargetDescriptor {
    context: Some(context),
    include_node_modules: Some(IncludeNodeModules::Bool(false)),
    is_library: Some(true),
    scope_hoist: Some(true),
    ..TargetDescriptor::default()
  }
}

fn default_dist_dir(package_path: &Path) -> PathBuf {
  package_path.parent().unwrap_or(package_path).join("dist")
}

fn fallback_output_format(context: EnvironmentContext) -> OutputFormat {
  match context {
    EnvironmentContext::Node => OutputFormat::CommonJS,
    EnvironmentContext::ElectronMain => OutputFormat::CommonJS,
    EnvironmentContext::ElectronRenderer => OutputFormat::CommonJS,
    _ => OutputFormat::Global,
  }
}

#[async_trait]
impl Request for TargetRequest {
  #[tracing::instrument(level = "info", skip_all)]
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    // TODO options.targets, should this still be supported?
    // TODO serve options
    let package_targets = self.resolve_package_targets(request_context)?;

    Ok(ResultAndInvalidations {
      invalidations: Vec::new(),
      result: RequestResult::Target(TargetRequestOutput {
        entry: self.entry.file_path.clone(),
        targets: package_targets.into_iter().flatten().collect(),
      }),
    })
  }
}

// TODO Add more tests when revisiting targets config structure
#[cfg(test)]
mod tests {
  use std::{num::NonZeroU16, sync::Arc};

  use regex::Regex;

  use atlaspack_core::types::{version::Version, AtlaspackOptions};
  use atlaspack_filesystem::in_memory_file_system::InMemoryFileSystem;

  use crate::test_utils::{request_tracker, RequestTrackerTestOptions};
  use pretty_assertions::assert_eq;

  use super::*;

  const BUILT_IN_TARGETS: [&str; 4] = ["browser", "main", "module", "types"];

  fn default_target() -> Target {
    Target {
      dist_dir: PathBuf::from("packages/test/dist"),
      env: Arc::new(Environment {
        context: EnvironmentContext::Browser,
        engines: Engines {
          browsers: Some(EnginesBrowsers::default()),
          ..Engines::default()
        },
        output_format: OutputFormat::Global,
        ..Environment::default()
      }),
      name: String::from("default"),
      ..Target::default()
    }
  }

  fn package_dir() -> PathBuf {
    PathBuf::from("packages").join("test")
  }

  async fn targets_from_config(
    package_json: String,
    browserslistrc: Option<String>,
    atlaspack_options: Option<AtlaspackOptions>,
  ) -> Result<RequestResult, anyhow::Error> {
    let fs = InMemoryFileSystem::default();
    let project_root = PathBuf::default();
    let package_dir = package_dir();

    fs.write_file(
      &project_root.join(&package_dir).join("package.json"),
      package_json,
    );

    if let Some(browserslistrc) = browserslistrc {
      fs.write_file(&project_root.join(".browserslistrc"), browserslistrc);
    }

    let request = TargetRequest {
      default_target_options: DefaultTargetOptions::default(),
      entry: Entry::default(),
      env: None,
      mode: BuildMode::Development,
    };

    request_tracker(RequestTrackerTestOptions {
      search_path: project_root.join(&package_dir),
      project_root,
      fs: Arc::new(fs),
      atlaspack_options: atlaspack_options.unwrap_or_default(),
      ..Default::default()
    })
    .run_request(request)
    .await
  }

  fn to_deterministic_error(error: anyhow::Error) -> String {
    let re = Regex::new(r"\d+").unwrap();
    re.replace_all(&format!("{:#}", error), "\\d").into_owned()
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_error_when_builtin_target_is_true() {
    for builtin_target in BUILT_IN_TARGETS {
      let targets = targets_from_config(
        format!(r#"{{ "targets": {{ "{builtin_target}": true }} }}"#,),
        None,
        None,
      )
      .await;

      assert_eq!(
        targets.map_err(to_deterministic_error),
        Err(format!("data did not match any variant of untagged enum BuiltInTargetDescriptor at line \\d column \\d in {}",
          package_dir().join("package.json").display()
        ))
      );
    }
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_error_when_builtin_target_does_not_reference_expected_extension() {
    for builtin_target in BUILT_IN_TARGETS {
      let targets = targets_from_config(
        format!(r#"{{ "{}": "dist/main.rs" }}"#, builtin_target),
        None,
        None,
      )
      .await;

      assert_eq!(
        targets.map_err(to_deterministic_error),
        Err(format!(
          "Unexpected file type \"main.rs\" in \"{}\" target at line \\d column \\d in {}",
          builtin_target,
          package_dir().join("package.json").display()
        ))
      );
    }
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_error_when_builtin_target_has_global_output_format() {
    for builtin_target in BUILT_IN_TARGETS {
      let targets = targets_from_config(
        format!(
          r#"{{
          "targets": {{
            "{builtin_target}": {{ "outputFormat": "global" }}
          }}
        }}"#
        ),
        None,
        None,
      )
      .await;

      assert_eq!(
        targets.map_err(to_deterministic_error),
        Err(format!(
          "The \"global\" output format is not supported in the {} target at line \\d column \\d in {}",
          builtin_target,
          package_dir().join("package.json").display()
        ))
      );
    }
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_error_when_output_format_does_not_match_inferred_output_format() {
    let assert_error = move |ext, module_format: Option<&'static str>, output_format| async move {
      let targets = targets_from_config(
        format!(
          r#"
          {{
            {}
            "custom": "dist/custom.{ext}",
            "targets": {{
              "custom": {{
                "outputFormat": "{output_format}"
              }}
            }}
          }}
        "#,
          module_format.map_or_else(String::default, |module_format| format!(
            r#""type": "{module_format}","#
          )),
        ),
        None,
        None,
      )
      .await;

      assert_eq!(
        targets.map_err(|err| err.to_string()),
        Err(format!(
          "Declared output format {output_format} does not match expected output format {}",
          if output_format == OutputFormat::CommonJS {
            "esmodule"
          } else {
            "commonjs"
          }
        ))
      );
    };

    assert_error("cjs", None, OutputFormat::EsModule).await;
    assert_error("cjs", Some("module"), OutputFormat::EsModule).await;

    assert_error("js", Some("commonjs"), OutputFormat::EsModule).await;
    assert_error("js", Some("module"), OutputFormat::CommonJS).await;

    assert_error("mjs", None, OutputFormat::CommonJS).await;
    assert_error("mjs", Some("commonjs"), OutputFormat::CommonJS).await;
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_error_when_scope_hoisting_disabled_for_library_targets() {
    let assert_error = move |name, package_json| async move {
      let targets = targets_from_config(package_json, None, None).await;

      assert_eq!(
        targets.map_err(to_deterministic_error),
        Err(format!(
          "Scope hoisting cannot be disabled for \"{}\" library target at line \\d column \\d in {}",
          name,
          package_dir().join("package.json").display()
        ))
      );
    };

    for target in BUILT_IN_TARGETS {
      assert_error(
        target,
        format!(
          r#"
            {{
              "{target}": "dist/target.{ext}",
              "targets": {{
                "{target}": {{
                  "isLibrary": true,
                  "scopeHoist": false
                }}
              }}
            }}
          "#,
          ext = if target == "types" { "ts" } else { "js" },
        ),
      )
      .await;
    }

    assert_error(
      "custom",
      String::from(
        r#"
          {
            "targets": {
              "custom": {
                "isLibrary": true,
                "scopeHoist": false
              }
            }
          }
        "#,
      ),
    )
    .await;
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_default_target_when_package_json_is_not_found() {
    let request = TargetRequest {
      default_target_options: DefaultTargetOptions::default(),
      entry: Entry::default(),
      env: None,
      mode: BuildMode::Development,
    };

    let targets = request_tracker(RequestTrackerTestOptions::default())
      .run_request(request)
      .await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: default_dist_dir(&PathBuf::default()),
          ..default_target()
        }],
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_default_target_when_builtin_targets_are_disabled() {
    for builtin_target in BUILT_IN_TARGETS {
      let targets = targets_from_config(
        format!(r#"{{ "targets": {{ "{builtin_target}": false }} }}"#),
        None,
        None,
      )
      .await;

      assert_eq!(
        targets.map_err(|e| e.to_string()),
        Ok(RequestResult::Target(TargetRequestOutput {
          entry: PathBuf::default(),
          targets: vec![default_target()]
        }))
      );
    }
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_default_target_when_no_targets_are_specified() {
    let targets = targets_from_config(String::from("{}"), None, None).await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![default_target()]
      }))
    );
  }

  fn builtin_default_env() -> Environment {
    Environment {
      include_node_modules: IncludeNodeModules::Bool(false),
      is_library: true,
      should_optimize: false,
      should_scope_hoist: true,
      ..Environment::default()
    }
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_default_builtin_browser_target() {
    let targets = targets_from_config(
      String::from(r#"{ "browser": "build/browser.js" }"#),
      None,
      None,
    )
    .await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir().join("build"),
          dist_entry: Some(PathBuf::from("browser.js")),
          env: Arc::new(Environment {
            context: EnvironmentContext::Browser,
            engines: Engines {
              browsers: Some(EnginesBrowsers::default()),
              ..Engines::default()
            },
            output_format: OutputFormat::CommonJS,
            ..builtin_default_env()
          }),
          name: String::from("browser"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_builtin_browser_target() {
    let targets = targets_from_config(
      String::from(
        r#"
        {
          "browser": "build/browser.js",
          "targets": {
            "browser": {
              "outputFormat": "esmodule"
            }
          }
        }
      "#,
      ),
      None,
      None,
    )
    .await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir().join("build"),
          dist_entry: Some(PathBuf::from("browser.js")),
          env: Arc::new(Environment {
            context: EnvironmentContext::Browser,
            engines: Engines {
              browsers: Some(EnginesBrowsers::default()),
              ..Engines::default()
            },
            output_format: OutputFormat::EsModule,
            ..builtin_default_env()
          }),
          name: String::from("browser"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_default_builtin_main_target() {
    let targets =
      targets_from_config(String::from(r#"{ "main": "./build/main.js" }"#), None, None).await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir().join("build"),
          dist_entry: Some(PathBuf::from("main.js")),
          env: Arc::new(Environment {
            context: EnvironmentContext::Node,
            output_format: OutputFormat::CommonJS,
            ..builtin_default_env()
          }),
          name: String::from("main"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_builtin_main_target() {
    let targets = targets_from_config(
      String::from(
        r#"
        {
          "main": "./build/main.js",
          "targets": {
            "main": {
              "optimize": true
            }
          }
        }
      "#,
      ),
      None,
      None,
    )
    .await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir().join("build"),
          dist_entry: Some(PathBuf::from("main.js")),
          env: Arc::new(Environment {
            context: EnvironmentContext::Node,
            output_format: OutputFormat::CommonJS,
            should_optimize: true,
            ..builtin_default_env()
          }),
          name: String::from("main"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_default_builtin_module_target() {
    let targets =
      targets_from_config(String::from(r#"{ "module": "module.js" }"#), None, None).await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir(),
          dist_entry: Some(PathBuf::from("module.js")),
          env: Arc::new(Environment {
            context: EnvironmentContext::Node,
            output_format: OutputFormat::EsModule,
            ..builtin_default_env()
          }),
          name: String::from("module"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_builtin_module_target() {
    let targets = targets_from_config(
      String::from(
        r#"
        {
          "module": "module.js",
          "targets": {
            "module": {
              "optimize": true
            }
          }
        }
      "#,
      ),
      None,
      None,
    )
    .await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir(),
          dist_entry: Some(PathBuf::from("module.js")),
          env: Arc::new(Environment {
            context: EnvironmentContext::Node,
            output_format: OutputFormat::EsModule,
            should_optimize: true,
            ..builtin_default_env()
          }),
          name: String::from("module"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_builtin_types_target() {
    let targets = targets_from_config(
      String::from(
        r#"
        {
          "types": "./types.d.ts",
          "targets": {
            "types": {
              "outputFormat": "esmodule"
            }
          }
        }
      "#,
      ),
      None,
      None,
    )
    .await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir(),
          dist_entry: Some(PathBuf::from("types.d.ts")),
          env: Arc::new(Environment {
            context: EnvironmentContext::Node,
            output_format: OutputFormat::EsModule,
            ..builtin_default_env()
          }),
          name: String::from("types"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_default_builtin_types_target() {
    let targets =
      targets_from_config(String::from(r#"{ "types": "./types.d.ts" }"#), None, None).await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir(),
          dist_entry: Some(PathBuf::from("types.d.ts")),
          env: Arc::new(Environment {
            context: EnvironmentContext::Node,
            output_format: OutputFormat::CommonJS,
            ..builtin_default_env()
          }),
          name: String::from("types"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_builtin_targets() {
    let targets = targets_from_config(
      String::from(
        r#"
        {
          "browser": "build/browser.js",
          "main": "./build/main.js",
          "module": "module.js",
          "types": "./types.d.ts",
          "browserslist": ["chrome 20"]
        }
      "#,
      ),
      None,
      None,
    )
    .await;

    let env = || Environment {
      engines: Engines {
        browsers: Some(EnginesBrowsers::new(vec![String::from("chrome 20")])),
        ..Engines::default()
      },
      ..builtin_default_env()
    };

    let package_dir = package_dir();

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![
          Target {
            dist_dir: package_dir.join("build"),
            dist_entry: Some(PathBuf::from("browser.js")),
            env: Arc::new(Environment {
              context: EnvironmentContext::Browser,
              output_format: OutputFormat::CommonJS,
              ..env()
            }),
            name: String::from("browser"),
            ..Target::default()
          },
          Target {
            dist_dir: package_dir.join("build"),
            dist_entry: Some(PathBuf::from("main.js")),
            env: Arc::new(Environment {
              context: EnvironmentContext::Node,
              output_format: OutputFormat::CommonJS,
              ..env()
            }),
            name: String::from("main"),
            ..Target::default()
          },
          Target {
            dist_dir: package_dir.clone(),
            dist_entry: Some(PathBuf::from("module.js")),
            env: Arc::new(Environment {
              context: EnvironmentContext::Node,
              output_format: OutputFormat::EsModule,
              ..env()
            }),
            name: String::from("module"),
            ..Target::default()
          },
          Target {
            dist_dir: package_dir,
            dist_entry: Some(PathBuf::from("types.d.ts")),
            env: Arc::new(Environment {
              context: EnvironmentContext::Node,
              output_format: OutputFormat::CommonJS,
              ..env()
            }),
            name: String::from("types"),
            ..Target::default()
          },
        ]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_custom_targets_with_defaults() {
    let targets = targets_from_config(
      String::from(r#"{ "targets": { "custom": {} } } "#),
      None,
      None,
    )
    .await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir().join("dist").join("custom"),
          dist_entry: None,
          env: Arc::new(Environment {
            context: EnvironmentContext::Browser,
            engines: Engines {
              browsers: Some(EnginesBrowsers::default()),
              ..Engines::default()
            },
            is_library: false,
            output_format: OutputFormat::Global,
            should_optimize: true,
            should_scope_hoist: false,
            ..Environment::default()
          }),
          name: String::from("custom"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_custom_targets_from_options_with_defaults() {
    let targets = targets_from_config(
      String::from(r#"{}"#),
      None,
      Some(AtlaspackOptions {
        targets: Some(Targets::CustomTarget(BTreeMap::from([(
          "custom".into(),
          TargetDescriptor::default(),
        )]))),
        ..Default::default()
      }),
    )
    .await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir().join("dist").join("custom"),
          dist_entry: None,
          env: Arc::new(Environment {
            context: EnvironmentContext::Browser,
            engines: Engines {
              browsers: Some(EnginesBrowsers::default()),
              ..Engines::default()
            },
            is_library: false,
            output_format: OutputFormat::Global,
            should_optimize: true,
            should_scope_hoist: false,
            ..Environment::default()
          }),
          name: String::from("custom"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_only_requested_custom_targets() {
    let targets = targets_from_config(
      String::from(
        r#"
        {
          "targets": {
            "custom-one": {
              "context": "node",
              "includeNodeModules": true,
              "outputFormat": "commonjs"
            },
            "custom-two": {
              "context": "browser",
              "outputFormat": "esmodule"
            }
          }
        }
      "#,
      ),
      None,
      Some(AtlaspackOptions {
        targets: Some(Targets::Filter(vec!["custom-two".into()])),
        ..Default::default()
      }),
    )
    .await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir().join("dist/custom-two"),
          dist_entry: None,
          env: Arc::new(Environment {
            context: EnvironmentContext::Browser,
            output_format: OutputFormat::EsModule,
            should_optimize: true,
            engines: Engines {
              browsers: Some(EnginesBrowsers::default()),
              ..Engines::default()
            },
            ..Environment::default()
          }),
          name: String::from("custom-two"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_custom_targets() {
    let targets = targets_from_config(
      String::from(
        r#"
        {
          "custom": "dist/custom.js",
          "targets": {
            "custom": {
              "context": "node",
              "includeNodeModules": true,
              "outputFormat": "commonjs"
            }
          }
        }
      "#,
      ),
      None,
      None,
    )
    .await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir().join("dist"),
          dist_entry: Some(PathBuf::from("custom.js")),
          env: Arc::new(Environment {
            context: EnvironmentContext::Node,
            include_node_modules: IncludeNodeModules::Bool(true),
            is_library: false,
            output_format: OutputFormat::CommonJS,
            should_optimize: true,
            ..Environment::default()
          }),
          name: String::from("custom"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_inferred_custom_browser_target() {
    let targets = targets_from_config(
      String::from(
        r#"
        {
          "custom": "dist/custom.js",
          "browserslist": ["chrome 20", "firefox > 1"],
          "targets": {
            "custom": {}
          }
        }
      "#,
      ),
      None,
      None,
    )
    .await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir().join("dist"),
          dist_entry: Some(PathBuf::from("custom.js")),
          env: Arc::new(Environment {
            context: EnvironmentContext::Browser,
            engines: Engines {
              browsers: Some(EnginesBrowsers::new(vec![
                String::from("chrome 20"),
                String::from("firefox > 1"),
              ])),
              ..Engines::default()
            },
            include_node_modules: IncludeNodeModules::Bool(true),
            output_format: OutputFormat::Global,
            should_optimize: true,
            ..Environment::default()
          }),
          name: String::from("custom"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_inferred_custom_browser_target_with_browserslistrc() {
    let targets = targets_from_config(
      String::from(
        r#"
        {
          "custom": "dist/custom.js",
          "targets": {
            "custom": {}
          }
        }
      "#,
      ),
      Some(String::from(
        r#"
          chrome 20
          firefox > 1
      "#,
      )),
      None,
    )
    .await;

    assert_eq!(
      targets.map_err(|e| e.to_string()),
      Ok(RequestResult::Target(TargetRequestOutput {
        entry: PathBuf::default(),
        targets: vec![Target {
          dist_dir: package_dir().join("dist"),
          dist_entry: Some(PathBuf::from("custom.js")),
          env: Arc::new(Environment {
            context: EnvironmentContext::Browser,
            engines: Engines {
              browsers: Some(EnginesBrowsers::new(vec![
                String::from("chrome 20"),
                String::from("firefox > 1"),
              ])),
              ..Engines::default()
            },
            include_node_modules: IncludeNodeModules::Bool(true),
            output_format: OutputFormat::Global,
            should_optimize: true,
            ..Environment::default()
          }),
          name: String::from("custom"),
          ..Target::default()
        }]
      }))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_inferred_custom_node_target() {
    let assert_targets = |targets: Result<RequestResult, anyhow::Error>, engines| {
      assert_eq!(
        targets.map_err(|e| e.to_string()),
        Ok(RequestResult::Target(TargetRequestOutput {
          entry: PathBuf::default(),
          targets: vec![Target {
            dist_dir: package_dir().join("dist"),
            dist_entry: Some(PathBuf::from("custom.js")),
            env: Arc::new(Environment {
              context: EnvironmentContext::Node,
              engines,
              include_node_modules: IncludeNodeModules::Bool(false),
              output_format: OutputFormat::CommonJS,
              should_optimize: true,
              ..Environment::default()
            }),
            name: String::from("custom"),
            ..Target::default()
          }]
        }))
      );
    };

    assert_targets(
      targets_from_config(
        String::from(
          r#"
          {
            "custom": "dist/custom.js",
            "engines": { "node": "^1.0.0" },
            "targets": { "custom": {} }
          }
        "#,
        ),
        None,
        None,
      )
      .await,
      Engines {
        node: Some(Version::new(NonZeroU16::new(1).unwrap(), 0)),
        ..Engines::default()
      },
    );

    assert_targets(
      targets_from_config(
        String::from(
          r#"
          {
            "custom": "dist/custom.js",
            "engines": { "node": "^1.0.0" },
            "browserslist": ["chrome 20"],
            "targets": { "custom": {} }
          }
        "#,
        ),
        None,
        None,
      )
      .await,
      Engines {
        node: Some(Version::new(NonZeroU16::new(1).unwrap(), 0)),
        browsers: Some(EnginesBrowsers::new(vec![String::from("chrome 20")])),
        ..Engines::default()
      },
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_custom_target_when_output_format_matches_inferred_output_format() {
    let assert_targets = move |ext, module_format: Option<ModuleFormat>, output_format| async move {
      let targets = targets_from_config(
        format!(
          r#"
          {{
            {}
            "custom": "dist/custom.{ext}",
            "targets": {{
              "custom": {{
                "outputFormat": "{output_format}"
              }}
            }}
          }}
        "#,
          module_format.map_or_else(String::default, |module_format| format!(
            r#""type": "{module_format}","#
          )),
        ),
        None,
        None,
      )
      .await;

      assert_eq!(
        targets.map_err(|e| e.to_string()),
        Ok(RequestResult::Target(TargetRequestOutput {
          entry: PathBuf::default(),
          targets: vec![Target {
            dist_dir: package_dir().join("dist"),
            dist_entry: Some(PathBuf::from(format!("custom.{ext}"))),
            env: Arc::new(Environment {
              context: EnvironmentContext::Browser,
              engines: Engines {
                browsers: Some(EnginesBrowsers::default()),
                ..Engines::default()
              },
              output_format,
              should_optimize: true,
              ..Environment::default()
            }),
            name: String::from("custom"),
            ..Target::default()
          }],
        }))
      );
    };

    assert_targets("cjs", None, OutputFormat::CommonJS).await;
    assert_targets("cjs", Some(ModuleFormat::CommonJS), OutputFormat::CommonJS).await;
    assert_targets("cjs", Some(ModuleFormat::Module), OutputFormat::CommonJS).await;

    assert_targets("js", None, OutputFormat::CommonJS).await;
    assert_targets("js", Some(ModuleFormat::CommonJS), OutputFormat::CommonJS).await;

    assert_targets("js", None, OutputFormat::EsModule).await;
    assert_targets("js", Some(ModuleFormat::Module), OutputFormat::EsModule).await;

    assert_targets("mjs", None, OutputFormat::EsModule).await;
    assert_targets("mjs", Some(ModuleFormat::CommonJS), OutputFormat::EsModule).await;
    assert_targets("mjs", Some(ModuleFormat::Module), OutputFormat::EsModule).await;
  }
}
