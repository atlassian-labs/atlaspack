use std::collections::HashMap;

use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::{
  ArrayPat, ArrowExpr, BinExpr, BinaryOp, BindingIdent, BlockStmtOrExpr, Expr, FnExpr, Function,
  Ident, IdentName, MemberExpr, MemberProp, ObjectPatProp, ParenExpr, Pat, PropName,
};
use swc_core::ecma::atoms::Atom;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};

use crate::constants::PROPS_IDENTIFIER_NAME;

type BindingId = (Atom, SyntaxContext);

type BindingChains = HashMap<BindingId, Vec<String>>;
type BindingDefaults = HashMap<BindingId, Expr>;

#[allow(unreachable_patterns)]
fn pattern_type(pat: &Pat) -> &'static str {
  match pat {
    Pat::Ident(_) => "Identifier",
    Pat::Array(_) => "ArrayPattern",
    Pat::Rest(_) => "RestElement",
    Pat::Object(_) => "ObjectPattern",
    Pat::Assign(_) => "AssignmentPattern",
    Pat::Invalid(_) => "InvalidPattern",
    Pat::Expr(_) => "Expression",
    _ => "UnknownPattern",
  }
}

fn expr_type(expr: &Expr) -> &'static str {
  match expr {
    Expr::Array(_) => "ArrayExpression",
    Expr::Arrow(_) => "ArrowFunctionExpression",
    Expr::Assign(_) => "AssignmentExpression",
    Expr::Await(_) => "AwaitExpression",
    Expr::Bin(_) => "BinaryExpression",
    Expr::Call(_) => "CallExpression",
    Expr::Class(_) => "ClassExpression",
    Expr::Cond(_) => "ConditionalExpression",
    Expr::Fn(_) => "FunctionExpression",
    Expr::Ident(_) => "Identifier",
    Expr::Lit(lit) => match lit {
      swc_core::ecma::ast::Lit::Str(_) => "StringLiteral",
      swc_core::ecma::ast::Lit::Num(_) => "NumericLiteral",
      swc_core::ecma::ast::Lit::Bool(_) => "BooleanLiteral",
      swc_core::ecma::ast::Lit::Null(_) => "NullLiteral",
      swc_core::ecma::ast::Lit::BigInt(_) => "BigIntLiteral",
      swc_core::ecma::ast::Lit::Regex(_) => "RegExpLiteral",
      swc_core::ecma::ast::Lit::JSXText(_) => "JSXText",
    },
    Expr::Member(_) => "MemberExpression",
    Expr::New(_) => "NewExpression",
    Expr::Object(_) => "ObjectExpression",
    Expr::OptChain(_) => "OptionalChainExpression",
    Expr::Paren(_) => "ParenthesizedExpression",
    Expr::Tpl(_) => "TemplateLiteral",
    Expr::Unary(_) => "UnaryExpression",
    Expr::Update(_) => "UpdateExpression",
    Expr::Invalid(_) => "InvalidExpression",
    Expr::MetaProp(_) => "MetaProperty",
    Expr::This(_) => "ThisExpression",
    Expr::Yield(_) => "YieldExpression",
    Expr::JSXElement(_) => "JSXElement",
    Expr::JSXFragment(_) => "JSXFragment",
    Expr::TaggedTpl(_) => "TaggedTemplateExpression",
    Expr::Seq(_) => "SequenceExpression",
    Expr::SuperProp(_) => "Super",
    Expr::TsTypeAssertion(_) => "TSTypeAssertion",
    Expr::TsConstAssertion(_) => "TSConstAssertion",
    Expr::TsNonNull(_) => "TSNonNullExpression",
    Expr::TsAs(_) => "TSAsExpression",
    Expr::TsInstantiation(_) => "TSInstantiationExpression",
    Expr::PrivateName(_) => "PrivateName",
    _ => "UnknownExpression",
  }
}

fn prop_name_type(name: &PropName) -> &'static str {
  match name {
    PropName::Ident(_) => "Identifier",
    PropName::Str(_) => "StringLiteral",
    PropName::Num(_) => "NumericLiteral",
    PropName::Computed(_) => "ComputedPropertyName",
    PropName::BigInt(_) => "BigIntLiteral",
  }
}

fn create_props_ident() -> Ident {
  Ident::new(
    PROPS_IDENTIFIER_NAME.into(),
    DUMMY_SP,
    SyntaxContext::empty(),
  )
}

fn create_nested_member_expression(chain: &[String]) -> Expr {
  if chain.is_empty() {
    panic!(
      "Could not build a Compiled component, due to objectChain being empty when generating a member expression. This is likely a bug with Compiled - please file a bug report."
    );
  }

  let mut expr = Expr::Ident(create_props_ident());
  for property in chain.iter().skip(1) {
    expr = Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(expr),
      prop: MemberProp::Ident(IdentName::new(property.clone().into(), DUMMY_SP)),
    });
  }

  expr
}

