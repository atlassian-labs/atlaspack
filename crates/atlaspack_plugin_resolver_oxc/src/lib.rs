use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_core::plugin::*;
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
}

#[async_trait]
impl ResolverPlugin for AtlaspackResolverOxc {
  async fn resolve(&self, ctx: ResolveContext) -> anyhow::Result<Resolved> {
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
