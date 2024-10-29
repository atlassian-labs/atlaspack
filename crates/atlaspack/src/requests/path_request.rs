use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use atlaspack_core::diagnostic_error;
use atlaspack_core::plugin::BuildProgressEvent;
use atlaspack_core::plugin::ReporterEvent;
use atlaspack_core::plugin::Resolution;
use atlaspack_core::plugin::ResolveContext;
use atlaspack_core::plugin::ResolvedResolution;
use atlaspack_core::plugin::ResolvingEvent;
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
  async fn run(
    &self,
    request_context: RunRequestContext,
  ) -> Result<ResultAndInvalidations, RunRequestError> {
    request_context.report(ReporterEvent::BuildProgress(BuildProgressEvent::Resolving(
      ResolvingEvent {
        dependency: Arc::clone(&self.dependency),
      },
    )));

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
      let result = resolver.resolve(ResolveContext {
        dependency: Arc::clone(&self.dependency),
        pipeline: parsed_pipeline.clone(),
        specifier: String::from(specifier),
      });

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
          })
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
    composite_reporter_plugin::CompositeReporterPlugin, Resolved, ResolverPlugin,
  };

  use crate::{
    plugins::{MockPlugins, PluginsRef},
    test_utils::{request_tracker, RequestTrackerTestOptions},
  };

  use super::*;

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

    assert_eq!(
      resolution.map_err(|e| e.to_string()),
      Ok(RequestResult::Path(PathRequestOutput::Excluded))
    );
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

    assert_eq!(
      resolution.map_err(|e| e.to_string()),
      Ok(RequestResult::Path(PathRequestOutput::Resolved {
        can_defer: false,
        code: None,
        path,
        pipeline: None,
        query: None,
        side_effects: false
      }))
    );
  }

  mod when_all_resolvers_return_unresolved {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_an_excluded_resolution_when_the_dependency_is_optional() {
      let request = PathRequest {
        dependency: Arc::new(Dependency {
          is_optional: true,
          specifier: String::from("a.js"),
          ..Default::default()
        }),
      };

      let resolution = request_tracker(RequestTrackerTestOptions {
        plugins: test_plugins!(vec![Arc::new(UnresolvedResolverPlugin {})]),
        ..RequestTrackerTestOptions::default()
      })
      .run_request(request)
      .await;

      assert_eq!(
        resolution.map_err(|e| e.to_string()),
        Ok(RequestResult::Path(PathRequestOutput::Excluded))
      );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_an_error_when_the_dependency_is_required() {
      let assert_error = move |dependency: Dependency, error: &'static str| async move {
        let request = PathRequest {
          dependency: Arc::new(Dependency {
            is_optional: false,
            ..dependency
          }),
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
        Dependency {
          specifier: String::from("a.js"),
          ..Dependency::default()
        },
        "Failed to resolve a.js",
      )
      .await;

      assert_error(
        Dependency {
          resolve_from: Some(PathBuf::from("rf.js")),
          specifier: String::from("a.js"),
          ..Dependency::default()
        },
        "Failed to resolve a.js from rf.js",
      )
      .await;

      assert_error(
        Dependency {
          source_path: Some(PathBuf::from("sp.js")),
          specifier: String::from("a.js"),
          ..Dependency::default()
        },
        "Failed to resolve a.js from sp.js",
      )
      .await;

      assert_error(
        Dependency {
          resolve_from: Some(PathBuf::from("rf.js")),
          source_path: Some(PathBuf::from("sp.js")),
          specifier: String::from("a.js"),
          ..Dependency::default()
        },
        "Failed to resolve a.js from rf.js",
      )
      .await;
    }
  }
}
