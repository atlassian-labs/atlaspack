use std::collections::BTreeMap;
use std::env;

use indexmap::IndexSet;
use swc_core::atoms::Atom;
use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::{
  ArrayLit, ArrowExpr, AssignPat, BindingIdent, BlockStmt, BlockStmtOrExpr, CallExpr, Callee, Decl,
  Expr, ExprOrSpread, Ident, IdentName, IfStmt, JSXAttr, JSXAttrName, JSXAttrOrSpread,
  JSXAttrValue, JSXClosingElement, JSXElement, JSXElementChild, JSXElementName, JSXExpr,
  JSXExprContainer, JSXOpeningElement, JSXText, KeyValuePatProp, Lit, MemberExpr, MemberProp,
  NewExpr, ObjectLit, ObjectPat, ObjectPatProp, Pat, PropOrSpread, RestPat, ReturnStmt,
  SpreadElement, Stmt, Str, ThrowStmt, VarDecl, VarDeclKind, VarDeclarator,
};
use swc_core::ecma::visit::{Visit, VisitWith};

use crate::constants::{
  DOM_PROPS_IDENTIFIER_NAME, PROPS_IDENTIFIER_NAME, REF_IDENTIFIER_NAME, STYLE_IDENTIFIER_NAME,
};
use crate::postcss::transform::transform_css;
use crate::types::{Metadata, Tag, TagType};
use crate::utils_ast::pick_function_body;
use crate::utils_build_css_variables::build_css_variables_with_transform;
use crate::utils_compress_class_names_for_runtime::compress_class_names_for_runtime;
use crate::utils_css_builders::get_item_css;
use crate::utils_find_open_selectors::find_open_selectors;
use crate::utils_get_runtime_class_name_library::get_runtime_class_name_library;
use crate::utils_hoist_sheet::hoist_sheet;
use crate::utils_is_prop_valid::is_prop_valid;
use crate::utils_transform_css_items::{
  apply_selectors, create_transform_css_options, transform_css_items,
};
use crate::utils_types::{CssItem, CssOutput, Variable};

fn ident(name: &str) -> Ident {
  Ident::new(Atom::from(name), DUMMY_SP, SyntaxContext::empty())
}

fn binding_ident(name: &str) -> BindingIdent {
  BindingIdent {
    id: ident(name),
    type_ann: None,
  }
}

