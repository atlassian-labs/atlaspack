use std::path::PathBuf;
use std::sync::Arc;

use atlaspack_core::diagnostic_error;
use atlaspack_core::plugin::Resolution;
use atlaspack_core::plugin::ResolveContext;
use atlaspack_core::plugin::ResolvedResolution;
use atlaspack_core::types::Dependency;
use atlaspack_resolver::parse_scheme;

use crate::plugins::PluginsRef;

pub enum ResolutionResult {
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

pub async fn resolve(
  dependency: Arc<Dependency>,
  plugins: PluginsRef,
) -> anyhow::Result<ResolutionResult> {
  let (parsed_pipeline, specifier) = parse_scheme(&dependency.specifier)
    .and_then(|(pipeline, specifier)| {
      if plugins
        .named_pipelines()
        .contains(&String::from(pipeline.as_ref()))
      {
        Ok((Some(pipeline.to_string()), specifier))
      } else {
        Err(())
      }
    })
    .unwrap_or((None, dependency.specifier.as_ref()));

  for resolver in plugins.resolvers()?.iter() {
    let result = resolver
      .resolve(ResolveContext {
        dependency: Arc::clone(&dependency),
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

    match resolved.resolution {
      Resolution::Unresolved => continue,
      Resolution::Excluded => return Ok(ResolutionResult::Excluded),
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

        return Ok(ResolutionResult::Resolved {
          can_defer,
          code,
          path: file_path,
          pipeline: pipeline.or(parsed_pipeline).or(dependency.pipeline.clone()),
          query,
          side_effects,
        });
      }
    };
  }

  if dependency.is_optional {
    return Ok(ResolutionResult::Excluded);
  }

  let resolve_from = dependency
    .resolve_from
    .as_ref()
    .or(dependency.source_path.as_ref());

  match resolve_from {
    None => Err(diagnostic_error!(
      "Failed to resolve {}",
      dependency.specifier
    )),
    Some(from) => Err(diagnostic_error!(
      "Failed to resolve {} from {}",
      dependency.specifier,
      from.display()
    )),
  }
}
