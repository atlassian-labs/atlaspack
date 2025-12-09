use swc_core::ecma::ast::Expr;

use crate::types::Metadata;
use crate::utils_create_result_pair::{ResultPair, create_result_pair};
use crate::utils_resolve_binding::{load_or_parse_module, resolve_binding};
use crate::utils_types::{BindingPathKind, EvaluateExpression, ImportBindingKind};

pub fn evaluate_namespace_import_path(
  expression: &Expr,
  meta: Metadata,
  path_name: &str,
  evaluate_expression: EvaluateExpression,
) -> Option<ResultPair> {
  let Expr::Ident(ident) = expression else {
    return None;
  };

  let binding = resolve_binding(ident.sym.as_ref(), meta.clone(), evaluate_expression)?;
  let Some(path) = binding.path.as_ref() else {
    return None;
  };

  let BindingPathKind::Import { source, kind } = &path.kind else {
    return None;
  };

  if !matches!(kind, ImportBindingKind::Namespace) {
    return None;
  }

  let cached = load_or_parse_module(&binding.meta, source)?;
  let module_meta = Metadata::new(cached.state);

  let export_binding = resolve_binding(path_name, module_meta.clone(), evaluate_expression)?;

  if let Some(node) = export_binding.node {
    let evaluated = (evaluate_expression)(&node, export_binding.meta.clone());
    return Some(create_result_pair(evaluated.value, evaluated.meta));
  }

  None
}
