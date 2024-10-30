use std::collections::BTreeMap;
use std::hash::Hash;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::config_loader::ConfigFile;
use atlaspack_core::config_loader::ConfigLoader;
use atlaspack_core::diagnostic_error;
use atlaspack_core::types::browsers::Browsers;
use atlaspack_core::types::engines::Engines;
use atlaspack_core::types::engines::EnginesBrowsers;
use atlaspack_core::types::BuildMode;
use atlaspack_core::types::CodeFrame;
use atlaspack_core::types::DefaultTargetOptions;
use atlaspack_core::types::Dependency;
use atlaspack_core::types::Diagnostic;
use atlaspack_core::types::DiagnosticBuilder;
use atlaspack_core::types::Environment;
use atlaspack_core::types::EnvironmentContext;
use atlaspack_core::types::ErrorKind;
use atlaspack_core::types::OutputFormat;
use atlaspack_core::types::SourceType;
use atlaspack_core::types::Target;
use atlaspack_core::types::TargetSourceMapOptions;
use atlaspack_resolver::IncludeNodeModules;
use pathdiff::diff_paths;

use super::super::entry::Entry;
use super::super::ActionQueue;
use super::package_json::BrowserField;
use super::package_json::BrowsersList;
use super::package_json::BuiltInTargetDescriptor;
use super::package_json::ModuleFormat;
use super::package_json::PackageJson;
use super::package_json::SourceField;
use super::package_json::SourceMapField;
use super::package_json::TargetDescriptor;
use crate::actions::path::PathAction;
use crate::actions::ActionType;
use crate::compilation::Compilation;
use crate::compilation::EnvMap;

/// Infers how and where source code is outputted
///
/// Targets will be generated from the project package.json file and input Atlaspack options.
///
#[derive(Debug, Hash)]
pub struct TargetAction {
  pub entry: Entry,
}

