use std::fmt;

use swc_core::common::{Span, Spanned};
use swc_core::ecma::ast::{Expr, Lit, Prop, PropName, PropOrSpread};

use crate::types::Metadata;

/// Object key used to denote extended selectors within cssMap variant definitions.
pub const EXTENDED_SELECTORS_KEY: &str = "selectors";

const AT_RULES: &[&str] = &[
  "@charset",
  "@counter-style",
  "@document",
  "@font-face",
  "@font-feature-values",
  "@font-palette-values",
  "@import",
  "@keyframes",
  "@layer",
  "@media",
  "@namespace",
  "@page",
  "@property",
  "@scope",
  "@scroll-timeline",
  "@starting-style",
  "@supports",
  "@viewport",
];

/// Enumerates the error messages surfaced by the cssMap helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorMessages {
  NoTaggedTemplate,
  NumberOfArgument,
  ArgumentType,
  AtRuleValueType,
  SelectorsBlockValueType,
  DefineMap,
  NoSpreadElement,
  NoObjectMethod,
  StaticVariantObject,
  DuplicateAtRule,
  DuplicateSelector,
  DuplicateSelectorsBlock,
  StaticPropertyKey,
  SelectorBlockWrongPlace,
  UseSelectorsWithAmpersand,
  UseVariantOfCssMap,
}

impl ErrorMessages {
  pub fn message(&self) -> &'static str {
    match self {
      ErrorMessages::NoTaggedTemplate => {
        "cssMap function cannot be used as a tagged template expression."
      }
      ErrorMessages::NumberOfArgument => "cssMap function can only receive one argument.",
      ErrorMessages::ArgumentType => "cssMap function can only receive an object.",
      ErrorMessages::AtRuleValueType => "Value of at-rule block must be an object.",
      ErrorMessages::SelectorsBlockValueType => "Value of `selectors` key must be an object.",
      ErrorMessages::DefineMap => "CSS Map must be declared at the top-most scope of the module.",
      ErrorMessages::NoSpreadElement => "Spread element is not supported in CSS Map.",
      ErrorMessages::NoObjectMethod => "Object method is not supported in CSS Map.",
      ErrorMessages::StaticVariantObject => "The variant object must be statically defined.",
      ErrorMessages::DuplicateAtRule => "Cannot declare an at-rule more than once in CSS Map.",
      ErrorMessages::DuplicateSelector => "Cannot declare a selector more than once in CSS Map.",
      ErrorMessages::DuplicateSelectorsBlock => {
        "Duplicate `selectors` key found in cssMap; expected either zero `selectors` keys or one."
      }
      ErrorMessages::StaticPropertyKey => "Property key may only be a static string.",
      ErrorMessages::SelectorBlockWrongPlace => "`selector` key was defined in the wrong place.",
      ErrorMessages::UseSelectorsWithAmpersand => {
        "This selector is applied to the parent element, and so you need to specify the ampersand symbol (&) directly before it. For example, `:hover` should be written as `&:hover`."
      }
      ErrorMessages::UseVariantOfCssMap => {
        "You must use the variant of a CSS Map object (eg. `styles.root`), not the root object itself, eg. `styles`."
      }
    }
  }
}

impl fmt::Display for ErrorMessages {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.message())
  }
}

/// Produces the multi-line error message used across cssMap helpers.
pub fn create_error_message(message: impl AsRef<str>) -> String {
  format!(
    "{}\n\nCheck out our documentation for cssMap examples: https://compiledcssinjs.com/docs/api-cssmap",
    message.as_ref()
  )
}

/// Determines whether the provided property key is a static literal value.
pub fn object_key_is_literal_value(key: &PropName) -> bool {
  match key {
    PropName::Ident(_) | PropName::Str(_) => true,
    PropName::Computed(comp) => {
      matches!(comp.expr.as_ref(), Expr::Ident(_) | Expr::Lit(Lit::Str(_)))
    }
    _ => false,
  }
}

/// Returns the string value of an identifier or string literal key.
pub fn get_key_value(key: &PropName) -> String {
  match key {
    PropName::Ident(ident) => ident.sym.as_ref().to_string(),
    PropName::Str(str) => str.value.as_ref().to_string(),
    PropName::Computed(comp) => match comp.expr.as_ref() {
      Expr::Ident(ident) => ident.sym.as_ref().to_string(),
      Expr::Lit(Lit::Str(str)) => str.value.as_ref().to_string(),
      _ => panic!("Expected an identifier or a string literal, got computed expression"),
    },
    _ => panic!(
      "Expected an identifier or a string literal, got type {}",
      match key {
        PropName::Num(_) => "NumericLiteral",
        PropName::Computed(_) => "Computed",
        PropName::BigInt(_) => "BigIntLiteral",
        PropName::Ident(_) | PropName::Str(_) => unreachable!("handled above"),
      }
    ),
  }
}

/// Determines whether the given key corresponds to a supported CSS at-rule.
pub fn is_at_rule_object(key: &PropName) -> bool {
  if !object_key_is_literal_value(key) {
    return false;
  }

  let value = get_key_value(key);
  AT_RULES.iter().any(|candidate| candidate == &value)
}

/// Determines if the provided selector targets the root element (i.e. begins with a pseudo).
pub fn is_plain_selector(selector: &str) -> bool {
  selector.starts_with(':')
}

/// Determines whether the provided property represents the `selectors` extended key.
pub fn has_extended_selectors_key(property: &PropOrSpread) -> bool {
  match property {
    PropOrSpread::Prop(prop) => match &**prop {
      Prop::KeyValue(key_value) => {
        object_key_is_literal_value(&key_value.key)
          && get_key_value(&key_value.key) == EXTENDED_SELECTORS_KEY
      }
      _ => false,
    },
    PropOrSpread::Spread(_) => false,
  }
}

