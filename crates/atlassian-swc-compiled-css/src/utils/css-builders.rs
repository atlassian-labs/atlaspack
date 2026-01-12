use once_cell::sync::Lazy;
use regex::Regex;
use swc_core::common::comments::CommentKind;
use swc_core::common::sync::Lrc;
use swc_core::common::{DUMMY_SP, SourceMap, SourceMapper, Spanned, SyntaxContext};
use swc_core::ecma::ast::{
  ArrayLit, ArrowExpr, BinExpr, BinaryOp, BlockStmt, BlockStmtOrExpr, CallExpr, Callee, CondExpr,
  Expr, ExprOrSpread, Ident, KeyValueProp, Lit, MemberExpr, MemberProp, ObjectLit, OptChainBase,
  Pat, Prop, PropName, PropOrSpread, ReturnStmt, SpreadElement, Stmt, TaggedTpl, Tpl, TplElement,
  TsType, UnaryExpr, UnaryOp,
};
use swc_core::ecma::codegen::text_writer::JsWriter;
use swc_core::ecma::codegen::{Config, Emitter, Node};
use swc_core::ecma::utils::ExprExt;

use crate::css_map::{CssMapUsage, visit_css_map_path_with_builder};
use crate::types::{Metadata, MetadataContext};
use crate::utils_ast::build_code_frame_error;
use crate::utils_create_result_pair::create_result_pair;
use crate::utils_css::{CssValue, add_unit_if_needed, css_affix_interpolation, kebab_case};
use crate::utils_css_map::{ErrorMessages, create_error_message};
use crate::utils_evaluate_expression::evaluate_expression;
use crate::utils_hash::hash;
use crate::utils_is_compiled::{
  is_compiled_css_call_expression, is_compiled_css_map_call_expression,
  is_compiled_css_tagged_template_expression, is_compiled_keyframes_call_expression,
  is_compiled_keyframes_tagged_template_expression,
};
use crate::utils_is_empty::is_empty_value;
use crate::utils_manipulate_template_literal::{
  has_nested_template_literals_with_conditional_rules, is_quasi_mid_statement,
  optimize_conditional_statement, recompose_template_literal,
};
use crate::utils_normalize_props_usage::normalize_props_usage;
use crate::utils_object_property_to_string::{
  can_be_statically_concatenated, expression_to_string, expression_type, object_property_to_string,
};
use crate::utils_resolve_binding::resolve_binding;
use crate::utils_types::{
  BindingSource, ConditionalCssItem, CssItem, CssMapItem, CssOutput, LogicalCssItem,
  LogicalOperator, PartialBindingWithMeta, SheetCssItem, UnconditionalCssItem, Variable,
};

fn print_expression(expr: &Expr) -> String {
  let cm: Lrc<SourceMap> = Default::default();
  let mut buffer = Vec::new();

  {
    let mut writer = JsWriter::new(cm.clone(), "\n", &mut buffer, None);
    writer.set_indent_str("  ");
    let mut emitter = Emitter {
      cfg: Config::default(),
      comments: None,
      cm,
      wr: writer,
    };

    expr.emit_with(&mut emitter).expect("emit expression");
  }

  String::from_utf8(buffer).expect("expression to utf8 string")
}

fn print_statement(stmt: &Stmt) -> String {
  let cm: Lrc<SourceMap> = Default::default();
  let mut buffer = Vec::new();

  {
    let mut writer = JsWriter::new(cm.clone(), "\n", &mut buffer, None);
    writer.set_indent_str("  ");
    let mut emitter = Emitter {
      cfg: Config::default(),
      comments: None,
      cm,
      wr: writer,
    };

    stmt.emit_with(&mut emitter).expect("emit statement");
  }

  String::from_utf8(buffer).expect("statement to utf8 string")
}

fn print_ts_type(ty: &TsType) -> String {
  let cm: Lrc<SourceMap> = Default::default();
  let mut buffer = Vec::new();

  {
    let mut writer = JsWriter::new(cm.clone(), "\n", &mut buffer, None);
    writer.set_indent_str("  ");
    let mut emitter = Emitter {
      cfg: Config::default(),
      comments: None,
      cm,
      wr: writer,
    };

    ty.emit_with(&mut emitter).expect("emit ts type");
  }

  String::from_utf8(buffer).expect("ts type to utf8 string")
}

fn print_pattern(pat: &Pat) -> String {
  let cm: Lrc<SourceMap> = Default::default();
  let mut buffer = Vec::new();

  {
    let mut writer = JsWriter::new(cm.clone(), "\n", &mut buffer, None);
    writer.set_indent_str("  ");
    let mut emitter = Emitter {
      cfg: Config::default(),
      comments: None,
      cm,
      wr: writer,
    };

    pat.emit_with(&mut emitter).expect("emit pattern");
  }

  String::from_utf8(buffer).expect("pattern to utf8 string")
}

fn babel_like_code_for_hash(expr: &Expr) -> String {
  // Aim to mirror Babel generator output for the expression shapes that
  // appear in keyframes hashing. This serializer purposefully does not fall
  // back to SWC codegen to avoid drift from Babel formatting.

  fn escape_string(value: &str, quote: char) -> String {
    let mut out = String::with_capacity(value.len() + 8);
    for ch in value.chars() {
      match ch {
        '\\' => out.push_str("\\\\"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        c if c == quote => {
          out.push('\\');
          out.push(quote);
        }
        _ => out.push(ch),
      }
    }
    out
  }

  fn print_object_pretty(obj: &ObjectLit, indent: usize) -> String {
    let pad_inner = " ".repeat(indent + 2);
    let mut entries: Vec<String> = Vec::new();
    for prop in &obj.props {
      if let PropOrSpread::Prop(p) = prop {
        if let Prop::KeyValue(kv) = p.as_ref() {
          let key = match &kv.key {
            PropName::Ident(i) => i.sym.as_ref().to_string(),
            PropName::Str(s) => {
              format!("'{}'", escape_string(s.value.as_ref(), '\''))
            }
            PropName::Num(n) => {
              let mut s = n.value.to_string();
              if s.ends_with(".0") {
                s.truncate(s.len() - 2);
              }
              s
            }
            PropName::Computed(c) => print_expr(&c.expr),
            PropName::BigInt(bi) => bi.value.to_string(),
          };
          let value = match &*kv.value {
            Expr::Object(inner) => {
              let inner_str = print_object_pretty(inner, indent + 2);
              format!("{{\n{}\n{}}}", inner_str, pad_inner)
            }
            other => print_expr(other),
          };
          entries.push(format!("{}{}: {}", pad_inner, key, value));
        }
      }
    }
    entries.join(",\n")
  }

  fn print_expr(e: &Expr) -> String {
    match e {
      Expr::Ident(id) => id.sym.as_ref().to_string(),
      Expr::Lit(Lit::Num(n)) => {
        let mut s = n.value.to_string();
        if s.ends_with(".0") {
          s.truncate(s.len() - 2);
        }
        s
      }
      Expr::Lit(Lit::Str(s)) => format!("'{}'", escape_string(s.value.as_ref(), '\'')),
      Expr::Lit(Lit::Bool(b)) => {
        if b.value {
          "true".to_string()
        } else {
          "false".to_string()
        }
      }
      Expr::Lit(Lit::Null(_)) => "null".to_string(),
      Expr::Lit(Lit::BigInt(bi)) => format!("{}n", bi.value),
      Expr::Lit(Lit::Regex(regex)) => format!("/{}/{}", regex.exp, regex.flags),
      Expr::Object(obj) => {
        if obj.props.is_empty() {
          "{}".to_string()
        } else {
          format!("{{\n{}\n}}", print_object_pretty(obj, 0))
        }
      }
      Expr::Array(arr) => {
        let mut items: Vec<String> = Vec::new();
        for el in &arr.elems {
          if let Some(el) = el {
            items.push(print_expr(&el.expr));
          }
        }
        format!("[{}]", items.join(", "))
      }
      Expr::Call(call) => {
        let callee = match &call.callee {
          Callee::Expr(c) => print_expr(c.as_ref()),
          _ => "".to_string(),
        };
        let mut args: Vec<String> = Vec::new();
        for a in &call.args {
          args.push(print_expr(&a.expr));
        }
        format!("{}({})", callee, args.join(", "))
      }
      Expr::TaggedTpl(tagged) => {
        // Print tag identifier and raw template contents preserving quasis
        let mut out = String::new();
        match tagged.tag.as_ref() {
          Expr::Ident(id) => out.push_str(id.sym.as_ref()),
          other => out.push_str(&print_expr(other)),
        }
        out.push('`');
        let tpl = &tagged.tpl;
        for (i, quasi) in tpl.quasis.iter().enumerate() {
          out.push_str(quasi.raw.as_ref());
          if i < tpl.exprs.len() {
            out.push_str("${");
            out.push_str(&print_expr(&tpl.exprs[i]));
            out.push('}');
          }
        }
        out.push('`');
        out
      }
      Expr::Tpl(tpl) => {
        let mut out = String::new();
        out.push('`');
        for (i, quasi) in tpl.quasis.iter().enumerate() {
          out.push_str(quasi.raw.as_ref());
          if i < tpl.exprs.len() {
            out.push_str("${");
            out.push_str(&print_expr(&tpl.exprs[i]));
            out.push('}');
          }
        }
        out.push('`');
        out
      }
      Expr::Member(member) => {
        let obj = print_expr(&member.obj);
        match &member.prop {
          MemberProp::Ident(prop) => format!("{}.{}", obj, prop.sym.as_ref()),
          MemberProp::Computed(c) => format!("{}[{}]", obj, print_expr(&c.expr)),
          MemberProp::PrivateName(_) => {
            panic!("unsupported private name in member expression for keyframes hash")
          }
        }
      }
      Expr::TsAs(ts_as) => format!(
        "{} as {}",
        print_expr(&ts_as.expr),
        print_ts_type(&ts_as.type_ann)
      ),
      Expr::TsTypeAssertion(assertion) => format!(
        "<{}> {}",
        print_ts_type(&assertion.type_ann),
        print_expr(&assertion.expr)
      ),
      Expr::TsConstAssertion(assertion) => {
        format!("{} as const", print_expr(&assertion.expr))
      }
      Expr::TsNonNull(non_null) => format!("{}!", print_expr(&non_null.expr)),
      Expr::Paren(p) => format!("({})", print_expr(&p.expr)),
      Expr::Unary(un) => {
        let op = match un.op {
          UnaryOp::Minus => "-",
          UnaryOp::Plus => "+",
          UnaryOp::Bang => "!",
          UnaryOp::Tilde => "~",
          UnaryOp::TypeOf => "typeof ",
          UnaryOp::Void => "void ",
          UnaryOp::Delete => "delete ",
        };
        format!("{}{}", op, print_expr(&un.arg))
      }
      Expr::Bin(bin) => {
        // Basic binary printing with spaces around operator.
        let op = match bin.op {
          BinaryOp::EqEq => "==",
          BinaryOp::NotEq => "!=",
          BinaryOp::EqEqEq => "===",
          BinaryOp::NotEqEq => "!==",
          BinaryOp::Lt => "<",
          BinaryOp::LtEq => "<=",
          BinaryOp::Gt => ">",
          BinaryOp::GtEq => ">=",
          BinaryOp::LShift => "<<",
          BinaryOp::RShift => ">>",
          BinaryOp::ZeroFillRShift => ">>>",
          BinaryOp::Add => "+",
          BinaryOp::Sub => "-",
          BinaryOp::Mul => "*",
          BinaryOp::Div => "/",
          BinaryOp::Mod => "%",
          BinaryOp::BitOr => "|",
          BinaryOp::BitXor => "^",
          BinaryOp::BitAnd => "&",
          BinaryOp::LogicalOr => "||",
          BinaryOp::LogicalAnd => "&&",
          BinaryOp::In => "in",
          BinaryOp::InstanceOf => "instanceof",
          BinaryOp::Exp => "**",
          BinaryOp::NullishCoalescing => "??",
        };
        format!(
          "{} {} {}",
          print_expr(&bin.left),
          op,
          print_expr(&bin.right)
        )
      }
      Expr::Cond(cond) => format!(
        "{} ? {} : {}",
        print_expr(&cond.test),
        print_expr(&cond.cons),
        print_expr(&cond.alt)
      ),
      Expr::Arrow(arrow) => {
        let params_code = format_arrow_params(arrow);
        let body_code = match arrow.body.as_ref() {
          BlockStmtOrExpr::Expr(body) => {
            let body = strip_parentheses_expr(body);
            babel_like_code_for_hash(body)
          }
          BlockStmtOrExpr::BlockStmt(block) => {
            let stmt = Stmt::Block(block.clone());
            print_statement(&stmt).trim().to_string()
          }
        };
        format!("{params_code} => {body_code}")
      }
      Expr::OptChain(opt) => match opt.base.as_ref() {
        OptChainBase::Member(member) => {
          let obj = print_expr(&member.obj);
          let accessor = match &member.prop {
            MemberProp::Ident(prop) => {
              format!(
                "{}{}",
                if opt.optional { "?." } else { "." },
                prop.sym.as_ref()
              )
            }
            MemberProp::Computed(c) => format!(
              "{}[{}]",
              if opt.optional { "?." } else { "." },
              print_expr(&c.expr)
            ),
            MemberProp::PrivateName(_) => {
              panic!("unsupported private name in optional chain for keyframes hash")
            }
          };
          format!("{obj}{accessor}")
        }
        OptChainBase::Call(call) => {
          let callee = print_expr(&call.callee);
          let mut args: Vec<String> = Vec::with_capacity(call.args.len());
          for arg in &call.args {
            args.push(print_expr(&arg.expr));
          }
          let call_prefix = if opt.optional { "?." } else { "" };
          format!("{callee}{call_prefix}({})", args.join(", "))
        }
      },
      other => panic!(
        "unsupported expression in keyframes hash serialization: {:?}",
        other
      ),
    }
  }

  print_expr(expr)
}

fn format_arrow_params(arrow: &ArrowExpr) -> String {
  if arrow.params.is_empty() {
    return "()".to_string();
  }

  if arrow.params.len() == 1 {
    if let Pat::Ident(binding) = &arrow.params[0] {
      if binding.type_ann.is_none() && !binding.optional {
        return binding.id.sym.as_ref().to_string();
      }
    }
  }

  let mut params: Vec<String> = Vec::with_capacity(arrow.params.len());
  for param in &arrow.params {
    params.push(print_pattern(param).trim().to_string());
  }

  format!("({})", params.join(", "))
}

enum BlockItem<'a> {
  Stmt(&'a Stmt),
  Comment { kind: CommentKind, text: String },
}

fn extract_comment_entries<'a>(snippet: &str, block: &BlockStmt) -> Vec<(u32, BlockItem<'a>)> {
  let mut entries = Vec::new();
  let bytes = snippet.as_bytes();
  let mut index = 0usize;
  let base = block.span.lo.0;

  while index + 1 < bytes.len() {
    if bytes[index] == b'/' && bytes[index + 1] == b'/' {
      let start = index;
      index += 2;
      let start_idx = index;
      while index < bytes.len() && bytes[index] != b'\n' {
        index += 1;
      }
      let text = snippet[start_idx..index].to_string();
      entries.push((
        base + start as u32,
        BlockItem::Comment {
          kind: CommentKind::Line,
          text,
        },
      ));
    } else if bytes[index] == b'/' && bytes[index + 1] == b'*' {
      let start = index;
      index += 2;
      let start_idx = index;
      while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
        index += 1;
      }
      let text = if index + 1 < bytes.len() {
        let slice = &snippet[start_idx..index];
        index += 2;
        slice.to_string()
      } else {
        snippet[start_idx..].to_string()
      };
      entries.push((
        base + start as u32,
        BlockItem::Comment {
          kind: CommentKind::Block,
          text,
        },
      ));
    } else {
      index += 1;
    }
  }

  entries
}

