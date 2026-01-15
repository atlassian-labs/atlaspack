use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_core::diagnostic_error;
use atlaspack_core::plugin::Resolution;
use atlaspack_core::plugin::ResolveContext;
use atlaspack_core::plugin::ResolvedResolution;
use atlaspack_core::types::Dependency;
use atlaspack_resolver::parse_scheme;

use crate::request_tracker::Request;
use crate::request_tracker::ResultAndInvalidations;
use crate::request_tracker::RunRequestContext;
use crate::request_tracker::RunRequestError;

use super::RequestResult;

#[derive(Hash, Debug)]
pub struct PathRequest {
  pub dependency: Arc<Dependency>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathRequestOutput {
  Excluded,
  Resolved {
    can_defer: bool,
    path: PathBuf,
    code: Option<String>,
    pipeline: Option<String>,
    query: Option<String>,
    side_effects: bool,
  },
}

// TODO tracing, dev deps
#[async_trait]
impl Request for PathRequest {
  #[tracing::instrument(level = "trace", skip_all)]
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    let (parsed_pipeline, specifier) = parse_scheme(&self.dependency.specifier)
      .and_then(|(pipeline, specifier)| {
        if request_context
          .plugins()
          .named_pipelines()
          .contains(&String::from(pipeline.as_ref()))
        {
          Ok((Some(pipeline.to_string()), specifier))
        } else {
          Err(())
        }
      })
      .unwrap_or((None, self.dependency.specifier.as_ref()));

    let mut invalidations = Vec::new();

    for resolver in request_context.plugins().resolvers()?.iter() {
      let result = resolver
        .resolve(ResolveContext {
          dependency: Arc::clone(&self.dependency),
          pipeline: parsed_pipeline.clone(),
          specifier: String::from(specifier),
        })
        .await;

      let resolved = match result {
        Ok(result) => result,
        Err(error) => {
          // TODO: Forward the resolver diagnostic to the result
          tracing::info!("Resolver {:?} failed.\n{}", resolver, error);
          // Resolve failed so track the error and try the next resolver
          continue;
        }
      };

      invalidations.extend(resolved.invalidations);

      match resolved.resolution {
        Resolution::Unresolved => continue,
        Resolution::Excluded => {
          return Ok(ResultAndInvalidations {
            invalidations: Vec::new(),
            result: RequestResult::Path(PathRequestOutput::Excluded),
          });
        }
        Resolution::Resolved(ResolvedResolution {
          can_defer,
          code,
          file_path,
          meta: _meta,
          pipeline,
          priority: _priority,
          query,
          side_effects,
        }) => {
          if !file_path.is_absolute() {
            return Err(diagnostic_error!(
              "{:?} must return an absolute path, but got {}",
              resolver,
              file_path.display()
            ));
          }

          // TODO resolution.diagnostics
          // TODO Set dependency meta and priority

          return Ok(ResultAndInvalidations {
            invalidations,
            result: RequestResult::Path(PathRequestOutput::Resolved {
              can_defer,
              code,
              path: file_path,
              pipeline: pipeline
                .or(parsed_pipeline)
                .or(self.dependency.pipeline.clone()),
              query,
              side_effects,
            }),
          });
        }
      };
    }

    if self.dependency.is_optional {
      return Ok(ResultAndInvalidations {
        invalidations,
        result: RequestResult::Path(PathRequestOutput::Excluded),
      });
    }

    let resolve_from = self
      .dependency
      .resolve_from
      .as_ref()
      .or(self.dependency.source_path.as_ref());

    match resolve_from {
      None => Err(diagnostic_error!(
        "Failed to resolve {}",
        self.dependency.specifier
      )),
      Some(from) => Err(diagnostic_error!(
        "Failed to resolve {} from {}",
        self.dependency.specifier,
        from.display()
      )),
    }
  }
}

#[cfg(test)]
mod tests {
  use std::fmt::Debug;

