use atlaspack_core::types::*;

pub fn builtin_target_descriptor(context: EnvironmentContext) -> TargetDescriptor {
  TargetDescriptor {
    context: Some(context),
    include_node_modules: Some(IncludeNodeModules::Bool(false)),
    is_library: Some(true),
    scope_hoist: Some(true),
    ..TargetDescriptor::default()
  }
}