fn format_block_like_babel(block: &BlockStmt, meta: &Metadata) -> String {
  let snippet = meta
    .state()
    .file()
    .source_map
    .span_to_snippet(block.span)
    .ok();

  if block.stmts.is_empty() {
    return "{\n}".into();
  }

  let mut entries: Vec<(u32, BlockItem)> = block
    .stmts
    .iter()
    .map(|stmt| (stmt.span().lo.0, BlockItem::Stmt(stmt)))
    .collect();

  if let Some(ref snippet) = snippet {
    entries.extend(extract_comment_entries(snippet, block));
  }

  entries.sort_by_key(|(pos, _)| *pos);

  let mut out = String::from("{\n");
  for (_, item) in entries {
    match item {
      BlockItem::Stmt(stmt) => {
        let stmt_code = print_statement(stmt).trim().replace('\n', "\n  ");
        out.push_str("  ");
        out.push_str(&stmt_code);
        if !stmt_code.ends_with(';') && !stmt_code.ends_with('}') {
          out.push(';');
        }
        out.push('\n');
      }
      BlockItem::Comment { kind, text } => {
        out.push_str("  ");
        match kind {
          CommentKind::Line => {
            out.push_str("//");
            let starts_with_ws = text
              .chars()
              .next()
              .map(|ch| ch.is_whitespace())
              .unwrap_or(false);
            if !starts_with_ws && !text.is_empty() {
              out.push(' ');
            }
            out.push_str(&text);
          }
          CommentKind::Block => {
            out.push_str("/*");
            out.push_str(&text);
            out.push_str("*/");
          }
        }
        out.push('\n');
      }
    }
  }
  out.push('}');
  out
}

fn babel_like_expression(expr: &Expr, meta: &Metadata) -> String {
  if let Expr::Arrow(arrow) = expr {
    let params_code = format_arrow_params(arrow);
    match arrow.body.as_ref() {
      BlockStmtOrExpr::Expr(body) => {
        let body_code = babel_like_code_for_hash(body);
        return format!("{} => {}", params_code, body_code);
      }
      BlockStmtOrExpr::BlockStmt(block) => {
        let body_code = format_block_like_babel(block, meta);
        return format!("{} => {}", params_code, body_code);
      }
    }
  }

  babel_like_code_for_hash(expr)
}

fn strip_parentheses<'a>(mut expr: &'a mut Expr) -> &'a mut Expr {
  loop {
    match expr {
      Expr::Paren(paren) => {
        expr = paren.expr.as_mut();
      }
      _ => return expr,
    }
  }
}

fn strip_parentheses_expr<'a>(mut expr: &'a Expr) -> &'a Expr {
  loop {
    match expr {
      Expr::Paren(paren) => {
        expr = paren.expr.as_ref();
      }
      _ => return expr,
    }
  }
}

fn call_arguments_as_array(call: &CallExpr) -> Expr {
  let elements = call
    .args
    .iter()
    .map(|arg| {
      if arg.spread.is_some() {
        panic!("Spread elements are not supported in keyframes arguments");
      }

      Some(ExprOrSpread {
        spread: None,
        expr: arg.expr.clone(),
      })
    })
    .collect();

  Expr::Array(ArrayLit {
    span: call.span,
    elems: elements,
  })
}

fn is_logical_expression(expr: &Expr) -> bool {
  if let Expr::Bin(bin) = expr {
    matches!(
      bin.op,
      BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing
    )
  } else {
    false
  }
}

fn normalize_content_value(value: &str) -> String {
  static CONTENT_VALUE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?s)^([A-Za-z\-]+\(.+|.*-quote|inherit|initial|none|normal|revert|unset)(\s|$)")
      .expect("valid content value regex")
  });

  if value.is_empty() {
    return String::from("\"\"");
  }

  if value.contains('"') || value.contains('\'') || CONTENT_VALUE_PATTERN.is_match(value) {
    value.to_string()
  } else {
    format!("\"{}\"", value)
  }
}

fn is_custom_property_name(value: &str) -> bool {
  value.starts_with("--")
}

fn get_variable_declarator_value_for_parent_expr(expr: &Expr, meta: &Metadata) -> (Expr, String) {
  let mut expression = expr.clone();
  let mut variable_name = babel_like_expression(expr, meta);

  if let Expr::Ident(ident) = expr {
    let base_name = ident.sym.as_ref();
    variable_name = match &meta.context {
      MetadataContext::Keyframes { keyframe } => format!("{keyframe}:{base_name}"),
      _ => base_name.to_string(),
    };

    if let Some(binding) = resolve_binding(base_name, meta.clone(), evaluate_expression) {
      if let Some(node) = &binding.node {
        expression = node.clone();
      }
    }
  }

  (expression, variable_name)
}

/// Merge consecutive unconditional CSS items while preserving the position of
/// any sheet entries. This mirrors the behaviour of the Babel helper and is
/// relied upon when normalising conditional CSS branches.
pub fn merge_subsequent_unconditional_css_items(items: Vec<CssItem>) -> Vec<CssItem> {
  let mut merged: Vec<CssItem> = Vec::new();
  let mut sheets: Vec<CssItem> = Vec::new();

  let mut index = 0usize;
  while index < items.len() {
    match &items[index] {
      CssItem::Sheet(_) => sheets.push(items[index].clone()),
      CssItem::Unconditional(_) => {
        let mut css = get_item_css(&items[index]);
        if css.trim_end().ends_with(':') && !css.contains('{') {
          // Treat selector-looking prefixes as empty so they don't corrupt following
          // declarations when merged (mirrors Babel which keeps them separate and inert).
          let has_selector_chars = css.contains('[')
            || css.contains(']')
            || css.contains('.')
            || css.contains('#')
            || css.contains('&')
            || css.contains(' ');
          if has_selector_chars {
            css.clear();
          }
        }
        let mut last_index = index;

        let mut lookahead = index + 1;
        while lookahead < items.len() {
          match &items[lookahead] {
            CssItem::Unconditional(_) => {
              css.push_str(&get_item_css(&items[lookahead]));
              last_index = lookahead;
            }
            CssItem::Sheet(_) => sheets.push(items[lookahead].clone()),
            _ => break,
          }
          lookahead += 1;
        }

        merged.push(CssItem::unconditional(css));
        index = last_index;
      }
      _ => merged.push(items[index].clone()),
    }

    index += 1;
  }

  sheets.into_iter().chain(merged.into_iter()).collect()
}

/// Helper that serialises a `CssItem` into the raw CSS string it represents.
/// This matches the behaviour of the Babel helper so downstream utilities can
/// reuse it during native transformations.
pub fn get_item_css(item: &CssItem) -> String {
  match item {
    CssItem::Conditional(conditional) => {
      let mut css = get_item_css(&conditional.consequent);
      css.push_str(&get_item_css(&conditional.alternate));
      css
    }
    CssItem::Unconditional(unconditional) => unconditional.css.clone(),
    CssItem::Logical(logical) => logical.css.clone(),
    CssItem::Sheet(sheet) => sheet.css.clone(),
    CssItem::Map(map) => map.css.clone(),
  }
}

/// Mirrors the Babel `generateCacheForCSSMap` helper by warming the cssMap cache for a given
/// identifier when possible. Returns `true` when the cache was populated.
pub fn generate_cache_for_css_map_with_builder<F>(
  identifier: &Ident,
  meta: &Metadata,
  build_css: &mut F,
) -> bool
where
  F: FnMut(&Expr, &Metadata) -> CssOutput,
{
  let name = identifier.sym.as_ref().to_string();

  {
    let state = meta.state();
    if state.css_map.contains_key(&name) || state.ignore_member_expressions.contains(&name) {
      return false;
    }
  }

  let resolved = resolve_binding(name.as_str(), meta.clone(), evaluate_expression);

  if let Some(PartialBindingWithMeta {
    node: Some(node),
    meta: binding_meta,
    ..
  }) = resolved
  {
    let is_css_map_call = {
      let state_ref = binding_meta.state();
      is_compiled_css_map_call_expression(&node, &state_ref)
    };

    if is_css_map_call {
      if let Expr::Call(call) = &node {
        visit_css_map_path_with_builder(
          CssMapUsage::Call(call),
          Some(identifier),
          &binding_meta,
          |expr, metadata| build_css(expr, metadata),
        );

        let has_cache = meta.state().css_map.contains_key(&name);
        if !has_cache {
          meta
            .state_mut()
            .ignore_member_expressions
            .insert(name.clone());
        }

        return has_cache;
      }
    }
  }

  meta.state_mut().ignore_member_expressions.insert(name);
  false
}

fn find_binding_identifier(expr: &Expr) -> Option<Ident> {
  match expr {
    Expr::Ident(ident) => Some(ident.clone()),
    Expr::Call(call) => match &call.callee {
      Callee::Expr(callee) => find_binding_identifier(callee),
      _ => None,
    },
    Expr::Member(member) => find_binding_identifier(member.obj.as_ref()),
    _ => None,
  }
}

fn callback_if_file_included(meta: &Metadata, next: &Metadata) {
  let should_include = {
    let current_filename = meta.state().filename.clone();
    let next_filename = next.state().filename.clone();
    current_filename != next_filename
  };

  if should_include {
    if let Some(location) = next.state().file().loc.as_ref() {
      meta
        .state_mut()
        .included_files
        .push(location.filename.clone());
    }
  }
}

fn assert_no_imported_css_variables(
  reference: &Expr,
  meta: &Metadata,
  binding: &PartialBindingWithMeta,
  result: &CssOutput,
) {
  if binding.source == BindingSource::Import && !result.variables.is_empty() {
    let error = build_code_frame_error(
      "Identifier contains values that can't be statically evaluated",
      Some(reference.span()),
      meta,
    );
    panic!("{error}");
  }
}

enum ConditionalBranch {
  Consequent,
  Alternate,
}

fn logical_items_from_conditional_expression(
  css: Vec<CssItem>,
  node: &CondExpr,
  branch: ConditionalBranch,
) -> Vec<CssItem> {
  css
    .into_iter()
    .map(|item| match item {
      CssItem::Conditional(_) => item,
      CssItem::Logical(logical) => {
        let mut span = logical.expression.span();
        if span == DUMMY_SP {
          span = node.test.span();
        }

        let expression = Expr::Bin(BinExpr {
          span,
          op: logical.operator.to_binary_op(),
          left: Box::new((*node.test).clone()),
          right: Box::new(logical.expression.clone()),
        });

        CssItem::Logical(LogicalCssItem {
          expression,
          operator: logical.operator,
          css: logical.css,
        })
      }
      _ => {
        let expression = match branch {
          ConditionalBranch::Consequent => (*node.test).clone(),
          ConditionalBranch::Alternate => Expr::Unary(UnaryExpr {
            span: node.test.span(),
            op: UnaryOp::Bang,
            arg: Box::new((*node.test).clone()),
          }),
        };

        CssItem::Logical(LogicalCssItem {
          expression,
          operator: LogicalOperator::And,
          css: get_item_css(&item),
        })
      }
    })
    .collect()
}

pub fn extract_member_expression_with_builder<F>(
  member: &MemberExpr,
  meta: &Metadata,
  fallback_to_evaluate: bool,
  build_css: &mut F,
) -> Option<CssOutput>
where
  F: FnMut(&Expr, &Metadata) -> CssOutput,
{
  if let Some(identifier) = find_binding_identifier(&Expr::Member(member.clone())) {
    let _ = generate_cache_for_css_map_with_builder(&identifier, meta, build_css);

    let has_cache = {
      let state = meta.state();
      state.css_map.contains_key(identifier.sym.as_ref())
    };

    if has_cache {
      let name = identifier.sym.as_ref().to_string();
      return Some(CssOutput {
        css: vec![CssItem::Map(CssMapItem {
          name,
          expression: Expr::Member(member.clone()),
          css: String::new(),
        })],
        variables: Vec::new(),
      });
    }
  }

  if fallback_to_evaluate {
    if let Some(identifier) = find_binding_identifier(&Expr::Member(member.clone())) {
      if let Some(binding) =
        resolve_binding(identifier.sym.as_ref(), meta.clone(), evaluate_expression)
      {
        if let Some(mut node) = binding.node.clone() {
          normalize_props_usage(&mut node);
          let state = binding.meta.state();
          let compiled = is_compiled_css_tagged_template_expression(&node, &state)
            || is_compiled_css_call_expression(&node, &state);
          drop(state);

          if compiled {
            let result = build_css(&node, &binding.meta);
            assert_no_imported_css_variables(
              &Expr::Member(member.clone()),
              meta,
              &binding,
              &result,
            );
            return Some(result);
          } else if let Expr::Object(obj) = &node {
            let has_nested_object_values = obj.props.iter().any(|prop| {
              matches!(prop, swc_core::ecma::ast::PropOrSpread::Prop(p) if matches!(p.as_ref(), swc_core::ecma::ast::Prop::KeyValue(kv) if matches!(kv.value.as_ref(), Expr::Object(_))))
            });
            if has_nested_object_values && matches!(member.prop, MemberProp::Computed(_)) {
              return Some(build_css(&node, &binding.meta));
            }
          }
        }
      }
    }

    let pair = evaluate_expression(&Expr::Member(member.clone()), meta.clone());
    // Avoid infinite recursion when evaluation makes no progress (e.g., dynamic
    // computed members that stay as Member expressions).
    if let Expr::Member(evaluated) = &pair.value {
      if evaluated.span == member.span {
        return Some(CssOutput::new());
      }
    }

    return Some(build_css(&pair.value, &pair.meta));
  }

  None
}

