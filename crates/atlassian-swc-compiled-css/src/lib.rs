use rustc_hash::{FxHashMap, FxHashSet};
use std::cell::Cell;
use swc_core::{
  atoms::Atom,
  common::{DUMMY_SP, SyntaxContext},
  ecma::{
    ast::*,
    visit::{VisitMut, VisitMutWith, visit_mut_pass},
  },
};

mod evaluate_expr;
use evaluate_expr::{ConstValue, eval_const_expr, eval_expr_to_const};
pub mod config;
pub use config::CompiledCssInJsTransformConfig;
mod css;
mod display_name;
mod emit;
mod imports;
mod keyframes;
mod shorthand;
mod styled;
mod template;
mod utils;

pub(crate) type RuntimeStyleEntry = (String, Expr, bool);
pub(crate) type RuntimeStyleEntries = Vec<RuntimeStyleEntry>;
pub(crate) type CssRuntimeResult = (Vec<String>, RuntimeStyleEntries);
pub(crate) type CssRuntimeResultPerKey = FxHashMap<String, CssRuntimeResult>;

// Thread-local flag indicating whether a JSX pragma `@jsxImportSource @compiled/react` was found
// in the original source. Our test harness sets this per transform.
thread_local! { static JSX_PRAGMA_COMPILED: Cell<bool> = const { Cell::new(false) }; }
thread_local! { static JSX_CLASSIC_PRAGMA_LOCAL: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }

/// Set whether the current transform source had `/** @jsxImportSource @compiled/react */` present.
pub fn set_jsx_import_source_compiled(present: bool) {
  JSX_PRAGMA_COMPILED.with(|c| c.set(present));
}

fn is_jsx_import_source_compiled() -> bool {
  JSX_PRAGMA_COMPILED.with(|c| c.get())
}

pub fn set_jsx_classic_pragma_local(name: Option<String>) {
  JSX_CLASSIC_PRAGMA_LOCAL.with(|c| *c.borrow_mut() = name);
}

fn get_jsx_classic_pragma_local() -> Option<String> {
  JSX_CLASSIC_PRAGMA_LOCAL.with(|c| c.borrow().clone())
}

pub struct CompiledCssInJsTransformResult {
  pub style_rules: Vec<String>,
}

#[derive(Default)]
pub(crate) struct AtomicCssCollector {
  rule_key_to_ident: FxHashMap<u64, Ident>,
  next_id: usize,
  css_local: Option<Atom>,
  css_map_local: Option<Atom>,
  styled_local: Option<Atom>,
  keyframes_local: Option<Atom>,
  hoisted: Vec<ModuleItem>,
  const_env: FxHashMap<Atom, ConstValue>,
  used_idents: FxHashSet<String>,
  // Grouping for wrapper-based blocks (e.g., @media/@supports)
  wrapper_key_to_rules: FxHashMap<String, Vec<String>>, // aggregated inner css blocks
  wrapper_key_to_seen: FxHashMap<String, FxHashSet<u64>>, // dedupe rule keys per wrapper
  wrapper_key_to_stack: FxHashMap<String, Vec<String>>, // original wrapper stack (outer -> inner)
  // styled/forwardRef support
  forward_ref_ident: Option<Ident>,
  need_forward_ref: bool,
  need_runtime_ax: bool,
  need_runtime_ix: bool,
  need_runtime_cc: bool,
  need_runtime_cs: bool,
  pending_display_names: Vec<String>,
  // Reserved for future module-level gating; currently unused
  // Local binding name for classic jsx import from @compiled/react, e.g. import { jsx as J } from '@compiled/react'
  classic_jsx_local: Option<Atom>,
  // Whether classic jsx pragma is present and bound to a compiled jsx import
  module_has_classic_compiled_jsx: bool,
  // Map compiled keyframes animation name -> runtime custom property entries to set on elements
  // using that animation. Each entry is (var_name, ix_arg_expr, wrap_iife)
  keyframes_name_to_runtime: FxHashMap<String, RuntimeStyleEntries>,
  // Whether to allow creating runtime CSS variables when expressions are not const
  allow_runtime_vars: bool,
  // Temporary bucket to collect runtime entries during a single collection pass
  tmp_runtime_entries: RuntimeStyleEntries,
  // Sequence to generate unique runtime var ids
  runtime_var_seq: u64,
  // Depth counter: when > 0, expressions within an xcss-like JSX attribute are not transformed
  in_xcss_attr_depth: usize,
  // Configuration: whether to emit CC and CS components (disabled in production)
  emit_cc_cs_components: bool,
  // Track all CSS constants for CS component generation
  css_constants_for_cs: Vec<Ident>,
  // Track CSS calls that should be replaced with null
  css_calls_to_nullify: FxHashSet<Atom>,
  // Collect CSS rules for reverse-order emission (to match Babel)
  css_rules_for_emission: Vec<(u64, String)>, // (rule_key, css_text)
  style_rules: FxHashSet<String>,
  // Removed diagnostics queue for non-const css outside JSX (we drop unattached runtime)
  // Results of css() assigned to identifiers: collected classes and runtime entries
  css_result_by_ident: FxHashMap<Atom, CssRuntimeResult>,
  // Results of cssMap() assigned to identifiers: key -> (classes, runtime entries)
  css_map_result_by_ident: FxHashMap<Atom, CssRuntimeResultPerKey>,
}

impl AtomicCssCollector {
  pub(crate) fn new(emit_cc_cs_components: bool) -> Self {
    Self {
      emit_cc_cs_components,
      ..Default::default()
    }
  }

  fn collect_atomic_classes_from_css_text_swc(&mut self, _css_text: &str) -> Option<Vec<String>> {
    None
  }

  fn begin_runtime_collection(&mut self) {
    self.allow_runtime_vars = true;
    self.tmp_runtime_entries.clear();
  }

  fn take_runtime_entries(&mut self) -> RuntimeStyleEntries {
    self.allow_runtime_vars = false;
    std::mem::take(&mut self.tmp_runtime_entries)
  }

  fn create_runtime_var_for_expr(&mut self, expr: &Expr, wrap_iife: bool) -> String {
    // Generate a stable-ish var name using a running sequence to avoid collisions
    self.runtime_var_seq = self.runtime_var_seq.wrapping_add(1);
    let key = format!("rv#{}", self.runtime_var_seq);
    let var_name = format!("--{}", self.hash_ix(&key));
    self
      .tmp_runtime_entries
      .push((var_name.clone(), expr.clone(), wrap_iife));
    var_name
  }

  // Attempt to resolve a cssMap member expression to classes and runtime entries.
  // Supports both `styles.primary` where `styles` is a const bound to cssMap(...)
  // and `(cssMap({...})).primary` inline with static key.
  fn try_resolve_css_map_member(&mut self, me: &MemberExpr) -> Option<CssRuntimeResult> {
    // Resolve property key if static
    let key_name: Option<String> = match &me.prop {
      MemberProp::Ident(i) => Some(i.sym.to_string()),
      MemberProp::PrivateName(_) => None,
      MemberProp::Computed(c) => {
        if let Some(ConstValue::Str(s)) = eval_const_expr(&c.expr) {
          Some(s)
        } else if let Some(ConstValue::Str(s)) = eval_expr_to_const(&c.expr, &self.const_env) {
          Some(s)
        } else {
          None
        }
      }
    };
    let key_name = key_name?;
    // Case 1: obj is an identifier bound to cssMap result
    if let Expr::Ident(obj_id) = &*me.obj
      && let Some(per_key) = self.css_map_result_by_ident.get(&obj_id.sym)
      && let Some((classes, entries)) = per_key.get(&key_name)
    {
      return Some((classes.clone(), entries.clone()));
    }
    // Case 2: obj is an inline cssMap call with object literal
    if let Expr::Call(call) = &*me.obj
      && let Callee::Expr(callee_expr) = &call.callee
    {
      // Resolve callee ident possibly behind SeqExpr
      let mut maybe_ident: Option<Ident> = None;
      match &**callee_expr {
        Expr::Ident(i) => maybe_ident = Some(i.clone()),
        Expr::Seq(seq) => {
          if let Some(last) = seq.exprs.last()
            && let Expr::Ident(i) = &**last
          {
            maybe_ident = Some(i.clone());
          }
        }
        _ => {}
      }
      if let Some(ci) = maybe_ident
        && self.css_map_local.as_ref().is_some_and(|w| ci.sym == *w)
        && let Some(arg0) = call.args.first()
        && let Expr::Object(outer) = &*arg0.expr
      {
        for p in &outer.props {
          if let PropOrSpread::Prop(pp) = p
            && let Prop::KeyValue(kv) = &**pp
          {
            let k = match &kv.key {
              PropName::Ident(i) => i.sym.to_string(),
              PropName::Str(s) => s.value.to_string(),
              PropName::Num(n) => n.value.to_string(),
              PropName::BigInt(b) => b.value.to_string(),
              PropName::Computed(_) => continue,
            };
            if k == key_name
              && let Expr::Object(style_obj) = &*kv.value
            {
              self.begin_runtime_collection();
              let classes = self.collect_atomic_classes_from_object(style_obj);
              let entries = self.take_runtime_entries();
              return Some((classes, entries));
            }
          }
        }
      }
    }
    None
  }

