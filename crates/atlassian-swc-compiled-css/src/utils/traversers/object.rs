use crate::utils_traversers_types::TraverserResult;

use swc_core::common::Spanned;
use swc_core::ecma::ast::{Expr, ObjectLit, Prop, PropName, PropOrSpread};

/// Locate a property on an object literal and return the expression assigned to
/// it along with the span of the value node. Mirrors the behaviour of the Babel
/// helper by returning the first matching property and ignoring spreads.
pub fn get_object_property_value(
  object: &ObjectLit,
  property_name: &str,
) -> Option<TraverserResult<Expr>> {
  for prop in &object.props {
    let PropOrSpread::Prop(prop) = prop else {
      continue;
    };

    match prop.as_ref() {
      Prop::KeyValue(key_value) => {
        let PropName::Ident(ident) = &key_value.key else {
          continue;
        };

        if ident.sym.as_ref() == property_name {
          let value = (*key_value.value).clone();
          return Some(TraverserResult {
            span: value.span(),
            node: value,
          });
        }
      }
      Prop::Shorthand(ident) => {
        if ident.sym.as_ref() == property_name {
          return Some(TraverserResult {
            span: ident.span,
            node: Expr::Ident(ident.clone()),
          });
        }
      }
      Prop::Assign(assign) => {
        if assign.key.sym.as_ref() == property_name {
          let value = (*assign.value).clone();
          return Some(TraverserResult {
            span: assign.span,
            node: value,
          });
        }
      }
      _ => {}
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use swc_core::common::{Spanned, DUMMY_SP};
  use swc_core::ecma::ast::{
    Expr, IdentName, KeyValueProp, Lit, ObjectLit, Prop, PropName, PropOrSpread, Str,
  };

  use super::get_object_property_value;

  fn object_with_property(name: &str, value: Expr) -> ObjectLit {
    ObjectLit {
      span: DUMMY_SP,
      props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(IdentName::new(name.into(), DUMMY_SP)),
        value: Box::new(value),
      })))],
    }
  }

  #[test]
  fn returns_value_for_matching_property() {
    let value = Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: "blue".into(),
      raw: None,
    }));
    let object = object_with_property("primary", value.clone());

    let result =
      get_object_property_value(&object, "primary").expect("property should be resolved");

    assert_eq!(result.node, value);
    assert_eq!(result.span, value.span());
  }

  #[test]
  fn returns_none_when_property_missing() {
    let object = object_with_property(
      "primary",
      Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: "blue".into(),
        raw: None,
      })),
    );

    assert!(get_object_property_value(&object, "secondary").is_none());
  }
}