pub fn extract_template_literal_with_builder<F>(
  node: &Tpl,
  meta: &Metadata,
  build_css: &mut F,
) -> CssOutput
where
  F: FnMut(&Expr, &Metadata) -> CssOutput,
{
  let mut template = node.clone();
  let has_property_prefix = template
    .quasis
    .iter()
    .any(|q| q.raw.as_ref().trim_end().ends_with(':'));

  if std::env::var("STACK_DEBUG").is_ok() {
    let raws: Vec<String> = template
      .quasis
      .iter()
      .map(|q| q.raw.as_ref().to_string())
      .collect();
    eprintln!("[template-start] raws={:?}", raws);
  }

  for index in 0..template.exprs.len() {
    if !is_quasi_mid_statement(&template.quasis[index]) {
      continue;
    }

    let has_nested = has_nested_template_literals_with_conditional_rules(&template, meta);

    let Some(expression) = template.exprs.get_mut(index) else {
      continue;
    };

    let Expr::Arrow(arrow) = expression.as_mut() else {
      continue;
    };

    let has_conditional_body = match arrow.body.as_mut() {
      BlockStmtOrExpr::Expr(body) => {
        let already_conditional = matches!(body.as_ref(), Expr::Cond(_));
        let expr = strip_parentheses(body.as_mut());
        let is_conditional = matches!(expr, Expr::Cond(_));
        if is_conditional && !already_conditional {
          *body = Box::new(expr.clone());
        }
        is_conditional
      }
      _ => false,
    };

    if !has_conditional_body {
      continue;
    }

    if has_nested {
      continue;
    }

    if template.quasis.len() <= index + 1 {
      continue;
    }

    let (head, tail) = template.quasis.split_at_mut(index + 1);
    let quasi = &mut head[index];
    if let Some(next_quasi) = tail.first_mut() {
      optimize_conditional_statement(quasi, Some(next_quasi), arrow);
    }
  }

  let mut css: Vec<CssItem> = Vec::new();
  let mut variables: Vec<Variable> = Vec::new();
  let mut literal_result = String::new();

  for index in 0..template.quasis.len() {
    let raw = template.quasis[index].raw.as_ref().to_string();
    let node_expression = template.exprs.get(index).map(|expr| expr.as_ref());
    let is_mid_statement = is_quasi_mid_statement(&template.quasis[index]);

    let arrow_has_logical_body = node_expression
      .and_then(|expr| match expr {
        Expr::Arrow(arrow) => {
          if let BlockStmtOrExpr::Expr(body) = arrow.body.as_ref() {
            Some(is_logical_expression(body))
          } else {
            Some(false)
          }
        }
        _ => None,
      })
      .unwrap_or(false);

    if std::env::var("STACK_DEBUG").is_ok() {
      let expr_kind = match node_expression {
        Some(Expr::Arrow(_)) => "Arrow",
        Some(Expr::Cond(_)) => "Cond",
        Some(Expr::Object(_)) => "Object",
        Some(Expr::Tpl(_)) => "Tpl",
        Some(Expr::Lit(Lit::Str(_))) => "Str",
        Some(Expr::Lit(Lit::Num(_))) => "Num",
        Some(_) => "Other",
        None => "None",
      };
      eprintln!(
        "[template] node_expr kind={} raw=\"{}\" literal_prefix=\"{}\"",
        expr_kind, raw, literal_result
      );
    }

    if node_expression.is_none() || arrow_has_logical_body {
      let suffix = match meta.context {
        MetadataContext::Keyframes { .. } | MetadataContext::Fragment => "",
        MetadataContext::Root => ";",
      };

      literal_result.push_str(&raw);
      literal_result.push_str(suffix);
      continue;
    }

    let node_expression = node_expression.unwrap();
    // COMPAT: For computed member expressions (e.g. colors[2]) do not attempt
    // to evaluate statically like Babel; keep them dynamic so they are emitted
    // as CSS variables rather than inlined literal strings.
    let force_variable = matches!(node_expression, Expr::Member(m) if matches!(m.prop, swc_core::ecma::ast::MemberProp::Computed(_)));
    let evaluated = if force_variable {
      create_result_pair((*node_expression).clone(), meta.clone())
    } else {
      evaluate_expression(node_expression, meta.clone())
    };
    callback_if_file_included(meta, &evaluated.meta);

    match &evaluated.value {
      Expr::Lit(Lit::Str(str_lit)) => {
        literal_result.push_str(&raw);
        literal_result.push_str(str_lit.value.as_ref());
        continue;
      }
      Expr::Lit(Lit::Num(num_lit)) => {
        literal_result.push_str(&raw);
        literal_result.push_str(&num_lit.value.to_string());
        continue;
      }
      Expr::Call(call) => {
        // Inline simple Math.* calls when arguments are numeric after evaluation
        use swc_core::ecma::ast::{
          Callee, Expr as E2, Ident as I2, MemberExpr as M2, MemberProp as P2,
        };
        if let Callee::Expr(callee_expr) = &call.callee {
          if let E2::Member(M2 { obj, prop, .. }) = &**callee_expr {
            if let E2::Ident(I2 { sym: obj_sym, .. }) = &**obj {
              if obj_sym.as_ref() == "Math" {
                if let P2::Ident(name) = prop {
                  let method = name.sym.as_ref();
                  let ctx = swc_core::ecma::utils::ExprCtx {
                    unresolved_ctxt: swc_core::common::SyntaxContext::empty(),
                    is_unresolved_ref_safe: false,
                    in_strict: false,
                  };
                  let mut nums: Vec<f64> = Vec::new();
                  for arg in &call.args {
                    let ev = evaluate_expression(&arg.expr, evaluated.meta.clone());
                    let arg_expr = ev.value;
                    // We don't have direct access to the internal try_static_evaluate here;
                    // rely on ExprExt as_pure_number on the evaluated form.
                    if let swc_core::ecma::utils::Value::Known(n) = arg_expr.as_pure_number(&ctx) {
                      nums.push(n);
                    } else {
                      nums.clear();
                      break;
                    }
                  }
                  if !nums.is_empty() {
                    let result = match method {
                      "max" => nums.into_iter().fold(f64::NEG_INFINITY, f64::max),
                      "min" => nums.into_iter().fold(f64::INFINITY, f64::min),
                      "abs" => nums.get(0).copied().map(f64::abs).unwrap_or(0.0),
                      "ceil" => nums.get(0).copied().map(f64::ceil).unwrap_or(0.0),
                      "floor" => nums.get(0).copied().map(f64::floor).unwrap_or(0.0),
                      "round" => nums.get(0).copied().map(f64::round).unwrap_or(0.0),
                      _ => f64::NAN,
                    };
                    if result.is_finite() || result.is_nan() {
                      literal_result.push_str(&raw);
                      literal_result.push_str(&result.to_string());
                      continue;
                    }
                  }
                }
              }
            }
          }
        }
      }
      _ => {}
    }

    let does_expression_have_conditional_css = matches!(
      node_expression,
      Expr::Arrow(arrow) if matches!(
          arrow.body.as_ref(),
          BlockStmtOrExpr::Expr(expr) if matches!(
              strip_parentheses_expr(expr.as_ref()),
              Expr::Cond(_)
          )
      )
    );

    let state = evaluated.meta.state();
    let does_expression_contain_css_block = matches!(evaluated.value, Expr::Object(_))
      || is_compiled_css_tagged_template_expression(&evaluated.value, &state)
      || is_compiled_css_call_expression(&evaluated.value, &state);
    drop(state);

    let conditional_branches_look_like_css = if does_expression_have_conditional_css {
      let looks_like_css_literal = |expr: &Expr, state: &_| {
        matches!(expr, Expr::Object(_))
          || matches!(expr, Expr::Tpl(tpl) if tpl.quasis.iter().any(|q| q.raw.as_ref().contains(':')))
          || matches!(expr, Expr::Lit(Lit::Str(str_lit)) if str_lit.value.contains(':'))
          || is_compiled_css_tagged_template_expression(expr, state)
          || is_compiled_css_call_expression(expr, state)
      };

      let maybe_cond = match node_expression {
        Expr::Arrow(arrow) => {
          if let BlockStmtOrExpr::Expr(body) = arrow.body.as_ref() {
            if let Expr::Cond(cond) = strip_parentheses_expr(body.as_ref()) {
              Some(cond)
            } else {
              None
            }
          } else {
            None
          }
        }
        Expr::Cond(cond) => Some(cond),
        _ => None,
      };

      let state = evaluated.meta.state();
      let looks = maybe_cond.map_or(false, |cond| {
        looks_like_css_literal(&cond.cons, &state) || looks_like_css_literal(&cond.alt, &state)
      });
      drop(state);
      looks
    } else {
      false
    };

    let property_like_prefix = {
      let mut key = raw.trim_end_matches(':').trim().to_string();
      if key.is_empty() {
        if has_property_prefix {
          key = "prop".to_string();
        } else if let Some(first) = template.quasis.first() {
          key = first.raw.as_ref().trim_end_matches(':').trim().to_string();
        }
      }
      if std::env::var("STACK_DEBUG").is_ok() {
        eprintln!(
          "[template-prop] raw=\"{}\" first=\"{}\" key=\"{}\"",
          raw,
          template
            .quasis
            .first()
            .map(|q| q.raw.as_ref())
            .unwrap_or_default(),
          key
        );
      }
      !key.is_empty()
        && !key.chars().any(|ch| {
          matches!(
            ch,
            '['
              | ']'
              | '.'
              | '#'
              | '&'
              | '>'
              | '+'
              | '~'
              | ':'
              | ' '
              | '\t'
              | '\n'
              | '\r'
              | '{'
              | '}'
              | '('
              | ')'
              | '@'
              | ','
          )
        })
    };

    // COMPAT: Avoid treating computed member lookups (e.g. () => MAP[size]) that evaluate
    // to an object literal as CSS blocks. Babel keeps these as value interpolations so the
    // result flows through the variable path instead of being expanded into CSS text.
    let is_computed_member_arrow_returning_object = matches!(
      (node_expression, &evaluated.value),
      (
        Expr::Arrow(arrow),
        Expr::Object(_)
      ) if matches!(
        arrow.body.as_ref(),
        BlockStmtOrExpr::Expr(expr) if matches!(
            strip_parentheses_expr(expr.as_ref()),
            Expr::Member(member) if matches!(member.prop, MemberProp::Computed(_))
        )
      )
    );

    let avoid_mid_statement_css_block = is_mid_statement
      && does_expression_contain_css_block
      && property_like_prefix
      && !is_computed_member_arrow_returning_object;

    let can_build_expression_as_css = (does_expression_contain_css_block
      && !is_computed_member_arrow_returning_object)
      // COMPAT: Treat conditional expressions as CSS only when their branches look like CSS
      // so value conditionals (e.g. padding ternaries) stay on the CSS variable path while
      // selector-ish conditionals (like in the calendar header) expand to CSS.
      || (does_expression_have_conditional_css
        && !is_computed_member_arrow_returning_object
        && (conditional_branches_look_like_css
          || does_expression_contain_css_block
          || property_like_prefix))
      || matches!(node_expression, Expr::Tpl(_));

    if std::env::var("STACK_DEBUG").is_ok() {
      eprintln!(
        "[template-debug] raw=\"{}\" mid={} cond_css={} cond_branches={} prop_like={} css_block={} avoid_block={} can_build={}",
        raw,
        is_mid_statement,
        does_expression_have_conditional_css,
        conditional_branches_look_like_css,
        property_like_prefix,
        does_expression_contain_css_block,
        avoid_mid_statement_css_block,
        can_build_expression_as_css
      );
    }

    if !avoid_mid_statement_css_block && can_build_expression_as_css {
      if std::env::var("STACK_DEBUG").is_ok() {
        eprintln!(
          "[template] conditional_css={} css_block={} tpl={}",
          does_expression_have_conditional_css,
          does_expression_contain_css_block,
          matches!(node_expression, Expr::Tpl(_))
        );
      }
      let nested_meta = meta.with_context(MetadataContext::Fragment);
      let build_meta = if matches!(node_expression, Expr::Tpl(_)) {
        nested_meta
      } else {
        evaluated.meta.clone()
      };
      let result = build_css(&evaluated.value, &build_meta);

      if std::env::var("STACK_DEBUG").is_ok() {
        eprintln!(
          "[template] result_len={} prefix=\"{}\" raw=\"{}\"",
          result.css.len(),
          literal_result,
          raw
        );
      }

      if !result.css.is_empty() {
        if std::env::var("STACK_DEBUG").is_ok() {
          let kinds: Vec<&'static str> = result
            .css
            .iter()
            .map(|item| match item {
              CssItem::Unconditional(_) => "Unconditional",
              CssItem::Conditional(_) => "Conditional",
              CssItem::Logical(_) => "Logical",
              CssItem::Sheet(_) => "Sheet",
              CssItem::Map(_) => "Map",
            })
            .collect();
          eprintln!("[template] css_items={:?}", kinds);
        }
        let prefix = format!("{literal_result}{raw}");
        if !prefix.is_empty() {
          css.push(CssItem::unconditional(prefix));
        }

        css.extend(result.css);
        variables.extend(result.variables);
        literal_result.clear();
        continue;
      }
    }

    let state = evaluated.meta.state();
    let is_keyframes = is_compiled_keyframes_call_expression(&evaluated.value, &state)
      || is_compiled_keyframes_tagged_template_expression(&evaluated.value, &state);
    drop(state);

    if is_keyframes {
      let mut keyframes_output =
        extract_keyframes_with_builder(&evaluated.value, &evaluated.meta, &raw, "", build_css);

      if let Some(sheet) = keyframes_output.css.get(0) {
        css.push(sheet.clone());
      }

      if let Some(unconditional) = keyframes_output.css.get(1) {
        literal_result.push_str(&get_item_css(unconditional));
      }

      variables.append(&mut keyframes_output.variables);
      continue;
    }

    let (mut variable_expression, variable_name) =
      get_variable_declarator_value_for_parent_expr(node_expression, meta);
    normalize_props_usage(&mut variable_expression);
    let Some(next_quasi) = template.quasis.get_mut(index + 1) else {
      panic!("Template literal missing trailing quasi for interpolation");
    };

    let (before, after) = css_affix_interpolation(&raw, next_quasi.raw.as_ref());

    next_quasi.raw = after.css.clone().into();
    next_quasi.cooked = Some(after.css.clone().into());

    let mut name = format!("--_{}", hash(&variable_name));
    if before.variable_prefix == "-" {
      name.push('-');
    }
    if let Ok(label) = std::env::var("DEBUG_CSS_FIXTURE") {
      if let Some(filename) = &meta.state().filename {
        if filename.contains(&label) {
          eprintln!(
            "[css-debug] fixture={label} var_name={} hashed={}",
            variable_name, name
          );
        }
      }
    }

    // If the interpolation fully reduces to a static literal, inline it instead of creating
    // a CSS variable. This mirrors Babel, which inlines template literals whose expressions
    // are resolved locally.
    if let Expr::Lit(lit) = &evaluated.value {
      let value_text = match lit {
        swc_core::ecma::ast::Lit::Str(str_lit) => str_lit.value.to_string(),
        swc_core::ecma::ast::Lit::Num(num_lit) => num_lit.value.to_string(),
        _ => String::new(),
      };
      if !value_text.is_empty() {
        literal_result.push_str(&before.css);
        literal_result.push_str(&value_text);
        next_quasi.raw = after.css.clone().into();
        next_quasi.cooked = Some(after.css.clone().into());
        continue;
      }
    }

    variables.push(Variable {
      name: name.clone(),
      expression: variable_expression,
      prefix: if before.variable_prefix.is_empty() {
        None
      } else {
        Some(before.variable_prefix.clone())
      },
      suffix: if after.variable_suffix.is_empty() {
        None
      } else {
        Some(after.variable_suffix.clone())
      },
    });

    literal_result.push_str(&before.css);
    literal_result.push_str(&format!("var({name})"));
  }

  css.push(CssItem::unconditional(literal_result));

  for expression in &template.exprs {
    if let Expr::Arrow(arrow) = expression.as_ref() {
      if let BlockStmtOrExpr::Expr(body) = arrow.body.as_ref() {
        if is_logical_expression(body) {
          let pair = evaluate_expression(body, meta.clone());
          callback_if_file_included(meta, &pair.meta);
          let result = build_css(&pair.value, &pair.meta);
          css.extend(result.css);
          variables.extend(result.variables);
        }
      }
    }
  }

  let merged = merge_subsequent_unconditional_css_items(css);

  if std::env::var("STACK_DEBUG").is_ok() {
    let items: Vec<String> = merged
      .iter()
      .map(|item| {
        let kind = match item {
          CssItem::Unconditional(_) => "Unconditional",
          CssItem::Conditional(_) => "Conditional",
          CssItem::Logical(_) => "Logical",
          CssItem::Sheet(_) => "Sheet",
          CssItem::Map(_) => "Map",
        };
        format!("{kind}:{}", get_item_css(item))
      })
      .collect();
    eprintln!("[template] merged_css {:?}", items);
  }

  CssOutput {
    css: merged,
    variables,
  }
}

