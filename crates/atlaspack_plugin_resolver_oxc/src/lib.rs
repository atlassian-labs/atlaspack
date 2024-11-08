mod built_in;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_core::plugin::*;
use atlaspack_core::types::IncludeNodeModules;
use normalize_path::NormalizePath;
use oxc_resolver::ResolveError as OxcResolveError;
use oxc_resolver::ResolveOptions as OxcResolveOptions;
use oxc_resolver::Resolver as OxcResolver;

pub struct AtlaspackResolverOxc {
  plugin_options: Arc<PluginOptions>,
  resolver: OxcResolver,
}

impl Debug for AtlaspackResolverOxc {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "AtlaspackResolverOxc")
  }
}

impl Hash for AtlaspackResolverOxc {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    env!("CARGO_PKG_VERSION").hash(state);
    self.plugin_options.mode.hash(state);
    self.plugin_options.project_root.hash(state);
  }
}

impl AtlaspackResolverOxc {
  pub fn new(ctx: &PluginContext) -> anyhow::Result<Self> {
    let oxc_options = OxcResolveOptions {
      alias_fields: vec![],
      alias: vec![],
      exports_fields: vec![vec!["exports".to_string()]],
      condition_names: vec![
        "import".to_string(),
        "require".to_string(),
        "module".to_string(),
        "webpack".to_string(),
        "development".to_string(),
        "browser".to_string(),
        "default".to_string(),
      ],
      extensions: vec![".tsx".into(), ".ts".into(), ".jsx".into(), ".js".into()],
      prefer_relative: true,
      ..OxcResolveOptions::default()
    };

    Ok(Self {
      resolver: OxcResolver::new(oxc_options),
      plugin_options: ctx.options.clone(),
    })
  }

  async fn resolve_builtin(&self, ctx: &ResolveContext, builtin: &str) -> anyhow::Result<Resolved> {
    let dep = &ctx.dependency;
    if dep.env.context.is_node() {
      return Ok(Resolved {
        invalidations: Vec::new(),
        resolution: Resolution::Excluded,
      });
    }

    if dep.env.is_library && should_include_node_module(&dep.env.include_node_modules, &builtin) {
      return Ok(Resolved {
        invalidations: Vec::new(),
        resolution: Resolution::Excluded,
      });
    }

    let browser_module = match builtin {
      "assert" => "assert/",
      "buffer" => "buffer/",
      "console" => "console-browserify",
      "constants" => "constants-browserify",
      "crypto" => "crypto-browserify",
      "domain" => "domain-browser",
      "events" => "events/",
      "http" => "stream-http",
      "https" => "https-browserify",
      "os" => "os-browserify",
      "path" => "path-browserify",
      "process" => "process/",
      "punycode" => "punycode/",
      "querystring" => "querystring-es3",
      "stream" => "stream-browserify",
      "string_decoder" => "string_decoder/",
      "sys" => "util/",
      "timers" => "timers-browserify",
      "tty" => "tty-browserify",
      "url" => "url/",
      "util" => "util/",
      "vm" => "vm-browserify",
      "zlib" => "browserify-zlib",
      _ => {
        return Ok(Resolved {
          invalidations: Vec::new(),
          resolution: Resolution::Resolved(self.resolve_empty(false)),
        });
      }
    };

    self
      .resolve(ResolveContext {
        // TODO: Can we get rid of the clones?
        dependency: Arc::clone(&ctx.dependency),
        pipeline: ctx.pipeline.clone(),
        specifier: browser_module.to_owned(),
      })
      .await
  }

  fn resolve_empty(&self, side_effects: bool) -> ResolvedResolution {
    ResolvedResolution {
      code: Some(String::default()),
      file_path: self.plugin_options.core_path.join("_empty.js"),
      side_effects,
      ..ResolvedResolution::default()
    }
  }
}

#[async_trait]
impl ResolverPlugin for AtlaspackResolverOxc {
  async fn resolve(&self, ctx: ResolveContext) -> anyhow::Result<Resolved> {
    if built_in::is_built_in(&ctx.specifier) {
      return self.resolve_builtin(&ctx, &ctx.specifier).await;
    }
    let mut resolve_from: PathBuf;
    if let Some(path) = ctx.dependency.source_path.as_ref() {
      resolve_from = path.clone();
    } else {
      resolve_from = self.plugin_options.project_root.clone();
    }
    if resolve_from.is_file() {
      resolve_from = resolve_from.parent().unwrap().to_path_buf();
    }

    let result = self.resolver.resolve(&resolve_from, &ctx.specifier);

    match result {
      Ok(result) => Ok(Resolved {
        invalidations: Vec::new(),
        resolution: Resolution::Resolved(ResolvedResolution {
          file_path: result.full_path().normalize(),
          can_defer: false,
          side_effects: true,
          code: None,
          meta: Default::default(),
          pipeline: Default::default(),
          priority: Default::default(),
          query: Default::default(),
        }),
      }),
      Err(OxcResolveError::NotFound(_)) => Ok(Resolved {
        invalidations: Vec::new(),
        resolution: Resolution::Unresolved,
      }),
      Err(err) => Err(anyhow::anyhow!(err)),
    }
  }
}

fn should_include_node_module(include_node_modules: &IncludeNodeModules, name: &str) -> bool {
  match include_node_modules {
    IncludeNodeModules::Bool(b) => *b,
    IncludeNodeModules::Array(arr) => {
      let Ok((module, _)) = parse_package_specifier(name) else {
        return true;
      };

      arr.iter().any(|m| m.as_str() == module)
    }
    IncludeNodeModules::Map(map) => {
      let Ok((module, _)) = parse_package_specifier(name) else {
        return true;
      };

      map.contains_key(module)
    }
  }
}

pub fn parse_package_specifier(specifier: &str) -> Result<(&str, &str), SpecifierError> {
  let idx = specifier.chars().position(|p| p == '/');
  if specifier.starts_with('@') {
    let idx = idx.ok_or(SpecifierError::InvalidPackageSpecifier)?;
    if let Some(next) = &specifier[idx + 1..].chars().position(|p| p == '/') {
      Ok((
        &specifier[0..idx + 1 + *next],
        &specifier[idx + *next + 2..],
      ))
    } else {
      Ok((specifier, ""))
    }
  } else if let Some(idx) = idx {
    Ok((&specifier[0..idx], &specifier[idx + 1..]))
  } else {
    Ok((specifier, ""))
  }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum SpecifierError {
  EmptySpecifier,
  InvalidPackageSpecifier,
  #[serde(serialize_with = "serialize_url_error")]
  UrlError(url::ParseError),
  InvalidFileUrl,
}

fn serialize_url_error<S>(value: &url::ParseError, serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  use serde::Serialize;
  value.to_string().serialize(serializer)
}
