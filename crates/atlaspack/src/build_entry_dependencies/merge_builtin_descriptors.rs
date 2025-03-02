use atlaspack_core::types::*;

use super::package_json::BuiltInTargetDescriptor;

pub fn merge_builtin_descriptors(
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
