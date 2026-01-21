use swc_core::ecma::ast::{Expr, ObjectLit};

use crate::types::Metadata;
use crate::utils_create_result_pair::{ResultPair, create_result_pair};
use crate::utils_traversers_object::get_object_property_value;

pub fn evaluate_object_path(
  expression: &ObjectLit,
  meta: Metadata,
  property_name: &str,
) -> ResultPair {
  if let Some(result) = get_object_property_value(expression, property_name) {
    return create_result_pair(result.node, meta);
  }

  if std::env::var("COMPILED_TRACE_MEMBER_OBJECT").is_ok() {
    let keys: Vec<String> = expression
      .props
      .iter()
      .filter_map(|prop| match prop {
        swc_core::ecma::ast::PropOrSpread::Prop(p) => match p.as_ref() {
          swc_core::ecma::ast::Prop::KeyValue(kv) => Some(kv.key.clone()),
          swc_core::ecma::ast::Prop::Method(m) => Some(m.key.clone()),
          swc_core::ecma::ast::Prop::Getter(g) => Some(g.key.clone()),
          swc_core::ecma::ast::Prop::Setter(s) => Some(s.key.clone()),
          swc_core::ecma::ast::Prop::Shorthand(ident) => {
            Some(swc_core::ecma::ast::PropName::Ident(ident.clone().into()))
          }
          _ => None,
        },
        _ => None,
      })
      .map(|name| match name {
        swc_core::ecma::ast::PropName::Ident(id) => id.sym.to_string(),
        swc_core::ecma::ast::PropName::Str(s) => s.value.to_string(),
        swc_core::ecma::ast::PropName::Num(n) => n.value.to_string(),
        swc_core::ecma::ast::PropName::Computed(_) => "<computed>".to_string(),
        swc_core::ecma::ast::PropName::BigInt(bi) => bi.value.to_string(),
      })
      .collect();

    eprintln!(
      "[compiled][member-object] missing property '{}' on object literal with keys={:?}",
      property_name, keys
    );
  }

  create_result_pair(Expr::Object(expression.clone()), meta)
}