pub fn extract_logical_expression_with_builder<F>(
  arrow: &ArrowExpr,
  meta: &Metadata,
  build_css: &mut F,
) -> CssOutput
where
  F: FnMut(&Expr, &Metadata) -> CssOutput,
{
  let mut css: Vec<CssItem> = Vec::new();
  let mut variables: Vec<Variable> = Vec::new();

  if let BlockStmtOrExpr::Expr(body_expr) = arrow.body.as_ref() {
    let pair = evaluate_expression(body_expr, meta.clone());
    let result = build_css(&pair.value, &pair.meta);

    callback_if_file_included(meta, &pair.meta);

    css.extend(result.css);
    variables.extend(result.variables);
  }

  CssOutput {
    css: merge_subsequent_unconditional_css_items(css),
    variables,
  }
}

pub fn extract_conditional_expression_with_builder<F>(
  node: &CondExpr,
  meta: &Metadata,
  build_css: &mut F,
) -> CssOutput
where
  F: FnMut(&Expr, &Metadata) -> CssOutput,
{
  let mut css: Vec<CssItem> = Vec::new();
  let mut variables: Vec<Variable> = Vec::new();

  let process_branch = |expr: &Expr,
                        meta: &Metadata,
                        build_css: &mut F,
                        variables: &mut Vec<Variable>|
   -> Option<CssItem> {
    let mut css_output: Option<CssOutput> = None;

    if let Expr::Lit(Lit::Str(str_lit)) = expr {
      css_output = Some(CssOutput {
        css: vec![CssItem::unconditional(str_lit.value.as_ref())],
        variables: Vec::new(),
      });
    } else if let Expr::Lit(Lit::Num(num_lit)) = expr {
      css_output = Some(CssOutput {
        css: vec![CssItem::unconditional(&num_lit.value.to_string())],
        variables: Vec::new(),
      });
    } else {
      let looks_like_css_literal = if matches!(expr, Expr::Object(_)) {
        true
      } else if let Expr::Lit(Lit::Str(str_lit)) = expr {
        str_lit.value.contains(':')
      } else if let Expr::Tpl(tpl) = expr {
        tpl
          .quasis
          .iter()
          .any(|quasi| quasi.raw.as_ref().contains(':'))
      } else {
        false
      };

      if looks_like_css_literal {
        css_output = Some(build_css(expr, meta));
      } else {
        let state = meta.state();
        let is_compiled_css = is_compiled_css_tagged_template_expression(expr, &state)
          || is_compiled_css_call_expression(expr, &state);
        drop(state);

        if is_compiled_css {
          let mut cloned = expr.clone();
          normalize_props_usage(&mut cloned);
          css_output = Some(build_css(&cloned, meta));
        } else if let Expr::Ident(identifier) = expr {
          if let Some(binding) =
            resolve_binding(identifier.sym.as_ref(), meta.clone(), evaluate_expression)
          {
            if let Some(mut node) = binding.node.clone() {
              normalize_props_usage(&mut node);
              let state = binding.meta.state();
              let compiled = is_compiled_css_tagged_template_expression(&node, &state)
                || is_compiled_css_call_expression(&node, &state);
              drop(state);

              if compiled {
                let result = build_css(&node, &binding.meta);
                assert_no_imported_css_variables(expr, meta, &binding, &result);
                css_output = Some(result);
              }
            }
          }
        } else if let Expr::Cond(inner_conditional) = expr {
          css_output = Some(extract_conditional_expression_with_builder(
            inner_conditional,
            meta,
            build_css,
          ));
        } else if let Expr::Member(member_expr) = expr {
          css_output = extract_member_expression_with_builder(member_expr, meta, false, build_css);
        }
      }
    }

    if let Some(mut output) = css_output {
      variables.append(&mut output.variables);
      let merged = merge_subsequent_unconditional_css_items(output.css);

      if merged.len() > 1 {
        let error = build_code_frame_error(
          "Conditional branch contains unexpected expression",
          Some(expr.span()),
          meta,
        );
        panic!("{error}");
      }

      return merged.into_iter().next();
    }

    None
  };

  // Treat branches that produce an effectively empty declaration as absent so downstream
  // logic mirrors Babel's folding of single-sided conditionals into logical expressions.
  fn css_text(item: &CssItem) -> String {
    match item {
      CssItem::Conditional(c) => {
        let mut s = css_text(&c.consequent);
        s.push_str(&css_text(&c.alternate));
        s
      }
      CssItem::Unconditional(u) => u.css.clone(),
      CssItem::Logical(l) => l.css.clone(),
      CssItem::Sheet(s) => s.css.clone(),
      // Mirror Babel: cssMap branches are considered meaningful even when the backing css
      // string is empty, so don't let them be folded away.
      CssItem::Map(m) => m.css.clone(),
    }
  }

  fn is_effectively_empty(item: &CssItem) -> bool {
    match item {
      // CssMap branches are always considered meaningful, even when their backing css
      // string is empty, so never treat them as "effectively empty".
      CssItem::Map(_) => false,
      // A conditional is only effectively empty when both branches are empty. This
      // ensures conditionals that eventually resolve to cssMap branches are preserved
      // rather than collapsed into single-sided logical expressions.
      CssItem::Conditional(c) => {
        is_effectively_empty(&c.consequent) && is_effectively_empty(&c.alternate)
      }
      _ => {
        let css = css_text(item);
        let trimmed = css.trim();
        if trimmed.is_empty() {
          return true;
        }
        if let Some(idx) = trimmed.find(':') {
          let value = trimmed[idx + 1..].trim().trim_end_matches(';').trim();
          return value.is_empty();
        }
        false
      }
    }
  }

  let mut consequent_css = process_branch(&node.cons, meta, build_css, &mut variables);
  if let Some(ref item) = consequent_css {
    if is_effectively_empty(item) {
      consequent_css = None;
    }
  }

  let mut alternate_css = process_branch(&node.alt, meta, build_css, &mut variables);
  if let Some(ref item) = alternate_css {
    if is_effectively_empty(item) {
      alternate_css = None;
    }
  }

  match (consequent_css, alternate_css) {
    (Some(consequent), Some(alternate)) => {
      css.push(CssItem::Conditional(ConditionalCssItem {
        test: (*node.test).clone(),
        consequent: Box::new(consequent),
        alternate: Box::new(alternate),
      }));
    }
    (Some(consequent), None) => css.extend(logical_items_from_conditional_expression(
      vec![consequent],
      node,
      ConditionalBranch::Consequent,
    )),
    (None, Some(alternate)) => css.extend(logical_items_from_conditional_expression(
      vec![alternate],
      node,
      ConditionalBranch::Alternate,
    )),
    (None, None) => {}
  }

  CssOutput { css, variables }
}

/// Extracts CSS rules from a keyframes expression while reusing the provided builder
/// for nested evaluation. Mirrors the Babel `extractKeyframes` helper by hashing the
/// expression source to produce a deterministic animation name and wrapping the
/// generated output in an `@keyframes` rule.
pub fn extract_keyframes_with_builder<F>(
  expression: &Expr,
  meta: &Metadata,
  prefix: &str,
  suffix: &str,
  build_css: &mut F,
) -> CssOutput
where
  F: FnMut(&Expr, &Metadata) -> CssOutput,
{
  // COMPAT: Babel computes the keyframe name by hashing `generate(expression).code`.
  // Our SWC port originally stringified the expression using swc_ecma_codegen which can
  // differ subtly (whitespace/formatting) from Babel, leading to different hashes.
  // To stay faithful to the original, prefer hashing the exact source snippet for the
  // expression span when available; this most closely mirrors Babel's generator output
  // for the subset we use (TaggedTemplate and Call with literals), and ensures stable
  // cross-implementation hashing. If a snippet can't be retrieved, fall back to SWC printing.
  // Use Babel-like serialization for hashing to match original plugin
  let mut code = babel_like_expression(expression, meta);
  // COMPAT: Normalize newlines
  if code.contains('\r') {
    code = code.replace("\r\n", "\n").replace('\r', "");
  }
  let name = format!("k{}", hash(&code));
  let selector = format!("@keyframes {name}");

  let keyframe_meta = meta.with_context(MetadataContext::Keyframes {
    keyframe: name.clone(),
  });

  let inner_output = match expression {
    Expr::Call(call) => {
      let array_expr = call_arguments_as_array(call);
      build_css(&array_expr, &keyframe_meta)
    }
    Expr::TaggedTpl(TaggedTpl { tpl, .. }) => {
      build_css(&Expr::Tpl((**tpl).clone()), &keyframe_meta)
    }
    Expr::Tpl(tpl) => build_css(&Expr::Tpl(tpl.clone()), &keyframe_meta),
    _ => build_css(expression, &keyframe_meta),
  };

  let wrapped = to_css_rule(&selector, &inner_output);

  if wrapped
    .css
    .iter()
    .any(|item| !matches!(item, CssItem::Unconditional(_)))
  {
    let error = build_code_frame_error(
      "Keyframes contains unexpected CSS",
      Some(expression.span()),
      meta,
    );
    panic!("{error}");
  }

  let sheet_css = wrapped
    .css
    .iter()
    .map(|item| get_item_css(item))
    .collect::<String>();

  CssOutput {
    css: vec![
      CssItem::Sheet(SheetCssItem { css: sheet_css }),
      CssItem::Unconditional(UnconditionalCssItem {
        css: format!("{prefix}{name}{suffix}"),
      }),
    ],
    variables: wrapped.variables,
  }
}

fn css_property_name(key: &str) -> String {
  if is_custom_property_name(key) {
    key.to_string()
  } else {
    kebab_case(key)
  }
}

pub fn extract_array_with_builder<F>(
  array: &ArrayLit,
  meta: &Metadata,
  build_css: &mut F,
) -> CssOutput
where
  F: FnMut(&Expr, &Metadata) -> CssOutput,
{
  let mut css: Vec<CssItem> = Vec::new();
  let mut variables: Vec<Variable> = Vec::new();

  for element in &array.elems {
    let Some(element) = element else {
      let message = "undefined isn't a supported CSS type - try using an object or string";
      let error = build_code_frame_error(message, Some(array.span), meta);
      panic!("{error}");
    };

    if element.spread.is_some() {
      let error = build_code_frame_error(
        "SpreadElement isn't a supported CSS type - try using an object or string",
        Some(element.expr.span()),
        meta,
      );
      panic!("{error}");
    }

    let expr = element.expr.as_ref();

    let result = if let Expr::Cond(cond) = expr {
      extract_conditional_expression_with_builder(cond, meta, build_css)
    } else {
      build_css(expr, meta)
    };

    css.extend(result.css);
    variables.extend(result.variables);
  }

  CssOutput { css, variables }
}

