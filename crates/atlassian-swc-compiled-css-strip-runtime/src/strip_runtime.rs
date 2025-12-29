use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, write};
use std::path::{Path, PathBuf};

use atlassian_swc_compiled_css::{SortOptions, TransformError, sort_atomic_style_sheet};
use swc_core::atoms::Atom;
use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::{
  ArrayLit, Callee, Expr, ExprOrSpread, Ident, ImportDecl, ImportSpecifier, Lit, ModuleDecl,
  ModuleItem, Program, Stmt, Str,
};
use swc_core::ecma::visit::{VisitMut, VisitMutWith, noop_visit_mut_type};

use crate::types::{PluginOptions, TransformConfig, TransformMetadata};
use crate::utils_is_automatic_runtime::is_automatic_runtime;
use crate::utils_is_cc_component::is_cc_component;
use crate::utils_is_create_element::is_create_element;
use crate::utils_remove_style_declarations::{
  remove_style_declarations_from_expr, remove_style_declarations_from_jsx_element,
};
use crate::utils_to_uri_component::to_uri_component;

type IdentId = Atom;

#[derive(Default, Clone)]
struct StyleBinding {
  value: Option<String>,
  collected: bool,
}

/// Native SWC transform mirroring `@compiled/babel-plugin-strip-runtime`.
pub struct StripRuntimeTransform {
  config: TransformConfig,
  metadata: TransformMetadata,
  style_rules: Vec<String>,
  bindings: HashMap<IdentId, StyleBinding>,
  bindings_to_remove: HashSet<IdentId>,
  errors: Vec<TransformError>,
}

impl StripRuntimeTransform {
  pub fn new(config: TransformConfig) -> Self {
    Self {
      config,
      metadata: TransformMetadata::default(),
      style_rules: Vec::new(),
      bindings: HashMap::new(),
      bindings_to_remove: HashSet::new(),
      errors: Vec::new(),
    }
  }

  pub fn finish(self) -> (TransformMetadata, Vec<TransformError>) {
    (self.metadata, self.errors)
  }

  pub fn options(&self) -> &PluginOptions {
    &self.config.options
  }

  fn push_error(&mut self, message: impl Into<String>) -> TransformError {
    let error = TransformError::new(message);
    self.errors.push(error.clone());
    error
  }

  fn ident_id(&self, ident: &swc_core::ecma::ast::Ident) -> IdentId {
    ident.sym.clone()
  }

  fn record_binding(&mut self, ident: &swc_core::ecma::ast::Ident, value: Option<String>) {
    let id = self.ident_id(ident);
    self
      .bindings
      .entry(id)
      .or_insert_with(StyleBinding::default)
      .value = value;
  }

  fn collect_style_identifier(&mut self, ident: &swc_core::ecma::ast::Ident) {
    let id = self.ident_id(ident);
    if let Some(binding) = self.bindings.get_mut(&id) {
      if let Some(value) = &binding.value {
        if !binding.collected {
          self.style_rules.push(value.clone());
          binding.collected = true;
        }
        self.bindings_to_remove.insert(id);
      }
    }
  }

  fn collect_styles_from_expr(&mut self, expr: &Expr) {
    let mut handler = |ident: &swc_core::ecma::ast::Ident| {
      self.collect_style_identifier(ident);
    };
    remove_style_declarations_from_expr(expr, &mut handler);
  }

  fn collect_styles_from_jsx(&mut self, element: &swc_core::ecma::ast::JSXElement) {
    let mut handler = |ident: &swc_core::ecma::ast::Ident| {
      self.collect_style_identifier(ident);
    };
    remove_style_declarations_from_jsx_element(element, &mut handler);
  }

