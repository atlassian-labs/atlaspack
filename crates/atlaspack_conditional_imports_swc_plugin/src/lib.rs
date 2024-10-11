use atlaspack_conditional_imports::{ConditionalImportsConfig, ConditionalImportsVisitor};
use swc_core::ecma::visit::VisitMutWith;
use swc_core::ecma::{ast::Program, visit::as_folder};
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};

#[plugin_transform]
pub fn process_transform(
  mut program: Program,
  metadata: TransformPluginProgramMetadata,
) -> Program {
  let config = match metadata.get_transform_plugin_config() {
    Some(config_string) => serde_json::from_str::<ConditionalImportsConfig>(&config_string)
      .expect("Invalid JSON configuration"),
    None => ConditionalImportsConfig::default(),
  };

  program.visit_mut_with(&mut as_folder(ConditionalImportsVisitor::new(
    metadata.unresolved_mark,
    config,
  )));

  program
}