pub fn extract_object_expression_with_builder<F>(
  object: &ObjectLit,
  meta: &Metadata,
  build_css: &mut F,
) -> CssOutput
where
  F: FnMut(&Expr, &Metadata) -> CssOutput,
{
  let mut css: Vec<CssItem> = Vec::new();
  let mut variables: Vec<Variable> = Vec::new();

  for property in &object.props {
    match property {
      PropOrSpread::Prop(prop) => {
        let mut synthesized: Option<KeyValueProp> = None;
        let key_value: &KeyValueProp = match prop.as_ref() {
          Prop::KeyValue(key_value) => key_value,
          Prop::Shorthand(ident) => synthesized.insert(KeyValueProp {
            key: PropName::Ident(ident.clone().into()),
            value: Box::new(Expr::Ident(ident.clone())),
          }),
          _ => continue,
        };

        let key = object_property_to_string(key_value, meta.clone());
        let evaluated = evaluate_expression(key_value.value.as_ref(), meta.clone());
        let mut prop_value = evaluated.value;
        let updated_meta = evaluated.meta;

        if std::env::var("COMPILED_TRACE_PROP_EVAL").is_ok() {
          eprintln!(
            "[compiled][prop-eval] key={} before={} after={} span={:?}",
            key,
            expression_type(key_value.value.as_ref()),
            expression_type(&prop_value),
            key_value.value.span()
          );
        }

        callback_if_file_included(meta, &updated_meta);

        if let Expr::Lit(Lit::Str(str_lit)) = &prop_value {
          let value = if key == "content" {
            normalize_content_value(str_lit.value.as_ref())
          } else {
            str_lit.value.as_ref().to_string()
          };

          css.push(CssItem::unconditional(format!(
            "{}: {};",
            css_property_name(&key),
            value
          )));

          continue;
        }

        if let Expr::Call(call) = &prop_value {
          if can_be_statically_concatenated(call) {
            if std::env::var("STACK_DEBUG_PROP").is_ok() {
              eprintln!(
                "[concat-prop] key={} expr_type={} span={:?}",
                key,
                super::utils_object_property_to_string::expression_type(&prop_value),
                prop_value.span()
              );
            }
            let value = expression_to_string(&prop_value, updated_meta.clone());
            let value = if key == "content" {
              normalize_content_value(&value)
            } else {
              value
            };

            css.push(CssItem::unconditional(format!(
              "{}: {};",
              css_property_name(&key),
              value
            )));

            continue;
          }
        }

        if let Expr::Lit(Lit::Num(num_lit)) = &prop_value {
          css.push(CssItem::unconditional(format!(
            "{}: {};",
            css_property_name(&key),
            add_unit_if_needed(&key, CssValue::Number(num_lit.value))
          )));

          continue;
        }

        // COMPAT: Mirror Babel behavior by emitting a runtime CSS variable
        // for boolean literal values inside object styles (e.g. `inherits: false`
        // within an `@property` block). This allows the value to be provided at
        // runtime via inline style while preserving identical hashing/IO.
        if let Expr::Lit(Lit::Bool(_)) = &prop_value {
          let (mut variable_expression, variable_name) =
            get_variable_declarator_value_for_parent_expr(&prop_value, &updated_meta);
          normalize_props_usage(&mut variable_expression);

          let name = format!("--_{}", hash(&variable_name));

          variables.push(Variable {
            name: name.clone(),
            expression: variable_expression,
            prefix: None,
            suffix: None,
          });

          css.push(CssItem::unconditional(format!(
            "{}: var({});",
            css_property_name(&key),
            name
          )));

          continue;
        }

        if is_empty_value(&prop_value) {
          continue;
        }

        let logical_expression = matches!(
            prop_value,
            Expr::Bin(ref bin)
                if matches!(
                    bin.op,
                    BinaryOp::LogicalAnd
                        | BinaryOp::LogicalOr
                        | BinaryOp::NullishCoalescing
                )
        );

        if matches!(prop_value, Expr::Object(_)) || logical_expression {
          let result = build_css(&prop_value, &updated_meta);
          let mapped = if logical_expression {
            to_css_rule(&key, &result)
          } else {
            to_css_rule(&key, &result)
          };
          css.extend(mapped.css);
          variables.extend(mapped.variables);
          continue;
        }

        if let Expr::Tpl(template) = &prop_value {
          let result = if template.exprs.len() == 1 {
            if let Some(first_expr) = template.exprs.first() {
              if let Expr::Arrow(arrow) = first_expr.as_ref() {
                if matches!(
                    arrow.body.as_ref(),
                    BlockStmtOrExpr::Expr(body)
                        if matches!(**body, Expr::Cond(_))
                ) {
                  let mut optimized = template.clone();
                  recompose_template_literal(
                    &mut optimized,
                    &format!("{}:", css_property_name(&key)),
                    ";",
                  );
                  extract_template_literal_with_builder(&optimized, &updated_meta, build_css)
                } else {
                  to_css_declaration(
                    &key,
                    &extract_template_literal_with_builder(template, &updated_meta, build_css),
                  )
                }
              } else {
                to_css_declaration(
                  &key,
                  &extract_template_literal_with_builder(template, &updated_meta, build_css),
                )
              }
            } else {
              to_css_declaration(
                &key,
                &extract_template_literal_with_builder(template, &updated_meta, build_css),
              )
            }
          } else {
            to_css_declaration(
              &key,
              &extract_template_literal_with_builder(template, &updated_meta, build_css),
            )
          };

          css.extend(result.css);
          variables.extend(result.variables);
          continue;
        }

        if let Expr::Arrow(mut arrow) = prop_value.clone() {
          if std::env::var("COMPILED_TRACE_ARROW").is_ok() {
            eprintln!(
              "[compiled][arrow] key={} params={} entry",
              key,
              arrow.params.len()
            );
          }
          if std::env::var("COMPILED_TRACE_ARROW").is_ok() && key.starts_with('&') {
            let body_expr: Expr = match arrow.body.as_ref() {
              swc_core::ecma::ast::BlockStmtOrExpr::BlockStmt(_) => {
                Expr::Ident(Ident::new("block".into(), DUMMY_SP, SyntaxContext::empty()))
              }
              swc_core::ecma::ast::BlockStmtOrExpr::Expr(expr) => (**expr).clone(),
            };
            eprintln!(
              "[compiled][arrow] key={} params={} body_kind={}",
              key,
              arrow.params.len(),
              expression_type(&body_expr)
            );
          }

          // If the function returns a literal early (e.g., an if/return block), mirror
          // Babel by treating the property as static using the first literal return.
          fn literal_from_stmt(stmt: &Stmt) -> Option<Expr> {
            match stmt {
              Stmt::Return(ReturnStmt { arg: Some(arg), .. }) => match arg.as_ref() {
                Expr::Lit(_) => Some((**arg).clone()),
                Expr::Tpl(tpl) if tpl.exprs.is_empty() => Some((**arg).clone()),
                _ => None,
              },
              Stmt::Block(block) => {
                for inner in &block.stmts {
                  if let Some(found) = literal_from_stmt(inner) {
                    return Some(found);
                  }
                }
                None
              }
              Stmt::If(if_stmt) => {
                if let Some(found) = literal_from_stmt(&*if_stmt.cons) {
                  return Some(found);
                }
                if let Some(alt) = &if_stmt.alt {
                  return literal_from_stmt(alt);
                }
                None
              }
              _ => None,
            }
          }

          fn literal_return_value(arrow: &ArrowExpr) -> Option<Expr> {
            match arrow.body.as_ref() {
              BlockStmtOrExpr::BlockStmt(block) => {
                for stmt in &block.stmts {
                  if let Some(found) = literal_from_stmt(stmt) {
                    return Some(found);
                  }
                }
                None
              }
              BlockStmtOrExpr::Expr(expr) => match expr.as_ref() {
                Expr::Lit(_) => Some((**expr).clone()),
                Expr::Tpl(tpl) if tpl.exprs.is_empty() => Some((**expr).clone()),
                _ => None,
              },
            }
          }

          fn literal_return_from_function(expr: &Expr, meta: &Metadata) -> Option<Expr> {
            match expr {
              Expr::Arrow(arrow) => literal_return_value(arrow),
              Expr::Fn(fn_expr) => {
                if let Some(body) = &fn_expr.function.body {
                  for stmt in &body.stmts {
                    if let Stmt::Return(ReturnStmt { arg: Some(arg), .. }) = stmt {
                      match arg.as_ref() {
                        Expr::Lit(_) => return Some((**arg).clone()),
                        Expr::Tpl(tpl) if tpl.exprs.is_empty() => return Some((**arg).clone()),
                        _ => {}
                      }
                    }
                  }
                }
                None
              }
              Expr::Ident(ident) => {
                resolve_binding(ident.sym.as_ref(), meta.clone(), evaluate_expression).and_then(
                  |binding| {
                    binding.node.as_ref().and_then(|node| {
                      let lit = literal_return_from_function(node, meta);
                      if let Some(ref value) = lit {
                        if std::env::var("COMPILED_TRACE_ARROW").is_ok() {
                          eprintln!(
                            "[compiled][literal-return] ident={} using bound function lit_kind={}",
                            ident.sym,
                            expression_type(value)
                          );
                        }
                      }
                      lit
                    })
                  },
                )
              }
              _ => None,
            }
          }

          let literal_return = literal_return_value(&arrow);

          if literal_return.is_none() {
            if let BlockStmtOrExpr::Expr(body) = arrow.body.as_ref() {
              if let Expr::Call(call) = body.as_ref() {
                if let Callee::Expr(callee_expr) = &call.callee {
                  if let Expr::Ident(ident) = callee_expr.as_ref() {
                    if let Some(binding) = resolve_binding(
                      ident.sym.as_ref(),
                      updated_meta.clone(),
                      evaluate_expression,
                    ) {
                      if let Some(node) = binding.node.as_ref() {
                        if let Some(lit) = literal_return_from_function(node, &binding.meta) {
                          if std::env::var("COMPILED_TRACE_ARROW").is_ok() {
                            eprintln!(
                              "[compiled][literal-return] via binding ident={} lit_kind={}",
                              ident.sym,
                              expression_type(&lit)
                            );
                          }
                          prop_value = lit;
                        }
                      }
                    }
                  }
                  if let Some(lit) =
                    literal_return_from_function(callee_expr.as_ref(), &updated_meta)
                  {
                    if std::env::var("COMPILED_TRACE_ARROW").is_ok() {
                      eprintln!(
                        "[compiled][literal-return] direct callee kind={} lit_kind={}",
                        expression_type(callee_expr.as_ref()),
                        expression_type(&lit)
                      );
                    }
                    prop_value = lit;
                  }
                }
              }
            }
          }

          // If the arrow directly returns an object expression, mirror Babel by treating it
          // as a nested rule (keeping dynamic values as runtime variables) instead of
          // serializing the selector as a property name.
          if let BlockStmtOrExpr::Expr(body) = arrow.body.as_ref() {
            let mut inner = body.as_ref();
            while let Expr::Paren(paren) = inner {
              inner = &paren.expr;
            }
            if let Expr::Object(object) = inner {
              let result = build_css_internal(&Expr::Object(object.clone()), &updated_meta);
              let mapped = to_css_rule(&key, &result);
              css.extend(mapped.css);
              variables.extend(mapped.variables);
              continue;
            }
          }

          if let Some(lit) = literal_return {
            prop_value = lit;
          }

          if let Expr::Lit(Lit::Str(str_lit)) = &prop_value {
            css.push(CssItem::unconditional(format!(
              "{}: {};",
              css_property_name(&key),
              str_lit.value.as_ref()
            )));
            continue;
          }

          if let Expr::Lit(Lit::Num(num_lit)) = &prop_value {
            css.push(CssItem::unconditional(format!(
              "{}: {};",
              css_property_name(&key),
              add_unit_if_needed(&key, CssValue::Number(num_lit.value))
            )));
            continue;
          }
          enum TemplateInfo {
            Direct {
              span: swc_core::common::Span,
            },
            FromTemplate {
              span: swc_core::common::Span,
              quasis: Vec<TplElement>,
            },
          }

          let mut info: Option<TemplateInfo> = None;

          if let BlockStmtOrExpr::Expr(body) = arrow.body.as_mut() {
            let expr = strip_parentheses(body.as_mut());
            match expr {
              Expr::Cond(_) => {
                info = Some(TemplateInfo::Direct { span: arrow.span });
              }
              Expr::Tpl(inner_tpl) => {
                if inner_tpl.exprs.len() == 1 {
                  if let Some(first_expr) = inner_tpl.exprs.first() {
                    if matches!(first_expr.as_ref(), Expr::Cond(_)) {
                      let (span, quasis, conditional) = {
                        let span = inner_tpl.span;
                        let quasis = inner_tpl.quasis.clone();
                        let conditional = first_expr.as_ref().clone();
                        (span, quasis, conditional)
                      };
                      // COMPAT: Babel doesn't preserve ParenthesisExpression
                      // wrappers around arrow bodies, so strip them when
                      // rewriting to keep behaviour identical.
                      *body = Box::new(conditional);
                      info = Some(TemplateInfo::FromTemplate { span, quasis });
                    }
                  }
                }
              }
              _ => {}
            }
          }

          if let Some(template_info) = info {
            let mut optimized = match template_info {
              TemplateInfo::Direct { span } => Tpl {
                span,
                exprs: vec![Box::new(Expr::Arrow(arrow.clone()))],
                quasis: vec![
                  TplElement {
                    span,
                    tail: false,
                    cooked: Some("".into()),
                    raw: "".into(),
                  },
                  TplElement {
                    span,
                    tail: true,
                    cooked: Some("".into()),
                    raw: "".into(),
                  },
                ],
              },
              TemplateInfo::FromTemplate { span, quasis } => Tpl {
                span,
                exprs: vec![Box::new(Expr::Arrow(arrow.clone()))],
                quasis,
              },
            };

            recompose_template_literal(
              &mut optimized,
              &format!("{}:", css_property_name(&key)),
              ";",
            );
            let result =
              extract_template_literal_with_builder(&optimized, &updated_meta, build_css);
            css.extend(result.css);
            variables.extend(result.variables);
            continue;
          }
        }

        let is_keyframes = {
          let state = updated_meta.state();
          let result = is_compiled_keyframes_call_expression(&prop_value, &state)
            || is_compiled_keyframes_tagged_template_expression(&prop_value, &state);
          result
        };

        if is_keyframes {
          let result = extract_keyframes_with_builder(
            &prop_value,
            &updated_meta,
            &format!("{}: ", css_property_name(&key)),
            ";",
            build_css,
          );
          css.extend(result.css);
          variables.extend(result.variables);
          continue;
        }

        let (mut variable_expression, variable_name) =
          get_variable_declarator_value_for_parent_expr(&prop_value, &updated_meta);
        normalize_props_usage(&mut variable_expression);
        let name = format!("--_{}", hash(&variable_name));
        if let Ok(label) = std::env::var("DEBUG_CSS_FIXTURE") {
          if let Some(filename) = &updated_meta.state().filename {
            if filename.contains(&label) {
              eprintln!(
                "[css-debug] fixture={label} var_name={} hashed={} json={:?}",
                variable_name, name, variable_name
              );
            }
          }
        }

        variables.push(Variable {
          name: name.clone(),
          expression: variable_expression,
          prefix: None,
          suffix: None,
        });

        css.push(CssItem::unconditional(format!(
          "{}: var({name});",
          css_property_name(&key)
        )));
      }
      PropOrSpread::Spread(SpreadElement { expr, .. }) => {
        let binding = if let Expr::Ident(identifier) = expr.as_ref() {
          resolve_binding(identifier.sym.as_ref(), meta.clone(), evaluate_expression).or_else(
            || {
              let error =
                build_code_frame_error("Variable could not be found", Some(identifier.span), meta);
              panic!("{error}");
            },
          )
        } else {
          None
        };

        let evaluated = evaluate_expression(expr, meta.clone());
        let result = build_css(&evaluated.value, &evaluated.meta);

        callback_if_file_included(meta, &evaluated.meta);

        if let Some(binding) = &binding {
          assert_no_imported_css_variables(expr, meta, binding, &result);
        }

        css.extend(result.css);
        variables.extend(result.variables);
      }
    }
  }

  CssOutput {
    css: merge_subsequent_unconditional_css_items(css),
    variables,
  }
}