/// Validates that an object property is a plain key-value pair without unsupported syntactic sugar.
pub fn error_if_not_valid_object_property(property: &PropOrSpread, meta: &Metadata) -> bool {
  match property {
    PropOrSpread::Prop(prop) => match &**prop {
      Prop::Method(_) | Prop::Getter(_) | Prop::Setter(_) => {
        report_css_map_error(meta, prop.span(), ErrorMessages::NoObjectMethod.message());
        return true;
      }
      _ => {}
    },
    PropOrSpread::Spread(spread) => {
      report_css_map_error(
        meta,
        spread.expr.span(),
        ErrorMessages::NoSpreadElement.message(),
      );
      return true;
    }
  }
  false
}

pub fn report_css_map_error(meta: &Metadata, span: Span, message: impl AsRef<str>) {
  let handler = meta.state().handler.clone();
  handler
    .struct_span_err(span, &create_error_message(message))
    .emit();
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{DUMMY_SP, SourceMap, SyntaxContext};
  use swc_core::ecma::ast::{
    Expr, Ident, KeyValueProp, Lit, Number, PropName, PropOrSpread, SpreadElement, Str,
  };

  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};

  fn create_metadata() -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm.clone(), Vec::new());
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));

    Metadata::new(state)
  }

  fn ident_key(name: &str) -> PropName {
    PropName::Ident(Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty()).into())
  }

  fn string_key(value: &str) -> PropName {
    PropName::Str(Str {
      span: DUMMY_SP,
      value: value.into(),
      raw: None,
    })
  }

  #[test]
  fn object_key_is_literal_value_matches_ident_and_string() {
    assert!(object_key_is_literal_value(&ident_key("key")));
    assert!(object_key_is_literal_value(&string_key("value")));
  }

  #[test]
  fn object_key_is_literal_value_rejects_computed() {
    use swc_core::ecma::ast::{ComputedPropName, Expr};

    let key = PropName::Computed(ComputedPropName {
      span: DUMMY_SP,
      expr: Box::new(Expr::Ident(Ident::new(
        "dynamic".into(),
        DUMMY_SP,
        SyntaxContext::empty(),
      ))),
    });

    assert!(!object_key_is_literal_value(&key));
  }

  #[test]
  fn get_key_value_returns_identifier_and_string_values() {
    assert_eq!(get_key_value(&ident_key("color")), "color");
    assert_eq!(get_key_value(&string_key("border")), "border");
  }

  #[test]
  #[should_panic(expected = "Expected an identifier or a string literal")]
  fn get_key_value_panics_on_non_literal_key() {
    use swc_core::ecma::ast::ComputedPropName;

    let key = PropName::Computed(ComputedPropName {
      span: DUMMY_SP,
      expr: Box::new(Expr::Lit(Lit::Num(Number {
        span: DUMMY_SP,
        value: 1.0,
        raw: None,
      }))),
    });

    get_key_value(&key);
  }

  #[test]
  fn is_at_rule_object_matches_known_rules() {
    assert!(is_at_rule_object(&string_key("@media")));
    assert!(!is_at_rule_object(&string_key(".selector")));
  }

  #[test]
  fn is_plain_selector_checks_prefix() {
    assert!(is_plain_selector(":hover"));
    assert!(!is_plain_selector("&:hover"));
  }

  #[test]
  fn has_extended_selectors_key_detects_literal_key() {
    let property = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: string_key(EXTENDED_SELECTORS_KEY),
      value: Box::new(Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: "".into(),
        raw: None,
      }))),
    })));

    assert!(has_extended_selectors_key(&property));
  }

  #[test]
  fn has_extended_selectors_key_returns_false_for_other_keys() {
    let property = PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: ident_key("color"),
      value: Box::new(Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: "red".into(),
        raw: None,
      }))),
    })));

    assert!(!has_extended_selectors_key(&property));
  }

  #[test]
  #[should_panic(expected = "Object method is not supported in CSS Map.")]
  fn error_if_not_valid_object_property_panics_on_methods() {
    use swc_core::ecma::ast::{BlockStmt, MethodProp};

    let method = Prop::Method(MethodProp {
      key: ident_key("color"),
      function: Box::new(swc_core::ecma::ast::Function {
        params: vec![],
        decorators: vec![],
        span: DUMMY_SP,
        body: Some(BlockStmt {
          span: DUMMY_SP,
          stmts: vec![],
          ctxt: Default::default(),
        }),
        is_generator: false,
        is_async: false,
        type_params: None,
        return_type: None,
        ctxt: Default::default(),
      }),
    });

    let property = PropOrSpread::Prop(Box::new(method));
    let meta = create_metadata();

    error_if_not_valid_object_property(&property, &meta);
  }

  #[test]
  #[should_panic(expected = "Spread element is not supported in CSS Map.")]
  fn error_if_not_valid_object_property_panics_on_spread() {
    let property = PropOrSpread::Spread(SpreadElement {
      dot3_token: DUMMY_SP,
      expr: Box::new(Expr::Ident(Ident::new(
        "other".into(),
        DUMMY_SP,
        SyntaxContext::empty(),
      ))),
    });

    let meta = create_metadata();

    error_if_not_valid_object_property(&property, &meta);
  }

  #[test]
  fn create_error_message_appends_docs_link() {
    let message = create_error_message("Some error");
    assert!(message.contains("Some error"));
    assert!(message.contains("compiledcssinjs.com/docs/api-cssmap"));
  }
}
