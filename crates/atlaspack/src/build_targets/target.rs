use std::path::PathBuf;

use atlaspack_core::types::TargetDescriptor;
use atlaspack_core::types::*;

use super::builtin_target_descriptor::builtin_target_descriptor;
use super::merge_builtin_descriptors::merge_builtin_descriptors;
use super::package_json::BrowserField;
use super::package_json::BuiltInTargetDescriptor;

pub struct BuiltInTarget {
  pub descriptor: BuiltInTargetDescriptor,
  pub dist: Option<PathBuf>,
  pub name: String,
}

pub struct CustomTarget<'a> {
  pub descriptor: &'a TargetDescriptor,
  pub name: String,
}

pub fn builtin_browser_target(
  descriptor: Option<BuiltInTargetDescriptor>,
  dist: Option<BrowserField>,
  name: Option<String>,
) -> BuiltInTarget {
  BuiltInTarget {
    descriptor: descriptor
      .map(|d| merge_builtin_descriptors(d, builtin_target_descriptor(EnvironmentContext::Browser)))
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
    name: "browser".to_string(),
  }
}

pub fn builtin_main_target(
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
    name: "main".to_string(),
  }
}

pub fn builtin_module_target(
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
    name: "module".to_string(),
  }
}

pub fn builtin_types_target(
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
    name: "types".to_string(),
  }
}
