use atlaspack_contextual_imports::{
  ContextualImportsConfig, ContextualImportsInlineRequireVisitor,
};
use swc_core::ecma::visit::VisitMutWith;
use swc_core::ecma::{ast::Program, visit::as_folder};
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};

#[plugin_transform]
pub fn process_transform(
  mut program: Program,
  metadata: TransformPluginProgramMetadata,
) -> Program {
  let config = match metadata.get_transform_plugin_config() {
    Some(config_string) => serde_json::from_str::<ContextualImportsConfig>(&config_string)
      .expect("Invalid JSON configuration"),
    None => ContextualImportsConfig::default(),
  };

  program.visit_mut_with(&mut as_folder(ContextualImportsInlineRequireVisitor::new(
    metadata.unresolved_mark,
    config,
  )));

  program
}