  // Resolve a css prop expression into atomic classes and runtime ix entries.
  // Returns None when expression should pass through to ax as-is.
  fn resolve_css_expr_to_classes_and_entries(&mut self, expr: &Expr) -> Option<CssRuntimeResult> {
    match expr {
      // styles.primary (or (cssMap({...})).primary)
      Expr::Member(me) => self.try_resolve_css_map_member(me),
      // Identifier bound to css() const or class string
      Expr::Ident(id) => {
        if let Some(ConstValue::Str(joined)) = self.const_env.get(&id.sym) {
          let classes = vec![joined.clone()];
          let mut entries = RuntimeStyleEntries::with_capacity(
            self
              .css_result_by_ident
              .get(&id.sym)
              .map_or(0, |(_, e)| e.len()),
          );
          if let Some((_cls, e)) = self.css_result_by_ident.get(&id.sym) {
            entries.extend(e.iter().cloned());
          }
          return Some((classes, entries));
        }
        None
      }
      // Inline object literal
      Expr::Object(o) => {
        // Unify on CSS-text path when fully static to leverage optimized text parser
        if let Some(css_text) = self.build_css_text_from_object(o) {
          let classes = self.collect_atomic_classes_from_css_text(&css_text);
          // Merge keyframes runtime entries if animation-name present
          let mut entries: RuntimeStyleEntries = RuntimeStyleEntries::new();
          for anim_name in crate::template::parse_animation_names_from_css_text(&css_text) {
            if let Some(kf_entries) = self.keyframes_name_to_runtime.get(&anim_name) {
              entries.extend(kf_entries.iter().cloned());
            }
          }
          Some((classes, entries))
        } else {
          self.begin_runtime_collection();
          let classes = self.collect_atomic_classes_from_object(o);
          let entries = self.take_runtime_entries();
          Some((classes, entries))
        }
      }
      // css({...}) or css`...`
      Expr::Call(call) => {
        if let Callee::Expr(callee_expr) = &call.callee {
          // Resolve ident from possible SeqExpr
          let mut maybe_ident: Option<Ident> = None;
          match &**callee_expr {
            Expr::Ident(i) => maybe_ident = Some(i.clone()),
            Expr::Seq(seq) => {
              if let Some(last) = seq.exprs.last()
                && let Expr::Ident(i) = &**last
              {
                maybe_ident = Some(i.clone());
              }
            }
            _ => {}
          }
          if let Some(id) = maybe_ident
            && self.css_local.as_ref().is_some_and(|w| id.sym == *w)
            && let Some(arg0) = call.args.first()
          {
            match &*arg0.expr {
              Expr::Object(o) => {
                if let Some(css_text) = self.build_css_text_from_object(o) {
                  let classes = self.collect_atomic_classes_from_css_text(&css_text);
                  let mut entries: RuntimeStyleEntries = RuntimeStyleEntries::new();
                  for anim_name in crate::template::parse_animation_names_from_css_text(&css_text) {
                    if let Some(kf_entries) = self.keyframes_name_to_runtime.get(&anim_name) {
                      entries.extend(kf_entries.iter().cloned());
                    }
                  }
                  return Some((classes, entries));
                } else {
                  self.begin_runtime_collection();
                  let classes = self.collect_atomic_classes_from_object(o);
                  let entries = self.take_runtime_entries();
                  return Some((classes, entries));
                }
              }
              Expr::Tpl(tpl) => {
                self.begin_runtime_collection();
                if let Some(css_text) = self.build_css_text_from_tpl(tpl) {
                  let mut entries = self.take_runtime_entries();
                  // Merge keyframes runtime entries if animation-name present
                  for anim_name in crate::template::parse_animation_names_from_css_text(&css_text) {
                    if let Some(kf_entries) = self.keyframes_name_to_runtime.get(&anim_name) {
                      entries.extend(kf_entries.iter().cloned());
                    }
                  }
                  let classes = self.collect_atomic_classes_from_css_text(&css_text);
                  return Some((classes, entries));
                }
              }
              _ => {}
            }
          }
        }
        None
      }
      // Template literal inline css text
      Expr::Tpl(tpl) => {
        self.begin_runtime_collection();
        if let Some(css_text) = self.build_css_text_from_tpl(tpl) {
          let mut entries = self.take_runtime_entries();
          for anim_name in crate::template::parse_animation_names_from_css_text(&css_text) {
            if let Some(kf_entries) = self.keyframes_name_to_runtime.get(&anim_name) {
              entries.extend(kf_entries.iter().cloned());
            }
          }
          let classes = self.collect_atomic_classes_from_css_text(&css_text);
          Some((classes, entries))
        } else {
          None
        }
      }
      // css`...`
      Expr::TaggedTpl(tt) => {
        let mut maybe_ident: Option<Ident> = None;
        match &*tt.tag {
          Expr::Ident(i) => maybe_ident = Some(i.clone()),
          Expr::Seq(seq) => {
            if let Some(last) = seq.exprs.last()
              && let Expr::Ident(i) = &**last
            {
              maybe_ident = Some(i.clone());
            }
          }
          _ => {}
        }
        if let Some(id) = maybe_ident
          && self.css_local.as_ref().is_some_and(|w| id.sym == *w)
        {
          self.begin_runtime_collection();
          if let Some(css_text) = self.build_css_text_from_tpl(&tt.tpl) {
            let mut entries = self.take_runtime_entries();
            for anim_name in crate::template::parse_animation_names_from_css_text(&css_text) {
              if let Some(kf_entries) = self.keyframes_name_to_runtime.get(&anim_name) {
                entries.extend(kf_entries.iter().cloned());
              }
            }
            let classes = self.collect_atomic_classes_from_css_text(&css_text);
            return Some((classes, entries));
          }
        }
        None
      }
      _ => None,
    }
  }