  fn replace_cc_jsx(&mut self, element: &mut swc_core::ecma::ast::JSXElement) -> Option<Expr> {
    use swc_core::ecma::ast::{JSXElementChild, JSXElementName, JSXExpr, JSXExprContainer};

    let JSXElementName::Ident(name) = &element.opening.name else {
      return None;
    };

    if name.sym.as_ref() != "CC" {
      return None;
    }

    let mut compiled_styles_child: Option<swc_core::ecma::ast::JSXElement> = None;
    let mut node_to_replace: Option<JSXElementChild> = None;

    for child in &element.children {
      match child {
        JSXElementChild::JSXElement(el) => {
          if matches!(
              el.opening.name,
              JSXElementName::Ident(ref ident) if ident.sym.as_ref() == "CS"
          ) {
            compiled_styles_child = Some((**el).clone());
          } else {
            node_to_replace = Some(child.clone());
          }
        }
        JSXElementChild::JSXFragment(_) => {
          node_to_replace = Some(child.clone());
        }
        JSXElementChild::JSXExprContainer(JSXExprContainer { expr, .. }) => {
          if matches!(expr, JSXExpr::Expr(_)) {
            node_to_replace = Some(child.clone());
          }
        }
        JSXElementChild::JSXText(text) => {
          if !text.value.as_ref().trim().is_empty() {
            node_to_replace = Some(child.clone());
          }
        }
        JSXElementChild::JSXSpreadChild(_) => {}
      }
    }

    let Some(styles) = compiled_styles_child else {
      return None;
    };

    self.collect_styles_from_jsx(&styles);

    let Some(replacement) = node_to_replace else {
      return None;
    };

    match replacement {
      JSXElementChild::JSXExprContainer(JSXExprContainer { expr, .. }) => {
        if let JSXExpr::Expr(expr) = expr {
          Some(*expr)
        } else {
          None
        }
      }
      JSXElementChild::JSXElement(el) => Some(Expr::JSXElement(el)),
      JSXElementChild::JSXFragment(fragment) => Some(Expr::JSXFragment(fragment)),
      JSXElementChild::JSXText(text) => Some(Expr::Lit(Lit::Str(Str {
        span: text.span,
        value: text.value,
        raw: None,
      }))),
      JSXElementChild::JSXSpreadChild(_) => None,
    }
  }

  fn replace_cc_create_element(&mut self, call: &swc_core::ecma::ast::CallExpr) -> Option<Expr> {
    if !matches!(call.callee, Callee::Expr(ref expr) if is_create_element(expr)) {
      return None;
    }

    let component = call.args.get(0)?.expr.as_ref();
    if !is_cc_component(component) {
      return None;
    }

    let styles_expr = call.args.get(2)?.expr.as_ref();
    self.collect_styles_from_expr(styles_expr);

    let node_to_replace = call.args.get(3)?.expr.clone();
    Some(*node_to_replace)
  }

  fn replace_cc_jsxs_call(&mut self, call: &mut swc_core::ecma::ast::CallExpr) -> Option<Expr> {
    if !is_automatic_runtime(call, "jsxs") {
      return None;
    }

    let component = call.args.get(0)?.expr.as_ref();
    if !is_cc_component(component) {
      return None;
    }

    let props_arg = call.args.get_mut(1)?;
    let Expr::Object(obj) = &mut *props_arg.expr else {
      return None;
    };

    let mut compiled_styles: Option<Expr> = None;
    let mut replacement: Option<Expr> = None;

    for prop in &obj.props {
      use swc_core::ecma::ast::{Prop, PropName};
      let Some(prop) = prop.as_prop() else {
        continue;
      };
      let Prop::KeyValue(kv) = &**prop else {
        continue;
      };
      let PropName::Ident(ref key) = kv.key else {
        continue;
      };
      if key.sym.as_ref() != "children" {
        continue;
      }

      if let Expr::Array(ArrayLit { elems, .. }) = &*kv.value {
        if let Some(Some(first)) = elems.get(0) {
          if first.spread.is_none() {
            compiled_styles = Some((*first.expr).clone());
          }
        }

        if let Some(Some(second)) = elems.get(1) {
          if second.spread.is_none() {
            replacement = Some((*second.expr).clone());
          }
        }
      }
    }

    let styles = compiled_styles?;
    self.collect_styles_from_expr(&styles);
    replacement
  }

  fn expr_to_jsx_child(expr: Expr) -> Option<swc_core::ecma::ast::JSXElementChild> {
    use swc_core::ecma::ast::{JSXElementChild, JSXExpr, JSXExprContainer, JSXText};

    match expr {
      Expr::JSXElement(el) => Some(JSXElementChild::JSXElement(el)),
      Expr::JSXFragment(fragment) => Some(JSXElementChild::JSXFragment(fragment)),
      Expr::Lit(Lit::Str(s)) => Some(JSXElementChild::JSXText(JSXText {
        span: s.span,
        value: s.value.clone(),
        raw: s.value,
      })),
      other => Some(JSXElementChild::JSXExprContainer(JSXExprContainer {
        span: DUMMY_SP,
        expr: JSXExpr::Expr(Box::new(other)),
      })),
    }
  }