  use async_trait::async_trait;
  use atlaspack_core::plugin::{
    Resolved, ResolverPlugin, composite_reporter_plugin::CompositeReporterPlugin,
  };

  use crate::{
    plugins::{MockPlugins, PluginsRef},
    test_utils::{RequestTrackerTestOptions, request_tracker},
  };

  use super::*;

  fn assert_path_result(
    actual: Result<Arc<RequestResult>, anyhow::Error>,
    expected: PathRequestOutput,
  ) {
    let Ok(result) = actual else {
      panic!("Expected Ok, got Err: {:?}", actual.err());
    };

    assert_eq!(result, Arc::new(RequestResult::Path(expected)));
  }

  macro_rules! test_plugins {
    ($resolvers:expr) => {{
      let mut plugins = MockPlugins::new();

      plugins.expect_named_pipelines().returning(|| Vec::new());

      plugins
        .expect_reporter()
        .returning(|| Arc::new(CompositeReporterPlugin::default()));

      plugins.expect_resolvers().returning(move || Ok($resolvers));

      let plugins: PluginsRef = Arc::new(plugins);

      Some(plugins)
    }};
  }

  #[derive(Debug, Hash)]
  struct ExcludedResolverPlugin {}

  #[async_trait]
  impl ResolverPlugin for ExcludedResolverPlugin {
    async fn resolve(&self, _ctx: ResolveContext) -> Result<Resolved, anyhow::Error> {
      Ok(Resolved {
        invalidations: Vec::new(),
        resolution: Resolution::Excluded,
      })
    }
  }

  #[derive(Hash)]
  struct ResolvedResolverPlugin {
    resolution: ResolvedResolution,
  }