fn wrap_with_selector(selector: &str, css: String) -> String {
  format!("{selector} {{ {css} }}")
}

fn to_css_rule_internal(selector: &str, item: &CssItem) -> CssItem {
  match item {
    CssItem::Conditional(conditional) => CssItem::Conditional(ConditionalCssItem {
      test: conditional.test.clone(),
      consequent: Box::new(to_css_rule_internal(selector, &conditional.consequent)),
      alternate: Box::new(to_css_rule_internal(selector, &conditional.alternate)),
    }),
    CssItem::Unconditional(unconditional) => CssItem::Unconditional(UnconditionalCssItem {
      css: wrap_with_selector(selector, unconditional.css.clone()),
    }),
    CssItem::Logical(logical) => CssItem::Logical(LogicalCssItem {
      expression: logical.expression.clone(),
      operator: logical.operator,
      css: wrap_with_selector(selector, logical.css.clone()),
    }),
    CssItem::Sheet(sheet) => CssItem::Sheet(SheetCssItem {
      css: wrap_with_selector(selector, sheet.css.clone()),
    }),
    CssItem::Map(map) => CssItem::Map(CssMapItem {
      name: map.name.clone(),
      expression: map.expression.clone(),
      css: wrap_with_selector(selector, map.css.clone()),
    }),
  }
}

/// Map the CSS output to rule blocks for the provided selector. Mirrors the
/// behaviour of the Babel helper which recursively wraps each item while
/// preserving conditional branches.
pub fn to_css_rule(selector: &str, result: &CssOutput) -> CssOutput {
  let css = result
    .css
    .iter()
    .map(|item| to_css_rule_internal(selector, item))
    .collect();

  CssOutput {
    css,
    variables: result.variables.clone(),
  }
}

fn declaration_css(key: &str, css: String) -> String {
  format!("{}: {};", kebab_case(key), css)
}

fn to_css_declaration_internal(key: &str, item: &CssItem) -> CssItem {
  match item {
    CssItem::Sheet(sheet) => CssItem::Sheet(sheet.clone()),
    CssItem::Conditional(conditional) => CssItem::Conditional(ConditionalCssItem {
      test: conditional.test.clone(),
      consequent: Box::new(to_css_declaration_internal(key, &conditional.consequent)),
      alternate: Box::new(to_css_declaration_internal(key, &conditional.alternate)),
    }),
    CssItem::Unconditional(unconditional) => CssItem::Unconditional(UnconditionalCssItem {
      css: declaration_css(key, unconditional.css.clone()),
    }),
    CssItem::Logical(logical) => CssItem::Logical(LogicalCssItem {
      expression: logical.expression.clone(),
      operator: logical.operator,
      css: declaration_css(key, logical.css.clone()),
    }),
    CssItem::Map(map) => CssItem::Map(CssMapItem {
      name: map.name.clone(),
      expression: map.expression.clone(),
      css: declaration_css(key, map.css.clone()),
    }),
  }
}

/// Convert the CSS output into property declarations for the provided key,
/// mirroring the Babel helper that kebab-cases the property name and reuses the
/// existing item shape.
pub fn to_css_declaration(key: &str, result: &CssOutput) -> CssOutput {
  let css = result
    .css
    .iter()
    .map(|item| to_css_declaration_internal(key, item))
    .collect();

  CssOutput {
    css,
    variables: result.variables.clone(),
  }
}

fn extract_template_literal(node: &Tpl, meta: &Metadata) -> CssOutput {
  let mut build_css = |expr: &Expr, metadata: &Metadata| build_css_internal(expr, metadata);
  extract_template_literal_with_builder(node, meta, &mut build_css)
}

fn extract_member_expression(
  member: &MemberExpr,
  meta: &Metadata,
  fallback: bool,
) -> Option<CssOutput> {
  let mut build_css = |expr: &Expr, metadata: &Metadata| build_css_internal(expr, metadata);
  extract_member_expression_with_builder(member, meta, fallback, &mut build_css)
}

fn extract_conditional_expression(node: &CondExpr, meta: &Metadata) -> CssOutput {
  let mut build_css = |expr: &Expr, metadata: &Metadata| build_css_internal(expr, metadata);
  extract_conditional_expression_with_builder(node, meta, &mut build_css)
}

fn extract_logical_expression(arrow: &ArrowExpr, meta: &Metadata) -> CssOutput {
  let mut build_css = |expr: &Expr, metadata: &Metadata| build_css_internal(expr, metadata);
  extract_logical_expression_with_builder(arrow, meta, &mut build_css)
}

fn extract_keyframes(expression: &Expr, meta: &Metadata, prefix: &str, suffix: &str) -> CssOutput {
  let mut build_css = |expr: &Expr, metadata: &Metadata| build_css_internal(expr, metadata);
  extract_keyframes_with_builder(expression, meta, prefix, suffix, &mut build_css)
}

fn build_css_internal(node: &Expr, meta: &Metadata) -> CssOutput {
  thread_local! {
      static DEBUG_DEPTH: std::cell::Cell<usize> = std::cell::Cell::new(0);
  }

  struct DepthGuard;
  impl DepthGuard {
    fn new() -> Self {
      DEBUG_DEPTH.with(|cell| cell.set(cell.get().saturating_add(1)));
      DepthGuard
    }
  }
  impl Drop for DepthGuard {
    fn drop(&mut self) {
      DEBUG_DEPTH.with(|cell| cell.set(cell.get().saturating_sub(1)));
    }
  }

  let _guard = DepthGuard::new();
  if std::env::var("STACK_DEBUG").is_ok() {
    let depth = DEBUG_DEPTH.with(|cell| cell.get());
    use swc_core::ecma::ast::Expr;
    let kind = match node {
      Expr::Array(_) => "Array",
      Expr::Arrow(_) => "Arrow",
      Expr::Assign(_) => "Assign",
      Expr::Await(_) => "Await",
      Expr::Bin(_) => "Bin",
      Expr::Call(_) => "Call",
      Expr::Cond(_) => "Cond",
      Expr::Fn(_) => "Fn",
      Expr::Lit(Lit::Str(_)) => "StrLit",
      Expr::Lit(Lit::Num(_)) => "NumLit",
      Expr::Lit(Lit::Bool(_)) => "BoolLit",
      Expr::Lit(_) => "OtherLit",
      Expr::Member(_) => "Member",
      Expr::Object(_) => "Object",
      Expr::Tpl(_) => "Tpl",
      Expr::Unary(_) => "Unary",
      _ => "Other",
    };
    eprintln!(
      "[build_css depth={}] kind={} span={:?}",
      depth,
      kind,
      node.span()
    );
    if depth > 200 {
      panic!("build_css depth exceeded");
    }
  }

  if let Expr::Array(array) = node {
    let mut build_css = |expr: &Expr, metadata: &Metadata| build_css_internal(expr, metadata);
    return extract_array_with_builder(array, meta, &mut build_css);
  }

  if let Expr::Lit(Lit::Str(str_lit)) = node {
    return CssOutput {
      css: vec![CssItem::unconditional(str_lit.value.as_ref())],
      variables: Vec::new(),
    };
  }

  if let Expr::TsAs(ts_as) = node {
    return build_css_internal(&ts_as.expr, meta);
  }

  if let Expr::TsConstAssertion(assertion) = node {
    return build_css_internal(&assertion.expr, meta);
  }

  if let Expr::TsTypeAssertion(assertion) = node {
    return build_css_internal(&assertion.expr, meta);
  }

  if let Expr::TsNonNull(non_null) = node {
    return build_css_internal(&non_null.expr, meta);
  }

  if let Expr::Paren(paren) = node {
    return build_css_internal(&paren.expr, meta);
  }

  if let Expr::Tpl(template) = node {
    return extract_template_literal(template, meta);
  }

  if let Expr::Object(object) = node {
    let mut build_css = |expr: &Expr, metadata: &Metadata| build_css_internal(expr, metadata);
    return extract_object_expression_with_builder(object, meta, &mut build_css);
  }

  if let Expr::Member(member) = node {
    return extract_member_expression(member, meta, true).unwrap_or_else(CssOutput::new);
  }

  if let Expr::Arrow(arrow) = node {
    if let BlockStmtOrExpr::Expr(body) = arrow.body.as_ref() {
      let mut body_expr = body.as_ref();
      while let Expr::Paren(paren) = body_expr {
        body_expr = &paren.expr;
      }

      return match body_expr {
        Expr::Object(object) => {
          let mut build_css = |expr: &Expr, metadata: &Metadata| build_css_internal(expr, metadata);
          extract_object_expression_with_builder(object, meta, &mut build_css)
        }
        Expr::Bin(bin)
          if matches!(
            bin.op,
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing
          ) =>
        {
          extract_logical_expression(arrow, meta)
        }
        Expr::Cond(cond) => extract_conditional_expression(cond, meta),
        Expr::Member(member) => {
          extract_member_expression(member, meta, true).unwrap_or_else(CssOutput::new)
        }
        _ => CssOutput::new(),
      };
    }

    return CssOutput::new();
  }

  if let Expr::Ident(identifier) = node {
    if std::env::var("STACK_DEBUG_BINDING").is_ok() {
      let own_keys = meta
        .own_scope()
        .map(|scope| {
          scope
            .borrow()
            .keys()
            .map(|k| k.as_str().to_string())
            .collect::<Vec<String>>()
        })
        .unwrap_or_default();
      let parent_keys = meta
        .parent_scope
        .borrow()
        .keys()
        .map(|k| k.as_str().to_string())
        .collect::<Vec<String>>();
      eprintln!(
        "[build_css][ident] name={} own_scope_keys={:?} parent_scope_keys={:?}",
        identifier.sym, own_keys, parent_keys
      );
    }

    let binding = resolve_binding(identifier.sym.as_ref(), meta.clone(), evaluate_expression)
      .unwrap_or_else(|| {
        let error =
          build_code_frame_error("Variable could not be found", Some(identifier.span), meta);
        panic!("{error}");
      });

    let binding_node = binding.node.clone().unwrap_or_else(|| {
      let error =
        build_code_frame_error("Variable could not be found", Some(identifier.span), meta);
      panic!("{error}");
    });

    {
      let state = meta.state();
      if state.css_map.contains_key(identifier.sym.as_ref()) {
        let message = create_error_message(ErrorMessages::UseVariantOfCssMap.to_string());
        let error = build_code_frame_error(&message, Some(identifier.span), meta);
        panic!("{error}");
      }
    }

    let mut normalized_node = binding_node;
    normalize_props_usage(&mut normalized_node);

    let result = build_css_internal(&normalized_node, &binding.meta);
    assert_no_imported_css_variables(&Expr::Ident(identifier.clone()), meta, &binding, &result);
    callback_if_file_included(meta, &binding.meta);
    return result;
  }

  if let Expr::Cond(cond) = node {
    return extract_conditional_expression(cond, meta);
  }

  if let Expr::Bin(bin) = node {
    if matches!(
      bin.op,
      BinaryOp::LogicalAnd | BinaryOp::LogicalOr | BinaryOp::NullishCoalescing
    ) {
      let expression = (*bin.left).clone();
      let result = build_css_internal(&bin.right, meta);
      let css = result
        .css
        .into_iter()
        .map(|item| match item {
          CssItem::Logical(mut logical) => {
            logical.expression = Expr::Bin(BinExpr {
              span: logical.expression.span(),
              op: logical.operator.to_binary_op(),
              left: Box::new(expression.clone()),
              right: Box::new(logical.expression.clone()),
            });
            CssItem::Logical(logical)
          }
          CssItem::Map(mut map) => {
            map.expression = Expr::Bin(BinExpr {
              span: map.expression.span(),
              op: bin.op,
              left: Box::new(expression.clone()),
              right: Box::new(map.expression.clone()),
            });
            CssItem::Map(map)
          }
          other => CssItem::Logical(LogicalCssItem {
            expression: expression.clone(),
            operator: match bin.op {
              BinaryOp::LogicalOr => LogicalOperator::Or,
              BinaryOp::NullishCoalescing => LogicalOperator::Nullish,
              _ => LogicalOperator::And,
            },
            css: get_item_css(&other),
          }),
        })
        .collect();

      return CssOutput {
        css,
        variables: result.variables,
      };
    }
  }

  if let Expr::TaggedTpl(tagged) = node {
    let state = meta.state();
    if is_compiled_css_tagged_template_expression(node, &state) {
      drop(state);
      return build_css_internal(&Expr::Tpl(*tagged.tpl.clone()), meta);
    }
    drop(state);
  }

  if let Expr::Call(call) = node {
    let state = meta.state();
    if is_compiled_css_call_expression(node, &state) {
      drop(state);
      if let Some(first) = call.args.first() {
        return build_css_internal(&first.expr, meta);
      }
      return CssOutput::new();
    }
    drop(state);
  }

  let state = meta.state();
  let has_imports = state
    .compiled_imports
    .as_ref()
    .map(|imports| {
      !imports.class_names.is_empty()
        || !imports.css.is_empty()
        || !imports.keyframes.is_empty()
        || !imports.styled.is_empty()
        || !imports.css_map.is_empty()
    })
    .unwrap_or(false);
  drop(state);

  let error_message = if has_imports {
    "try to define them statically using Compiled APIs instead"
  } else {
    "no Compiled APIs were found in scope, if you're using createStrictAPI make sure to configure importSources"
  };

  let message = format!(
    "This {} was unable to have its styles extracted — {}",
    expression_type(node),
    error_message
  );
  let error = build_code_frame_error(&message, Some(node.span()), meta);
  panic!("{error}");
}

static INVALID_DYNAMIC_INDIRECT_SELECTOR_REGEX: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"(?s)(\+|~|\||\|\|)[^=\{]+\{[^\}]+var\(--_").expect("valid dynamic selector regex")
});

pub fn build_css(node: &Expr, meta: &Metadata) -> CssOutput {
  let output = build_css_internal(node, meta);

  let has_invalid_selector = output.css.iter().any(|item| {
    matches!(item, CssItem::Unconditional(_) | CssItem::Conditional(_))
      && INVALID_DYNAMIC_INDIRECT_SELECTOR_REGEX.is_match(&get_item_css(item))
  });

  if has_invalid_selector {
    let error = build_code_frame_error(
      "Found a mix of an indirect selector and a dynamic variable which is unsupported with Compiled.  See: https://compiledcssinjs.com/docs/limitations#mixing-dynamic-styles-and-indirect-selectors",
      None,
      meta,
    );
    panic!("{error}");
  }

  output
}

