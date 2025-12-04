use swc_core::ecma::ast::{Expr, JSXAttr, JSXAttrName, JSXAttrOrSpread};

/// Locate a JSX attribute on the provided node, mirroring the Babel helper by
/// returning the mutable attribute reference and its index when found.
pub fn get_jsx_attribute<'a>(node: &'a mut Expr, name: &str) -> (Option<&'a mut JSXAttr>, isize) {
  let Expr::JSXElement(element) = node else {
    return (None, -1);
  };

  let element = &mut **element;

  let mut found_index: Option<usize> = None;

  for (index, attr) in element.opening.attrs.iter().enumerate() {
    let JSXAttrOrSpread::JSXAttr(attribute) = attr else {
      continue;
    };

    if let JSXAttrName::Ident(ident) = &attribute.name {
      if ident.sym.as_ref() == name {
        found_index = Some(index);
        break;
      }
    }
  }

  match found_index {
    Some(index) => {
      let attribute = element
        .opening
        .attrs
        .get_mut(index)
        .and_then(|attr| match attr {
          JSXAttrOrSpread::JSXAttr(attribute) => Some(attribute),
          _ => None,
        });

      (attribute, index as isize)
    }
    None => (None, -1),
  }
}

#[cfg(test)]
mod tests {
  use swc_core::common::sync::Lrc;
  use swc_core::common::{FileName, SourceMap};
  use swc_core::ecma::ast::{Expr, JSXAttrOrSpread, JSXAttrValue, Lit, Str};
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};

  use super::get_jsx_attribute;

  fn parse_jsx_expression(code: &str) -> Expr {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("expr.jsx".into()).into(), code.into());
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

  #[test]
  fn returns_none_for_non_jsx_nodes() {
    let mut expr = Expr::Lit(Lit::Str(Str {
      span: Default::default(),
      value: "value".into(),
      raw: None,
    }));

    let (attribute, index) = get_jsx_attribute(&mut expr, "className");
    assert!(attribute.is_none());
    assert_eq!(index, -1);
  }

  #[test]
  fn finds_attribute_by_name() {
    let mut expr = parse_jsx_expression("<div className=\"foo\" style={{}} />");
    let (attribute, index) = get_jsx_attribute(&mut expr, "className");

    assert_eq!(index, 0);

    let attr = attribute.expect("expected className attribute");
    match attr.value {
      Some(JSXAttrValue::Lit(Lit::Str(Str { ref value, .. }))) => {
        assert_eq!(value.as_ref(), "foo");
      }
      _ => panic!("unexpected attribute value"),
    }
  }

  #[test]
  fn accounts_for_spread_attributes_when_reporting_index() {
    let mut expr = parse_jsx_expression("<div {...props} className=\"foo\" />");
    let (attribute, index) = get_jsx_attribute(&mut expr, "className");

    assert_eq!(index, 1);

    drop(attribute);

    if let Expr::JSXElement(element) = expr {
      let attr = &element.opening.attrs[index as usize];
      assert!(matches!(attr, JSXAttrOrSpread::JSXAttr(_)));
    } else {
      panic!("expected JSX element");
    }
  }

  #[test]
  fn returns_negative_index_when_missing() {
    let mut expr = parse_jsx_expression("<div id=\"foo\" />");
    let (attribute, index) = get_jsx_attribute(&mut expr, "className");

    assert!(attribute.is_none());
    assert_eq!(index, -1);
  }
}