fn string_lit(value: &str) -> Expr {
  Expr::Lit(Lit::Str(Str {
    span: DUMMY_SP,
    value: Atom::from(value),
    raw: None,
  }))
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

fn jsx_attribute(name: &str, value: JSXAttrValue) -> JSXAttrOrSpread {
  JSXAttrOrSpread::JSXAttr(JSXAttr {
    span: DUMMY_SP,
    name: JSXAttrName::Ident(IdentName::new(Atom::from(name), DUMMY_SP)),
    value: Some(value),
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

fn member_expression(object: &str, property: &str) -> Expr {
  Expr::Member(MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(Expr::Ident(ident(object))),
    prop: MemberProp::Ident(IdentName::new(Atom::from(property), DUMMY_SP)),
  })
}

fn build_component_tag_expr(tag: &Tag) -> Expr {
  match tag.tag_type {
    TagType::InBuiltComponent => string_lit(&tag.name),
    TagType::UserDefinedComponent => Expr::Ident(ident(&tag.name)),
  }
}

fn component_name_from_tag(tag: &Tag) -> Option<String> {
  match tag.tag_type {
    TagType::UserDefinedComponent => Some(tag.name.clone()),
    _ => None,
  }
}

fn styled_style_prop(variables: &[Variable]) -> Expr {
  let mut props: Vec<PropOrSpread> = Vec::new();
  props.push(PropOrSpread::Spread(SpreadElement {
    dot3_token: DUMMY_SP,
    expr: Box::new(Expr::Ident(ident(STYLE_IDENTIFIER_NAME))),
  }));

  let mut dynamic = build_css_variables_with_transform(variables, |expr| pick_function_body(expr));
  props.append(&mut dynamic);

  Expr::Object(ObjectLit {
    span: DUMMY_SP,
    props,
  })
}

struct InvalidDomPropsCollector {
  invalids: IndexSet<String>,
}

impl Visit for InvalidDomPropsCollector {
  fn visit_member_expr(&mut self, member: &MemberExpr) {
    if let Expr::Ident(object) = &*member.obj {
      if object.sym.as_ref() == PROPS_IDENTIFIER_NAME {
        if let MemberProp::Ident(prop) = &member.prop {
          let name = prop.sym.as_ref();
          if name != "children" && !is_prop_valid(name) {
            self.invalids.insert(name.to_string());
          }
        }
      }
    }

    member.visit_children_with(self);
  }
}

fn get_invalid_dom_props(meta: &Metadata) -> Vec<String> {
  let Some(parent) = meta.parent_expr() else {
    return Vec::new();
  };

  let mut collector = InvalidDomPropsCollector {
    invalids: IndexSet::new(),
  };
  parent.visit_with(&mut collector);
  collector.invalids.into_iter().collect()
}

fn dedupe_sheets(sheets: &[String], meta: &Metadata) -> Vec<Expr> {
  let mut unique = IndexSet::new();
  let mut expressions = Vec::new();

  for sheet in sheets {
    if unique.insert(sheet.clone()) {
      let ident = hoist_sheet(sheet, meta);
      expressions.push(Expr::Ident(ident));
    }
  }

  expressions
}

fn is_development_env() -> bool {
  let babel_env = env::var("BABEL_ENV").ok();
  let node_env = env::var("NODE_ENV").ok();

  let babel_falsy = babel_env.as_ref().map(|v| v.is_empty()).unwrap_or(true);
  let node_falsy = node_env.as_ref().map(|v| v.is_empty()).unwrap_or(true);

  (babel_falsy && node_falsy)
    || matches!(babel_env.as_deref(), Some("development") | Some("test"))
    || matches!(node_env.as_deref(), Some("development") | Some("test"))
}

fn should_include_component_name(add_component_name: bool) -> bool {
  if !add_component_name {
    return false;
  }

  env::var("NODE_ENV")
    .map(|value| value != "production")
    .unwrap_or(true)
}

fn component_class_expr(name: Option<&str>, add_component_name: bool) -> Option<Expr> {
  if !should_include_component_name(add_component_name) {
    return None;
  }

  name.map(|component| string_lit(&format!("c_{}", component)))
}

fn build_runtime_call(helper: &str, values: Vec<Expr>) -> Expr {
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

fn build_class_name_array(
  component_name: Option<&str>,
  add_component_name: bool,
  unconditional: &str,
  conditional: &[Expr],
) -> Vec<Expr> {
  let mut class_names = Vec::new();

  if let Some(expr) = component_class_expr(component_name, add_component_name) {
    class_names.push(expr);
  }

  class_names.push(string_lit(unconditional));
  class_names.extend(conditional.iter().cloned());
  class_names.push(member_expression(PROPS_IDENTIFIER_NAME, "className"));

  class_names
}

fn build_cs_element(
  sheets: Vec<String>,
  meta: &Metadata,
  nonce: Option<&String>,
) -> JSXElementChild {
  let sheet_exprs = dedupe_sheets(&sheets, meta);
  let mut attrs = Vec::new();

  if let Some(value) = nonce {
    let attr_value = JSXAttrValue::JSXExprContainer(JSXExprContainer {
      span: DUMMY_SP,
      expr: JSXExpr::Expr(Box::new(Expr::Ident(ident(value)))),
    });
    attrs.push(jsx_attribute("nonce", attr_value));
  }

  let children = vec![JSXElementChild::JSXExprContainer(JSXExprContainer {
    span: DUMMY_SP,
    expr: JSXExpr::Expr(Box::new(array_expression(sheet_exprs))),
  })];

  JSXElementChild::JSXElement(Box::new(JSXElement {
    span: DUMMY_SP,
    opening: JSXOpeningElement {
      span: DUMMY_SP,
      name: jsx_name("CS"),
      attrs,
      self_closing: false,
      type_args: None,
    },
    closing: Some(JSXClosingElement {
      span: DUMMY_SP,
      name: jsx_name("CS"),
    }),
    children,
  }))
}

fn build_c_element(spread_ident: &str, style_expr: Expr, class_expr: Expr) -> JSXElementChild {
  let spread = JSXAttrOrSpread::SpreadElement(SpreadElement {
    dot3_token: DUMMY_SP,
    expr: Box::new(Expr::Ident(ident(spread_ident))),
  });

  let style_attr = jsx_attribute(
    "style",
    JSXAttrValue::JSXExprContainer(JSXExprContainer {
      span: DUMMY_SP,
      expr: JSXExpr::Expr(Box::new(style_expr)),
    }),
  );

  let ref_attr = jsx_attribute(
    "ref",
    JSXAttrValue::JSXExprContainer(JSXExprContainer {
      span: DUMMY_SP,
      expr: JSXExpr::Expr(Box::new(Expr::Ident(ident(REF_IDENTIFIER_NAME)))),
    }),
  );

  let class_attr = jsx_attribute(
    "className",
    JSXAttrValue::JSXExprContainer(JSXExprContainer {
      span: DUMMY_SP,
      expr: JSXExpr::Expr(Box::new(class_expr)),
    }),
  );

  let attrs = vec![spread, style_attr, ref_attr, class_attr];

  JSXElementChild::JSXElement(Box::new(JSXElement {
    span: DUMMY_SP,
    opening: JSXOpeningElement {
      span: DUMMY_SP,
      name: jsx_name("C"),
      attrs,
      self_closing: true,
      type_args: None,
    },
    closing: None,
    children: Vec::new(),
  }))
}

fn build_inner_ref_guard() -> Stmt {
  let test = member_expression(PROPS_IDENTIFIER_NAME, "innerRef");

  let throw_stmt = Stmt::Throw(ThrowStmt {
    span: DUMMY_SP,
    arg: Box::new(Expr::New(NewExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Box::new(Expr::Ident(ident("Error"))),
      args: Some(vec![ExprOrSpread {
        spread: None,
        expr: Box::new(string_lit("Please use 'ref' instead of 'innerRef'.")),
      }]),
      type_args: None,
    })),
  });

  Stmt::If(IfStmt {
    span: DUMMY_SP,
    test: Box::new(test),
    cons: Box::new(Stmt::Block(BlockStmt {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      stmts: vec![throw_stmt],
    })),
    alt: None,
  })
}

fn build_invalid_props_statement(invalids: &[String]) -> Stmt {
  let mut props: Vec<ObjectPatProp> = invalids
    .iter()
    .map(|name| {
      ObjectPatProp::KeyValue(KeyValuePatProp {
        key: IdentName::new(Atom::from(name.as_str()), DUMMY_SP).into(),
        value: Box::new(Pat::Ident(binding_ident(name))),
      })
    })
    .collect();

  props.push(ObjectPatProp::Rest(RestPat {
    span: DUMMY_SP,
    dot3_token: DUMMY_SP,
    arg: Box::new(Pat::Ident(binding_ident(DOM_PROPS_IDENTIFIER_NAME))),
    type_ann: None,
  }));

  let pattern = Pat::Object(ObjectPat {
    span: DUMMY_SP,
    optional: false,
    type_ann: None,
    props,
  });

  let declarator = VarDeclarator {
    span: DUMMY_SP,
    name: pattern,
    init: Some(Box::new(Expr::Ident(ident(PROPS_IDENTIFIER_NAME)))),
    definite: false,
  };

  Stmt::Decl(Decl::Var(Box::new(VarDecl {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    kind: VarDeclKind::Const,
    declare: false,
    decls: vec![declarator],
  })))
}

fn object_param(tag: &Tag) -> Pat {
  let as_prop = ObjectPatProp::KeyValue(KeyValuePatProp {
    key: IdentName::new(Atom::from("as"), DUMMY_SP).into(),
    value: Box::new(Pat::Assign(AssignPat {
      span: DUMMY_SP,
      left: Box::new(Pat::Ident(binding_ident("C"))),
      right: Box::new(build_component_tag_expr(tag)),
    })),
  });

  let style_prop = ObjectPatProp::KeyValue(KeyValuePatProp {
    key: IdentName::new(Atom::from("style"), DUMMY_SP).into(),
    value: Box::new(Pat::Ident(binding_ident(STYLE_IDENTIFIER_NAME))),
  });

  let rest_prop = ObjectPatProp::Rest(RestPat {
    span: DUMMY_SP,
    dot3_token: DUMMY_SP,
    arg: Box::new(Pat::Ident(binding_ident(PROPS_IDENTIFIER_NAME))),
    type_ann: None,
  });

  Pat::Object(ObjectPat {
    span: DUMMY_SP,
    optional: false,
    type_ann: None,
    props: vec![as_prop, style_prop, rest_prop],
  })
}

fn ref_param() -> Pat {
  Pat::Ident(binding_ident(REF_IDENTIFIER_NAME))
}

fn serialize_css_items(css: &[CssItem]) -> (String, Vec<CssItem>) {
  let mut unconditional = String::new();
  let mut conditional = Vec::new();

  for item in css {
    match item {
      CssItem::Logical(_) | CssItem::Conditional(_) => {
        let mut cloned = item.clone();
        if let Some(selectors) = find_open_selectors(&unconditional) {
          apply_selectors(&mut cloned, &selectors);
        }
        conditional.push(cloned);
      }
      _ => unconditional.push_str(&get_item_css(item)),
    }
  }

  (unconditional, conditional)
}

fn extract_first_class_from_sheet(sheet: &str) -> Option<String> {
  if let Some(dot) = sheet.find('.') {
    let rest = &sheet[dot + 1..];
    let end = rest
      .find(|c: char| c == '{' || c == ' ' || c == ',')
      .unwrap_or(rest.len());
    let name = &rest[..end];
    if !name.is_empty() {
      return Some(name.to_string());
    }
  }
  None
}

fn first_property_from_sheet(sheet: &str) -> Option<String> {
  if let Some(open) = sheet.find('{') {
    let rest = &sheet[open + 1..];
    if let Some(colon) = rest.find(':') {
      let prop = &rest[..colon];
      let trimmed = prop.trim();
      if !trimmed.is_empty() {
        return Some(trimmed.to_string());
      }
    }
  }
  None
}

fn shorthand_bucket(prop: &str) -> Option<u32> {
  match prop {
    "all" => Some(0),
    // Level 1
    "animation"
    | "animation-range"
    | "background"
    | "border"
    | "border-image"
    | "border-radius"
    | "column-rule"
    | "columns"
    | "contain-intrinsic-size"
    | "container"
    | "flex"
    | "flex-flow"
    | "font"
    | "font-synthesis"
    | "gap"
    | "grid"
    | "grid-area"
    | "inset"
    | "list-style"
    | "mask"
    | "mask-border"
    | "offset"
    | "outline"
    | "overflow"
    | "overscroll-behavior"
    | "padding"
    | "place-content"
    | "place-items"
    | "place-self"
    | "position-try"
    | "scroll-margin"
    | "scroll-padding"
    | "scroll-timeline"
    | "text-decoration"
    | "text-emphasis"
    | "text-wrap"
    | "transition"
    | "view-timeline" => Some(1),
    // Level 2
    "border-color"
    | "border-style"
    | "border-width"
    | "font-variant"
    | "grid-column"
    | "grid-row"
    | "grid-template"
    | "inset-block"
    | "inset-inline"
    | "margin-block"
    | "margin-inline"
    | "padding-block"
    | "padding-inline"
    | "scroll-margin-block"
    | "scroll-margin-inline"
    | "scroll-padding-block"
    | "scroll-padding-inline" => Some(2),
    // Level 3
    "border-block" | "border-inline" => Some(3),
    // Level 4
    "border-top" | "border-right" | "border-bottom" | "border-left" => Some(4),
    // Level 5
    "border-block-start" | "border-block-end" | "border-inline-start" | "border-inline-end" => {
      Some(5)
    }
    _ => None,
  }
}

fn parent_shorthand(prop: &str) -> Option<&'static str> {
  match prop {
    // padding
    "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => Some("padding"),
    "padding-block-start" | "padding-block-end" => Some("padding-block"),
    "padding-inline-start" | "padding-inline-end" => Some("padding-inline"),
    // margin
    "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => Some("margin"),
    "margin-block-start" | "margin-block-end" => Some("margin-block"),
    "margin-inline-start" | "margin-inline-end" => Some("margin-inline"),
    // border
    "border-bottom-color"
    | "border-top-color"
    | "border-left-color"
    | "border-right-color"
    | "border-inline-color"
    | "border-block-color"
    | "border-inline-start-color"
    | "border-inline-end-color"
    | "border-block-start-color"
    | "border-block-end-color" => Some("border-color"),
    "border-bottom-style"
    | "border-top-style"
    | "border-left-style"
    | "border-right-style"
    | "border-inline-style"
    | "border-block-style"
    | "border-inline-start-style"
    | "border-inline-end-style"
    | "border-block-start-style"
    | "border-block-end-style" => Some("border-style"),
    "border-bottom-width"
    | "border-top-width"
    | "border-left-width"
    | "border-right-width"
    | "border-inline-width"
    | "border-block-width"
    | "border-inline-start-width"
    | "border-inline-end-width"
    | "border-block-start-width"
    | "border-block-end-width" => Some("border-width"),
    _ => None,
  }
}

fn order_class_names_by_bucket(class_names: &[String], sheets: &[String]) -> Vec<String> {
  // Build class->prop map from sheets
  use std::collections::HashMap;
  let mut prop_map: HashMap<&str, String> = HashMap::new();
  for sheet in sheets {
    if let (Some(class), Some(prop)) = (
      extract_first_class_from_sheet(sheet),
      first_property_from_sheet(sheet),
    ) {
      prop_map.insert(Box::leak(class.into_boxed_str()), prop);
    }
  }
  // Stable sort by bucket, then by original index
  let mut with_index: Vec<(usize, &String)> = class_names.iter().enumerate().collect();
  with_index.sort_by_key(|(idx, name)| {
    let prop = prop_map.get(name.as_str()).map(|s| s.as_str());
    let bucket = prop
      .and_then(|p| shorthand_bucket(p).or_else(|| parent_shorthand(p).and_then(shorthand_bucket)))
      .unwrap_or(u32::MAX);
    let family_rank: u32 = match prop {
      Some(p) if p == "gap" => 0,
      Some(p) if p.starts_with("padding") => 1,
      Some(p) if p == "border" || p.starts_with("border-") => 2,
      _ => 3,
    };
    (bucket, family_rank, *idx)
  });
  with_index.into_iter().map(|(_i, s)| s.clone()).collect()
}

fn compress_class_names(
  class_names: &[String],
  compression_map: Option<&BTreeMap<String, String>>,
  _sheets: &[String],
) -> String {
  let compressed = compress_class_names_for_runtime(class_names, compression_map);
  compressed.join(" ")
}

/// Builds the styled component wrapper mirroring the Babel helper.
pub fn build_styled_component(tag: Tag, css_output: CssOutput, meta: &Metadata) -> Expr {
  // Mark that we're using runtime wrappers so imports are added
  meta.state_mut().uses_runtime_wrappers = true;

  let style_expr = if css_output.variables.is_empty() {
    Expr::Ident(ident(STYLE_IDENTIFIER_NAME))
  } else {
    styled_style_prop(&css_output.variables)
  };

  let (add_component_name, nonce) = {
    let state = meta.state();
    (
      state.opts.add_component_name.unwrap_or(false),
      state.opts.nonce.clone(),
    )
  };

  let invalid_dom_props = if tag.tag_type == TagType::InBuiltComponent {
    get_invalid_dom_props(meta)
  } else {
    Vec::new()
  };
  let has_invalid_dom_props = !invalid_dom_props.is_empty();

  let (unconditional_css, conditional_items) = serialize_css_items(&css_output.css);
  let (options, compression_map) = create_transform_css_options(meta);

  if let Ok(label) = std::env::var("DEBUG_CSS_FIXTURE") {
    if let Some(filename) = &meta.state().filename {
      if filename.contains(&label) {
        eprintln!(
          "[css-debug] fixture={label} unconditional_css=`{}`",
          unconditional_css.trim()
        );
      }
    }
  }

  if std::env::var("COMPILED_TRACE_TRANSFORM_CSS").is_ok() {
    eprintln!(
      "[compiled][transform-css] unconditional_css=`{}` options={:?}",
      unconditional_css.trim(),
      options
    );
  }

  let css_result =
    transform_css(&unconditional_css, options.clone()).unwrap_or_else(|err| panic!("{err}"));

  let conditional_output = transform_css_items(&conditional_items, meta);

  let class_map_ref = compression_map.as_ref();
  let unconditional_class_names =
    compress_class_names(&css_result.class_names, class_map_ref, &css_result.sheets);

  let component_name = component_name_from_tag(&tag);
  let helper = get_runtime_class_name_library(meta);

  let class_array = build_class_name_array(
    component_name.as_deref(),
    add_component_name,
    unconditional_class_names.trim(),
    &conditional_output.class_names,
  );

  let runtime_call = build_runtime_call(helper, class_array);

  let sheets: Vec<String> = css_result
    .sheets
    .into_iter()
    .chain(conditional_output.sheets.into_iter())
    .collect();
  let cs_child = build_cs_element(sheets, meta, nonce.as_ref());

  let spread_ident = if has_invalid_dom_props {
    DOM_PROPS_IDENTIFIER_NAME
  } else {
    PROPS_IDENTIFIER_NAME
  };
  let c_child = build_c_element(spread_ident, style_expr, runtime_call);

  let mut children = Vec::new();
  children.push(jsx_text("\n      "));
  children.push(cs_child);
  children.push(jsx_text("\n      "));
  children.push(c_child);
  children.push(jsx_text("\n    "));

  let mut stmts = Vec::new();

  if is_development_env() {
    stmts.push(build_inner_ref_guard());
  }

  if has_invalid_dom_props {
    stmts.push(build_invalid_props_statement(&invalid_dom_props));
  }

  stmts.push(Stmt::Return(ReturnStmt {
    span: DUMMY_SP,
    arg: Some(Box::new(Expr::JSXElement(Box::new(JSXElement {
      span: DUMMY_SP,
      opening: JSXOpeningElement {
        span: DUMMY_SP,
        name: jsx_name("CC"),
        attrs: Vec::new(),
        self_closing: false,
        type_args: None,
      },
      closing: Some(JSXClosingElement {
        span: DUMMY_SP,
        name: jsx_name("CC"),
      }),
      children,
    })))),
  }));

  let arrow = ArrowExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    params: vec![object_param(&tag), ref_param()],
    body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      stmts,
    })),
    is_generator: false,
    is_async: false,
    type_params: None,
    return_type: None,
  };

  Expr::Call(CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Ident(ident("forwardRef")))),
    args: vec![ExprOrSpread {
      spread: None,
      expr: Box::new(Expr::Arrow(arrow)),
    }],
    type_args: None,
  })
}

