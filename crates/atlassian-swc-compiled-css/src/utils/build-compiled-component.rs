use indexmap::IndexSet;
use swc_core::atoms::Atom;
use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::{
  ArrayLit, CallExpr, Callee, Expr, ExprOrSpread, Ident, IdentName, JSXAttr, JSXAttrName,
  JSXAttrOrSpread, JSXAttrValue, JSXClosingElement, JSXElement, JSXElementChild, JSXElementName,
  JSXExpr, JSXExprContainer, JSXOpeningElement, JSXText, ObjectLit, PropOrSpread, SpreadElement,
};

use crate::types::Metadata;
use crate::utils_build_css_variables::build_css_variables;
use crate::utils_get_jsx_attribute::get_jsx_attribute;
use crate::utils_get_runtime_class_name_library::get_runtime_class_name_library;
use crate::utils_hoist_sheet::hoist_sheet;
use crate::utils_transform_css_items::transform_css_items;
use crate::utils_types::{CssOutput, Variable};

fn ident(name: &str) -> Ident {
  Ident::new(Atom::from(name), DUMMY_SP, SyntaxContext::empty())
}

fn jsx_name(name: &str) -> JSXElementName {
  JSXElementName::Ident(ident(name))
}

fn jsx_text(value: &str) -> JSXElementChild {
  let atom: Atom = value.into();
  JSXElementChild::JSXText(JSXText {
    span: DUMMY_SP,
    value: atom.clone(),
    raw: atom,
  })
}

fn array_expression(values: Vec<Expr>) -> Expr {
  Expr::Array(ArrayLit {
    span: DUMMY_SP,
    elems: values
      .into_iter()
      .map(|expr| {
        Some(ExprOrSpread {
          spread: None,
          expr: Box::new(expr),
        })
      })
      .collect(),
  })
}

fn runtime_call(helper: &str, values: Vec<Expr>) -> Expr {
  Expr::Call(CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Ident(ident(helper)))),
    args: vec![ExprOrSpread {
      spread: None,
      expr: Box::new(array_expression(values)),
    }],
    type_args: None,
  })
}

fn jsx_attribute(name: &str, value: JSXAttrValue) -> JSXAttrOrSpread {
  JSXAttrOrSpread::JSXAttr(JSXAttr {
    span: DUMMY_SP,
    name: JSXAttrName::Ident(IdentName::new(Atom::from(name), DUMMY_SP)),
    value: Some(value),
  })
}

fn attribute_value_to_expr(value: &JSXAttrValue) -> Expr {
  match value {
    JSXAttrValue::Lit(lit) => Expr::Lit(lit.clone()),
    JSXAttrValue::JSXExprContainer(container) => match &container.expr {
      JSXExpr::Expr(expr) => *expr.clone(),
      JSXExpr::JSXEmptyExpr(_) => panic!("Empty expression not supported."),
    },
    JSXAttrValue::JSXElement(element) => Expr::JSXElement(element.clone()),
    JSXAttrValue::JSXFragment(fragment) => Expr::JSXFragment(fragment.clone()),
  }
}

fn build_nonce_attribute(nonce: &str) -> JSXAttrOrSpread {
  let value = JSXAttrValue::JSXExprContainer(JSXExprContainer {
    span: DUMMY_SP,
    expr: JSXExpr::Expr(Box::new(Expr::Ident(ident(nonce)))),
  });

  jsx_attribute("nonce", value)
}

fn collect_sheet_idents(sheets: &[String], meta: &Metadata) -> Vec<Expr> {
  let mut unique = IndexSet::new();
  let mut idents = Vec::new();

  for sheet in sheets {
    // COMPAT(AFB-1871): Skip empty/invalid sheets that can arise from skeletal
    // CSS (e.g. all-dynamic styled components producing ";&:hover { ; }").
    if sheet.trim().is_empty() || !sheet.contains('{') {
      continue;
    }
    if unique.insert(sheet.clone()) {
      let ident = hoist_sheet(sheet, meta);
      idents.push(Expr::Ident(ident));
    }
  }

  idents
}

fn extract_key_attribute(node: &Expr) -> Option<JSXAttrValue> {
  let mut clone = node.clone();
  let (attribute, _) = get_jsx_attribute(&mut clone, "key");
  attribute.and_then(|attr| attr.value.clone())
}