fn collect_defaults_from_assignment(expr: &Expr) -> HashMap<String, Expr> {
  let mut defaults = HashMap::new();
  let Expr::Object(object) = expr else {
    panic!(
      "This syntax for objects in arrow function parameters isn't supported by Compiled. (Left-hand side ObjectPattern and right-hand side {})",
      expr_type(expr)
    );
  };

  for prop in &object.props {
    let swc_core::ecma::ast::PropOrSpread::Prop(prop) = prop else {
      continue;
    };
    let swc_core::ecma::ast::Prop::KeyValue(key_value) = prop.as_ref() else {
      continue;
    };
    let name = match &key_value.key {
      PropName::Ident(ident) => ident.sym.to_string(),
      other => {
        panic!(
          "This syntax for objects in arrow function parameters isn't supported by Compiled. (Left-hand side {} and right-hand side {})",
          prop_name_type(other),
          expr_type(&key_value.value)
        );
      }
    };
    defaults.insert(name, (*key_value.value.clone()).clone());
  }

  defaults
}

fn collect_binding_info(
  pat: &Pat,
  current_chain: &[String],
  chains: &mut BindingChains,
  defaults: &mut BindingDefaults,
) {
  match pat {
    Pat::Ident(binding_ident) => {
      let id = (binding_ident.id.sym.clone(), binding_ident.id.ctxt);
      chains.insert(id, current_chain.to_vec());
    }
    Pat::Assign(assign) => {
      collect_binding_info(&assign.left, current_chain, chains, defaults);
      if let Pat::Ident(ident) = &*assign.left {
        let id = (ident.id.sym.clone(), ident.id.ctxt);
        defaults.insert(id, (*assign.right.clone()).clone());
      } else {
        panic!(
          "This syntax for assignments in arrow function parameters isn't supported by Compiled. (Left-hand side {} and right-hand side {})",
          pattern_type(&assign.left),
          expr_type(&assign.right)
        );
      }
    }
    Pat::Object(object) => {
      for prop in &object.props {
        match prop {
          ObjectPatProp::KeyValue(key_value) => {
            let key = match &key_value.key {
              PropName::Ident(ident) => ident.sym.to_string(),
              other => {
                panic!(
                  "This syntax for objects in arrow function parameters isn't supported by Compiled. (Left-hand side {} and right-hand side {})",
                  prop_name_type(other),
                  pattern_type(&key_value.value)
                );
              }
            };
            let mut next_chain = current_chain.to_vec();
            next_chain.push(key);
            collect_binding_info(&key_value.value, &next_chain, chains, defaults);
          }
          ObjectPatProp::Assign(assign) => {
            let mut next_chain = current_chain.to_vec();
            next_chain.push(assign.key.sym.to_string());
            let id = (assign.key.sym.clone(), assign.key.ctxt);
            chains.insert(id.clone(), next_chain);
            if let Some(default) = &assign.value {
              defaults.insert(id, (*default.clone()).clone());
            }
          }
          ObjectPatProp::Rest(rest) => {
            if let Pat::Ident(ident) = rest.arg.as_ref() {
              let id = (ident.id.sym.clone(), ident.id.ctxt);
              chains.insert(id, current_chain.to_vec());
            }
          }
        }
      }
    }
    Pat::Array(ArrayPat { span: _, elems, .. }) => {
      if elems.iter().any(|elem| elem.is_some()) {
        panic!("Compiled does not support arrays given in the parameters of an arrow function.");
      }
    }
    Pat::Rest(rest) => {
      if let Pat::Ident(ident) = rest.arg.as_ref() {
        let id = (ident.id.sym.clone(), ident.id.ctxt);
        chains.insert(id, current_chain.to_vec());
      }
    }
    _ => {}
  }
}

fn build_binding_maps(pat: &Pat) -> (BindingChains, BindingDefaults) {
  let mut chains = BindingChains::new();
  let mut defaults = BindingDefaults::new();

  let mut base_chain = Vec::new();
  base_chain.push(PROPS_IDENTIFIER_NAME.to_string());

  match pat {
    Pat::Assign(assign) => {
      collect_binding_info(&assign.left, &base_chain, &mut chains, &mut defaults);
      if let Pat::Object(_) = assign.left.as_ref() {
        let object_defaults = collect_defaults_from_assignment(&assign.right);
        for (name, expr) in object_defaults {
          if let Some((id, _)) = chains
            .iter()
            .find(|(_, chain)| chain.last().map(|last| last == &name).unwrap_or(false))
          {
            defaults.entry(id.clone()).or_insert(expr);
          }
        }
      }
    }
    _ => {
      collect_binding_info(pat, &base_chain, &mut chains, &mut defaults);
    }
  }

  (chains, defaults)
}