  fn classes_to_array_elem_static(classes: Vec<String>) -> Option<ExprOrSpread> {
    if classes.is_empty() {
      return None;
    }
    let mut joined = String::new();
    let mut first = true;
    for c in classes {
      if !first {
        joined.push(' ');
      }
      joined.push_str(&c);
      first = false;
    }
    Some(ExprOrSpread {
      spread: None,
      expr: Box::new(Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: joined.into(),
        raw: None,
      }))),
    })
  }

  fn collect_from_css_expr(
    &mut self,
    expr: &Expr,
    ax_array_elems: &mut Vec<ExprOrSpread>,
    runtime_entries: &mut RuntimeStyleEntries,
  ) {
    match expr {
      Expr::Array(arr) => {
        for el in arr.elems.iter().flatten() {
          self.collect_from_css_expr(&el.expr, ax_array_elems, runtime_entries);
        }
      }
      _ => {
        if let Some((classes, entries)) = self.resolve_css_expr_to_classes_and_entries(expr) {
          runtime_entries.extend(entries);
          if let Some(e) = Self::classes_to_array_elem_static(classes) {
            ax_array_elems.push(e);
          }
        } else {
          ax_array_elems.push(ExprOrSpread {
            spread: None,
            expr: Box::new(expr.clone()),
          });
        }
      }
    }
  }

  // Remove unused imports from @compiled/react
  fn remove_unused_compiled_imports(&self, m: &mut Module) {
    m.body.retain_mut(|item| {
      if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item {
        if &*import_decl.src.value == "@compiled/react" {
          // Filter out transformed imports (css, styled, cssMap, keyframes are compiled away)
          import_decl.specifiers.retain(|spec| {
            if let ImportSpecifier::Named(named) = spec {
              let import_name = match &named.imported {
                Some(ModuleExportName::Ident(i)) => &*i.sym,
                Some(ModuleExportName::Str(s)) => &*s.value,
                None => &*named.local.sym,
              };

              match import_name {
                "css" | "styled" | "cssMap" | "keyframes" | "jsx" => false, // Always remove these after transformation
                _ => true,                                                  // Keep other imports
              }
            } else {
              true // Keep default and namespace imports
            }
          });

          // If no specifiers remain, remove the entire import
          !import_decl.specifiers.is_empty()
        } else {
          true // Keep non-@compiled/react imports
        }
      } else {
        true // Keep non-import items
      }
    });
  }

  fn jsx_element_has_css_transformation(&self, element: &JSXElement) -> bool {
    // Check if any attribute is className with ax() call
    for attr in &element.opening.attrs {
      if let JSXAttrOrSpread::JSXAttr(jsx_attr) = attr
        && let JSXAttrName::Ident(name) = &jsx_attr.name
        && &*name.sym == "className"
        && let Some(JSXAttrValue::JSXExprContainer(container)) = &jsx_attr.value
        && let JSXExpr::Expr(expr) = &container.expr
        && self.is_ax_call(expr)
      {
        return true;
      }
    }
    false
  }

  fn extract_css_constants_for_element(&self, element: &JSXElement) -> Vec<Ident> {
    let mut css_constants = Vec::new();

    // Find className attribute with ax() call
    for attr in &element.opening.attrs {
      if let JSXAttrOrSpread::JSXAttr(jsx_attr) = attr
        && let JSXAttrName::Ident(name) = &jsx_attr.name
        && &*name.sym == "className"
        && let Some(JSXAttrValue::JSXExprContainer(container)) = &jsx_attr.value
        && let JSXExpr::Expr(expr) = &container.expr
        && let Expr::Call(call) = &**expr
        && let Callee::Expr(callee_expr) = &call.callee
        && let Expr::Ident(callee_ident) = &**callee_expr
        && &*callee_ident.sym == "ax"
      {
        // Extract class names from ax([...]) call
        if let Some(ExprOrSpread { expr: arg_expr, .. }) = call.args.first()
          && let Expr::Array(array) = &**arg_expr
        {
          for elem in &array.elems {
            if let Some(ExprOrSpread {
              expr: elem_expr, ..
            }) = elem
              && let Expr::Lit(Lit::Str(class_str)) = &**elem_expr
            {
              // Parse space-separated class names and map to CSS constants
              let class_names: Vec<&str> = class_str.value.split_whitespace().collect();
              for class_name in class_names {
                if let Some(ident) = self.find_css_constant_for_class(class_name) {
                  css_constants.push(ident);
                }
              }
            }
          }
        }
      }
    }

    css_constants
  }

  fn find_css_constant_for_class(&self, class_name: &str) -> Option<Ident> {
    // Look through all CSS rules to find the one that contains this class name
    for (rule_key, _) in &self.css_rules_for_emission {
      if let Some(ident) = self.rule_key_to_ident.get(rule_key) {
        // Check if this CSS rule contains the class name
        // We need to check the CSS text to see if it starts with the class selector
        if let Some(css_text) = self.get_css_text_for_rule(*rule_key)
          && css_text.contains(&format!(".{}", class_name))
        {
          return Some(ident.clone());
        }
      }
    }
    None
  }

  fn get_css_text_for_rule(&self, rule_key: u64) -> Option<String> {
    for (key, css_text) in &self.css_rules_for_emission {
      if *key == rule_key {
        return Some(css_text.clone());
      }
    }
    None
  }

  // Check if an expression is an ax() call
  fn is_ax_call(&self, expr: &Expr) -> bool {
    match expr {
      Expr::Call(call) => {
        if let Callee::Expr(callee) = &call.callee
          && let Expr::Ident(ident) = &**callee
        {
          return &*ident.sym == "ax";
        }
        false
      }
      _ => false,
    }
  }
}

fn collect_idents_from_pat(pat: &Pat, out: &mut FxHashSet<String>) {
  match pat {
    Pat::Ident(BindingIdent { id, .. }) => {
      out.insert(id.sym.to_string());
    }
    Pat::Array(arr) => {
      for p in arr.elems.iter().flatten() {
        collect_idents_from_pat(p, out);
      }
    }
    Pat::Object(obj) => {
      for prop in &obj.props {
        match prop {
          ObjectPatProp::KeyValue(kv) => collect_idents_from_pat(&kv.value, out),
          ObjectPatProp::Assign(a) => {
            out.insert(a.key.sym.to_string());
          }
          ObjectPatProp::Rest(r) => collect_idents_from_pat(&r.arg, out),
        }
      }
    }
    Pat::Rest(r) => collect_idents_from_pat(&r.arg, out),
    Pat::Assign(a) => collect_idents_from_pat(&a.left, out),
    _ => {}
  }
}

pub(crate) fn base_encode36(mut v: u64) -> String {
  let alphabet = b"0123456789abcdefghijklmnopqrstuvwxyz";
  let mut buf = [0u8; 16];
  let mut i = 16;
  if v == 0 {
    return "0".into();
  }
  while v > 0 && i > 0 {
    i -= 1;
    buf[i] = alphabet[(v % 36) as usize];
    v /= 36;
  }
  String::from_utf8(buf[i..].to_vec()).unwrap()
}