/// Returns a generated AST for the Compiled runtime wrapper, mirroring the
/// Babel helper by emitting `<CC>` and `<CS>` elements that wrap the original
/// node and hoisted sheets.
pub fn compiled_template(node: Expr, sheets: &[String], meta: &Metadata) -> Expr {
  // Mark that we're using runtime wrappers so imports are added
  meta.state_mut().uses_runtime_wrappers = true;

  let key_attribute = extract_key_attribute(&node);
  let sheet_idents = collect_sheet_idents(sheets, meta);

  let nonce = meta.state().opts.nonce.clone();

  let mut cs_attrs = Vec::new();
  if let Some(nonce_value) = nonce {
    cs_attrs.push(build_nonce_attribute(&nonce_value));
  }

  let cs_children = vec![JSXElementChild::JSXExprContainer(JSXExprContainer {
    span: DUMMY_SP,
    expr: JSXExpr::Expr(Box::new(array_expression(sheet_idents))),
  })];

  let cs_element = JSXElementChild::JSXElement(Box::new(JSXElement {
    span: DUMMY_SP,
    opening: JSXOpeningElement {
      span: DUMMY_SP,
      name: jsx_name("CS"),
      attrs: cs_attrs,
      self_closing: false,
      type_args: None,
    },
    closing: Some(JSXClosingElement {
      span: DUMMY_SP,
      name: jsx_name("CS"),
    }),
    children: cs_children,
  }));

  let mut cc_attrs = Vec::new();
  if let Some(value) = key_attribute {
    cc_attrs.push(jsx_attribute("key", value));
  }

  let cc_children = vec![
    jsx_text("\n  "),
    cs_element,
    jsx_text("\n  "),
    JSXElementChild::JSXExprContainer(JSXExprContainer {
      span: DUMMY_SP,
      expr: JSXExpr::Expr(Box::new(node)),
    }),
    jsx_text("\n"),
  ];

  Expr::JSXElement(Box::new(JSXElement {
    span: DUMMY_SP,
    opening: JSXOpeningElement {
      span: DUMMY_SP,
      name: jsx_name("CC"),
      attrs: cc_attrs,
      self_closing: false,
      type_args: None,
    },
    closing: Some(JSXClosingElement {
      span: DUMMY_SP,
      name: jsx_name("CC"),
    }),
    children: cc_children,
  }))
}

fn build_style_attribute(
  variables: &[Variable],
  existing_value: Option<JSXAttrValue>,
) -> JSXAttrOrSpread {
  let mut dynamic_properties = build_css_variables(variables);

  if let Some(value) = existing_value {
    if let JSXAttrValue::JSXExprContainer(container) = value {
      if let JSXExpr::Expr(expr) = container.expr {
        match *expr {
          Expr::Object(ObjectLit { props, .. }) => {
            for (index, prop) in props.into_iter().enumerate() {
              dynamic_properties.insert(index, prop);
            }
          }
          other => {
            dynamic_properties.insert(
              0,
              PropOrSpread::Spread(SpreadElement {
                dot3_token: DUMMY_SP,
                expr: Box::new(other),
              }),
            );
          }
        }
      }
    }
  }

  let object = Expr::Object(ObjectLit {
    span: DUMMY_SP,
    props: dynamic_properties,
  });

  let value = JSXAttrValue::JSXExprContainer(JSXExprContainer {
    span: DUMMY_SP,
    expr: JSXExpr::Expr(Box::new(object)),
  });

  jsx_attribute("style", value)
}