#[cfg(test)]
mod tests {
  use super::{
    assert_no_imported_css_variables, build_css_internal, callback_if_file_included,
    extract_conditional_expression_with_builder, extract_keyframes_with_builder,
    extract_logical_expression_with_builder, extract_member_expression_with_builder,
    extract_template_literal_with_builder, find_binding_identifier,
    generate_cache_for_css_map_with_builder, get_item_css,
    merge_subsequent_unconditional_css_items, print_expression, to_css_declaration, to_css_rule,
  };
  use crate::types::{
    CompiledImports, Metadata, MetadataContext, PluginOptions, TransformFile, TransformFileOptions,
    TransformState,
  };
  use crate::utils_hash::hash;
  use crate::utils_types::{
    BindingSource, ConditionalCssItem, CssItem, CssOutput, LogicalCssItem, LogicalOperator,
    PartialBindingWithMeta, SheetCssItem, UnconditionalCssItem, Variable,
  };
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{DUMMY_SP, FileName, SourceMap, SyntaxContext};
  use swc_core::ecma::ast::{
    CallExpr, Callee, Expr, ExprOrSpread, Ident, KeyValueProp, ObjectLit, Prop, PropName,
    PropOrSpread,
  };
  use swc_core::ecma::parser::{Parser, StringInput, Syntax, lexer::Lexer};