impl VisitMut for AtomicCssCollector {
  fn visit_mut_jsx_element(&mut self, n: &mut JSXElement) {
    // Process the element's children and attributes first
    n.visit_mut_children_with(self);

    // Check if this element has CSS transformations and should be wrapped with CC/CS
    if self.emit_cc_cs_components && self.jsx_element_has_css_transformation(n) {
      // Extract CSS constants used by this specific element
      let css_constants_for_this_element = self.extract_css_constants_for_element(n);

      if !css_constants_for_this_element.is_empty() {
        self.need_runtime_cc = true;
        self.need_runtime_cs = true;

        // Create CS component with array of CSS constants for this element
        let css_array_elems: Vec<Option<ExprOrSpread>> = css_constants_for_this_element
          .iter()
          .map(|ident| {
            Some(ExprOrSpread {
              spread: None,
              expr: Box::new(Expr::Ident(ident.clone())),
            })
          })
          .collect();

        let cs_children = JSXExpr::Expr(Box::new(Expr::Array(ArrayLit {
          span: DUMMY_SP,
          elems: css_array_elems,
        })));

        let cs_element = JSXElement {
          span: DUMMY_SP,
          opening: JSXOpeningElement {
            name: JSXElementName::Ident(Ident::new("CS".into(), DUMMY_SP, SyntaxContext::empty())),
            span: DUMMY_SP,
            attrs: vec![],
            self_closing: false,
            type_args: None,
          },
          children: vec![JSXElementChild::JSXExprContainer(JSXExprContainer {
            span: DUMMY_SP,
            expr: cs_children,
          })],
          closing: Some(JSXClosingElement {
            span: DUMMY_SP,
            name: JSXElementName::Ident(Ident::new("CS".into(), DUMMY_SP, SyntaxContext::empty())),
          }),
        };

        // Create CC component wrapping CS and the original JSX element
        let cc_children = vec![
          JSXElementChild::JSXElement(Box::new(cs_element)),
          JSXElementChild::JSXExprContainer(JSXExprContainer {
            span: DUMMY_SP,
            expr: JSXExpr::Expr(Box::new(Expr::JSXElement(Box::new(n.clone())))),
          }),
        ];

        // Replace the current element with the CC wrapper
        *n = JSXElement {
          span: DUMMY_SP,
          opening: JSXOpeningElement {
            name: JSXElementName::Ident(Ident::new("CC".into(), DUMMY_SP, SyntaxContext::empty())),
            span: DUMMY_SP,
            attrs: vec![],
            self_closing: false,
            type_args: None,
          },
          children: cc_children,
          closing: Some(JSXClosingElement {
            span: DUMMY_SP,
            name: JSXElementName::Ident(Ident::new("CC".into(), DUMMY_SP, SyntaxContext::empty())),
          }),
        };
      }
    }
  }
  fn visit_mut_module(&mut self, m: &mut Module) {
    // Compute classic JSX pragma availability for this module, for gating in JSX visitors
    let mut classic_jsx_local_from_import: Option<String> = None;
    for item in &m.body {
      if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = item
        && &*import_decl.src.value == "@compiled/react"
      {
        for s in &import_decl.specifiers {
          if let ImportSpecifier::Named(named) = s {
            let is_jsx = match &named.imported {
              Some(ModuleExportName::Ident(i)) => &*i.sym == "jsx",
              Some(ModuleExportName::Str(s)) => &*s.value == "jsx",
              None => &*named.local.sym == "jsx",
            };
            if is_jsx {
              classic_jsx_local_from_import = Some(named.local.sym.to_string());
            }
          }
        }
      }
    }
    self.module_has_classic_compiled_jsx = get_jsx_classic_pragma_local()
      .and_then(|local| classic_jsx_local_from_import.map(|imp| imp == local))
      .unwrap_or(false);
    // Seed used identifiers with top-level declarations before generating our own
    for item in &m.body {
      if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(v))) = item {
        for d in &v.decls {
          collect_idents_from_pat(&d.name, &mut self.used_idents);
        }
      }
    }
    m.visit_mut_children_with(self);
    // Insert dev-only displayNames after styled component declarations
    if !self.pending_display_names.is_empty() {
      let mut new_body: Vec<ModuleItem> =
        Vec::with_capacity(m.body.len() + self.pending_display_names.len());
      for item in m.body.drain(..) {
        let mut pushed = false;
        match &item.clone() {
          ModuleItem::Stmt(Stmt::Decl(Decl::Var(v))) => {
            new_body.push(item.clone());
            pushed = true;
            for d in &v.decls {
              if let Pat::Ident(BindingIdent { id, .. }) = &d.name {
                let name_str = id.sym.to_string();
                if self.pending_display_names.iter().any(|n| n == &name_str) {
                  new_body.push(Self::build_display_if(&name_str));
                }
              }
            }
          }
          ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ed)) => {
            if let Decl::Var(v) = &ed.decl {
              new_body.push(item.clone());
              pushed = true;
              for d in &v.decls {
                if let Pat::Ident(BindingIdent { id, .. }) = &d.name {
                  let name_str = id.sym.to_string();
                  if self.pending_display_names.iter().any(|n| n == &name_str) {
                    new_body.push(Self::build_display_if(&name_str));
                  }
                }
              }
            }
          }
          _ => {}
        }
        if !pushed {
          new_body.push(item);
        }
      }
      m.body = new_body;
      self.pending_display_names.clear();
    }
    // Ensure needed imports
    crate::imports::ensure_forward_ref_import(self, m);
    crate::imports::ensure_runtime_ax_import(self, m);
    crate::imports::ensure_runtime_ix_import(self, m);
    crate::imports::ensure_runtime_cc_cs_imports(self, m);

    // Emit wrapper-grouped constants after traversal
    if !self.wrapper_key_to_rules.is_empty() {
      let mut keys: Vec<String> = self.wrapper_key_to_rules.keys().cloned().collect();
      keys.sort();
      for key in keys {
        if let Some(chunks) = self.wrapper_key_to_rules.remove(&key) {
          let wrappers = self.wrapper_key_to_stack.remove(&key).unwrap_or_default();
          let mut inner = String::with_capacity(chunks.iter().map(|s| s.len()).sum());
          for c in chunks {
            inner.push_str(&c);
          }
          // Wrap outer -> inner
          for w in wrappers.iter().rev() {
            let mut out = String::with_capacity(w.len() + 1 + inner.len() + 1);
            out.push_str(w);
            out.push('{');
            out.push_str(&inner);
            out.push('}');
            inner = out;
          }
          // Hoist as a const
          self.next_id += 1;
          let name = if self.next_id == 1 {
            "_".to_string()
          } else {
            format!("_{}", self.next_id)
          };
          let ident = Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty());
          let decl = ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind: VarDeclKind::Const,
            declare: false,
            decls: vec![VarDeclarator {
              span: DUMMY_SP,
              name: Pat::Ident(BindingIdent {
                id: ident.clone(),
                type_ann: None,
              }),
              init: Some(Box::new(Expr::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                value: inner.into(),
                raw: None,
              })))),
              definite: false,
            }],
          }))));
          self.hoisted.push(decl);
        }
        self.wrapper_key_to_seen.remove(&key);
      }
    }
    if !self.hoisted.is_empty() {
      let mut items = Vec::with_capacity(self.hoisted.len() + m.body.len());
      items.append(&mut self.hoisted);
      items.append(&mut m.body);
      m.body = items;
    }

    // Remove unused @compiled/react imports
    self.remove_unused_compiled_imports(m);

    // Add comment marker to the start of the module
    m.body.insert(
      0,
      ModuleItem::Stmt(Stmt::Expr(ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: "/* COMPILED_TRANSFORMED_ASSET */".into(),
          raw: None,
        }))),
      })),
    );
  }
  fn visit_mut_var_decl(&mut self, n: &mut VarDecl) {
    // Track declared identifiers to avoid name clashes
    for d in &n.decls {
      collect_idents_from_pat(&d.name, &mut self.used_idents);
    }
    if n.kind == VarDeclKind::Const {
      for d in &n.decls {
        if let Pat::Ident(BindingIdent { id, .. }) = &d.name
          && let Some(init) = &d.init
        {
          if let Some(v) = eval_const_expr(init) {
            self.const_env.insert(id.sym.clone(), v);
          } else if let Some(v) = eval_expr_to_const(init, &self.const_env) {
            self.const_env.insert(id.sym.clone(), v);
          } else {
            // Record class string for consts assigned from css({...}) or css`...`
            match &**init {
              Expr::Call(call) => {
                if let Callee::Expr(callee_expr) = &call.callee {
                  // resolve ident from possible seq expr
                  let mut maybe_ident: Option<Ident> = None;
                  match &**callee_expr {
                    Expr::Ident(i) => maybe_ident = Some(i.clone()),
                    Expr::Seq(seq) => {
                      if let Some(last) = seq.exprs.last()
                        && let Expr::Ident(i) = &**last
                      {
                        maybe_ident = Some(i.clone());
                      }
                    }
                    _ => {}
                  }
                  if let Some(ci) = maybe_ident {
                    // Handle cssMap(...) assigned to const ident
                    if self.css_map_local.as_ref().is_some_and(|w| ci.sym == *w)
                      && let Some(arg0) = call.args.first()
                      && let Expr::Object(outer) = &*arg0.expr
                    {
                      // Build const env object and per-key runtime entries
                      let mut obj_map: FxHashMap<String, evaluate_expr::ConstValue> =
                        FxHashMap::default();
                      let mut per_key: CssRuntimeResultPerKey = FxHashMap::default();
                      for p in &outer.props {
                        if let PropOrSpread::Prop(pp) = p
                          && let Prop::KeyValue(kv) = &**pp
                        {
                          let key_name = match &kv.key {
                            PropName::Ident(i) => i.sym.to_string(),
                            PropName::Str(s) => s.value.to_string(),
                            PropName::Num(n) => n.value.to_string(),
                            PropName::BigInt(b) => b.value.to_string(),
                            PropName::Computed(_) => continue,
                          };
                          if let Expr::Object(style_obj) = &*kv.value {
                            // Collect classes with runtime fallback enabled
                            self.begin_runtime_collection();
                            let classes = self.collect_atomic_classes_from_object(style_obj);
                            let runtime_entries = self.take_runtime_entries();
                            // Store results
                            let joined = join_classes_space(classes.clone());
                            obj_map
                              .insert(key_name.clone(), evaluate_expr::ConstValue::Str(joined));
                            per_key.insert(key_name, (classes, runtime_entries));
                          }
                        }
                      }
                      if !obj_map.is_empty() {
                        self
                          .const_env
                          .insert(id.sym.clone(), evaluate_expr::ConstValue::Obj(obj_map));
                      }
                      if !per_key.is_empty() {
                        self.css_map_result_by_ident.insert(id.sym.clone(), per_key);
                      }
                    }
                    if self.css_local.as_ref().is_some_and(|w| ci.sym == *w)
                      && let Some(arg0) = call.args.first()
                    {
                      match &*arg0.expr {
                        Expr::Object(o) => {
                          // Prefer static fast path for consts
                          let classes = if let Some(css_text) = self.build_css_text_from_object(o) {
                            self.collect_atomic_classes_from_css_text(&css_text)
                          } else {
                            self.begin_runtime_collection();
                            let classes = self.collect_atomic_classes_from_object(o);
                            let runtime_entries = self.take_runtime_entries();
                            self
                              .css_result_by_ident
                              .insert(id.sym.clone(), (Vec::new(), runtime_entries));
                            classes
                          };
                          let joined = join_classes_space(classes);
                          self
                            .const_env
                            .insert(id.sym.clone(), evaluate_expr::ConstValue::Str(joined));

                          // Mark for null replacement when CC/CS components are enabled
                          self.css_calls_to_nullify.insert(id.sym.clone());
                        }
                        Expr::Tpl(tpl) => {
                          if let Some(css_text) = self.build_css_text_from_tpl(tpl) {
                            let classes = self.collect_atomic_classes_from_css_text(&css_text);
                            let joined = join_classes_space(classes);
                            self
                              .const_env
                              .insert(id.sym.clone(), evaluate_expr::ConstValue::Str(joined));

                            self.css_calls_to_nullify.insert(id.sym.clone());
                          } else {
                            // Enable runtime var fallback and try again for templates
                            self.begin_runtime_collection();
                            if let Some(css_text) = self.build_css_text_from_tpl(tpl) {
                              let runtime_entries = self.take_runtime_entries();
                              let classes = self.collect_atomic_classes_from_css_text(&css_text);
                              let joined = join_classes_space(classes);
                              self
                                .const_env
                                .insert(id.sym.clone(), evaluate_expr::ConstValue::Str(joined));
                              self
                                .css_result_by_ident
                                .insert(id.sym.clone(), (Vec::new(), runtime_entries));

                              self.css_calls_to_nullify.insert(id.sym.clone());
                            }
                          }
                        }
                        _ => {}
                      }
                    }
                  }
                }
              }
              Expr::TaggedTpl(tt) => {
                // css`...`
                let mut maybe_ident: Option<Ident> = None;
                match &*tt.tag {
                  Expr::Ident(i) => maybe_ident = Some(i.clone()),
                  Expr::Seq(seq) => {
                    if let Some(last) = seq.exprs.last()
                      && let Expr::Ident(i) = &**last
                    {
                      maybe_ident = Some(i.clone());
                    }
                  }
                  _ => {}
                }
                if let Some(ci) = maybe_ident
                  && self.css_local.as_ref().is_some_and(|w| ci.sym == *w)
                  && let Some(css_text) = self.build_css_text_from_tpl(&tt.tpl)
                {
                  let classes = self.collect_atomic_classes_from_css_text(&css_text);
                  let joined = join_classes_space(classes);
                  self
                    .const_env
                    .insert(id.sym.clone(), evaluate_expr::ConstValue::Str(joined));

                  self.css_calls_to_nullify.insert(id.sym.clone());
                }
              }
              Expr::Object(obj) => {
                // Capture object literal with css() values into const_env as an object of strings
                let mut map: FxHashMap<String, evaluate_expr::ConstValue> = FxHashMap::default();
                for p in &obj.props {
                  if let PropOrSpread::Prop(pp) = p
                    && let Prop::KeyValue(kv) = &**pp
                  {
                    let key_name = match &kv.key {
                      PropName::Ident(i) => i.sym.to_string(),
                      PropName::Str(s) => s.value.to_string(),
                      PropName::Num(n) => n.value.to_string(),
                      PropName::BigInt(b) => b.value.to_string(),
                      PropName::Computed(_) => continue,
                    };
                    // css({}) or css`` inside object
                    match &*kv.value {
                      Expr::Call(call) => {
                        if let Callee::Expr(callee_expr) = &call.callee {
                          let mut maybe_ident: Option<Ident> = None;
                          match &**callee_expr {
                            Expr::Ident(i) => maybe_ident = Some(i.clone()),
                            Expr::Seq(seq) => {
                              if let Some(last) = seq.exprs.last()
                                && let Expr::Ident(i) = &**last
                              {
                                maybe_ident = Some(i.clone());
                              }
                            }
                            _ => {}
                          }
                          if let Some(ci) = maybe_ident
                            && self.css_local.as_ref().is_some_and(|w| ci.sym == *w)
                            && let Some(arg0) = call.args.first()
                          {
                            match &*arg0.expr {
                              Expr::Object(o) => {
                                let classes = self.collect_atomic_classes_from_object(o);
                                let joined = join_classes_space(classes);
                                map.insert(key_name, evaluate_expr::ConstValue::Str(joined));
                              }
                              Expr::Tpl(tpl) => {
                                if let Some(css_text) = self.build_css_text_from_tpl(tpl) {
                                  let classes =
                                    self.collect_atomic_classes_from_css_text(&css_text);
                                  let joined = join_classes_space(classes);
                                  map.insert(key_name, evaluate_expr::ConstValue::Str(joined));
                                }
                              }
                              _ => {}
                            }
                          }
                        }
                      }
                      Expr::TaggedTpl(tt2) => {
                        // css`...`
                        let mut maybe_ident: Option<Ident> = None;
                        match &*tt2.tag {
                          Expr::Ident(i) => maybe_ident = Some(i.clone()),
                          Expr::Seq(seq) => {
                            if let Some(last) = seq.exprs.last()
                              && let Expr::Ident(i) = &**last
                            {
                              maybe_ident = Some(i.clone());
                            }
                          }
                          _ => {}
                        }
                        if let Some(ci) = maybe_ident
                          && self.css_local.as_ref().is_some_and(|w| ci.sym == *w)
                          && let Some(css_text) = self.build_css_text_from_tpl(&tt2.tpl)
                        {
                          let classes = self.collect_atomic_classes_from_css_text(&css_text);
                          let joined = join_classes_space(classes);
                          map.insert(key_name, evaluate_expr::ConstValue::Str(joined));
                        }
                      }
                      _ => {}
                    }
                  }
                }
                if !map.is_empty() {
                  self
                    .const_env
                    .insert(id.sym.clone(), evaluate_expr::ConstValue::Obj(map));
                }
              }
              _ => {}
            }
          }
          // Additionally, if the initializer is an object literal containing css(...) or css`...`,
          // record a ConstValue::Obj mapping of keys to joined class strings so that spreads like
          // ...styles.default can be resolved later.
          if let Expr::Object(obj2) = &**init {
            let mut map2: FxHashMap<String, evaluate_expr::ConstValue> = FxHashMap::default();
            for p in &obj2.props {
              if let PropOrSpread::Prop(pp) = p
                && let Prop::KeyValue(kv) = &**pp
              {
                let key_name = match &kv.key {
                  PropName::Ident(i) => i.sym.to_string(),
                  PropName::Str(s) => s.value.to_string(),
                  PropName::Num(n) => n.value.to_string(),
                  PropName::BigInt(b) => b.value.to_string(),
                  PropName::Computed(_) => continue,
                };
                match &*kv.value {
                  Expr::Call(call) => {
                    if let Callee::Expr(callee_expr) = &call.callee {
                      let mut maybe_ident: Option<Ident> = None;
                      match &**callee_expr {
                        Expr::Ident(i) => maybe_ident = Some(i.clone()),
                        Expr::Seq(seq) => {
                          if let Some(last) = seq.exprs.last()
                            && let Expr::Ident(i) = &**last
                          {
                            maybe_ident = Some(i.clone());
                          }
                        }
                        _ => {}
                      }
                      if let Some(ci) = maybe_ident
                        && self.css_local.as_ref().is_some_and(|w| ci.sym == *w)
                        && let Some(arg0) = call.args.first()
                      {
                        match &*arg0.expr {
                          Expr::Object(o) => {
                            let classes = self.collect_atomic_classes_from_object(o);
                            let joined = join_classes_space(classes);
                            map2.insert(key_name, evaluate_expr::ConstValue::Str(joined));
                          }
                          Expr::Tpl(tpl) => {
                            if let Some(css_text) = self.build_css_text_from_tpl(tpl) {
                              let classes = self.collect_atomic_classes_from_css_text(&css_text);
                              let joined = join_classes_space(classes);
                              map2.insert(key_name, evaluate_expr::ConstValue::Str(joined));
                            }
                          }
                          _ => {}
                        }
                      }
                    }
                  }
                  Expr::TaggedTpl(tt2) => {
                    let mut maybe_ident: Option<Ident> = None;
                    match &*tt2.tag {
                      Expr::Ident(i) => maybe_ident = Some(i.clone()),
                      Expr::Seq(seq) => {
                        if let Some(last) = seq.exprs.last()
                          && let Expr::Ident(i) = &**last
                        {
                          maybe_ident = Some(i.clone());
                        }
                      }
                      _ => {}
                    }
                    if let Some(ci) = maybe_ident
                      && self.css_local.as_ref().is_some_and(|w| ci.sym == *w)
                      && let Some(css_text) = self.build_css_text_from_tpl(&tt2.tpl)
                    {
                      let classes = self.collect_atomic_classes_from_css_text(&css_text);
                      let joined = join_classes_space(classes);
                      map2.insert(key_name, evaluate_expr::ConstValue::Str(joined));
                    }
                  }
                  _ => {}
                }
              }
            }
            if !map2.is_empty() {
              self
                .const_env
                .insert(id.sym.clone(), evaluate_expr::ConstValue::Obj(map2));
            }
          }
        }
      }
      // Compile keyframes assigned to consts and record mapping to generated name
      if n.kind == VarDeclKind::Const {
        for d in &mut n.decls {
          if let Pat::Ident(BindingIdent { id, .. }) = &d.name
            && let Some(init) = &mut d.init
          {
            let mut compiled: Option<String> = None;
            match &mut **init {
              Expr::Call(call) => {
                if let Callee::Expr(callee_expr) = &call.callee
                  && let Expr::Ident(i) = &**callee_expr
                  && self.keyframes_local.as_ref().is_some_and(|w| i.sym == *w)
                  && let Some(arg0) = call.args.first()
                {
                  match &*arg0.expr {
                    Expr::Object(o) => compiled = Some(self.compile_keyframes_from_object(o)),
                    Expr::Tpl(t) => {
                      if let Some(text) = self.build_css_text_from_tpl(t) {
                        compiled = Some(self.compile_keyframes_from_css_text(&text));
                      } else if let Some((body, entries)) =
                        self.build_keyframes_body_from_tpl_with_vars(t)
                      {
                        let name = self.ensure_keyframes_rule(&body);
                        if !entries.is_empty() {
                          self.keyframes_name_to_runtime.insert(name.clone(), entries);
                        }
                        compiled = Some(name);
                      }
                    }
                    _ => {}
                  }
                }
              }
              Expr::TaggedTpl(tt) => {
                if let Expr::Ident(tag_ident) = &*tt.tag
                  && self
                    .keyframes_local
                    .as_ref()
                    .is_some_and(|w| tag_ident.sym == *w)
                {
                  if let Some(text) = self.build_css_text_from_tpl(&tt.tpl) {
                    compiled = Some(self.compile_keyframes_from_css_text(&text));
                  } else if let Some((body, entries)) =
                    self.build_keyframes_body_from_tpl_with_vars(&tt.tpl)
                  {
                    let name = self.ensure_keyframes_rule(&body);
                    if !entries.is_empty() {
                      self.keyframes_name_to_runtime.insert(name.clone(), entries);
                    }
                    compiled = Some(name);
                  }
                }
              }
              _ => {}
            }
            if let Some(name) = compiled {
              **init = Expr::Lit(Lit::Null(Null { span: DUMMY_SP }));
              self
                .const_env
                .insert(id.sym.clone(), evaluate_expr::ConstValue::Str(name));
            }
          }
        }
      }
    }
    // Handle styled.* assignments here so we can inject displayName later
    for d in &mut n.decls {
      if let Some(init) = &mut d.init {
        // styled.tag({...})
        let mut replaced: Option<Expr> = None;
        match &mut **init {
          Expr::Call(call) => {
            if let Callee::Expr(callee_expr) = &call.callee
              && let Expr::Member(me) = &**callee_expr
              && let Expr::Ident(obj_id) = &*me.obj
              && self.styled_local.as_ref().is_some_and(|w| obj_id.sym == *w)
              && let Some(expr) = self.try_transform_styled_call(me, call)
            {
              replaced = Some(expr);
            }
          }
          Expr::TaggedTpl(tt) => {
            if let Expr::Member(me) = &*tt.tag
              && let Expr::Ident(obj_id) = &*me.obj
              && self.styled_local.as_ref().is_some_and(|w| obj_id.sym == *w)
              && let Some(expr) = self.try_transform_styled_tagged_tpl(me, tt)
            {
              replaced = Some(expr);
            }
          }
          _ => {}
        }
        if let Some(new_expr) = replaced {
          **init = new_expr;
          if let Pat::Ident(BindingIdent { id, .. }) = &d.name {
            self.pending_display_names.push(id.sym.to_string());
          }
        }

        // Replace css() and css`` calls with null
        if let Pat::Ident(BindingIdent { id, .. }) = &d.name
          && self.css_calls_to_nullify.contains(&id.sym)
        {
          **init = Expr::Lit(Lit::Null(Null { span: DUMMY_SP }));
        }
      }
    }
    n.visit_mut_children_with(self);
  }

  fn visit_mut_import_decl(&mut self, n: &mut ImportDecl) {
    let src = &*n.src.value;
    let is_compiled_react = src == "@compiled/react";
    let is_atlaskit_css = src == "@atlaskit/css";
    if is_compiled_react || is_atlaskit_css {
      for s in &n.specifiers {
        if let ImportSpecifier::Named(named) = s {
          // Track local imported binding name
          self.used_idents.insert(named.local.sym.to_string());
          // Only @compiled/react can carry the classic jsx import we track
          if is_compiled_react {
            if let Some(imported) = &named.imported {
              if let ModuleExportName::Ident(i) = imported
                && &*i.sym == "jsx"
              {
                self.classic_jsx_local = Some(named.local.sym.clone());
              }
              if let ModuleExportName::Str(s) = imported
                && &*s.value == "jsx"
              {
                self.classic_jsx_local = Some(named.local.sym.clone());
              }
            } else if &*named.local.sym == "jsx" {
              self.classic_jsx_local = Some(named.local.sym.clone());
            }
          }
          // Map core APIs regardless of source (@compiled/react or @atlaskit/css)
          if let Some(imported) = &named.imported {
            match imported {
              ModuleExportName::Ident(i) => {
                if &*i.sym == "css" {
                  self.css_local = Some(named.local.sym.clone());
                }
                if &*i.sym == "cssMap" {
                  self.css_map_local = Some(named.local.sym.clone());
                }
                if &*i.sym == "styled" {
                  self.styled_local = Some(named.local.sym.clone());
                }
                if &*i.sym == "keyframes" {
                  self.keyframes_local = Some(named.local.sym.clone());
                }
              }
              ModuleExportName::Str(s) => {
                if &*s.value == "css" {
                  self.css_local = Some(named.local.sym.clone());
                }
                if &*s.value == "cssMap" {
                  self.css_map_local = Some(named.local.sym.clone());
                }
                if &*s.value == "styled" {
                  self.styled_local = Some(named.local.sym.clone());
                }
                if &*s.value == "keyframes" {
                  self.keyframes_local = Some(named.local.sym.clone());
                }
              }
            }
          } else {
            if &*named.local.sym == "css" {
              self.css_local = Some(named.local.sym.clone());
            }
            if &*named.local.sym == "cssMap" {
              self.css_map_local = Some(named.local.sym.clone());
            }
            if &*named.local.sym == "styled" {
              self.styled_local = Some(named.local.sym.clone());
            }
            if &*named.local.sym == "keyframes" {
              self.keyframes_local = Some(named.local.sym.clone());
            }
          }
        }
        if let ImportSpecifier::Default(default) = s {
          self.used_idents.insert(default.local.sym.to_string());
        }
        if let ImportSpecifier::Namespace(ns) = s {
          self.used_idents.insert(ns.local.sym.to_string());
        }
      }
    }
  }

  fn visit_mut_expr(&mut self, n: &mut Expr) {
    // Handle non-JSX expressions (JSX is handled in visit_mut_jsx_element)
    match n {
      Expr::JSXElement(_) | Expr::JSXFragment(_) => {
        // Just process normally, wrapping is handled in visit_mut_jsx_element
        n.visit_mut_children_with(self);
      }
      Expr::Call(call) => {
        if let Callee::Expr(expr) = &call.callee
          && let Expr::Member(me) = &**expr
          && let Expr::Ident(obj_id) = &*me.obj
          && self.styled_local.as_ref().is_some_and(|w| obj_id.sym == *w)
          && let Some(repl) = self.try_transform_styled_call(me, call)
        {
          *n = repl;
          return;
        }
        n.visit_mut_children_with(self);
      }
      Expr::TaggedTpl(tt) => {
        if let Expr::Member(me) = &*tt.tag
          && let Expr::Ident(obj_id) = &*me.obj
          && self.styled_local.as_ref().is_some_and(|w| obj_id.sym == *w)
          && let Some(repl) = self.try_transform_styled_tagged_tpl(me, tt)
        {
          *n = repl;
          return;
        }
        n.visit_mut_children_with(self);
      }
      _ => n.visit_mut_children_with(self),
    }
  }

  fn visit_mut_jsx_opening_element(&mut self, n: &mut JSXOpeningElement) {
    // Only transform inline css prop when either the module imports from @compiled/react,
    // the JSX import source pragma opts-in, or classic JSX pragma bound to compiled jsx is present.
    let has_compiled_import = self.css_local.is_some()
      || self.css_map_local.is_some()
      || self.styled_local.is_some()
      || self.keyframes_local.is_some();
    if !(has_compiled_import
      || is_jsx_import_source_compiled()
      || self.module_has_classic_compiled_jsx)
    {
      n.visit_mut_children_with(self);
      return;
    }
    // Process existing attributes first
    let mut next_attrs: Vec<JSXAttrOrSpread> = Vec::with_capacity(n.attrs.len());
    let mut ax_array_elems: Vec<ExprOrSpread> = Vec::new();
    let mut existing_class_expr: Option<Box<Expr>> = None;
    let mut existing_style_expr: Option<Box<Expr>> = None;
    let mut style_insert_index: Option<usize> = None;
    let mut runtime_style_entries_from_css: Vec<(String, Expr, bool)> = Vec::new();
    // removed: replaced by Self::classes_to_array_elem_static

    let mut _transformed_any_css = false;
    // Resolve css prop purely via unified resolver; no pending diagnostics tracking
    for attr in n.attrs.drain(..) {
      match attr {
        JSXAttrOrSpread::JSXAttr(a) => {
          if let JSXAttrName::Ident(name) = &a.name {
            if &*name.sym == "css" {
              _transformed_any_css = true;
              // Extract styles from css prop into ax array elements
              if let Some(JSXAttrValue::JSXExprContainer(c)) = &a.value {
                match &c.expr {
                  JSXExpr::Expr(expr) => {
                    self.collect_from_css_expr(
                      expr,
                      &mut ax_array_elems,
                      &mut runtime_style_entries_from_css,
                    );
                  }
                  JSXExpr::JSXEmptyExpr(_) => {}
                }
              }
              // Skip adding this attr (we replace it with className)
              continue;
            }
            // Handle any prop ending with "xcss" (case-insensitive) when value is an inline object
            {
              let lower = name.sym.to_string().to_lowercase();
              if lower.ends_with("xcss")
                && let Some(JSXAttrValue::JSXExprContainer(c)) = &a.value
                && let JSXExpr::Expr(expr) = &c.expr
              {
                // Temporarily set in_xcss_attr_depth to gate nested css() transforms within
                self.in_xcss_attr_depth += 1;
                let result = if let Expr::Object(o) = &**expr {
                  // Collect with runtime fallback enabled, but panic if any runtime vars were needed
                  self.begin_runtime_collection();
                  let classes = self.collect_atomic_classes_from_object(o);
                  let runtime_entries = self.take_runtime_entries();
                  if !runtime_entries.is_empty() {
                    panic!("xcss prop values must be statically resolvable");
                  }
                  // Join into a single string token (space-separated)
                  let mut joined = String::new();
                  let mut first = true;
                  for c in classes {
                    if !first {
                      joined.push(' ');
                    }
                    joined.push_str(&c);
                    first = false;
                  }
                  // Replace attribute with the same name and a string literal value
                  let new_attr = JSXAttrOrSpread::JSXAttr(JSXAttr {
                    span: DUMMY_SP,
                    name: a.name.clone(),
                    value: Some(JSXAttrValue::Lit(Lit::Str(Str {
                      span: DUMMY_SP,
                      value: joined.into(),
                      raw: None,
                    }))),
                  });
                  Some(new_attr)
                } else {
                  // Not an inline object: leave attribute intact and do not traverse inside
                  next_attrs.push(JSXAttrOrSpread::JSXAttr(a));
                  self.in_xcss_attr_depth -= 1;
                  continue;
                };
                self.in_xcss_attr_depth -= 1;
                if let Some(new_attr) = result {
                  next_attrs.push(new_attr);
                  continue;
                }
              }
              // For non-object expressions (identifiers, calls, conditionals, etc.) leave as-is
            }
            if &*name.sym == "className" {
              // Capture existing className expression to merge later
              if let Some(JSXAttrValue::JSXExprContainer(c)) = &a.value
                && let JSXExpr::Expr(expr) = &c.expr
              {
                existing_class_expr = Some(expr.clone());
              }
              // Do not push existing className; we'll re-create it
              continue;
            }
            if &*name.sym == "style" {
              if let Some(JSXAttrValue::JSXExprContainer(c)) = &a.value
                && let JSXExpr::Expr(expr) = &c.expr
              {
                existing_style_expr = Some(expr.clone());
              }
              // Remember original position to re-insert style later to preserve order
              style_insert_index = Some(next_attrs.len());
              continue;
            }
          }
          next_attrs.push(JSXAttrOrSpread::JSXAttr(a));
        }
        other => next_attrs.push(other),
      }
    }

    // Emit/restore className
    // Build pending className attribute (we'll append after style insertion to keep style before className)
    let mut pending_class_attr: Option<JSXAttrOrSpread> = None;
    if !ax_array_elems.is_empty() {
      if let Some(expr) = existing_class_expr {
        ax_array_elems.push(ExprOrSpread { spread: None, expr });
      }
      let ax_ident = Ident::new("ax".into(), DUMMY_SP, SyntaxContext::empty());
      self.need_runtime_ax = true;
      let call_expr = Expr::Call(CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Callee::Expr(Box::new(Expr::Ident(ax_ident))),
        args: vec![ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Array(ArrayLit {
            span: DUMMY_SP,
            elems: ax_array_elems.into_iter().map(Some).collect(),
          })),
        }],
        type_args: None,
      });
      let class_attr = JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: DUMMY_SP,
        name: JSXAttrName::Ident(
          Ident::new("className".into(), DUMMY_SP, SyntaxContext::empty()).into(),
        ),
        value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
          span: DUMMY_SP,
          expr: JSXExpr::Expr(Box::new(call_expr)),
        })),
      });
      pending_class_attr = Some(class_attr);
    } else if let Some(expr) = existing_class_expr {
      let class_attr = JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: DUMMY_SP,
        name: JSXAttrName::Ident(
          Ident::new("className".into(), DUMMY_SP, SyntaxContext::empty()).into(),
        ),
        value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
          span: DUMMY_SP,
          expr: JSXExpr::Expr(expr),
        })),
      });
      pending_class_attr = Some(class_attr);
    }

    // If we need to inject runtime custom properties from css templates, build/merge style
    if !runtime_style_entries_from_css.is_empty() {
      // mark ix import
      self.need_runtime_ix = true;
      // Deduplicate by var name (FxHashSet for perf)
      let mut seen: rustc_hash::FxHashSet<String> = rustc_hash::FxHashSet::default();
      let mut props: Vec<PropOrSpread> =
        Vec::with_capacity(runtime_style_entries_from_css.len() + 1);
      if let Some(style_expr) = existing_style_expr.take() {
        props.push(PropOrSpread::Spread(SpreadElement {
          dot3_token: DUMMY_SP,
          expr: style_expr,
        }));
      }
      for (var_name, ix_arg, wrap_iife) in runtime_style_entries_from_css {
        if !seen.insert(var_name.clone()) {
          continue;
        }
        let key = PropName::Str(Str {
          span: DUMMY_SP,
          value: var_name.into(),
          raw: None,
        });
        let ix_arg_expr: Expr = if wrap_iife {
          Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(Expr::Paren(ParenExpr {
              span: DUMMY_SP,
              expr: Box::new(Expr::Arrow(ArrowExpr {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                params: vec![],
                body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
                  span: DUMMY_SP,
                  ctxt: SyntaxContext::empty(),
                  stmts: vec![Stmt::Return(ReturnStmt {
                    span: DUMMY_SP,
                    arg: Some(Box::new(ix_arg)),
                  })],
                })),
                is_async: false,
                is_generator: false,
                type_params: None,
                return_type: None,
              })),
            }))),
            args: vec![],
            type_args: None,
          })
        } else {
          ix_arg
        };
        let call = Expr::Call(CallExpr {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
            "ix".into(),
            DUMMY_SP,
            SyntaxContext::empty(),
          )))),
          args: vec![ExprOrSpread {
            spread: None,
            expr: Box::new(ix_arg_expr),
          }],
          type_args: None,
        });
        props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key,
          value: Box::new(call),
        }))));
      }
      let style_obj = Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props,
      });
      let style_attr = JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: DUMMY_SP,
        name: JSXAttrName::Ident(
          Ident::new("style".into(), DUMMY_SP, SyntaxContext::empty()).into(),
        ),
        value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
          span: DUMMY_SP,
          expr: JSXExpr::Expr(Box::new(style_obj)),
        })),
      });
      if let Some(idx) = style_insert_index {
        next_attrs.insert(idx, style_attr);
      } else {
        next_attrs.push(style_attr);
      }
    } else if let Some(style_expr) = existing_style_expr.take() {
      // Preserve an existing style if we didn't inject, at original position if possible
      let style_attr = JSXAttrOrSpread::JSXAttr(JSXAttr {
        span: DUMMY_SP,
        name: JSXAttrName::Ident(
          Ident::new("style".into(), DUMMY_SP, SyntaxContext::empty()).into(),
        ),
        value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
          span: DUMMY_SP,
          expr: JSXExpr::Expr(style_expr),
        })),
      });
      if let Some(idx) = style_insert_index {
        next_attrs.insert(idx, style_attr);
      } else {
        next_attrs.push(style_attr);
      }
    }
    // Append className last (to match expected ordering in fixtures)
    if let Some(class_attr) = pending_class_attr.take() {
      next_attrs.push(class_attr);
    }

    n.attrs = next_attrs;
    // No diagnostics for non-const css outside JSX; unattached dynamic css is dropped.
    // Do not recurse into attributes; we've fully handled transformations for this element.
  }
}