fn merge_class_name(node: &mut Expr, class_names: &[Expr], meta: &Metadata) {
  let helper = get_runtime_class_name_library(meta);

  let (existing_value, index) = {
    let (attribute, index) = get_jsx_attribute(node, "className");
    let value = attribute.and_then(|attr| attr.value.clone());
    (
      value,
      if index >= 0 {
        Some(index as usize)
      } else {
        None
      },
    )
  };

  let mut values = class_names.to_vec();

  if let Some(attr_index) = index {
    if let Some(value) = existing_value {
      values.push(attribute_value_to_expr(&value));

      if let Expr::JSXElement(element) = node {
        if let Some(JSXAttrOrSpread::JSXAttr(attribute)) = element.opening.attrs.get_mut(attr_index)
        {
          attribute.value = Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
            span: DUMMY_SP,
            expr: JSXExpr::Expr(Box::new(runtime_call(helper, values))),
          }));
        }
      }
    } else {
      // Attribute exists but has no value – treat it as though it was missing.
      if let Expr::JSXElement(element) = node {
        element.opening.attrs.remove(attr_index);
      }

      let call = runtime_call(helper, class_names.to_vec());
      if let Expr::JSXElement(element) = node {
        element.opening.attrs.push(jsx_attribute(
          "className",
          JSXAttrValue::JSXExprContainer(JSXExprContainer {
            span: DUMMY_SP,
            expr: JSXExpr::Expr(Box::new(call)),
          }),
        ));
      }
    }
  } else {
    let call = runtime_call(helper, class_names.to_vec());
    if let Expr::JSXElement(element) = node {
      element.opening.attrs.push(jsx_attribute(
        "className",
        JSXAttrValue::JSXExprContainer(JSXExprContainer {
          span: DUMMY_SP,
          expr: JSXExpr::Expr(Box::new(call)),
        }),
      ));
    }
  }
}

fn merge_style_attribute(node: &mut Expr, variables: &[Variable]) {
  if variables.is_empty() {
    return;
  }

  let (existing_value, index) = {
    let (attribute, index) = get_jsx_attribute(node, "style");
    let value = attribute.and_then(|attr| attr.value.clone());
    (
      value,
      if index >= 0 {
        Some(index as usize)
      } else {
        None
      },
    )
  };

  if let Some(attr_index) = index {
    if let Expr::JSXElement(element) = node {
      element.opening.attrs.remove(attr_index);
    }
  }

  if let Expr::JSXElement(element) = node {
    element
      .opening
      .attrs
      .push(build_style_attribute(variables, existing_value));
  }
}

/// Returns the Compiled component wrapper for the provided JSX element and CSS
/// output, mirroring the behaviour of the Babel helper.
pub fn build_compiled_component(mut node: Expr, css_output: &CssOutput, meta: &Metadata) -> Expr {
  let transform_result = transform_css_items(&css_output.css, meta);

  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    let state = meta.state();
    let filename = state.filename.clone().unwrap_or_default();
    let css_alias = state.imported_compiled_imports.css.clone();
    eprintln!(
      "[build_compiled_component] file={} css_alias={:?} css_items={} variables={} sheets={} style_rules={} uses_runtime_wrappers={}",
      filename,
      css_alias,
      transform_result.class_names.len(),
      css_output.variables.len(),
      transform_result.sheets.len(),
      state.style_rules.len(),
      state.uses_runtime_wrappers
    );
  }

  // Validate that all sheets are non-empty CSS strings containing at least one rule.
  // The CS runtime component expects each sheet to be a string (it calls sheet.includes()).
  // If a non-string value reaches the runtime, it causes "sheet.includes is not a function".
  validate_sheets(
    &transform_result.sheets,
    &transform_result.class_names,
    meta,
  );

  merge_class_name(&mut node, &transform_result.class_names, meta);
  merge_style_attribute(&mut node, &css_output.variables);

  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    let state = meta.state();
    eprintln!(
      "[build_compiled_component] before template style_rules={} sheets={} uses_runtime_wrappers={}",
      state.style_rules.len(),
      state.sheets.len(),
      state.uses_runtime_wrappers
    );
  }

  let result = compiled_template(node, &transform_result.sheets, meta);

  if std::env::var("COMPILED_CLI_TRACE").is_ok() {
    let state = meta.state();
    eprintln!(
      "[build_compiled_component] after template style_rules={} sheets={} uses_runtime_wrappers={}",
      state.style_rules.len(),
      state.sheets.len(),
      state.uses_runtime_wrappers
    );
  }

  result
}

/// Validate that sheets and class_names produced by transform_css_items are well-formed.
/// Emits diagnostics for any issues that could cause "sheet.includes is not a function"
/// at runtime in the CS (Style) component.
fn validate_sheets(sheets: &[String], class_names: &[Expr], meta: &Metadata) {
  let filename = meta.state().filename.clone().unwrap_or_default();

  for (i, sheet) in sheets.iter().enumerate() {
    if sheet.trim().is_empty() {
      let message = format!(
        "Compiled CSS: Empty sheet string at index {} in {}. \
         The CS runtime component expects non-empty CSS strings. \
         This may cause unexpected behavior at runtime.",
        i, filename
      );
      eprintln!("[compiled-css] WARNING: {}", message);
      meta.add_diagnostic(crate::errors::create_diagnostic(
        message,
        module_path!(),
        None,
        None,
      ));
    }
  }

  // Validate that class_name expressions are string-safe.
  // The className values flow into ax([...]) which concatenates them. Non-string
  // values (e.g., arrays, objects) would cause issues in the CS children array.
  for (i, class_expr) in class_names.iter().enumerate() {
    validate_class_expression(class_expr, i, &filename, meta);
  }
}