impl TargetAction {
  pub async fn run(
    self,
    q: ActionQueue,

    Compilation {
      env,
      mode,
      config_loader,
      default_target_options,
      asset_graph,
      project_root,
      ..
    }: &Compilation,
  ) -> anyhow::Result<()> {
    // TODO options.targets, should this still be supported?
    // TODO serve options
    let package_targets =
      self.resolve_package_targets(env, mode, config_loader, default_target_options)?;
    let targets = package_targets
      .into_iter()
      .filter_map(std::convert::identity)
      .collect::<Vec<Target>>();

    for target in targets {
      let entry = diff_paths(&self.entry.file_path, &project_root)
        .unwrap_or_else(|| self.entry.file_path.clone());

      let dependency = Dependency::entry(entry.to_str().unwrap().to_string(), target);

      let _dep_node = asset_graph
        .write()
        .await
        .add_entry_dependency(dependency.clone());

      // self
      //   .request_id_to_dep_node_index
      //   .insert(request.id(), dep_node);

      q.next(ActionType::Path(PathAction {
        dependency: Arc::new(dependency),
      }))?;
    }

    Ok(())
  }
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

impl TargetAction {
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
        let browsers = engines.browsers.clone().unwrap_or_default();
        engines.node.is_some() && Browsers::from(browsers).is_empty()
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
        .and_then(|format| match format {
          ModuleFormat::CommonJS => Some(OutputFormat::CommonJS),
          ModuleFormat::Module => Some(OutputFormat::EsModule),
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

  fn load_package_json(
    &self,
    env: &EnvMap,
    mode: &BuildMode,
    config: &ConfigLoader,
  ) -> Result<ConfigFile<PackageJson>, anyhow::Error> {
    // TODO Invalidations
    let mut package_json = match config.load_package_json::<PackageJson>() {
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

    if package_json.contents.engines.as_ref().is_some_and(|e| {
      let browsers = e.browsers.clone().unwrap_or_default();
      !Browsers::from(browsers).is_empty()
    }) {
      return Ok(package_json);
    }

    let env = {
      if let Some(value) = env.get("BROWSERSLIST_ENV") {
        value.clone()
      } else if let Some(value) = env.get("NODE_ENV") {
        value.clone()
      } else {
        mode.to_string()
      }
    };

    match package_json.contents.browserslist.clone() {
      // TODO Process browserslist config file
      None => {}
      Some(browserslist) => {
        let browserslist = match browserslist {
          BrowsersList::Browser(browser) => vec![browser],
          BrowsersList::Browsers(browsers) => browsers,
          BrowsersList::BrowsersByEnv(browsers_by_env) => browsers_by_env
            .get(&env)
            .map(|b| b.clone())
            .unwrap_or_default(),
        };

        package_json.contents.engines = Some(Engines {
          browsers: Some(EnginesBrowsers::new(browserslist)),
          ..match package_json.contents.engines {
            None => Engines::default(),
            Some(engines) => engines,
          }
        });
      }
    };

    Ok(package_json)
  }

  fn resolve_package_targets(
    &self,
    env: &EnvMap,
    mode: &BuildMode,
    config_loader: &ConfigLoader,
    default_target_options: &DefaultTargetOptions,
  ) -> Result<Vec<Option<Target>>, anyhow::Error> {
    let package_json = self.load_package_json(env, &mode, config_loader)?;
    let mut targets: Vec<Option<Target>> = Vec::new();

    let builtin_targets = [
      self.builtin_browser_target(
        package_json.contents.targets.browser.clone(),
        package_json.contents.browser.clone(),
        package_json.contents.name.clone(),
      ),
      self.builtin_main_target(
        package_json.contents.targets.main.clone(),
        package_json.contents.main.clone(),
      ),
      self.builtin_module_target(
        package_json.contents.targets.module.clone(),
        package_json.contents.module.clone(),
      ),
      self.builtin_types_target(
        package_json.contents.targets.types.clone(),
        package_json.contents.types.clone(),
      ),
    ];

    for builtin_target in builtin_targets {
      if builtin_target.dist.is_none() {
        continue;
      }

      match builtin_target.descriptor {
        BuiltInTargetDescriptor::Disabled(_disabled) => continue,
        BuiltInTargetDescriptor::TargetDescriptor(builtin_target_descriptor) => {
          targets.push(self.target_from_descriptor(
            default_target_options.clone(),
            builtin_target.dist,
            &package_json,
            builtin_target_descriptor,
            builtin_target.name,
          )?);
        }
      }
    }

    let custom_targets = package_json
      .contents
      .targets
      .custom_targets
      .iter()
      .map(|(name, descriptor)| CustomTarget { descriptor, name });

    for custom_target in custom_targets {
      let mut dist = None;
      if let Some(value) = package_json.contents.fields.get(custom_target.name) {
        match value {
          serde_json::Value::String(str) => {
            dist = Some(PathBuf::from(str));
          }
          _ => {
            return Err(diagnostic_error!(DiagnosticBuilder::default()
              .code_frames(vec![CodeFrame::from(&package_json)])
              .message(format!("Invalid path for target {}", custom_target.name))));
          }
        }
      }

      targets.push(self.target_from_descriptor(
        default_target_options.clone(),
        dist,
        &package_json,
        custom_target.descriptor.clone(),
        &custom_target.name,
      )?);
    }

    if targets.is_empty() {
      let context = self.infer_environment_context(&package_json.contents, None);

      let is_library = default_target_options.is_library.unwrap_or(false);
      let package_json_engines = package_json
        .contents
        .engines
        .unwrap_or_else(|| self.get_default_engines_for_context(default_target_options, context));

      tracing::debug!("Package JSON engines: {:?}", package_json_engines);

      targets.push(Some(Target {
        dist_dir: default_target_options
          .dist_dir
          .clone()
          .unwrap_or_else(|| default_dist_dir(&package_json.path)),
        dist_entry: None,
        env: Arc::new(Environment {
          context,
          engines: package_json_engines,
          include_node_modules: IncludeNodeModules::from(context),
          is_library,
          loc: None,
          output_format: default_target_options
            .output_format
            .unwrap_or_else(|| fallback_output_format(context)),
          should_optimize: default_target_options
            .should_optimize
            .unwrap_or_else(|| *mode == BuildMode::Production && !is_library),
          should_scope_hoist: default_target_options
            .should_scope_hoist
            .unwrap_or_else(|| *mode == BuildMode::Production && !is_library),
          source_map: default_target_options
            .source_maps
            .then(|| TargetSourceMapOptions::default()),
          source_type: SourceType::Module,
        }),
        loc: None,
        name: String::from("default"),
        public_url: default_target_options.public_url.clone(),
        ..Target::default()
      }));
    }

    Ok(targets)
  }

  fn get_default_engines_for_context(
    &self,
    default_target_options: &DefaultTargetOptions,
    context: EnvironmentContext,
  ) -> Engines {
    let defaults = default_target_options.engines.clone();
    if context.is_browser() {
      Engines {
        browsers: defaults.browsers,
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

  fn skip_target(
    &self,
    target_name: &str,
    source: &Option<SourceField>,
  ) -> bool {
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
    default_target_options: DefaultTargetOptions,
    dist: Option<PathBuf>,
    package_json: &ConfigFile<PackageJson>,
    target_descriptor: TargetDescriptor,
    target_name: &str,
  ) -> Result<Option<Target>, anyhow::Error> {
    if self.skip_target(&target_name, &target_descriptor.source) {
      return Ok(None);
    }

    let engines = target_descriptor
      .engines
      .clone()
      .or_else(|| package_json.contents.engines.clone())
      .unwrap_or_else(|| default_target_options.engines);

    // TODO LOC
    let context = target_descriptor.context.unwrap_or_else(|| {
      self.infer_environment_context(&package_json.contents, Some(engines.clone()))
    });

    let dist_entry = target_descriptor.dist_entry.clone().or_else(|| {
      dist
        .as_ref()
        .and_then(|d| d.file_name().map(|f| PathBuf::from(f)))
    });

    let inferred_output_format =
      self.infer_output_format(&dist_entry, &package_json, &target_descriptor)?;

    let output_format = target_descriptor
      .output_format
      .or(default_target_options.output_format)
      .or(inferred_output_format)
      .unwrap_or_else(|| match target_name {
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
      .unwrap_or_else(|| default_target_options.is_library.unwrap_or(false));

    let target_descriptor_engines = target_descriptor.engines.clone();

    tracing::debug!("Target descriptor engines: {:?}", target_descriptor_engines);

    Ok(Some(Target {
      dist_dir: match dist.as_ref() {
        None => default_target_options
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
        should_optimize: default_target_options.should_optimize.map_or_else(
          || target_descriptor.optimize.unwrap_or(!is_library),
          |o| o && target_descriptor.optimize.unwrap_or(!is_library),
        ),
        should_scope_hoist: (is_library
          || default_target_options.should_scope_hoist.unwrap_or(false))
          && (target_descriptor.scope_hoist.is_none()
            || target_descriptor.scope_hoist.is_some_and(|s| s != false)),
        source_map: match default_target_options.source_maps {
          false => None,
          true => match target_descriptor.source_map.as_ref() {
            None => Some(TargetSourceMapOptions::default()),
            Some(SourceMapField::Bool(source_maps)) => {
              source_maps.then(|| TargetSourceMapOptions::default())
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
        .unwrap_or(default_target_options.public_url.clone()),
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
  package_path
    .parent()
    .unwrap_or_else(|| &package_path)
    .join("dist")
}

fn fallback_output_format(context: EnvironmentContext) -> OutputFormat {
  match context {
    EnvironmentContext::Node => OutputFormat::CommonJS,
    EnvironmentContext::ElectronMain => OutputFormat::CommonJS,
    EnvironmentContext::ElectronRenderer => OutputFormat::CommonJS,
    _ => OutputFormat::Global,
  }
}