  fn ident_expr(name: &str) -> Expr {
    Expr::Ident(Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty()))
  }

  fn create_metadata() -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    cm.new_source_file(FileName::Custom("test.js".into()).into(), String::new());
    let file = TransformFile::transform_compiled_with_options(
      cm.clone(),
      Vec::new(),
      TransformFileOptions {
        filename: Some("test.js".into()),
        loc_filename: Some("test.js".into()),
        ..TransformFileOptions::default()
      },
    );
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));

    Metadata::new(state)
  }

  fn create_metadata_with_filename(filename: &str) -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    cm.new_source_file(FileName::Custom(filename.into()).into(), String::new());
    let file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      TransformFileOptions {
        filename: Some(filename.into()),
        loc_filename: Some(filename.into()),
        ..TransformFileOptions::default()
      },
    );
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));

    Metadata::new(state)
  }

  fn parse_expression(code: &str) -> Expr {
    let cm: Lrc<SourceMap> = Default::default();
    parse_expression_with_source_map(&cm, code)
  }

  fn parse_expression_with_source_map(cm: &Lrc<SourceMap>, code: &str) -> Expr {
    let source_file = cm.new_source_file(FileName::Custom("test.js".into()).into(), code.into());
    let lexer = Lexer::new(
      Syntax::Es(Default::default()),
      Default::default(),
      StringInput::from(&*source_file),
      None,
    );
    let mut parser = Parser::new_from(lexer);
    *parser.parse_expr().expect("parse expression")
  }

  fn parse_object_literal(code: &str) -> ObjectLit {
    match parse_expression(code) {
      Expr::Object(object) => object,
      Expr::Paren(paren) => match *paren.expr {
        Expr::Object(object) => object,
        other => panic!("expected object literal, found {other:?}"),
      },
      other => panic!("expected object literal expression, found {other:?}"),
    }
  }

  fn assert_keyframe_sheet(
    css_output: &CssOutput,
    expected_name: &str,
    prefix: &str,
    suffix: &str,
  ) {
    assert_eq!(css_output.variables.len(), 0);
    assert_eq!(css_output.css.len(), 2);

    match &css_output.css[0] {
      CssItem::Sheet(SheetCssItem { css }) => {
        let normalized: String = css.chars().filter(|ch| !ch.is_whitespace()).collect();
        let expected = format!("@keyframes{expected_name}{{0%{{opacity:1}}to{{opacity:0}}}}");
        assert_eq!(normalized, expected);
      }
      other => panic!("expected sheet css, found {other:?}"),
    }

    match &css_output.css[1] {
      CssItem::Unconditional(UnconditionalCssItem { css }) => {
        assert_eq!(css, &format!("{prefix}{expected_name}{suffix}"));
      }
      other => panic!("expected unconditional css, found {other:?}"),
    }
  }

  fn css_map_call() -> Expr {
    let selector_ident = Ident::new("primary".into(), DUMMY_SP, SyntaxContext::empty());
    let variant = ObjectLit {
      span: DUMMY_SP,
      props: Vec::new(),
    };

    let argument = ObjectLit {
      span: DUMMY_SP,
      props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(selector_ident.into()),
        value: Box::new(Expr::Object(variant)),
      })))],
    };

    Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
        "cssMap".into(),
        DUMMY_SP,
        SyntaxContext::empty(),
      )))),
      args: vec![ExprOrSpread {
        spread: None,
        expr: Box::new(Expr::Object(argument)),
      }],
      type_args: None,
    })
  }

  #[test]
  fn extract_keyframes_from_call_expression() {
    let metadata = create_metadata();
    let expr = parse_expression("keyframes({ from: { opacity: 1 }, to: { opacity: 0 } })");
    let mut build_css = |value: &Expr, meta: &Metadata| {
      match &meta.context {
        MetadataContext::Keyframes { keyframe } => {
          assert_eq!(keyframe.len(), 8);
        }
        other => panic!("expected keyframes context, found {other:?}"),
      }

      match value {
        Expr::Array(array) => {
          assert_eq!(array.elems.len(), 1);
          CssOutput {
            css: vec![CssItem::unconditional("0%{opacity:1}to{opacity:0}")],
            variables: Vec::new(),
          }
        }
        other => panic!("unexpected expression {other:?}"),
      }
    };

    let output =
      extract_keyframes_with_builder(&expr, &metadata, "animation: ", ";", &mut build_css);

    assert_keyframe_sheet(&output, "k1m8j3od", "animation: ", ";");
  }

  #[test]
  fn extract_keyframes_from_tagged_template() {
    let metadata = create_metadata();
    let expr = parse_expression("keyframes`from { opacity: 1; } to { opacity: 0; }`");
    let mut build_css = |value: &Expr, meta: &Metadata| {
      match &meta.context {
        MetadataContext::Keyframes { keyframe } => {
          assert_eq!(keyframe.len(), 7);
        }
        other => panic!("expected keyframes context, found {other:?}"),
      }

      match value {
        Expr::Tpl(_) => CssOutput {
          css: vec![CssItem::unconditional("0%{opacity:1}to{opacity:0}")],
          variables: Vec::new(),
        },
        other => panic!("unexpected expression {other:?}"),
      }
    };

    let output =
      extract_keyframes_with_builder(&expr, &metadata, "animation: ", ";", &mut build_css);

    assert_keyframe_sheet(&output, "kqbs1so", "animation: ", ";");
  }

  #[test]
  fn conditional_minheight_does_not_emit_variable() {
    unsafe {
      std::env::set_var("STACK_DEBUG", "1");
    }
    let metadata = create_metadata();
    let expr = parse_expression(
      "({ minHeight: ({ isFlexible, isSwimlaneMode }) => isFlexible && !isSwimlaneMode ? fg('avoid_board_scroll_container_style_changes') ? '100%' : 'calc(var(--board-scroll-element-height) * 1px - 8px)' : undefined })",
    );

    let output = build_css_internal(&expr, &metadata);
    assert!(
      output.variables.is_empty(),
      "expected no variables, found {:?}",
      output.variables
    );
    // Stop here for debugging; transforming the CSS items currently overflows.
  }

  #[test]
  fn extract_template_literal_inlines_literals() {
    let metadata = create_metadata();
    let expr = parse_expression("`color: ${'red'};`");
    let mut build_css = |_value: &Expr, _meta: &Metadata| -> CssOutput {
      panic!("builder should not be invoked for literal interpolations")
    };

    let Expr::Tpl(template) = expr else {
      panic!("expected template literal expression");
    };

    let output = extract_template_literal_with_builder(&template, &metadata, &mut build_css);

    assert!(output.variables.is_empty(), "{:?}", output);
    assert_eq!(output.css.len(), 1);

    match &output.css[0] {
      CssItem::Unconditional(unconditional) => {
        assert_eq!(unconditional.css, "color: red;;");
      }
      other => panic!("expected unconditional css item, found {other:?}"),
    }
  }

  #[test]
  fn extract_template_literal_invokes_builder_for_objects() {
    let metadata = create_metadata();
    let expr = parse_expression("`color: ${({ marginTop: 4 })}`");
    let mut invoked_with_object = false;

    let mut build_css = |value: &Expr, _meta: &Metadata| {
      invoked_with_object = matches!(value, Expr::Object(_));
      CssOutput {
        css: vec![CssItem::unconditional("margin-top: 4px;")],
        variables: Vec::new(),
      }
    };

    let Expr::Tpl(template) = expr else {
      panic!("expected template literal expression");
    };

    let output = extract_template_literal_with_builder(&template, &metadata, &mut build_css);

    assert!(
      invoked_with_object,
      "expected builder to be invoked with object expression"
    );
    assert!(output.css.iter().any(
      |item| matches!(item, CssItem::Unconditional(unconditional) if unconditional
                .css
                .contains("margin-top: 4px;"))
    ));
  }

  #[test]
  fn extract_template_literal_creates_variables() {
    let metadata = create_metadata();
    let expr = parse_expression("`font-size: ${fontSize}px;`");
    let mut build_css = |_value: &Expr, _meta: &Metadata| CssOutput::new();

    let Expr::Tpl(template) = expr else {
      panic!("expected template literal expression");
    };

    let output = extract_template_literal_with_builder(&template, &metadata, &mut build_css);

    assert_eq!(output.variables.len(), 1);
    let variable = &output.variables[0];

    let expected_name = format!("--_{}", hash("fontSize"));
    assert_eq!(variable.name, expected_name);
    assert_eq!(variable.prefix, None);
    assert_eq!(variable.suffix.as_deref(), Some("px"));

    assert_eq!(output.css.len(), 1);
    let css = get_item_css(&output.css[0]);
    assert!(css.contains(&format!("var({})", variable.name)));
  }

  #[test]
  fn extract_object_expression_builds_template_literal_arrow_branch() {
    let metadata = create_metadata();
    let object =
      parse_object_literal("({ fontSize: `${(props) => props.isHeading ? 20 : 14}px` })");
    let mut build_css = |expr: &Expr, meta: &Metadata| super::build_css_internal(expr, meta);

    let output = super::extract_object_expression_with_builder(&object, &metadata, &mut build_css);

    assert!(output.variables.is_empty(), "{:?}", output);
    assert_eq!(output.css.len(), 2);

    match &output.css[0] {
      CssItem::Conditional(conditional) => {
        if let CssItem::Unconditional(unconditional) = conditional.consequent.as_ref() {
          assert_eq!(unconditional.css, "font-size:20px");
        } else {
          panic!("expected unconditional consequent");
        }

        if let CssItem::Unconditional(unconditional) = conditional.alternate.as_ref() {
          assert_eq!(unconditional.css, "font-size:14px");
        } else {
          panic!("expected unconditional alternate");
        }
      }
      other => panic!("expected conditional css item, found {other:?}"),
    }

    match &output.css[1] {
      CssItem::Unconditional(unconditional) => {
        assert_eq!(unconditional.css, ";");
      }
      other => panic!("expected trailing unconditional item, found {other:?}"),
    }
  }

  #[test]
  fn extract_object_expression_builds_arrow_template_branch() {
    let metadata = create_metadata();
    let object = parse_object_literal("({ fontSize: (props) => `${props.isLast ? 5 : 10}px` })");
    let mut build_css = |expr: &Expr, meta: &Metadata| super::build_css_internal(expr, meta);

    let output = super::extract_object_expression_with_builder(&object, &metadata, &mut build_css);

    assert!(output.variables.is_empty(), "{:?}", output);
    assert_eq!(output.css.len(), 2);

    match &output.css[0] {
      CssItem::Conditional(conditional) => {
        if let CssItem::Unconditional(unconditional) = conditional.consequent.as_ref() {
          assert_eq!(unconditional.css, "font-size:5px");
        } else {
          panic!("expected unconditional consequent");
        }

        if let CssItem::Unconditional(unconditional) = conditional.alternate.as_ref() {
          assert_eq!(unconditional.css, "font-size:10px");
        } else {
          panic!("expected unconditional alternate");
        }
      }
      other => panic!("expected conditional css item, found {other:?}"),
    }

    match &output.css[1] {
      CssItem::Unconditional(unconditional) => {
        assert_eq!(unconditional.css, ";");
      }
      other => panic!("expected trailing unconditional item, found {other:?}"),
    }
  }

  #[test]
  fn extract_object_expression_handles_parenthesized_arrow_conditional() {
    let metadata = create_metadata();
    let object =
      parse_object_literal("({ cursor: (props) => (props.isClickable ? 'pointer' : 'auto') })");
    let mut build_css = |expr: &Expr, meta: &Metadata| super::build_css_internal(expr, meta);

    let output = super::extract_object_expression_with_builder(&object, &metadata, &mut build_css);

    assert!(output.variables.is_empty(), "{:?}", output);
    let conditional = output
      .css
      .iter()
      .find_map(|item| match item {
        CssItem::Conditional(cond) => Some(cond),
        _ => None,
      })
      .expect("expected conditional css item");

    if let CssItem::Unconditional(unconditional) = conditional.consequent.as_ref() {
      assert_eq!(unconditional.css, "cursor:pointer");
    } else {
      panic!("expected unconditional consequent");
    }

    if let CssItem::Unconditional(unconditional) = conditional.alternate.as_ref() {
      assert_eq!(unconditional.css, "cursor:auto");
    } else {
      panic!("expected unconditional alternate");
    }
  }

  #[test]
  fn strip_parentheses_unwraps_conditional() {
    let mut expr = parse_expression("(props.isClickable ? 'pointer' : 'auto')");
    let result = super::strip_parentheses(&mut expr);

    assert!(
      matches!(result, Expr::Cond(_)),
      "expected conditional expression, found {result:?}"
    );
  }

  #[test]
  fn extract_object_expression_template_handles_multiple_arrows() {
    let metadata = create_metadata();
    let object = parse_object_literal(
      r#"({
                fontSize: `font-size: ${props => props.isHeading ? 20 : 14}px; line-height: ${props => props.isHeading ? 24 : 18}px;`
            })"#,
    );
    let mut build_css = |expr: &Expr, meta: &Metadata| super::build_css_internal(expr, meta);

    let output = super::extract_object_expression_with_builder(&object, &metadata, &mut build_css);

    assert!(output.variables.is_empty());

    let mut font_size_checked = false;
    let mut line_height_checked = false;
    let mut trailing_semicolon = false;

    for item in &output.css {
      match item {
        CssItem::Conditional(conditional) => {
          if let CssItem::Unconditional(unconditional) = conditional.consequent.as_ref() {
            if unconditional.css == "font-size: font-size: 20px;" {
              font_size_checked = true;
            }
            if unconditional.css == "font-size:  line-height: 24px;" {
              line_height_checked = true;
            }
          }

          if let CssItem::Unconditional(unconditional) = conditional.alternate.as_ref() {
            if unconditional.css == "font-size: font-size: 14px;" {
              font_size_checked = true;
            }
            if unconditional.css == "font-size:  line-height: 18px;" {
              line_height_checked = true;
            }
          }
        }
        CssItem::Unconditional(unconditional) => {
          if unconditional.css == "font-size: ;;" {
            trailing_semicolon = true;
          }
        }
        _ => {}
      }
    }

    assert!(font_size_checked, "expected font-size conditional branches");
    assert!(
      line_height_checked,
      "expected line-height conditional branches"
    );
    assert!(
      trailing_semicolon,
      "expected trailing unconditional semicolon item"
    );
  }

  #[test]
  fn extract_object_expression_creates_variable_for_arrow_expression() {
    let metadata = create_metadata();
    let object = parse_object_literal("({ fontSize: (props) => props.dynamicSize })");
    let mut build_css = |expr: &Expr, meta: &Metadata| super::build_css_internal(expr, meta);

    let output = super::extract_object_expression_with_builder(&object, &metadata, &mut build_css);

    assert_eq!(output.variables.len(), 1);
    assert_eq!(output.css.len(), 1);

    let arrow_expr = parse_expression("(props) => props.dynamicSize");
    let expected_name = format!("--_{}", hash(&print_expression(&arrow_expr)));

    let variable = &output.variables[0];
    assert_eq!(variable.name, expected_name);
    assert!(variable.prefix.is_none());
    assert!(variable.suffix.is_none());

    match &output.css[0] {
      CssItem::Unconditional(unconditional) => {
        assert_eq!(
          unconditional.css,
          format!("font-size: var({expected_name});")
        );
      }
      other => panic!("expected unconditional css item, found {other:?}"),
    }
  }

  #[test]
  fn merges_adjacent_unconditional_items() {
    let items = vec![
      CssItem::unconditional("color: red;"),
      CssItem::unconditional("background: blue;"),
      CssItem::Logical(LogicalCssItem {
        css: "display: none;".into(),
        expression: ident_expr("flag"),
        operator: LogicalOperator::And,
      }),
      CssItem::unconditional("border: 0;"),
    ];

    let merged = merge_subsequent_unconditional_css_items(items);
    assert_eq!(merged.len(), 3);
    assert_eq!(get_item_css(&merged[0]), "color: red;background: blue;");
    assert!(matches!(merged[1], CssItem::Logical(_)));
    assert_eq!(get_item_css(&merged[2]), "border: 0;");
  }

  #[test]
  fn preserves_sheets_when_merging_unconditionals() {
    let items = vec![
      CssItem::Sheet(SheetCssItem {
        css: ".a{color:red;}".into(),
      }),
      CssItem::unconditional("margin: 0;"),
      CssItem::unconditional("padding: 0;"),
    ];

    let merged = merge_subsequent_unconditional_css_items(items);
    assert_eq!(merged.len(), 2);
    assert!(matches!(merged[0], CssItem::Sheet(_)));
    assert_eq!(get_item_css(&merged[1]), "margin: 0;padding: 0;");
  }

  #[test]
  fn get_item_css_serialises_variants() {
    let unconditional = CssItem::unconditional("color: red;");
    assert_eq!(get_item_css(&unconditional), "color: red;");

    let logical = CssItem::Logical(LogicalCssItem {
      css: "display: none;".into(),
      expression: ident_expr("flag"),
      operator: LogicalOperator::And,
    });
    assert_eq!(get_item_css(&logical), "display: none;");
  }

  #[test]
  fn to_css_rule_wraps_items_in_selector() {
    let item = CssItem::unconditional("color: red;");
    let output = CssOutput {
      css: vec![item],
      variables: Vec::new(),
    };

    let result = to_css_rule(".foo", &output);
    assert_eq!(result.css.len(), 1);

    match &result.css[0] {
      CssItem::Unconditional(unconditional) => {
        assert_eq!(unconditional.css, ".foo { color: red; }");
      }
      _ => panic!("expected unconditional item"),
    }
  }

  #[test]
  fn to_css_rule_recursively_wraps_conditionals() {
    let conditional = CssItem::Conditional(ConditionalCssItem {
      test: Expr::Ident(Ident::new("flag".into(), DUMMY_SP, SyntaxContext::empty())),
      consequent: Box::new(CssItem::unconditional("color: red;")),
      alternate: Box::new(CssItem::unconditional("color: blue;")),
    });

    let output = CssOutput {
      css: vec![conditional],
      variables: Vec::new(),
    };

    let result = to_css_rule(".foo", &output);
    match &result.css[0] {
      CssItem::Conditional(mapped) => {
        if let CssItem::Unconditional(unconditional) = mapped.consequent.as_ref() {
          assert_eq!(unconditional.css, ".foo { color: red; }");
        } else {
          panic!("expected unconditional consequent");
        }

        if let CssItem::Unconditional(unconditional) = mapped.alternate.as_ref() {
          assert_eq!(unconditional.css, ".foo { color: blue; }");
        } else {
          panic!("expected unconditional alternate");
        }
      }
      _ => panic!("expected conditional item"),
    }
  }

  #[test]
  fn to_css_declaration_maps_values() {
    let item = CssItem::Logical(LogicalCssItem {
      css: "var(--token)".into(),
      expression: ident_expr("flag"),
      operator: LogicalOperator::And,
    });

    let output = CssOutput {
      css: vec![item],
      variables: Vec::new(),
    };

    let result = to_css_declaration("fontWeight", &output);
    match &result.css[0] {
      CssItem::Logical(logical) => {
        assert_eq!(logical.css, "font-weight: var(--token);");
      }
      _ => panic!("expected logical item"),
    }
  }

  #[test]
  fn finds_binding_identifier_on_nested_member() {
    let expr = parse_expression("theme.colors.primary");
    if let Expr::Member(member) = expr {
      let ident = find_binding_identifier(&Expr::Member(member)).expect("identifier");
      assert_eq!(ident.sym.as_ref(), "theme");
    } else {
      panic!("expected member expression");
    }
  }

  #[test]
  fn callback_if_file_included_tracks_imports() {
    let meta = create_metadata_with_filename("root.tsx");
    let next = create_metadata_with_filename("imported.tsx");

    callback_if_file_included(&meta, &next);

    let state = meta.state();
    assert_eq!(state.included_files, vec!["imported.tsx".to_string()]);
  }

  #[test]
  #[should_panic(expected = "Identifier contains values that can't be statically evaluated")]
  fn assert_no_imported_css_variables_panics_for_imports() {
    let meta = create_metadata();
    let binding_meta = create_metadata();
    let binding = PartialBindingWithMeta::new(
      None,
      None,
      true,
      binding_meta.clone(),
      BindingSource::Import,
    );

    let css_output = CssOutput {
      css: Vec::new(),
      variables: vec![Variable {
        name: "--token".into(),
        expression: ident_expr("value"),
        prefix: None,
        suffix: None,
      }],
    };

    let source_map = {
      let state = meta.state();
      state.file().source_map.clone()
    };
    let reference = parse_expression_with_source_map(&source_map, "styles");
    assert!(matches!(reference, Expr::Ident(_)));
    assert_no_imported_css_variables(&reference, &meta, &binding, &css_output);
  }

  #[test]
  fn assert_no_imported_css_variables_allows_local_bindings() {
    let meta = create_metadata();
    let binding_meta = create_metadata();
    let binding = PartialBindingWithMeta::new(
      None,
      None,
      true,
      binding_meta.clone(),
      BindingSource::Module,
    );

    let css_output = CssOutput::new();
    let reference = ident_expr("styles");
    assert_no_imported_css_variables(&reference, &meta, &binding, &css_output);
  }

  #[test]
  fn extract_member_expression_returns_map_item() {
    let meta = create_metadata();
    {
      let mut state = meta.state_mut();
      state
        .css_map
        .insert("styles".into(), vec![".a{color:red;}".into()]);
    }

    let expr = parse_expression("styles.primary");
    if let Expr::Member(member) = expr {
      let output =
        extract_member_expression_with_builder(&member, &meta, false, &mut |_expr, _meta| {
          CssOutput::new()
        })
        .expect("map output");

      assert_eq!(output.css.len(), 1);
      match &output.css[0] {
        CssItem::Map(map) => assert_eq!(map.name, "styles"),
        _ => panic!("expected map item"),
      }
    } else {
      panic!("expected member expression");
    }
  }

  #[test]
  fn extract_member_expression_falls_back_to_evaluation() {
    let meta = create_metadata();
    let binding_meta = create_metadata();
    let binding = PartialBindingWithMeta::new(
      Some(parse_expression("({ primary: { color: 'red' } })")),
      None,
      true,
      binding_meta.clone(),
      BindingSource::Module,
    );
    meta.insert_parent_binding("theme", binding);

    let expr = parse_expression("theme.primary");
    let mut invoked = false;

    if let Expr::Member(member) = expr {
      let output =
        extract_member_expression_with_builder(&member, &meta, true, &mut |expr, _meta| {
          invoked = true;
          assert!(matches!(expr, Expr::Object(_)));
          CssOutput {
            css: vec![CssItem::unconditional("color: red;")],
            variables: Vec::new(),
          }
        })
        .expect("css output");

      assert!(invoked);
      assert_eq!(output.css.len(), 1);
    } else {
      panic!("expected member expression");
    }
  }

  #[test]
  fn extract_logical_expression_evaluates_body() {
    let meta = create_metadata();
    let expr = parse_expression("() => ({ color: 'red' })");
    let mut invoked = false;

    if let Expr::Arrow(arrow) = expr {
      let result = extract_logical_expression_with_builder(&arrow, &meta, &mut |expr, _meta| {
        invoked = true;
        assert!(matches!(expr, Expr::Object(_)));
        CssOutput {
          css: vec![CssItem::unconditional("color: red;")],
          variables: Vec::new(),
        }
      });

      assert!(invoked);
      assert_eq!(result.css.len(), 1);
      assert_eq!(get_item_css(&result.css[0]), "color: red;");
    } else {
      panic!("expected arrow expression");
    }
  }

  #[test]
  fn extract_conditional_expression_handles_both_branches() {
    let meta = create_metadata();
    let expr = parse_expression("flag ? { color: 'red' } : { color: 'blue' }");
    let mut call_count = 0;

    if let Expr::Cond(cond) = expr {
      let result = extract_conditional_expression_with_builder(&cond, &meta, &mut |_, _| {
        call_count += 1;
        let css = if call_count == 1 {
          "color: red;"
        } else {
          "color: blue;"
        };
        CssOutput {
          css: vec![CssItem::unconditional(css)],
          variables: Vec::new(),
        }
      });

      assert_eq!(call_count, 2);
      assert_eq!(result.css.len(), 1);
      match &result.css[0] {
        CssItem::Conditional(conditional) => {
          assert!(matches!(
            *conditional.consequent.clone(),
            CssItem::Unconditional(_)
          ));
          assert!(matches!(
            *conditional.alternate.clone(),
            CssItem::Unconditional(_)
          ));
        }
        _ => panic!("expected conditional css item"),
      }
    } else {
      panic!("expected conditional expression");
    }
  }

  #[test]
  fn extract_conditional_expression_converts_single_branch_to_logical() {
    let meta = create_metadata();
    let expr = parse_expression("flag ? { color: 'red' } : value");
    let mut invoked = false;

    if let Expr::Cond(cond) = expr {
      let result = extract_conditional_expression_with_builder(&cond, &meta, &mut |_, _| {
        if !invoked {
          invoked = true;
          CssOutput {
            css: vec![CssItem::unconditional("color: red;")],
            variables: Vec::new(),
          }
        } else {
          CssOutput::new()
        }
      });

      assert!(invoked);
      assert_eq!(result.css.len(), 1);
      match &result.css[0] {
        CssItem::Logical(logical) => {
          assert_eq!(logical.css, "color: red;");
          assert_eq!(logical.operator, LogicalOperator::And);
        }
        _ => panic!("expected logical css item"),
      }
    } else {
      panic!("expected conditional expression");
    }
  }

  #[test]
  fn to_css_declaration_preserves_sheets() {
    let sheet_css = ".a { color: red; }".to_string();
    let sheet = CssItem::Sheet(SheetCssItem {
      css: sheet_css.clone(),
    });

    let output = CssOutput {
      css: vec![sheet],
      variables: Vec::new(),
    };

    let result = to_css_declaration("color", &output);
    assert_eq!(result.css.len(), 1);

    match &result.css[0] {
      CssItem::Sheet(mapped) => assert_eq!(mapped.css, sheet_css),
      _ => panic!("expected sheet item"),
    }
  }

  #[test]
  fn generate_cache_populates_css_map() {
    let metadata = create_metadata();
    {
      let mut state = metadata.state_mut();
      state.compiled_imports = Some(CompiledImports {
        css_map: vec!["cssMap".into()],
        ..CompiledImports::default()
      });
    }

    let binding_meta = metadata.clone();
    let css_map_expr = css_map_call();
    let binding = PartialBindingWithMeta::new(
      Some(css_map_expr.clone()),
      None,
      true,
      binding_meta.clone(),
      BindingSource::Module,
    );
    binding_meta.insert_parent_binding("styles", binding);

    let ident = Ident::new("styles".into(), DUMMY_SP, SyntaxContext::empty());
    let mut calls = 0usize;
    let mut build_css = |expr: &Expr, _meta: &Metadata| {
      calls += 1;
      assert!(matches!(expr, Expr::Object(_)));
      CssOutput {
        css: vec![CssItem::Sheet(SheetCssItem {
          css: ".a{color:red;}".into(),
        })],
        variables: Vec::new(),
      }
    };

    let populated = generate_cache_for_css_map_with_builder(&ident, &metadata, &mut build_css);

    assert!(populated);
    assert_eq!(calls, 1);

    let state = metadata.state();
    let sheets = state.css_map.get("styles").expect("cache entry");
    assert_eq!(sheets.len(), 1);
    assert!(state.ignore_member_expressions.is_empty());
  }

  #[test]
  fn generate_cache_marks_identifier_when_binding_missing() {
    let metadata = create_metadata();
    let ident = Ident::new("styles".into(), DUMMY_SP, SyntaxContext::empty());
    let mut build_css = |_expr: &Expr, _meta: &Metadata| CssOutput::new();

    let populated = generate_cache_for_css_map_with_builder(&ident, &metadata, &mut build_css);

    assert!(!populated);

    let state = metadata.state();
    assert!(state.ignore_member_expressions.contains("styles"));
    assert!(state.css_map.is_empty());
  }
}