/// Validates that a class expression will produce a string-compatible value at runtime.
/// Non-string values (arrays, objects, numeric literals) in the className array would
/// eventually flow to the CS component children and cause "sheet.includes is not a function".
fn validate_class_expression(expr: &Expr, index: usize, filename: &str, meta: &Metadata) {
  match expr {
    // String literals are always valid
    Expr::Lit(swc_core::ecma::ast::Lit::Str(_)) => {}
    // Identifiers resolve at runtime - we can't validate them statically,
    // but they're the common case for cssMap lookups (e.g., styles.root)
    // This also covers `undefined` which is used as a fallback in conditionals.
    Expr::Ident(_) => {}
    // Member expressions (e.g., styles.root) are valid cssMap lookups
    Expr::Member(_) => {}
    // Conditional expressions (ternary) are valid if both branches are valid
    Expr::Cond(cond) => {
      validate_class_expression(&cond.cons, index, filename, meta);
      validate_class_expression(&cond.alt, index, filename, meta);
    }
    // Binary expressions (&&, ||, ??) are valid if right side is valid
    Expr::Bin(bin) => {
      if matches!(
        bin.op,
        swc_core::ecma::ast::BinaryOp::LogicalAnd
          | swc_core::ecma::ast::BinaryOp::LogicalOr
          | swc_core::ecma::ast::BinaryOp::NullishCoalescing
      ) {
        validate_class_expression(&bin.right, index, filename, meta);
      }
    }
    // Parenthesized expressions - validate inner
    Expr::Paren(paren) => {
      validate_class_expression(&paren.expr, index, filename, meta);
    }
    // Call expressions (e.g., ax([...])) are valid
    Expr::Call(_) => {}
    // Template literals are valid
    Expr::Tpl(_) => {}
    // Array literals, object literals, or other non-string expressions are suspicious
    Expr::Array(_) | Expr::Object(_) => {
      let message = format!(
        "Compiled CSS: class expression at index {} in {} is an array or object literal. \
         The CS runtime component expects string values in the className array. \
         This will cause \"sheet.includes is not a function\" at runtime. \
         Expression: {:?}",
        index, filename, expr
      );
      eprintln!("[compiled-css] WARNING: {}", message);
      meta.add_diagnostic(crate::errors::create_diagnostic(
        message,
        module_path!(),
        None,
        None,
      ));
    }
    // Other expression types - emit a trace-level warning for debugging
    _ => {
      if std::env::var("COMPILED_CLI_TRACE").is_ok() {
        eprintln!(
          "[compiled-css] TRACE: class expression at index {} in {} has type {:?} - \
           verify this produces a string at runtime",
          index,
          filename,
          std::mem::discriminant(expr)
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::{build_compiled_component, compiled_template, ident};
  use crate::types::{Metadata, PluginOptions, TransformFile, TransformState};
  use crate::utils_types::{CssItem, CssOutput, Variable};
  use swc_core::common::sync::Lrc;
  use swc_core::common::{FileName, SourceMap};
  use swc_core::ecma::ast::{
    Expr, JSXAttrOrSpread, JSXAttrValue, JSXElementChild, JSXExpr, Lit, Str,
  };
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};

  fn create_metadata(options: PluginOptions) -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm, Vec::new());
    let state = std::rc::Rc::new(std::cell::RefCell::new(TransformState::new(file, options)));

    Metadata::new(state)
  }

  fn parse_jsx_expression(code: &str) -> Expr {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("expr.jsx".into()).into(), code.to_string());
    let lexer = Lexer::new(
      Syntax::Es(EsSyntax {
        jsx: true,
        ..Default::default()
      }),
      Default::default(),
      StringInput::from(&*fm),
      None,
    );

    let mut parser = Parser::new_from(lexer);
    *parser.parse_expr().expect("parse JSX expression")
  }

  fn simple_css_output() -> CssOutput {
    CssOutput {
      css: vec![CssItem::unconditional(".test{font-size:12px;}".to_string())],
      variables: Vec::new(),
    }
  }

  #[test]
  fn compiled_template_wraps_node_and_deduplicates_sheets() {
    let meta = create_metadata(PluginOptions::default());
    let node = parse_jsx_expression("<div />");
    let sheets = vec![
      "._1wyb1fwx{font-size:12px}".to_string(),
      "._1wyb1fwx{font-size:12px}".to_string(),
    ];

    let wrapped = compiled_template(node, &sheets, &meta);

    match wrapped {
      Expr::JSXElement(element) => {
        assert_eq!(element.children.len(), 5);
        // The CS child should contain a single hoisted identifier.
        let cs_element = match &element.children[1] {
          swc_core::ecma::ast::JSXElementChild::JSXElement(child) => &**child,
          other => panic!("expected JSX element, found {:?}", other),
        };

        let child = match &cs_element.children[0] {
          swc_core::ecma::ast::JSXElementChild::JSXExprContainer(container) => &container.expr,
          other => panic!("expected expression container, found {:?}", other),
        };

        match child {
          JSXExpr::Expr(expr) => match &**expr {
            Expr::Array(array) => {
              assert_eq!(array.elems.len(), 1);
            }
            other => panic!("expected array expression, found {:?}", other),
          },
          other => panic!("expected expression, found {:?}", other),
        }

        let state = meta.state();
        assert!(state.sheets.contains_key("._1wyb1fwx{font-size:12px}"));
      }
      other => panic!("expected JSX element, found {:?}", other),
    }
  }

  #[test]
  fn build_compiled_component_adds_runtime_class_name() {
    let meta = create_metadata(PluginOptions::default());
    let node = parse_jsx_expression("<div />");
    let output = build_compiled_component(node, &simple_css_output(), &meta);

    let Expr::JSXElement(wrapper) = output else {
      panic!("expected JSX element");
    };

    let JSXElementChild::JSXExprContainer(container) = &wrapper.children[3] else {
      panic!("expected expression container child");
    };

    let inner = match &container.expr {
      JSXExpr::Expr(expr) => match &**expr {
        Expr::JSXElement(element) => element.as_ref(),
        other => panic!("expected JSX element inside container, found {:?}", other),
      },
      other => panic!("expected expression, found {:?}", other),
    };

    let attrs = &inner.opening.attrs;
    assert_eq!(attrs.len(), 1);

    let JSXAttrOrSpread::JSXAttr(class_attr) = &attrs[0] else {
      panic!("expected class attribute");
    };

    let Some(JSXAttrValue::JSXExprContainer(expr_container)) = &class_attr.value else {
      panic!("expected expression container");
    };

    match &expr_container.expr {
      JSXExpr::Expr(expr) => match &**expr {
        Expr::Call(call) => {
          assert_eq!(call.args.len(), 1);
          let array = match &*call.args[0].expr {
            Expr::Array(array) => array,
            other => panic!("expected array expression, found {:?}", other),
          };

          assert!(!array.elems.is_empty());
        }
        other => panic!("expected call expression, found {:?}", other),
      },
      other => panic!("expected expression, found {:?}", other),
    }
  }

  #[test]
  fn merges_existing_class_name_expression() {
    let meta = create_metadata(PluginOptions::default());
    let node = parse_jsx_expression("<div className=\"existing\" />");
    let output = build_compiled_component(node, &simple_css_output(), &meta);

    let Expr::JSXElement(wrapper) = output else {
      panic!("expected JSX element");
    };

    let JSXElementChild::JSXExprContainer(container) = &wrapper.children[3] else {
      panic!("expected expression container child");
    };

    let inner = match &container.expr {
      JSXExpr::Expr(expr) => match &**expr {
        Expr::JSXElement(element) => element.as_ref(),
        other => panic!("expected JSX element, found {:?}", other),
      },
      _ => panic!("expected expression"),
    };

    let JSXAttrOrSpread::JSXAttr(class_attr) = &inner.opening.attrs[0] else {
      panic!("expected class attribute");
    };

    let Some(JSXAttrValue::JSXExprContainer(container)) = &class_attr.value else {
      panic!("expected expression container");
    };

    let JSXExpr::Expr(expr) = &container.expr else {
      panic!("expected expression");
    };

    let Expr::Call(call) = &**expr else {
      panic!("expected call expression");
    };

    let array = match &*call.args[0].expr {
      Expr::Array(array) => array,
      other => panic!("expected array expression, found {:?}", other),
    };

    assert_eq!(array.elems.len(), 2);

    let first = array.elems[0].as_ref().unwrap();
    match &*first.expr {
      Expr::Lit(Lit::Str(Str { value, .. })) => {
        assert!(!value.as_ref().is_empty());
      }
      other => panic!("expected string literal, found {:?}", other),
    }

    let second = array.elems[1].as_ref().unwrap();
    match &*second.expr {
      Expr::Lit(Lit::Str(Str { value, .. })) => {
        assert_eq!(value.as_ref(), "existing");
      }
      other => panic!("expected string literal, found {:?}", other),
    }
  }

  #[test]
  fn merges_style_attribute_with_variables() {
    let mut options = PluginOptions::default();
    options.nonce = Some("nonceValue".into());
    let meta = create_metadata(options);

    let node = parse_jsx_expression("<div style={{ color: 'blue' }} />");

    let output = build_compiled_component(
      node,
      &CssOutput {
        css: vec![CssItem::unconditional(".test{font-size:12px;}".to_string())],
        variables: vec![Variable {
          name: "--color".into(),
          expression: Expr::Ident(ident("value")),
          prefix: None,
          suffix: None,
        }],
      },
      &meta,
    );

    let Expr::JSXElement(wrapper) = output else {
      panic!("expected JSX element");
    };

    let JSXElementChild::JSXExprContainer(container) = &wrapper.children[3] else {
      panic!("expected expression container child");
    };

    let inner = match &container.expr {
      JSXExpr::Expr(expr) => match &**expr {
        Expr::JSXElement(element) => element.as_ref(),
        other => panic!("expected JSX element, found {:?}", other),
      },
      _ => panic!("expected expression"),
    };

    let attrs = &inner.opening.attrs;
    assert_eq!(attrs.len(), 2);

    let JSXAttrOrSpread::JSXAttr(style_attr) = &attrs[1] else {
      panic!("expected style attribute");
    };

    let Some(JSXAttrValue::JSXExprContainer(container)) = &style_attr.value else {
      panic!("expected expression container");
    };

    let JSXExpr::Expr(expr) = &container.expr else {
      panic!("expected expression");
    };

    let Expr::Object(object) = &**expr else {
      panic!("expected object expression");
    };

    assert_eq!(object.props.len(), 2);
  }

  #[test]
  fn build_compiled_component_sets_runtime_wrapper_flag() {
    let meta = create_metadata(PluginOptions::default());
    let node = parse_jsx_expression("<div />");

    let state_before = meta.state();
    assert!(
      !state_before.uses_runtime_wrappers,
      "flag should be false before transform"
    );
    drop(state_before);

    let _ = build_compiled_component(node, &simple_css_output(), &meta);

    let state_after = meta.state();
    assert!(
      state_after.uses_runtime_wrappers,
      "flag should be true after build_compiled_component"
    );
  }

  #[test]
  fn compiled_template_wraps_in_cc_and_cs_elements() {
    let meta = create_metadata(PluginOptions::default());
    let node = parse_jsx_expression("<div />");
    let sheets = vec!["._1wyb1fwx{font-size:12px}".to_string()];

    let wrapped = compiled_template(node, &sheets, &meta);

    // Verify CC wrapper
    let Expr::JSXElement(cc_element) = wrapped else {
      panic!("expected JSX element");
    };

    match &cc_element.opening.name {
      swc_core::ecma::ast::JSXElementName::Ident(ident) => {
        assert_eq!(ident.sym.as_ref(), "CC", "outer element should be CC");
      }
      other => panic!("expected CC identifier, found {:?}", other),
    }

    // Verify CS child
    let cs_found = cc_element.children.iter().any(|child| match child {
      JSXElementChild::JSXElement(el) => {
        matches!(&el.opening.name, swc_core::ecma::ast::JSXElementName::Ident(ident) if ident.sym.as_ref() == "CS")
      }
      _ => false,
    });

    assert!(cs_found, "CC should contain CS element");
  }
}