// Lightweight parser for extracting animation-name values from a css`...` text
// removed: use crate::template::parse_animation_names_from_css_text

fn join_classes_space(mut classes: Vec<String>) -> String {
  if classes.is_empty() {
    return String::new();
  }
  if classes.len() == 1 {
    return classes.pop().unwrap();
  }
  let total_len: usize =
    classes.iter().map(|c| c.len()).sum::<usize>() + classes.len().saturating_sub(1);
  let mut out = String::with_capacity(total_len);
  let mut first = true;
  for c in classes {
    if !first {
      out.push(' ');
    }
    out.push_str(&c);
    first = false;
  }
  out
}

pub fn process_transform(program: Program) -> Program {
  process_transform_with_config(program, true)
}

pub fn process_transform_with_config(program: Program, emit_cc_cs: bool) -> Program {
  let mut program = program;
  let mut v = AtomicCssCollector {
    emit_cc_cs_components: emit_cc_cs,
    ..Default::default()
  };
  program.visit_mut_with(&mut v);
  program
}

#[cfg(feature = "plugin")]
mod plugin_entry {
  use super::*;
  use swc_core::plugin::metadata::TransformPluginProgramMetadata;
  use swc_plugin_macro::plugin_transform;

  #[plugin_transform]
  pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
    super::process_transform(program)
  }
}