struct ReplaceBindingsVisitor<'a> {
  chains: &'a BindingChains,
  defaults: &'a BindingDefaults,
  targets: std::collections::HashSet<BindingId>,
}

fn pattern_binds_target(pat: &Pat, targets: &std::collections::HashSet<BindingId>) -> bool {
  match pat {
    Pat::Ident(binding) => targets.contains(&(binding.id.sym.clone(), binding.id.ctxt)),
    Pat::Array(array) => array
      .elems
      .iter()
      .flatten()
      .any(|elem| pattern_binds_target(elem, targets)),
    Pat::Object(obj) => obj.props.iter().any(|prop| match prop {
      ObjectPatProp::KeyValue(kv) => pattern_binds_target(&kv.value, targets),
      ObjectPatProp::Assign(assign) => targets.contains(&(assign.key.sym.clone(), assign.key.ctxt)),
      ObjectPatProp::Rest(rest) => pattern_binds_target(&rest.arg, targets),
    }),
    Pat::Assign(assign) => pattern_binds_target(&assign.left, targets),
    Pat::Rest(rest) => pattern_binds_target(&rest.arg, targets),
    _ => false,
  }
}

impl<'a> ReplaceBindingsVisitor<'a> {
  fn replace_identifier(&self, ident: &Ident) -> Option<Expr> {
    let id = (ident.sym.clone(), ident.ctxt);
    let chain = self.chains.get(&id)?;
    let member = create_nested_member_expression(chain);

    if let Some(default) = self.defaults.get(&id) {
      // Wrap conditional expressions (ternary operators) in parentheses to maintain correct operator precedence
      let right_expr = match default {
        Expr::Cond(_) => Expr::Paren(ParenExpr {
          span: DUMMY_SP,
          expr: Box::new(default.clone()),
        }),
        _ => default.clone(),
      };

      Some(Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op: BinaryOp::NullishCoalescing,
        left: Box::new(member),
        right: Box::new(right_expr),
      }))
    } else {
      Some(member)
    }
  }
}

impl<'a> VisitMut for ReplaceBindingsVisitor<'a> {
  fn visit_mut_arrow_expr(&mut self, arrow: &mut ArrowExpr) {
    let shadows = arrow
      .params
      .iter()
      .any(|param| pattern_binds_target(param, &self.targets));
    if !shadows {
      arrow.body.visit_mut_with(self);
    }
  }

  fn visit_mut_fn_expr(&mut self, func: &mut FnExpr) {
    let shadows = func
      .function
      .params
      .iter()
      .any(|param| pattern_binds_target(&param.pat, &self.targets));
    if !shadows {
      func.function.body.visit_mut_with(self);
    }
  }

  fn visit_mut_function(&mut self, func: &mut Function) {
    let shadows = func
      .params
      .iter()
      .any(|param| pattern_binds_target(&param.pat, &self.targets));
    if !shadows {
      func.body.visit_mut_with(self);
    }
  }

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    if let Expr::Ident(ident) = expr {
      if let Some(replacement) = self.replace_identifier(ident) {
        *expr = replacement;
        return;
      }
    }

    expr.visit_mut_children_with(self);
  }
}

fn replace_bindings(
  body: &mut BlockStmtOrExpr,
  chains: &BindingChains,
  defaults: &BindingDefaults,
) {
  let targets = chains.keys().cloned().collect();
  let mut visitor = ReplaceBindingsVisitor {
    chains,
    defaults,
    targets,
  };
  body.visit_mut_with(&mut visitor);
}

struct PropsUsageNormalizer;

impl PropsUsageNormalizer {
  fn process_arrow(&mut self, arrow: &mut ArrowExpr) {
    if arrow.params.is_empty() {
      return;
    }

    let first_param = arrow.params[0].clone();
    match &first_param {
      Pat::Ident(binding_ident) => {
        if binding_ident.id.sym.as_ref() != PROPS_IDENTIFIER_NAME {
          let id = (binding_ident.id.sym.clone(), binding_ident.id.ctxt);
          let chains = HashMap::from([(id, vec![PROPS_IDENTIFIER_NAME.to_string()])]);
          let defaults = BindingDefaults::new();
          replace_bindings(arrow.body.as_mut(), &chains, &defaults);
        }
      }
      Pat::Object(_) | Pat::Assign(_) => {
        let (chains, defaults) = build_binding_maps(&first_param);
        replace_bindings(arrow.body.as_mut(), &chains, &defaults);
      }
      _ => {}
    }

    arrow.params[0] = Pat::Ident(BindingIdent {
      id: create_props_ident(),
      type_ann: None,
    });
  }
}