#[cfg(test)]
mod tests {
  use super::{build_styled_component, member_expression};
  use crate::constants::PROPS_IDENTIFIER_NAME;
  use crate::types::{Metadata, PluginOptions, Tag, TagType, TransformFile, TransformState};
  use crate::utils_types::{CssItem, CssOutput};
  use once_cell::sync::Lazy;
  use std::cell::RefCell;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{BytePos, DUMMY_SP, FileName, SourceMap, Span};
  use swc_core::ecma::ast::{
    ArrayLit, BlockStmtOrExpr, Expr, ExprOrSpread, JSXAttrName, JSXAttrOrSpread, JSXAttrValue,
    JSXElementChild, JSXElementName, JSXExpr, Lit, Stmt,
  };

  static ENV_MUTEX: Lazy<std::sync::Mutex<()>> = Lazy::new(|| std::sync::Mutex::new(()));

  fn metadata() -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      crate::types::TransformFileOptions {
        filename: Some("file.tsx".into()),
        loc_filename: Some("file.tsx".into()),
        ..Default::default()
      },
    );
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));
    Metadata::new(state)
  }

  fn css_output() -> CssOutput {
    CssOutput {
      css: vec![CssItem::unconditional("._1wybgktf{font-size:20px}")],
      variables: Vec::new(),
    }
  }

  #[test]
  fn builds_forward_ref_structure() {
    let _guard = ENV_MUTEX.lock().unwrap();
    unsafe {
      std::env::set_var("NODE_ENV", "test");
      std::env::remove_var("BABEL_ENV");
    }

    let meta = metadata();
    let tag = Tag {
      name: "div".into(),
      tag_type: TagType::InBuiltComponent,
    };
    let expr = build_styled_component(tag, css_output(), &meta);

    match expr {
      Expr::Call(call) => {
        assert_eq!(call.args.len(), 1);
        match &*call.args[0].expr {
          Expr::Arrow(arrow) => match &*arrow.body {
            BlockStmtOrExpr::BlockStmt(block) => {
              assert!(matches!(block.stmts[0], Stmt::If(_)));
              assert!(matches!(block.stmts.last(), Some(Stmt::Return(_))));
            }
            _ => panic!("expected block statement"),
          },
          _ => panic!("expected arrow expression"),
        }
      }
      _ => panic!("expected call expression"),
    }
  }

  #[test]
  fn destructures_invalid_dom_props() {
    let meta = metadata();
    let parent_expr = Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: vec![
        Some(ExprOrSpread {
          spread: None,
          expr: Box::new(member_expression(PROPS_IDENTIFIER_NAME, "children")),
        }),
        Some(ExprOrSpread {
          spread: None,
          expr: Box::new(member_expression(PROPS_IDENTIFIER_NAME, "textSize")),
        }),
      ],
    });
    let meta = meta.with_parent_expr(Some(&parent_expr));

    let tag = Tag {
      name: "div".into(),
      tag_type: TagType::InBuiltComponent,
    };
    let expr = build_styled_component(tag, css_output(), &meta);

    if let Expr::Call(call) = expr {
      if let Expr::Arrow(arrow) = &*call.args[0].expr {
        if let BlockStmtOrExpr::BlockStmt(block) = &*arrow.body {
          assert!(block.stmts.iter().any(|stmt| matches!(stmt, Stmt::Decl(_))));
        }
      }
    }
  }

  #[test]
  fn injects_component_name_when_enabled() {
    let _guard = ENV_MUTEX.lock().unwrap();
    unsafe {
      std::env::set_var("NODE_ENV", "development");
      std::env::remove_var("BABEL_ENV");
    }

    let meta = metadata();
    {
      let mut state = meta.state_mut();
      state.opts.add_component_name = Some(true);
    }

    let source_map = {
      let state = meta.state();
      state.file().source_map.clone()
    };
    let file = source_map.new_source_file(
      FileName::Custom("component.tsx".into()).into(),
      "const FooBar = styled.div``;".into(),
    );
    let start = file.start_pos + BytePos(6);
    let span = Span::new(start, start + BytePos(6));
    let meta = meta.with_parent_span(Some(span));

    let tag = Tag {
      name: "div".into(),
      tag_type: TagType::InBuiltComponent,
    };
    let expr = build_styled_component(tag, css_output(), &meta);

    let call = match expr {
      Expr::Call(call) => call,
      other => panic!("expected call expression, received {other:?}"),
    };
    let arrow = match &*call.args[0].expr {
      Expr::Arrow(arrow) => arrow,
      other => panic!("expected arrow expression, received {other:?}"),
    };
    let block = match &*arrow.body {
      BlockStmtOrExpr::BlockStmt(block) => block,
      other => panic!("expected block statement, received {other:?}"),
    };
    let return_stmt = block
      .stmts
      .iter()
      .find_map(|stmt| match stmt {
        Stmt::Return(ret) => Some(ret),
        _ => None,
      })
      .expect("expected return statement");
    let element = match return_stmt.arg.as_ref().expect("return value").as_ref() {
      Expr::JSXElement(element) => element,
      other => panic!("expected JSX element, received {other:?}"),
    };

    let class_attr = element
            .children
            .iter()
            .filter_map(|child| match child {
                JSXElementChild::JSXElement(element) => Some(element),
                _ => None,
            })
            .find(|child| matches!(child.opening.name, JSXElementName::Ident(ref ident) if ident.sym.as_ref() == "C"))
            .and_then(|child| {
                child.opening.attrs.iter().find_map(|attr| match attr {
                    JSXAttrOrSpread::JSXAttr(attr)
                        if matches!(attr.name, JSXAttrName::Ident(ref name) if name.sym.as_ref() == "className") =>
                    {
                        attr.value.as_ref()
                    }
                    _ => None,
                })
            })
            .expect("expected className attribute");

    let container = match class_attr {
      JSXAttrValue::JSXExprContainer(container) => container,
      other => panic!("expected JSX expression container, received {other:?}"),
    };
    let runtime_call = match &container.expr {
      JSXExpr::Expr(expr) => expr.as_ref(),
      other => panic!("expected expression, received {other:?}"),
    };
    let call = match runtime_call {
      Expr::Call(call) => call,
      other => panic!("expected runtime call, received {other:?}"),
    };
    let class_array = match &*call.args[0].expr {
      Expr::Array(array) => array,
      other => panic!("expected class array, received {other:?}"),
    };
    let first = class_array
      .elems
      .first()
      .and_then(|elem| elem.as_ref())
      .expect("expected first class entry");

    match first.expr.as_ref() {
      Expr::Lit(Lit::Str(str_lit)) => assert_eq!(str_lit.value.as_ref(), "c_FooBar"),
      other => panic!("expected string literal, received {other:?}"),
    }

    unsafe {
      std::env::remove_var("NODE_ENV");
    }
  }
}