#[cfg(feature = "plugin")]
pub use plugin_entry::process_transform as __swc_plugin_process_transform;

// Native transform entry point for integration into a Rust pipeline
/// Apply the compiled atomic transform in-place to a Program (native pipeline).
pub fn apply_compiled_atomic(program: &mut Program) -> CompiledCssInJsTransformResult {
  apply_compiled_atomic_with_config(program, CompiledCssInJsTransformConfig::default())
}

pub fn compiled_css_in_js_visitor(config: &CompiledCssInJsTransformConfig) -> impl Pass {
  let emit_cc_cs = !config.extract.unwrap_or(false);

  visit_mut_pass(AtomicCssCollector::new(emit_cc_cs))
}

/// Apply the compiled atomic transform in-place to a Program with configuration.
pub fn apply_compiled_atomic_with_config(
  program: &mut Program,
  config: CompiledCssInJsTransformConfig,
) -> CompiledCssInJsTransformResult {
  let emit_cc_cs = !config.extract.unwrap_or(false);
  let mut v = AtomicCssCollector::new(emit_cc_cs);
  program.visit_mut_with(&mut v);

  CompiledCssInJsTransformResult {
    style_rules: v.style_rules.into_iter().collect(),
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    config::CompiledCssInJsTransformConfig,
    tests::utils::{assert_contains, transform_code},
  };
  pub(crate) mod utils;

  #[test]
  fn test_extract() {
    let code = r#"
    import { css } from '@compiled/react';
  
    const styles = css({
      color: 'blue',
    });
  
    <div css={styles} />;
    "#;

    let (actual_code, result) = transform_code(
      code,
      "test.jsx",
      Some(CompiledCssInJsTransformConfig {
        extract: Some(true),
        ..Default::default()
      }),
    );

    assert_contains(&actual_code, "._syaz13q2{color:blue}");
    assert_eq!(result.style_rules.len(), 1);
    assert_eq!(result.style_rules[0], "._syaz13q2{color:blue}");
  }
}