impl VisitMut for PropsUsageNormalizer {
  fn visit_mut_arrow_expr(&mut self, arrow: &mut ArrowExpr) {
    self.process_arrow(arrow);
    arrow.body.visit_mut_with(self);
  }
}

/// Normalizes props usage within styled/css invocations to match the Babel behaviour.
pub fn normalize_props_usage(expr: &mut Expr) {
  let mut normalizer = PropsUsageNormalizer;
  expr.visit_mut_with(&mut normalizer);
}

#[cfg(test)]
mod tests {
  use super::normalize_props_usage;
  use crate::constants::PROPS_IDENTIFIER_NAME;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{FileName, SourceMap};
  use swc_core::ecma::ast::Expr;
  use swc_core::ecma::codegen::text_writer::JsWriter;
  use swc_core::ecma::codegen::{Config, Emitter, Node};
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax, lexer::Lexer};

  fn parse_expression(code: &str) -> Expr {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("test.tsx".into()).into(), code.into());
    let mut es_config = EsSyntax::default();
    es_config.jsx = true;
    let lexer = Lexer::new(
      Syntax::Es(es_config),
      Default::default(),
      StringInput::from(&*fm),
      None,
    );
    let mut parser = Parser::new_from(lexer);
    *parser.parse_expr().expect("parse expression")
  }

  fn print_expression(expr: &Expr) -> String {
    let cm: Lrc<SourceMap> = Default::default();
    let mut buffer = Vec::new();
    {
      let writer = JsWriter::new(cm.clone(), "\n", &mut buffer, None);
      let mut emitter = Emitter {
        cfg: Config::default(),
        comments: None,
        cm,
        wr: writer,
      };
      expr.emit_with(&mut emitter).expect("emit expr");
    }
    String::from_utf8(buffer).expect("utf8")
  }

  fn transform(code: &str) -> String {
    let mut expr = parse_expression(code);
    normalize_props_usage(&mut expr);
    print_expression(&expr).replace('\n', "").replace(' ', "")
  }

  #[test]
  fn renames_props_param() {
    let output = transform("styled.div(({ color }) => color)");
    assert!(output.contains(PROPS_IDENTIFIER_NAME));
    assert!(output.contains(&format!("{}.color", PROPS_IDENTIFIER_NAME)));
  }

  #[test]
  fn reconstructs_destructured_param() {
    let output = transform("styled.div(({ customColor }) => ({ backgroundColor: customColor }))");
    assert!(output.contains(&format!("({})=>", PROPS_IDENTIFIER_NAME)));
    assert!(output.contains(&format!(
      "backgroundColor:{}.customColor",
      PROPS_IDENTIFIER_NAME
    )));
  }

  #[test]
  fn reconstructs_nested_destructuring() {
    let output =
      transform("styled.div(({ theme: { colors: { dark } } }) => dark ? dark.red : 'black')");
    assert!(output.contains(&format!("({})=>", PROPS_IDENTIFIER_NAME)));
    assert!(output.contains(&format!("{}.theme.colors.dark", PROPS_IDENTIFIER_NAME)));
    assert!(output.contains(&format!("{}.theme.colors.dark.red", PROPS_IDENTIFIER_NAME)));
  }

  #[test]
  fn handles_rest_elements() {
    let output = transform("styled.div(({ width, ...rest }) => rest.height)");
    assert!(output.contains(&format!("({})=>", PROPS_IDENTIFIER_NAME)));
    assert!(output.contains(&format!("{}.height", PROPS_IDENTIFIER_NAME)));
  }

  #[test]
  fn applies_default_values() {
    let output = transform("styled.div(({ a, b = 16 }) => `${b}px ${a}px`)");
    assert!(output.contains(&format!("{}.b??16", PROPS_IDENTIFIER_NAME)));
    assert!(output.contains(&format!("{}.a", PROPS_IDENTIFIER_NAME)));
  }

  #[test]
  fn applies_assignment_pattern_defaults() {
    let output = transform("styled.div(({ a, b } = { a: 100, b: 200 }) => `${a}px ${b}px`)");
    assert!(output.contains(&format!("{}.a??100", PROPS_IDENTIFIER_NAME)));
    assert!(output.contains(&format!("{}.b??200", PROPS_IDENTIFIER_NAME)));
  }

  #[test]
  #[should_panic]
  fn rejects_array_destructuring() {
    let _ = transform("styled.div(({ a: [, second] }) => second)");
  }
}