  impl Debug for ResolvedResolverPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "ResolvedResolverPlugin")
    }
  }

  #[async_trait]
  impl ResolverPlugin for ResolvedResolverPlugin {
    async fn resolve(&self, _ctx: ResolveContext) -> Result<Resolved, anyhow::Error> {
      Ok(Resolved {
        invalidations: Vec::new(),
        resolution: Resolution::Resolved(self.resolution.clone()),
      })
    }
  }

  #[derive(Debug, Hash)]
  struct UnresolvedResolverPlugin {}

  #[async_trait]
  impl ResolverPlugin for UnresolvedResolverPlugin {
    async fn resolve(&self, _ctx: ResolveContext) -> Result<Resolved, anyhow::Error> {
      Ok(Resolved {
        invalidations: Vec::new(),
        resolution: Resolution::Unresolved,
      })
    }
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_excluded_resolution() {
    let request = PathRequest {
      dependency: Arc::new(Dependency::default()),
    };

    let resolution = request_tracker(RequestTrackerTestOptions {
      plugins: test_plugins!(vec![Arc::new(ExcludedResolverPlugin {})]),
      ..RequestTrackerTestOptions::default()
    })
    .run_request(request)
    .await;

    assert_path_result(resolution, PathRequestOutput::Excluded);
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_an_error_when_resolved_file_path_is_not_absolute() {
    let request = PathRequest {
      dependency: Arc::new(Dependency::default()),
    };

    let resolution = request_tracker(RequestTrackerTestOptions {
      plugins: test_plugins!(vec![Arc::new(ResolvedResolverPlugin {
        resolution: ResolvedResolution {
          file_path: PathBuf::from("./"),
          ..ResolvedResolution::default()
        },
      })]),
      ..RequestTrackerTestOptions::default()
    })
    .run_request(request)
    .await;

    assert_eq!(
      resolution.map_err(|e| e.to_string()),
      Err(String::from(
        "ResolvedResolverPlugin must return an absolute path, but got ./"
      ))
    );
  }

  #[tokio::test(flavor = "multi_thread")]
  async fn returns_the_first_resolved_resolution() {
    #[cfg(not(target_os = "windows"))]
    let root = PathBuf::from(std::path::MAIN_SEPARATOR_STR);

    #[cfg(target_os = "windows")]
    let root = PathBuf::from("c:\\windows");

    let request = PathRequest {
      dependency: Arc::new(Dependency::default()),
    };

    let path = root.join("a.js");

    let resolution = request_tracker(RequestTrackerTestOptions {
      plugins: test_plugins!(vec![
        Arc::new(UnresolvedResolverPlugin {}),
        Arc::new(ResolvedResolverPlugin {
          resolution: ResolvedResolution {
            file_path: root.join("a.js"),
            ..ResolvedResolution::default()
          },
        }),
        Arc::new(ResolvedResolverPlugin {
          resolution: ResolvedResolution {
            file_path: root.join("b.js"),
            ..ResolvedResolution::default()
          },
        }),
      ]),
      ..RequestTrackerTestOptions::default()
    })
    .run_request(request)
    .await;

    assert_path_result(
      resolution,
      PathRequestOutput::Resolved {
        can_defer: false,
        code: None,
        path,
        pipeline: None,
        query: None,
        side_effects: false,
      },
    );
  }

  mod when_all_resolvers_return_unresolved {
    use atlaspack_core::types::{DependencyBuilder, Environment, Priority, SpecifierType};

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_an_excluded_resolution_when_the_dependency_is_optional() {
      let request = PathRequest {
        dependency: Arc::new(
          DependencyBuilder::default()
            .is_optional(true)
            .specifier("a.js".to_string())
            .env(Arc::new(Environment::default()))
            .specifier_type(SpecifierType::default())
            .priority(Priority::default())
            .build(),
        ),
      };

      let resolution = request_tracker(RequestTrackerTestOptions {
        plugins: test_plugins!(vec![Arc::new(UnresolvedResolverPlugin {})]),
        ..RequestTrackerTestOptions::default()
      })
      .run_request(request)
      .await;

      assert_path_result(resolution, PathRequestOutput::Excluded);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_an_error_when_the_dependency_is_required() {
      let assert_error = move |dependency: Dependency, error: &'static str| async move {
        let mut dep = dependency.clone();
        dep.is_optional = false;

        let request = PathRequest {
          dependency: Arc::new(dep),
        };

        let resolution = request_tracker(RequestTrackerTestOptions {
          plugins: test_plugins!(vec![Arc::new(UnresolvedResolverPlugin {})]),
          ..RequestTrackerTestOptions::default()
        })
        .run_request(request)
        .await;

        assert_eq!(
          resolution.map_err(|e| e.to_string()),
          Err(String::from(error))
        );
      };

      assert_error(
        DependencyBuilder::default()
          .specifier("a.js".to_string())
          .env(Arc::new(Environment::default()))
          .specifier_type(SpecifierType::default())
          .priority(Priority::default())
          .build(),
        "Failed to resolve a.js",
      )
      .await;

      assert_error(
        DependencyBuilder::default()
          .resolve_from(PathBuf::from("rf.js"))
          .specifier("a.js".to_string())
          .env(Arc::new(Environment::default()))
          .specifier_type(SpecifierType::default())
          .priority(Priority::default())
          .build(),
        "Failed to resolve a.js from rf.js",
      )
      .await;

      assert_error(
        DependencyBuilder::default()
          .source_path(PathBuf::from("sp.js"))
          .specifier("a.js".to_string())
          .env(Arc::new(Environment::default()))
          .specifier_type(SpecifierType::default())
          .priority(Priority::default())
          .build(),
        "Failed to resolve a.js from sp.js",
      )
      .await;

      assert_error(
        DependencyBuilder::default()
          .resolve_from(PathBuf::from("rf.js"))
          .source_path(PathBuf::from("sp.js"))
          .specifier("a.js".to_string())
          .env(Arc::new(Environment::default()))
          .specifier_type(SpecifierType::default())
          .priority(Priority::default())
          .build(),
        "Failed to resolve a.js from rf.js",
      )
      .await;
    }
  }
}