  fn finalize_program(&mut self, program: &mut Program) {
    if self.style_rules.is_empty() {
      return;
    }

    // COMPAT: Deduplicate redundant universal descendant selectors (e.g., " * *")
    // that can arise from nested universal rules. Babel's postcss pipeline emits
    // only a single level of universal descendant for this pattern.
    let reported = self.config.filename.as_deref().unwrap_or("undefined");
    if self.config.filename.is_none() || reported.is_empty() {
      self.push_error(format!(
        "@compiled/babel-plugin-strip-runtime expected the filename not to be empty, but actually got '{}'.",
        reported
      ));
      return;
    }

    if self.options().compiled_require_exclude.unwrap_or(false) {
      self
        .metadata
        .style_rules
        .extend(self.style_rules.iter().cloned());
      return;
    }

    if let Some(path) = self.config.options.style_sheet_path.clone() {
      self.insert_requires(program, path.as_str());
    }

    if let Some(extract) = self.config.options.extract_styles_to_directory.clone() {
      if let Err(err) = self.extract_styles_to_directory(program, extract) {
        self.errors.push(err);
      }
    }
  }

  fn insert_requires(&mut self, program: &mut Program, style_sheet_path: &str) {
    let stmts = self.style_rules.clone();
    if stmts.is_empty() {
      return;
    }

    let requires: Vec<ModuleItem> = stmts
      .iter()
      .map(|rule| {
        let encoded = to_uri_component(rule);
        let arg = Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: Atom::from(format!("{}?style={}", style_sheet_path, encoded)),
          raw: None,
        }));
        let call = Expr::Call(swc_core::ecma::ast::CallExpr {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
            Atom::from("require"),
            DUMMY_SP,
            SyntaxContext::empty(),
          )))),
          args: vec![ExprOrSpread {
            spread: None,
            expr: Box::new(arg),
          }],
          type_args: None,
        });
        ModuleItem::Stmt(Stmt::Expr(swc_core::ecma::ast::ExprStmt {
          span: DUMMY_SP,
          expr: Box::new(call),
        }))
      })
      .collect();

    match program {
      Program::Module(module) => {
        for (idx, item) in requires.into_iter().enumerate() {
          module.body.insert(idx, item);
        }
      }
      Program::Script(script) => {
        for (idx, item) in requires.into_iter().enumerate() {
          if let ModuleItem::Stmt(stmt) = item {
            script.body.insert(idx, stmt);
          }
        }
      }
    }
  }

  fn extract_styles_to_directory(
    &mut self,
    program: &mut Program,
    extract: crate::types::ExtractStylesToDirectory,
  ) -> Result<(), TransformError> {
    let filename = self
      .config
      .filename
      .as_ref()
      .and_then(|name| if name.is_empty() { None } else { Some(name) })
      .ok_or_else(|| {
        TransformError::new(format!(
          "@compiled/babel-plugin-strip-runtime expected the filename not to be empty, but actually got '{}'",
          self.config.filename.as_deref().unwrap_or("undefined")
        ))
      })?;

    let source_file_name = self
      .config
      .source_file_name
      .as_ref()
      .filter(|name| !name.is_empty())
      .ok_or_else(|| TransformError::new("Source filename was not defined"))?;

    let Some(relative_start) = source_file_name.find(&extract.source) else {
      return Err(TransformError::new(format!(
        "Source directory '{}' was not found relative to source file ('{}')",
        extract.source, source_file_name
      )));
    };

    let css_filename = Path::new(filename)
      .file_stem()
      .map(|stem| format!("{}.compiled.css", stem.to_string_lossy()))
      .unwrap_or_else(|| "compiled.css".to_string());

    let relative = &source_file_name[relative_start + extract.source.len()..];

    let cwd = self
      .config
      .cwd
      .clone()
      .map(PathBuf::from)
      .map(Ok)
      .unwrap_or_else(std::env::current_dir)
      .map_err(|err| TransformError::new(format!("Failed to resolve current directory: {err}")))?;

    let css_path = cwd
      .join(&extract.dest)
      .join(
        Path::new(relative)
          .parent()
          .unwrap_or_else(|| Path::new("")),
      )
      .join(&css_filename);

    if let Some(parent) = css_path.parent() {
      create_dir_all(parent).map_err(|err| {
        TransformError::new(format!(
          "Failed to create CSS output directory '{}': {err}",
          parent.display()
        ))
      })?;
    }

    let mut rules = self.style_rules.clone();
    rules.sort();
    let css = rules.join("\n");
    let sorted = sort_atomic_style_sheet(
      &css,
      SortOptions {
        sort_at_rules: self.options().sort_at_rules,
        sort_shorthand: self.options().sort_shorthand,
      },
    );

    write(&css_path, sorted).map_err(|err| {
      TransformError::new(format!(
        "Failed to write extracted stylesheet '{}': {err}",
        css_path.display()
      ))
    })?;

    let import = ModuleDecl::Import(ImportDecl {
      span: DUMMY_SP,
      specifiers: Vec::new(),
      src: Box::new(Str {
        span: DUMMY_SP,
        value: Atom::from(format!("./{}", css_filename)),
        raw: None,
      }),
      type_only: false,
      with: None,
      phase: swc_core::ecma::ast::ImportPhase::Evaluation,
    });

    match program {
      Program::Module(module) => {
        module.body.insert(0, ModuleItem::ModuleDecl(import));
      }
      Program::Script(script) => {
        let require_stmt = Stmt::Expr(swc_core::ecma::ast::ExprStmt {
          span: DUMMY_SP,
          expr: Box::new(Expr::Call(swc_core::ecma::ast::CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
              Atom::from("require"),
              DUMMY_SP,
              SyntaxContext::empty(),
            )))),
            args: vec![ExprOrSpread {
              spread: None,
              expr: Box::new(Expr::Lit(Lit::Str(Str {
                span: DUMMY_SP,
                value: Atom::from(format!("./{}", css_filename)),
                raw: None,
              }))),
            }],
            type_args: None,
          })),
        });
        script.body.insert(0, require_stmt);
      }
    }
    Ok(())
  }

  fn prune_empty_nodes(&mut self, program: &mut Program) {
    match program {
      Program::Module(module) => {
        for item in &mut module.body {
          if let ModuleItem::Stmt(Stmt::Decl(swc_core::ecma::ast::Decl::Var(var))) = item {
            var.decls.retain(|decl| match &decl.name {
              swc_core::ecma::ast::Pat::Ident(binding) => !self
                .bindings_to_remove
                .contains(&self.ident_id(&binding.id)),
              _ => true,
            });
          }
        }

        module.body.retain(|item| match item {
          ModuleItem::Stmt(Stmt::Decl(swc_core::ecma::ast::Decl::Var(var))) => {
            !var.decls.is_empty()
          }
          ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => !import.specifiers.is_empty(),
          _ => true,
        });
      }
      Program::Script(script) => {
        for stmt in &mut script.body {
          if let Stmt::Decl(swc_core::ecma::ast::Decl::Var(var)) = stmt {
            var.decls.retain(|decl| match &decl.name {
              swc_core::ecma::ast::Pat::Ident(binding) => !self
                .bindings_to_remove
                .contains(&self.ident_id(&binding.id)),
              _ => true,
            });
          }
        }

        script.body.retain(|stmt| match stmt {
          Stmt::Decl(swc_core::ecma::ast::Decl::Var(var)) => !var.decls.is_empty(),
          _ => true,
        });
      }
    }
  }
}

