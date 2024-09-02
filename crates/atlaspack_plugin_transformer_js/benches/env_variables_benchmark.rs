use std::collections::HashMap;

use atlaspack_plugin_transformer_js::js_transformer_config::InlineEnvironment;
use criterion::{criterion_group, criterion_main, Criterion};
use glob_match::glob_match;
use swc_core::atoms::Atom;

fn default_env_vars(
  env_vars: &HashMap<String, String>,
  inline_environment: &InlineEnvironment,
) -> HashMap<Atom, Atom> {
  if env_vars.is_empty() {
    return HashMap::new();
  }

  match inline_environment {
    InlineEnvironment::Enabled(enabled) => match enabled {
      false => todo!(),
      true => env_vars
        .iter()
        .map(|(key, value)| (key.as_str().into(), value.as_str().into()))
        .collect::<HashMap<Atom, Atom>>(),
    },
    InlineEnvironment::Environments(environments) => {
      let mut vars: HashMap<Atom, Atom> = HashMap::new();
      for env_glob in environments {
        for (env_var, value) in env_vars
          .iter()
          .filter(|(key, _value)| glob_match(&env_glob, key))
        {
          vars.insert(env_var.as_str().into(), value.as_str().into());
        }
      }

      vars
    }
  }
}

#[derive(Clone, Default)]
struct EnvVariablesCache {
  enabled: Option<HashMap<Atom, Atom>>,
  environments: Option<HashMap<Atom, Atom>>,
}

#[derive(Clone)]
struct CachedEnvVariables<'a> {
  cache: EnvVariablesCache,
  env_vars: &'a HashMap<String, String>,
  inline_environment: &'a InlineEnvironment,
}

impl<'a> CachedEnvVariables<'a> {
  pub fn new(
    env_vars: &'a HashMap<String, String>,
    inline_environment: &'a InlineEnvironment,
  ) -> Self {
    Self {
      cache: EnvVariablesCache::default(),
      env_vars,
      inline_environment,
    }
  }

  pub fn environment_variables(&mut self) -> HashMap<Atom, Atom> {
    if self.env_vars.is_empty() {
      return HashMap::new();
    }

    match self.inline_environment {
      InlineEnvironment::Enabled(enabled) => match enabled {
        false => todo!(),
        true => {
          if let Some(vars) = self.cache.enabled.as_ref() {
            return vars.clone();
          }

          let vars = self
            .env_vars
            .iter()
            .map(|(key, value)| (key.as_str().into(), value.as_str().into()))
            .collect::<HashMap<Atom, Atom>>();

          self.cache.enabled = Some(vars.clone());

          vars
        }
      },
      InlineEnvironment::Environments(environments) => {
        if let Some(vars) = self.cache.environments.as_ref() {
          return vars.clone();
        }

        let mut vars: HashMap<Atom, Atom> = HashMap::new();
        for env_glob in environments {
          for (env_var, value) in self
            .env_vars
            .iter()
            .filter(|(key, _value)| glob_match(&env_glob, key))
          {
            vars.insert(env_var.as_str().into(), value.as_str().into());
          }
        }

        self.cache.environments = Some(vars.clone());

        vars
      }
    }
  }
}

fn create_env_vars() -> HashMap<String, String> {
  let mut env_vars: HashMap<String, String> = HashMap::new();

  env_vars.insert("NODE_ENV".into(), "test".into());
  for i in 0..100 {
    env_vars.insert(format!("ENV_{i}"), format!("A_LONG_LONG_LONG_VALUE_{i}"));
  }

  env_vars
}

fn create_environment_allowlist() -> InlineEnvironment {
  let mut environments = Vec::new();

  for i in 0..100 {
    environments.push(format!("ENV_{i}*"));
  }

  InlineEnvironment::Environments(environments)
}

fn env_variables_enabled_benchmark(c: &mut Criterion) {
  let env_vars = create_env_vars();
  let inline_environment = InlineEnvironment::Enabled(true);
  let mut cached = CachedEnvVariables::new(&env_vars, &inline_environment);

  c.bench_function("default", |b| {
    b.iter(|| default_env_vars(&env_vars, &inline_environment))
  });

  c.bench_function("cached", |b| b.iter(|| cached.environment_variables()));
}

fn env_variables_environments_benchmark(c: &mut Criterion) {
  let env_vars = create_env_vars();
  let inline_environment = create_environment_allowlist();
  let mut cached = CachedEnvVariables::new(&env_vars, &inline_environment);

  c.bench_function("default", |b| {
    b.iter(|| default_env_vars(&env_vars, &inline_environment))
  });

  c.bench_function("cached", |b| b.iter(|| cached.environment_variables()));
}

criterion_group!(
  benches,
  env_variables_enabled_benchmark,
  env_variables_environments_benchmark
);

criterion_main!(benches);