impl VisitMut for StripRuntimeTransform {
  noop_visit_mut_type!();

  fn visit_mut_jsx_fragment(&mut self, fragment: &mut swc_core::ecma::ast::JSXFragment) {
    fragment.visit_mut_children_with(self);

    // Unwrap any CC children inside fragments (common for return <>...).
    for child in &mut fragment.children {
      if let swc_core::ecma::ast::JSXElementChild::JSXElement(child_el) = child {
        let is_cc = matches!(
            child_el.opening.name,
            swc_core::ecma::ast::JSXElementName::Ident(ref ident) if ident.sym.as_ref() == "CC"
        );
        if is_cc {
          if let Some(replacement) = self.replace_cc_jsx(child_el) {
            if let Some(new_child) = Self::expr_to_jsx_child(replacement) {
              *child = new_child;
            }
          }
        }
      }
    }
  }

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    expr.visit_mut_children_with(self);

    if let Expr::JSXElement(element) = expr {
      if let Some(replacement) = self.replace_cc_jsx(element) {
        *expr = replacement;
        return;
      }
    }

    if let Expr::Call(call) = expr {
      if let Some(replacement) = self.replace_cc_create_element(call) {
        *expr = replacement;
        return;
      }

      if let Some(replacement) = self.replace_cc_jsxs_call(call) {
        *expr = replacement;
        return;
      }
    }
  }

  fn visit_mut_jsx_element(&mut self, element: &mut swc_core::ecma::ast::JSXElement) {
    use swc_core::ecma::ast::JSXElementName;

    // First, traverse into children
    element.visit_mut_children_with(self);

    // Replace any CC children inline (nested wrappers), mirroring Babel's JSXElement behavior.
    for child in &mut element.children {
      if let swc_core::ecma::ast::JSXElementChild::JSXElement(child_el) = child {
        let is_cc = matches!(
            child_el.opening.name,
            JSXElementName::Ident(ref ident) if ident.sym.as_ref() == "CC"
        );
        if is_cc {
          if let Some(replacement) = self.replace_cc_jsx(child_el) {
            if let Some(new_child) = Self::expr_to_jsx_child(replacement) {
              *child = new_child;
            }
          }
        }
      }
    }
  }

  fn visit_mut_var_declarator(&mut self, declarator: &mut swc_core::ecma::ast::VarDeclarator) {
    declarator.visit_mut_children_with(self);
    if let swc_core::ecma::ast::Pat::Ident(binding) = &declarator.name {
      let value = declarator.init.as_ref().and_then(|expr| match &**expr {
        Expr::Lit(Lit::Str(str)) => Some(str.value.to_string()),
        _ => None,
      });
      self.record_binding(&binding.id, value);
    }
  }

  fn visit_mut_var_decl(&mut self, decl: &mut swc_core::ecma::ast::VarDecl) {
    decl.visit_mut_children_with(self);
  }

  fn visit_mut_import_decl(&mut self, import: &mut ImportDecl) {
    import.visit_mut_children_with(self);
    import.specifiers.retain(|specifier| match specifier {
      ImportSpecifier::Named(named) => match &named.imported {
        Some(swc_core::ecma::ast::ModuleExportName::Ident(ident)) => {
          let name = ident.sym.as_ref();
          !(name == "CC" || name == "CS")
        }
        None => {
          let local = named.local.sym.as_ref();
          !(local == "CC" || local == "CS")
        }
        _ => true,
      },
      _ => true,
    });
  }

  fn visit_mut_program(&mut self, program: &mut Program) {
    program.visit_mut_children_with(self);
    self.prune_empty_nodes(program);
    self.finalize_program(program);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::{ExtractStylesToDirectory, PluginOptions, TransformConfig};
  use std::fs::{File, read_to_string};
  use std::io::Write as _;
  use swc_core::common::comments::{Comments, SingleThreadedComments};
  use swc_core::common::sync::Lrc;
  use swc_core::common::{FileName, SourceMap};
  use swc_core::ecma::codegen::{Config, Emitter, text_writer::JsWriter};
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{Parser, StringInput, Syntax};
  use tempfile::tempdir;

  fn parse(source: &str) -> (Program, Lrc<SourceMap>, SingleThreadedComments) {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Anon.into(), source.into());
    let comments = SingleThreadedComments::default();
    let lexer = Lexer::new(
      Syntax::Es(Default::default()),
      Default::default(),
      StringInput::from(&*fm),
      Some(&comments),
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser.parse_module().expect("failed to parse module");
    (Program::Module(module), cm, comments)
  }

  fn parse_script(source: &str) -> (Program, Lrc<SourceMap>, SingleThreadedComments) {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Anon.into(), source.into());
    let comments = SingleThreadedComments::default();
    let lexer = Lexer::new(
      Syntax::Es(Default::default()),
      Default::default(),
      StringInput::from(&*fm),
      Some(&comments),
    );
    let mut parser = Parser::new_from(lexer);
    let script = parser.parse_script().expect("failed to parse script");
    (Program::Script(script), cm, comments)
  }

  fn print(program: &Program, cm: &Lrc<SourceMap>, comments: &SingleThreadedComments) -> String {
    let mut buf = Vec::new();
    {
      let mut emitter = Emitter {
        cfg: Config::default(),
        comments: Some(comments as &dyn Comments),
        cm: cm.clone(),
        wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
      };
      match program {
        Program::Module(module) => emitter.emit_module(module).unwrap(),
        Program::Script(script) => emitter.emit_script(script).unwrap(),
      }
    }
    String::from_utf8(buf).unwrap()
  }

  #[test]
  fn strips_create_element_runtime() {
    let (program, cm, comments) = parse(
      "import { CC, CS } from '@compiled/react';\n\
             const _a = '._a{color:red}';\n\
             const _b = '._b{font-size:12px}';\n\
             const Component = () => React.createElement(CC, null, [_a, _b], React.createElement('div'));",
    );

    let output = crate::transform(
      program,
      TransformConfig {
        filename: Some("app.tsx".into()),
        options: PluginOptions {
          compiled_require_exclude: Some(true),
          ..Default::default()
        },
        ..Default::default()
      },
    )
    .expect("strip runtime transform should succeed");

    assert_eq!(
      output.metadata.style_rules,
      vec![
        "._a{color:red}".to_string(),
        "._b{font-size:12px}".to_string()
      ]
    );

    let printed = print(&output.program, &cm, &comments);
    assert!(!printed.contains("const _a"));
    assert!(!printed.contains("CC"));
    assert!(printed.contains("React.createElement('div')"));
  }

  #[test]
  fn strips_jsxs_runtime_calls() {
    let (program, cm, comments) = parse(
      "import { CC, CS } from '@compiled/react';\n\
             import { jsxs as _jsxs, jsx as _jsx } from 'react/jsx-runtime';\n\
             const _a = '._a{color:red}';\n\
             const Component = () => _jsxs(CC, { children: [_a, _jsx('div', { children: 'hello world' })] });",
    );

    let output = crate::transform(
      program,
      TransformConfig {
        filename: Some("app.tsx".into()),
        ..Default::default()
      },
    )
    .expect("strip runtime transform should succeed");

    assert!(output.metadata.style_rules.is_empty());

    let printed = print(&output.program, &cm, &comments);
    assert!(!printed.contains("const _a"));
    assert!(!printed.contains("_jsxs(CC"));
    assert!(!printed.contains("CC"));
    assert!(printed.contains("_jsx('div'"));
  }

  #[test]
  fn strips_jsxs_sequence_runtime_calls() {
    let (program, cm, comments) = parse(
      "import { CC, CS } from '@compiled/react';\n\
             import * as _jsxRuntime from 'react/jsx-runtime';\n\
             const _a = '._a{color:red}';\n\
             const Component = () => /*#__PURE__*/ (0, _jsxRuntime.jsxs)(CC, { children: [_a, (0, _jsxRuntime.jsx)('div', { children: 'hello world' })] });",
    );

    let output = crate::transform(
      program,
      TransformConfig {
        filename: Some("app.tsx".into()),
        ..Default::default()
      },
    )
    .expect("strip runtime transform should succeed");

    assert!(output.metadata.style_rules.is_empty());

    let printed = print(&output.program, &cm, &comments);
    assert!(!printed.contains("const _a"));
    assert!(!printed.contains("_jsxRuntime.jsxs(CC"));
    assert!(!printed.contains("CC"));
    assert!(printed.contains("_jsxRuntime.jsx)('div'"));
  }

  #[test]
  fn injects_require_calls_when_style_sheet_path_present() {
    let (program, cm, comments) = parse(
      "import { CC, CS } from '@compiled/react';\n\
             const _a = '._a{color:red}';\n\
             const _b = '._b{font-size:12px}';\n\
             const Component = () => React.createElement(CC, null, [_a, _b], React.createElement('div'));",
    );

    let output = crate::transform(
      program,
      TransformConfig {
        filename: Some("app.tsx".into()),
        options: PluginOptions {
          style_sheet_path: Some("@compiled/loader.css".into()),
          ..Default::default()
        },
        ..Default::default()
      },
    )
    .expect("strip runtime transform should succeed");

    assert!(output.metadata.style_rules.is_empty());

    let printed = print(&output.program, &cm, &comments);
    assert!(printed.contains("require(\"@compiled/loader.css?style=._a%7Bcolor%3Ared%7D\");"));
    assert!(printed.contains("require(\"@compiled/loader.css?style=._b%7Bfont-size%3A12px%7D\");"));
    assert!(!printed.contains("const _a"));
  }

  #[test]
  fn preserves_leading_comments_when_inserting_requires() {
    let (program, cm, comments) = parse(
      "// @license Example\n\
             import { CC, CS } from '@compiled/react';\n\
             const _a = '._a{color:red}';\n\
             const Component = () => React.createElement(CC, null, [_a], React.createElement('div'));",
    );

    let output = crate::transform(
      program,
      TransformConfig {
        filename: Some("app.tsx".into()),
        options: PluginOptions {
          style_sheet_path: Some("@compiled/loader.css".into()),
          ..Default::default()
        },
        ..Default::default()
      },
    )
    .expect("strip runtime transform should succeed");

    let printed = print(&output.program, &cm, &comments);
    let mut lines = printed.lines();
    assert_eq!(lines.next(), Some("// @license Example"));
    let require_line = lines.next().unwrap_or_default();
    assert!(require_line.starts_with("require(\"@compiled/loader.css?style="));
  }

  #[test]
  fn collects_metadata_when_compiled_require_exclude_enabled() {
    let (program, cm, comments) = parse(
      "import { CC, CS } from '@compiled/react';\n\
             const _a = '._a{color:red}';\n\
             const Component = () => React.createElement(CC, null, [_a], React.createElement('div'));",
    );

    let output = crate::transform(
      program,
      TransformConfig {
        filename: Some("app.tsx".into()),
        options: PluginOptions {
          style_sheet_path: Some("@compiled/loader.css".into()),
          compiled_require_exclude: Some(true),
          ..Default::default()
        },
        ..Default::default()
      },
    )
    .expect("strip runtime transform should succeed");

    assert_eq!(
      output.metadata.style_rules,
      vec!["._a{color:red}".to_string()]
    );

    let printed = print(&output.program, &cm, &comments);
    assert!(!printed.contains("require(\"@compiled/loader.css"));
    assert!(!printed.contains("const _a"));
  }

  #[test]
  fn inserts_requires_for_scripts_when_style_sheet_path_present() {
    let (program, cm, comments) = parse_script(
      "const { CC } = require('@compiled/react');\n\
             const _a = '._a{color:red}';\n\
             const Component = () => React.createElement(CC, null, [_a], React.createElement('div'));",
    );

    let output = crate::transform(
      program,
      TransformConfig {
        filename: Some("app.js".into()),
        options: PluginOptions {
          style_sheet_path: Some("@compiled/loader.css".into()),
          ..Default::default()
        },
        ..Default::default()
      },
    )
    .expect("strip runtime transform should succeed");

    let printed = print(&output.program, &cm, &comments);
    let mut lines = printed.lines();
    let require_line = lines.next().unwrap_or_default();
    assert!(require_line.starts_with("require(\"@compiled/loader.css?style="));
    assert!(!printed.contains("const _a"));
  }

  #[test]
  fn collects_metadata_sorted_when_multiple_rules() {
    let (program, _, _) = parse(
      "import { CC, CS } from '@compiled/react';\n\
             const _b = '._b{font-size:12px}';\n\
             const _a = '._a{color:red}';\n\
             const Component = () => React.createElement(CC, null, [_b, _a], React.createElement('div'));",
    );

    let output = crate::transform(
      program,
      TransformConfig {
        filename: Some("app.tsx".into()),
        options: PluginOptions {
          compiled_require_exclude: Some(true),
          ..Default::default()
        },
        ..Default::default()
      },
    )
    .expect("strip runtime transform should succeed");

    assert_eq!(
      output.metadata.style_rules,
      vec![
        "._a{color:red}".to_string(),
        "._b{font-size:12px}".to_string()
      ]
    );
  }

  #[test]
  fn returns_error_when_filename_missing() {
    let (program, _, _) = parse(
      "import { CC, CS } from '@compiled/react';\n\
             const _a = '._a{color:red}';\n\
             const Component = () => React.createElement(CC, null, [_a], React.createElement('div'));",
    );

    let result = crate::transform(
      program,
      TransformConfig {
        filename: None,
        options: PluginOptions {
          compiled_require_exclude: Some(true),
          ..Default::default()
        },
        ..Default::default()
      },
    );

    let errors = result.expect_err("missing filename should return an error");
    assert!(
      errors.iter().any(|err| err.message.contains(
        "@compiled/babel-plugin-strip-runtime expected the filename not to be empty, but actually got 'undefined'."
      )),
      "unexpected errors: {errors:?}"
    );
  }

  #[test]
  fn extracts_styles_to_disk_and_inserts_import() {
    let temp = tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    std::fs::create_dir_all(&src_dir).expect("create src");
    let filename = src_dir.join("app.tsx");

    // Ensure the file exists for nicer error messages mimicking Babel behaviour.
    let mut file = File::create(&filename).expect("create file");
    file.write_all(b"// stub").expect("write file");

    let source_file_name = src_dir.join("app.tsx");

    let (program, cm, comments) = parse(
      "import { CC, CS } from '@compiled/react';\n\
             const _a = '._a{color:red}';\n\
             const _b = '._b{font-size:12px}';\n\
             const Component = () => React.createElement(CC, null, [_a, _b], React.createElement('div'));",
    );

    let output = crate::transform(
      program,
      TransformConfig {
        filename: Some(filename.to_string_lossy().into()),
        source_file_name: Some(source_file_name.to_string_lossy().into()),
        cwd: Some(temp.path().to_string_lossy().into()),
        options: PluginOptions {
          extract_styles_to_directory: Some(ExtractStylesToDirectory {
            source: "src/".into(),
            dest: "dist/".into(),
          }),
          ..Default::default()
        },
        ..Default::default()
      },
    )
    .expect("strip runtime transform should succeed");

    let printed = print(&output.program, &cm, &comments);
    assert!(printed.starts_with("import \"./app.compiled.css\";"));

    let css_path = temp.path().join("dist").join("app.compiled.css");
    let css = read_to_string(css_path).expect("read css");
    assert_eq!(css, "._a{color:red}\n._b{font-size:12px}");
  }

  #[test]
  fn extracts_styles_to_disk_for_scripts() {
    let temp = tempdir().expect("tempdir");
    let src_dir = temp.path().join("src");
    std::fs::create_dir_all(&src_dir).expect("create src");
    let filename = src_dir.join("app.js");

    let mut file = File::create(&filename).expect("create file");
    file.write_all(b"// stub").expect("write file");

    let source_file_name = src_dir.join("app.js");

    let (program, cm, comments) = parse_script(
      "const { CC } = require('@compiled/react');\n\
             const _a = '._a{color:red}';\n\
             const _b = '._b{font-size:12px}';\n\
             const Component = () => React.createElement(CC, null, [_a, _b], React.createElement('div'));",
    );

    let output = crate::transform(
      program,
      TransformConfig {
        filename: Some(filename.to_string_lossy().into()),
        source_file_name: Some(source_file_name.to_string_lossy().into()),
        cwd: Some(temp.path().to_string_lossy().into()),
        options: PluginOptions {
          extract_styles_to_directory: Some(ExtractStylesToDirectory {
            source: "src/".into(),
            dest: "dist/".into(),
          }),
          ..Default::default()
        },
        ..Default::default()
      },
    )
    .expect("strip runtime transform should succeed");

    let printed = print(&output.program, &cm, &comments);
    let mut lines = printed.lines();
    let require_line = lines.next().unwrap_or_default();
    assert_eq!(require_line, "require(\"./app.compiled.css\");");

    let css_path = temp.path().join("dist").join("app.compiled.css");
    let css = read_to_string(css_path).expect("read css");
    assert_eq!(css, "._a{color:red}\n._b{font-size:12px}");
  }
}
