use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::Path;

use atlaspack_core::types::Condition;
use atlaspack_core::types::DependencyKind;
use path_slash::PathBufExt;
use serde::Deserialize;
use serde::Serialize;
use swc_core::common::DUMMY_SP;
use swc_core::common::Span;
use swc_core::common::Spanned;
use swc_core::common::sync::Lrc;
use swc_core::common::{Mark, SourceMap, SyntaxContext};
use swc_core::ecma::ast::*;
use swc_core::ecma::atoms::Atom;
use swc_core::ecma::atoms::atom;
use swc_core::ecma::utils::member_expr;
use swc_core::ecma::visit::VisitMut;
use swc_core::ecma::visit::VisitMutWith;

use crate::Config;
use crate::utils::*;

macro_rules! hash {
  ($str:expr) => {{
    let mut hasher = DefaultHasher::new();
    $str.hash(&mut hasher);
    hasher.finish()
  }};
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DependencyDescriptor {
  pub kind: DependencyKind,
  pub loc: SourceLocation,
  /// The text specifier associated with the import/export statement.
  pub specifier: swc_core::ecma::atoms::Atom,
  pub attributes: Option<HashMap<swc_core::ecma::atoms::Atom, bool>>,
  pub is_optional: bool,
  pub is_helper: bool,
  pub source_type: Option<SourceType>,
  pub placeholder: Option<String>,
}

/// This pass collects dependencies in a module and compiles references as needed to work with Atlaspack's JSRuntime.
pub fn dependency_collector<'a>(
  source_map: Lrc<SourceMap>,
  items: &'a mut Vec<DependencyDescriptor>,
  ignore_mark: swc_core::common::Mark,
  unresolved_mark: swc_core::common::Mark,
  config: &'a Config,
  diagnostics: &'a mut Vec<Diagnostic>,
  conditions: &'a mut BTreeSet<Condition>,
) -> impl VisitMut + 'a {
  DependencyCollector {
    source_map,
    items,
    in_try: false,
    in_promise: false,
    require_node: None,
    ignore_mark,
    unresolved_mark,
    config,
    diagnostics,
    import_meta: None,
    conditions,
  }
}

struct DependencyCollector<'a> {
  source_map: Lrc<SourceMap>,
  items: &'a mut Vec<DependencyDescriptor>,
  in_try: bool,
  in_promise: bool,
  require_node: Option<CallExpr>,
  ignore_mark: swc_core::common::Mark,
  unresolved_mark: swc_core::common::Mark,
  config: &'a Config,
  diagnostics: &'a mut Vec<Diagnostic>,
  import_meta: Option<VarDecl>,
  conditions: &'a mut BTreeSet<Condition>,
}

impl DependencyCollector<'_> {
  fn add_dependency(
    &mut self,
    mut specifier: Atom,
    span: swc_core::common::Span,
    kind: DependencyKind,
    attributes: Option<HashMap<swc_core::ecma::atoms::Atom, bool>>,
    is_optional: bool,
    source_type: SourceType,
  ) -> Option<Atom> {
    // Rewrite SWC helpers from ESM to CJS for library output.
    let mut is_specifier_rewritten = false;
    if self.config.is_library
      && !self.config.is_esm_output
      && let Some(rest) = specifier.strip_prefix("@swc/helpers/_/")
    {
      specifier = format!("@swc/helpers/cjs/{}.cjs", rest).into();
      is_specifier_rewritten = true;
    }

    // For ESM imports, the specifier will remain unchanged.
    // For other types of dependencies, the specifier will be changed to a hash
    // that also contains the dependency kind. This way, multiple kinds of dependencies
    // to the same specifier can be used within the same file.
    let placeholder = match kind {
      DependencyKind::Import | DependencyKind::Export => {
        if is_specifier_rewritten {
          Some(specifier.as_ref().to_owned())
        } else {
          None
        }
      }
      _ if !self.config.standalone => Some(format!(
        "{:x}",
        hash!(format!(
          "{}:{}:{}",
          self.get_project_relative_filename(),
          specifier,
          kind
        )),
      )),
      _ => None,
    };

    self.items.push(DependencyDescriptor {
      kind,
      loc: SourceLocation::from(&self.source_map, span),
      specifier,
      attributes,
      is_optional,
      is_helper: span.is_dummy(),
      source_type: Some(source_type),
      placeholder: placeholder.clone(),
    });

    placeholder.map(|p| p.into())
  }

  fn add_url_dependency(
    &mut self,
    specifier: Atom,
    span: swc_core::common::Span,
    kind: DependencyKind,
    source_type: SourceType,
  ) -> Expr {
    // If not a library, replace with a require call pointing to a runtime that will resolve the url dynamically.
    if !self.config.is_library && !self.config.standalone {
      let placeholder =
        self.add_dependency(specifier.clone(), span, kind, None, false, source_type);
      let specifier = if let Some(placeholder) = placeholder {
        placeholder
      } else {
        specifier
      };
      return Expr::Call(self.create_require(specifier));
    }

    // For library builds, we need to create something that can be statically analyzed by another bundler,
    // so rather than replacing with a require call that is resolved by a runtime, replace with a `new URL`
    // call with a placeholder for the relative path to be replaced during packaging.
    let placeholder = if self.config.standalone {
      specifier.as_ref().into()
    } else {
      format!(
        "{:x}",
        hash!(format!(
          "atlaspack_url:{}:{}:{}",
          self.config.filename, specifier, kind
        ))
      )
    };
    self.items.push(DependencyDescriptor {
      kind,
      loc: SourceLocation::from(&self.source_map, span),
      specifier,
      attributes: None,
      is_optional: false,
      is_helper: span.is_dummy(),
      source_type: Some(source_type),
      placeholder: Some(placeholder.clone()),
    });

    create_url_constructor(
      Expr::Lit(Lit::Str(placeholder.into())),
      self.config.is_esm_output,
    )
  }

  fn create_require(&mut self, specifier: Atom) -> CallExpr {
    let mut res = create_require(specifier, self.unresolved_mark);

    // For scripts, we replace with __parcel__require__, which is later replaced
    // by a real atlaspackRequire of the resolved asset in the packager.
    if self.config.source_type == SourceType::Script {
      res.callee = Callee::Expr(Box::new(Expr::Ident(Ident::new_no_ctxt(
        "__parcel__require__".into(),
        DUMMY_SP,
      ))));
    }
    res
  }

  fn add_script_error(&mut self, span: Span) {
    // Only add the diagnostic for imports/exports in scripts once.
    if self.diagnostics.iter().any(|d| d.message == "SCRIPT_ERROR") {
      return;
    }

    self.diagnostics.push(Diagnostic {
      message: "SCRIPT_ERROR".to_string(),
      code_highlights: Some(vec![CodeHighlight {
        message: None,
        loc: SourceLocation::from(&self.source_map, span),
      }]),
      hints: None,
      show_environment: true,
      severity: DiagnosticSeverity::Error,
      documentation_url: Some(String::from(
        "https://parceljs.org/languages/javascript/#classic-scripts",
      )),
    });
  }
}

impl VisitMut for DependencyCollector<'_> {
  fn visit_mut_module(&mut self, node: &mut Module) {
    node.visit_mut_children_with(self);
    if let Some(decl) = self.import_meta.take() {
      node
        .body
        .insert(0, ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(decl)))));
    }
  }

  fn visit_mut_module_decl(&mut self, node: &mut ModuleDecl) {
    // If an import or export is seen within a script, flag it to throw an error from JS.
    if self.config.source_type == SourceType::Script {
      match node {
        ModuleDecl::Import(ImportDecl { span, .. })
        | ModuleDecl::ExportAll(ExportAll { span, .. })
        | ModuleDecl::ExportDecl(ExportDecl { span, .. })
        | ModuleDecl::ExportDefaultDecl(ExportDefaultDecl { span, .. })
        | ModuleDecl::ExportDefaultExpr(ExportDefaultExpr { span, .. })
        | ModuleDecl::ExportNamed(NamedExport { span, .. }) => self.add_script_error(*span),
        _ => {}
      };

      return;
    }

    node.visit_mut_children_with(self);
  }

  fn visit_mut_import_decl(&mut self, node: &mut ImportDecl) {
    if node.type_only {
      return;
    }

    let rewritten_import_src = self.add_dependency(
      node.src.value.clone(),
      node.src.span,
      DependencyKind::Import,
      None,
      false,
      self.config.source_type,
    );

    if let Some(value) = rewritten_import_src {
      node.src.value = value;
    }
  }

  fn visit_mut_named_export(&mut self, node: &mut NamedExport) {
    if node.type_only {
      return;
    }

    if let Some(export_src) = &mut node.src {
      let rewritten_export_src = self.add_dependency(
        export_src.value.clone(),
        export_src.span,
        DependencyKind::Export,
        None,
        false,
        self.config.source_type,
      );

      if let Some(value) = rewritten_export_src {
        export_src.value = value;
      }
    }
  }

  fn visit_mut_export_all(&mut self, node: &mut ExportAll) {
    let rewritten_export_all_src = self.add_dependency(
      node.src.value.clone(),
      node.src.span,
      DependencyKind::Export,
      None,
      false,
      self.config.source_type,
    );

    if let Some(value) = rewritten_export_all_src {
      node.src.value = value;
    }
  }

  fn visit_mut_try_stmt(&mut self, node: &mut TryStmt) {
    // Track if we're inside a try block to mark dependencies as optional.
    self.in_try = true;
    node.block.visit_mut_with(self);
    self.in_try = false;

    if let Some(handler) = node.handler.as_mut() {
      handler.visit_mut_with(self);
    }

    if let Some(finalizer) = node.finalizer.as_mut() {
      finalizer.visit_mut_with(self);
    }
  }

  fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
    let kind = match &node.callee {
      Callee::Import(_) => DependencyKind::DynamicImport,
      Callee::Expr(expr) => {
        match &**expr {
          Expr::Ident(ident)
            if self.config.conditional_bundling
              && ident.sym.to_string().as_str() == "importCond" =>
          {
            DependencyKind::ConditionalImport
          }
          Expr::Ident(ident) => {
            // Bail if defined in scope
            if !is_unresolved(ident, self.unresolved_mark) {
              node.visit_mut_children_with(self);
              return;
            }

            match ident.sym.to_string().as_str() {
              "require" => {
                if self.in_promise {
                  DependencyKind::DynamicImport
                } else {
                  DependencyKind::Require
                }
              }
              "importScripts" => {
                if self.config.is_worker {
                  let (msg, span) = if self.config.source_type == SourceType::Script {
                    // Ignore if argument is not a string literal.
                    let span = if let Some(ExprOrSpread { expr, .. }) = node.args.first() {
                      match &**expr {
                        Expr::Lit(Lit::Str(Str { value, span, .. })) => {
                          // Ignore absolute URLs.
                          if value.starts_with("http:")
                            || value.starts_with("https:")
                            || value.starts_with("//")
                          {
                            node.visit_mut_children_with(self);
                            return;
                          }

                          span
                        }
                        _ => {
                          node.visit_mut_children_with(self);
                          return;
                        }
                      }
                    } else {
                      node.visit_mut_children_with(self);
                      return;
                    };

                    (
                      "Argument to importScripts() must be a fully qualified URL.",
                      *span,
                    )
                  } else {
                    (
                      "importScripts() is not supported in module workers.",
                      node.span,
                    )
                  };
                  self.diagnostics.push(Diagnostic {
                    message: msg.to_string(),
                    code_highlights: Some(vec![CodeHighlight {
                      message: None,
                      loc: SourceLocation::from(&self.source_map, span),
                    }]),
                    hints: Some(vec![String::from(
                      "Use a static `import`, or dynamic `import()` instead.",
                    )]),
                    show_environment: self.config.source_type == SourceType::Script,
                    severity: DiagnosticSeverity::Error,
                    documentation_url: Some(String::from(
                      "https://parceljs.org/languages/javascript/#classic-script-workers",
                    )),
                  });
                }

                node.visit_mut_children_with(self);
                return;
              }
              "__parcel__require__" => {
                node.visit_mut_children_with(self);

                node.callee = Callee::Expr(Box::new(Expr::Ident(Ident::new(
                  "require".into(),
                  DUMMY_SP,
                  SyntaxContext::empty().apply_mark(self.ignore_mark),
                ))));

                return;
              }
              "__parcel__import__" => {
                node.visit_mut_children_with(self);

                node.callee = Callee::Expr(Box::new(Expr::Ident(Ident::new(
                  "import".into(),
                  DUMMY_SP,
                  SyntaxContext::empty().apply_mark(self.ignore_mark),
                ))));

                return;
              }
              "__parcel__importScripts__" => {
                node.visit_mut_children_with(self);

                node.callee = Callee::Expr(Box::new(Expr::Ident(Ident::new(
                  "importScripts".into(),
                  DUMMY_SP,
                  SyntaxContext::empty().apply_mark(self.ignore_mark),
                ))));

                return;
              }
              "parcelRequire" => {
                if self.config.hmr_improvements {
                  if let Some(ExprOrSpread { expr, .. }) = node.args.first()
                    && let Some((id, span)) = match_str(expr)
                  {
                    self.items.push(DependencyDescriptor {
                      kind: DependencyKind::Id,
                      loc: SourceLocation::from(&self.source_map, span),
                      specifier: id,
                      attributes: None,
                      is_optional: false,
                      is_helper: false,
                      source_type: None,
                      placeholder: None,
                    });
                  }
                  node.visit_mut_children_with(self);

                  if !self.config.scope_hoist {
                    node.callee = Callee::Expr(Box::new(Expr::Member(member_expr!(
                      Default::default(),
                      node.span,
                      module.bundle.root
                    ))));
                  }

                  return;
                } else {
                  // Mimic the behaviour of the default case
                  node.visit_mut_children_with(self);
                  return;
                }
              }
              _ => {
                node.visit_mut_children_with(self);
                return;
              }
            }
          }
          Expr::Member(member) => {
            if match_member_expr(member, vec!["module", "require"], self.unresolved_mark) {
              DependencyKind::Require
            } else if self.config.is_browser
              && match_member_expr(
                member,
                vec!["navigator", "serviceWorker", "register"],
                self.unresolved_mark,
              )
            {
              DependencyKind::ServiceWorker
            } else if self.config.is_browser
              && match_member_expr(
                member,
                vec!["CSS", "paintWorklet", "addModule"],
                self.unresolved_mark,
              )
            {
              DependencyKind::Worklet
            } else {
              let was_in_promise = self.in_promise;

              // Match compiled dynamic imports (Atlaspack)
              // Promise.resolve(require('foo'))
              if match_member_expr(member, vec!["Promise", "resolve"], self.unresolved_mark)
                && let Some(expr) = node.args.first()
                && match_require(&expr.expr, self.unresolved_mark, Mark::fresh(Mark::root()))
                  .is_some()
              {
                self.in_promise = true;
                node.visit_mut_children_with(self);
                self.in_promise = was_in_promise;
                return;
              }

              // Match compiled dynamic imports (TypeScript)
              // Promise.resolve().then(() => require('foo'))
              // Promise.resolve().then(() => { return require('foo') })
              // Promise.resolve().then(function () { return require('foo') })
              //   but not
              // Promise.resolve(require('foo'))
              if let Expr::Call(call) = &*member.obj
                && let Callee::Expr(e) = &call.callee
                  && let Expr::Member(m) = &**e
                    && match_member_expr(m, vec!["Promise", "resolve"], self.unresolved_mark) &&
                      // Make sure the arglist is empty.
                      // I.e. do not proceed with the below unless Promise.resolve has an empty arglist
                      // because build_promise_chain() will not work in this case.
                      call.args.is_empty()
                      && let MemberProp::Ident(id) = &member.prop
                        && id.sym.to_string().as_str() == "then"
                          && let Some(arg) = node.args.first()
              {
                match &*arg.expr {
                  Expr::Fn(_) | Expr::Arrow(_) => {
                    if self.config.nested_promise_import_fix {
                      self.in_promise = true;
                      // Reset require_node to capture only requires within this
                      // promise.then
                      let old_require_node = self.require_node.take();
                      node.visit_mut_children_with(self);
                      self.in_promise = was_in_promise;
                      // Get the require node captured during visiting children
                      // and reset the require_node to its previous value
                      let require_node = self.require_node.take();
                      self.require_node = old_require_node;

                      // Transform Promise.resolve().then(() => __importStar(require('foo')))
                      //   => Promise.resolve().then(() => require('foo')).then(res => __importStar(res))
                      if let Some(require_node) = require_node {
                        build_promise_chain(node, require_node);
                        return;
                      }
                    } else {
                      self.in_promise = true;
                      node.visit_mut_children_with(self);
                      self.in_promise = was_in_promise;

                      // Transform Promise.resolve().then(() => __importStar(require('foo')))
                      //   => Promise.resolve().then(() => require('foo')).then(res => __importStar(res))
                      if let Some(require_node) = self.require_node.clone() {
                        self.require_node = None;
                        build_promise_chain(node, require_node);
                        return;
                      }
                    }
                  }
                  _ => {}
                }
              }

              node.visit_mut_children_with(self);
              return;
            }
          }
          _ => {
            node.visit_mut_children_with(self);
            return;
          }
        }
      }
      _ => {
        node.visit_mut_children_with(self);
        return;
      }
    };

    // Convert import attributes for dynamic import
    let mut attributes = None;
    if kind == DependencyKind::DynamicImport
      && let Some(arg) = node.args.get(1)
      && let Expr::Object(arg) = &*arg.expr
    {
      let mut attrs = HashMap::new();
      for key in &arg.props {
        let prop = match key {
          PropOrSpread::Prop(prop) => prop,
          _ => continue,
        };

        let kv = match &**prop {
          Prop::KeyValue(kv) => kv,
          _ => continue,
        };

        let k = match &kv.key {
          PropName::Ident(IdentName { sym, .. }) | PropName::Str(Str { value: sym, .. }) => {
            sym.clone()
          }
          _ => continue,
        };

        let v = match &*kv.value {
          Expr::Lit(Lit::Bool(Bool { value, .. })) => *value,
          _ => continue,
        };

        attrs.insert(k, v);
      }

      attributes = Some(attrs);
    }

    if let Some(arg) = node.args.first() {
      if kind == DependencyKind::ServiceWorker || kind == DependencyKind::Worklet {
        let (source_type, opts) = if kind == DependencyKind::ServiceWorker {
          match_worker_type(node.args.get(1))
        } else {
          // Worklets are always modules
          (SourceType::Module, None)
        };

        let (specifier, span) = if let Some(s) = self.match_new_url(&arg.expr) {
          s
        } else if let Expr::Lit(Lit::Str(str_)) = &*arg.expr {
          let (msg, docs) = if kind == DependencyKind::ServiceWorker {
            (
              "Registering service workers with a string literal is not supported.",
              "https://parceljs.org/languages/javascript/#service-workers",
            )
          } else {
            (
              "Registering worklets with a string literal is not supported.",
              "https://parceljs.org/languages/javascript/#worklets",
            )
          };

          self.diagnostics.push(Diagnostic {
            message: msg.to_string(),
            code_highlights: Some(vec![CodeHighlight {
              message: None,
              loc: SourceLocation::from(&self.source_map, str_.span),
            }]),
            hints: Some(vec![format!(
              "Replace with: new URL('{}', import.meta.url)",
              str_.value,
            )]),
            show_environment: false,
            severity: DiagnosticSeverity::Error,
            documentation_url: Some(String::from(docs)),
          });

          return;
        } else {
          return;
        };

        *node.args[0].expr = self.add_url_dependency(specifier, span, kind, source_type);

        match opts {
          Some(opts) => {
            node.args[1] = opts;
          }
          None => {
            node.args.truncate(1);
          }
        }

        return;
      } else if let Some((specifier, span)) = match_str(&arg.expr) {
        // require() calls aren't allowed in scripts, flag as an error.
        if kind == DependencyKind::Require && self.config.source_type == SourceType::Script {
          self.add_script_error(node.span);
          return;
        }

        // Special case dependency behaviour for conditional imports
        if !self.config.conditional_bundling || kind != DependencyKind::ConditionalImport {
          let placeholder = self.add_dependency(
            specifier,
            span,
            kind.clone(),
            attributes,
            kind == DependencyKind::Require && self.in_try,
            self.config.source_type,
          );

          if let Some(placeholder) = placeholder {
            *node.args[0].expr = Expr::Lit(Lit::Str(Str {
              raw: None,
              span,
              value: placeholder,
            }));
          }
        }
      }
    }

    // Replace import() with require()
    if kind == DependencyKind::DynamicImport {
      let call = node;
      if !self.config.scope_hoist && !self.config.standalone {
        let name = match &self.config.source_type {
          SourceType::Module => "require",
          SourceType::Script => "__parcel__require__",
        };

        call.callee = Callee::Expr(Box::new(Expr::Ident(Ident::new_no_ctxt(
          name.into(),
          DUMMY_SP,
        ))));
      }

      // Drop import attributes
      call.args.truncate(1);

      // Track the returned require call to be replaced with a promise chain.
      self.require_node = Some(call.clone());
    } else if kind == DependencyKind::Require {
      // Bail traversing so that `require` is not replaced with undefined
    } else if kind == DependencyKind::ConditionalImport {
      let call = node;

      if call.args.len() != 3 {
        self.diagnostics.push(Diagnostic {
          message: "importCond requires three arguments".to_string(),
          code_highlights: Some(vec![CodeHighlight {
            message: None,
            loc: SourceLocation::from(&self.source_map, call.span),
          }]),
          show_environment: false,
          severity: DiagnosticSeverity::Error,
          hints: None,
          documentation_url: None,
        });

        return;
      }

      if match_str(&call.args[1].expr).unwrap().0 == match_str(&call.args[2].expr).unwrap().0 {
        self.diagnostics.push(Diagnostic {
          message: "importCond requires unique dependencies".to_string(),
          code_highlights: Some(vec![CodeHighlight {
            message: None,
            loc: SourceLocation::from(&self.source_map, call.span),
          }]),
          show_environment: false,
          severity: DiagnosticSeverity::Error,
          hints: None,
          documentation_url: None,
        });

        return;
      }

      let mut placeholders = Vec::new();
      // For the if_true and if_false arms of the conditional import, create a dependency for each arm
      for arg in &call.args[1..] {
        let specifier = match_str(&arg.expr).unwrap().0;
        let placeholder = self.add_dependency(
          specifier.clone(),
          arg.span(),
          DependencyKind::ConditionalImport,
          None,
          false,
          self.config.source_type,
        );

        placeholders.push(placeholder.unwrap());
      }

      // Create a condition we pass back to JS
      let condition = Condition {
        key: match_str(&call.args[0].expr).unwrap().0.to_string(),
        if_true_placeholder: Some(placeholders[0].to_string()),
        if_false_placeholder: Some(placeholders[1].to_string()),
      };

      self.conditions.insert(condition);

      if self.config.scope_hoist {
        // write out code like importCond(depIfTrue, depIfFalse) - while we use the first dep as the link to the conditions
        // we need both deps to ensure scope hoisting can make sure both arms are treated as "used"
        call.args[0] = ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Lit(Lit::Str(Str {
            value: format!("{}", placeholders[0]).into(),
            span: DUMMY_SP,
            raw: None,
          }))),
        };

        call.args[1] = ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Lit(Lit::Str(Str {
            value: format!("{}", placeholders[1]).into(),
            span: DUMMY_SP,
            raw: None,
          }))),
        };

        call.args.truncate(2);
      } else {
        // If we're not scope hoisting, then change this `importCond` to a require so the deps are resolved correctly
        call.callee = Callee::Expr(Box::new(Expr::Ident(Ident::new_no_ctxt(
          "require".into(),
          DUMMY_SP,
        ))));

        // Flip these so require will have the ifFalse/default placeholder.
        // That placeholder is used by the runtime to transform into a conditional expression
        call.args[0] = ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Lit(Lit::Str(Str {
            value: placeholders[1].clone(),
            span: DUMMY_SP,
            raw: None,
          }))),
        };

        call.args[1] = ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Lit(Lit::Str(Str {
            value: placeholders[0].clone(),
            span: DUMMY_SP,
            raw: None,
          }))),
        };

        call.args.truncate(2);
      }
    } else {
      node.visit_mut_children_with(self);
    }
  }

  fn visit_mut_unary_expr(&mut self, node: &mut UnaryExpr) {
    // Do not traverse `typeof require` further to not replace `require` with undefined
    if let UnaryExpr {
      op: UnaryOp::TypeOf,
      arg,
      ..
    } = &node
      && let Expr::Ident(ident) = &**arg
      && ident.sym == atom!("require")
      && is_unresolved(ident, self.unresolved_mark)
    {
      return;
    }

    node.visit_mut_children_with(self);
  }

  fn visit_mut_new_expr(&mut self, node: &mut NewExpr) {
    let matched = match &*node.callee {
      Expr::Ident(id) => {
        if id.sym == "Worker" || id.sym == "SharedWorker" {
          // Bail if defined in scope
          self.config.is_browser && is_unresolved(id, self.unresolved_mark)
        } else if id.sym == "Promise" {
          // Match requires inside promises (e.g. Rollup compiled dynamic imports)
          // new Promise(resolve => resolve(require('foo')))
          // new Promise(resolve => { resolve(require('foo')) })
          // new Promise(function (resolve) { resolve(require('foo')) })
          self.visit_mut_new_promise(node);
          return;
        } else {
          if id.sym == "__parcel__URL__" {
            // new __parcel__URL__(url) -> new URL(url, import.meta.url)
            if let Some(args) = &node.args {
              let mut url = *args[0].expr.clone();
              url.visit_mut_with(self);

              if let Expr::New(new) = create_url_constructor(url, self.config.is_esm_output) {
                *node = new;
                return;
              }
            }

            unreachable!();
          }
          false
        }
      }
      _ => false,
    };

    if !matched {
      node.visit_mut_children_with(self);
      return;
    }

    if let Some(args) = &node.args
      && !args.is_empty()
    {
      let (specifier, span) = if let Some(s) = self.match_new_url(&args[0].expr) {
        s
      } else if let Expr::Lit(Lit::Str(str_)) = &*args[0].expr {
        let constructor = match &*node.callee {
          Expr::Ident(id) => id.sym.to_string(),
          _ => "Worker".to_string(),
        };

        self.diagnostics.push(Diagnostic {
          message: format!(
            "Constructing a {} with a string literal is not supported.",
            constructor
          ),
          code_highlights: Some(vec![CodeHighlight {
            message: None,
            loc: SourceLocation::from(&self.source_map, str_.span),
          }]),
          hints: Some(vec![format!(
            "Replace with: new URL('{}', import.meta.url)",
            str_.value
          )]),
          show_environment: false,
          severity: DiagnosticSeverity::Error,
          documentation_url: Some(String::from(
            "https://parceljs.org/languages/javascript/#web-workers",
          )),
        });

        return;
      } else {
        return;
      };

      let (source_type, opts) = match_worker_type(args.get(1));
      let placeholder =
        self.add_url_dependency(specifier, span, DependencyKind::WebWorker, source_type);

      // Replace argument with a require call to resolve the URL at runtime.
      if let Some(mut args) = node.args.clone() {
        *args[0].expr = placeholder;

        // If module workers aren't supported natively, remove the `type: 'module'` option.
        // If no other options are passed, remove the argument entirely.
        if !self.config.supports_module_workers {
          match opts {
            None => {
              args.truncate(1);
            }
            Some(opts) => {
              args[1] = opts;
            }
          }
        }

        node.args = Some(args);
      }

      return;
    }

    node.visit_mut_children_with(self);
  }

  fn visit_mut_member_expr(&mut self, node: &mut MemberExpr) {
    node.obj.visit_mut_with(self);

    if let MemberProp::Computed(_) = node.prop {
      node.prop.visit_mut_with(self);
    }
  }

  fn visit_mut_expr(&mut self, node: &mut Expr) {
    if self.is_import_meta(node) {
      *node = self.get_import_meta();
      return;
    }

    if self.is_import_meta_url(node) {
      *node = self.get_import_meta_url();
      return;
    }

    if let Some((specifier, span)) = self.match_new_url(node) {
      let url = self.add_url_dependency(
        specifier,
        span,
        DependencyKind::Url,
        self.config.source_type,
      );

      // If this is a library, we will already have a URL object. Otherwise, we need to
      // construct one from the string returned by the JSRuntime.
      if !self.config.is_library && !self.config.standalone {
        *node = Expr::New(NewExpr {
          span: DUMMY_SP,
          callee: Box::new(Expr::Ident(Ident::new_no_ctxt(atom!("URL"), DUMMY_SP))),
          ctxt: SyntaxContext::empty(),
          args: Some(vec![ExprOrSpread {
            expr: Box::new(url),
            spread: None,
          }]),
          type_args: None,
        });

        return;
      }

      *node = url;
      return;
    }

    let is_require = match &node {
      Expr::Ident(ident) => {
        // Free `require` -> undefined
        ident.sym == atom!("require") && is_unresolved(ident, self.unresolved_mark)
      }
      Expr::Member(MemberExpr { obj: expr, .. }) => {
        // e.g. `require.extensions` -> undefined
        if let Expr::Ident(ident) = &**expr {
          ident.sym == atom!("require") && is_unresolved(ident, self.unresolved_mark)
        } else {
          false
        }
      }
      _ => false,
    };

    if is_require {
      *node = Expr::Ident(get_undefined_ident(self.unresolved_mark));
      return;
    }

    node.visit_mut_children_with(self);
  }

  fn visit_mut_var_declarator(&mut self, node: &mut VarDeclarator) {
    if self.config.conditional_bundling
      && let Some(init) = node.init.clone()
      && let Expr::Call(call) = *init
      && let Callee::Expr(callee) = &call.callee
      && let Expr::Ident(ident) = &**callee
      && ident.sym.as_str() == "importCond"
    {
      // Drill down to default value in source, as the importCond API accesses this value directly
      node.init = Some(Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: call.into(),
        prop: MemberProp::Ident(IdentName::new("default".into(), DUMMY_SP)),
      })));
    }

    node.visit_mut_children_with(self);
  }
}

impl DependencyCollector<'_> {
  fn visit_mut_new_promise(&mut self, node: &mut NewExpr) {
    // Match requires inside promises (e.g. Rollup compiled dynamic imports)
    // new Promise(resolve => resolve(require('foo')))
    // new Promise(resolve => { resolve(require('foo')) })
    // new Promise(function (resolve) { resolve(require('foo')) })
    // new Promise(function (resolve) { return resolve(require('foo')) })
    if let Some(args) = &node.args
      && let Some(arg) = args.first()
    {
      let (resolve, expr) = match &*arg.expr {
        Expr::Fn(f) => {
          let param = f.function.params.first().map(|param| &param.pat);
          let body = if let Some(body) = &f.function.body {
            self.match_block_stmt_expr(body)
          } else {
            None
          };
          (param, body)
        }
        Expr::Arrow(f) => {
          let param = f.params.first();
          let body = match &*f.body {
            BlockStmtOrExpr::Expr(expr) => Some(&**expr),
            BlockStmtOrExpr::BlockStmt(block) => self.match_block_stmt_expr(block),
          };
          (param, body)
        }
        _ => (None, None),
      };

      let resolve_id = match resolve {
        Some(Pat::Ident(id)) => id.to_id(),
        _ => {
          node.visit_mut_children_with(self);
          return;
        }
      };

      if let Some(Expr::Call(call)) = expr
        && let Callee::Expr(callee) = &call.callee
        && let Expr::Ident(id) = &**callee
        && id.to_id() == resolve_id
        && let Some(arg) = call.args.first()
        && match_require(&arg.expr, self.unresolved_mark, Mark::fresh(Mark::root())).is_some()
      {
        let was_in_promise = self.in_promise;
        self.in_promise = true;
        node.visit_mut_children_with(self);
        self.in_promise = was_in_promise;
        return;
      }
    }

    node.visit_mut_children_with(self);
  }

  fn match_block_stmt_expr<'x>(&self, block: &'x BlockStmt) -> Option<&'x Expr> {
    match block.stmts.last() {
      Some(Stmt::Expr(ExprStmt { expr, .. })) => Some(&**expr),
      Some(Stmt::Return(ReturnStmt { arg, .. })) => {
        if let Some(arg) = arg {
          Some(&**arg)
        } else {
          None
        }
      }
      _ => None,
    }
  }
}

// If the `require` call is not immediately returned (e.g. wrapped in another function),
// then transform the AST to create a promise chain so that the require is by itself.
// This is because the require will return a promise rather than the module synchronously.
// For example, TypeScript generates the following with the esModuleInterop flag:
//   Promise.resolve().then(() => __importStar(require('./foo')));
// This is transformed into:
//   Promise.resolve().then(() => require('./foo')).then(res => __importStar(res));
fn build_promise_chain(node: &mut CallExpr, require_node: CallExpr) {
  let mut transformer = PromiseTransformer {
    require_node: Some(require_node),
  };

  node.visit_mut_with(&mut transformer);

  if let Some(require_node) = &transformer.require_node
    && let Some(f) = node.args.first()
  {
    // Add `res` as an argument to the original function
    let f = match &*f.expr {
      Expr::Fn(f) => {
        let mut f = f.clone();
        f.function.params.insert(
          0,
          Param {
            pat: Pat::Ident(BindingIdent::from(Ident::new_no_ctxt(
              "res".into(),
              DUMMY_SP,
            ))),
            decorators: vec![],
            span: DUMMY_SP,
          },
        );
        Expr::Fn(f)
      }
      Expr::Arrow(f) => {
        let mut f = f.clone();
        f.params.insert(
          0,
          Pat::Ident(BindingIdent::from(Ident::new_no_ctxt(
            "res".into(),
            DUMMY_SP,
          ))),
        );
        Expr::Arrow(f)
      }
      _ => return,
    };

    *node = CallExpr {
      callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: (Box::new(Expr::Call(CallExpr {
          callee: node.callee.clone(),
          args: vec![ExprOrSpread {
            expr: Box::new(Expr::Fn(FnExpr {
              ident: None,
              function: Box::new(Function {
                body: Some(BlockStmt {
                  span: DUMMY_SP,
                  stmts: vec![Stmt::Return(ReturnStmt {
                    span: DUMMY_SP,
                    arg: Some(Box::new(Expr::Call(require_node.clone()))),
                  })],
                  ctxt: SyntaxContext::empty(),
                }),
                params: vec![],
                decorators: vec![],
                is_async: false,
                is_generator: false,
                return_type: None,
                type_params: None,
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
              }),
            })),
            spread: None,
          }],
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          type_args: None,
        }))),
        prop: MemberProp::Ident(IdentName::new("then".into(), DUMMY_SP)),
      }))),
      args: vec![ExprOrSpread {
        expr: Box::new(f),
        spread: None,
      }],
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      type_args: None,
    };
  }
}

fn create_url_constructor(url: Expr, use_import_meta: bool) -> Expr {
  let expr = if use_import_meta {
    Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::MetaProp(MetaPropExpr {
        kind: MetaPropKind::ImportMeta,
        span: DUMMY_SP,
      })),
      prop: MemberProp::Ident(IdentName::new(atom!("url"), DUMMY_SP)),
    })
  } else {
    // CJS output: "file:" + __filename
    Expr::Bin(BinExpr {
      span: DUMMY_SP,
      left: Box::new(Expr::Lit(Lit::Str("file:".into()))),
      op: BinaryOp::Add,
      right: Box::new(Expr::Ident(Ident::new_no_ctxt(
        "__filename".into(),
        DUMMY_SP,
      ))),
    })
  };

  Expr::New(NewExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Box::new(Expr::Ident(Ident::new_no_ctxt(atom!("URL"), DUMMY_SP))),
    args: Some(vec![
      ExprOrSpread {
        expr: Box::new(url),
        spread: None,
      },
      ExprOrSpread {
        expr: Box::new(expr),
        spread: None,
      },
    ]),
    type_args: None,
  })
}

struct PromiseTransformer {
  require_node: Option<CallExpr>,
}

impl VisitMut for PromiseTransformer {
  fn visit_mut_return_stmt(&mut self, node: &mut ReturnStmt) {
    // If the require node is returned, no need to do any replacement.
    if let Some(arg) = &node.arg
      && let Expr::Call(call) = &**arg
      && let Some(require_node) = &self.require_node
      && require_node == call
    {
      self.require_node = None
    }

    node.visit_mut_children_with(self);
  }

  fn visit_mut_arrow_expr(&mut self, node: &mut ArrowExpr) {
    if let BlockStmtOrExpr::Expr(expr) = &*node.body
      && let Expr::Call(call) = &**expr
      && let Some(require_node) = &self.require_node
      && require_node == call
    {
      self.require_node = None
    }

    node.visit_mut_children_with(self);
  }

  fn visit_mut_expr(&mut self, node: &mut Expr) {
    node.visit_mut_children_with(self);

    // Replace the original require node with a reference to a variable `res`, which will be added
    // as a parameter to the parent function.
    if let Expr::Call(call) = &node
      && let Some(require_node) = &self.require_node
      && require_node == call
    {
      *node = Expr::Ident(Ident::new_no_ctxt("res".into(), DUMMY_SP));
    }
  }
}

impl DependencyCollector<'_> {
  fn match_new_url(&mut self, expr: &Expr) -> Option<(Atom, swc_core::common::Span)> {
    if let Expr::New(new) = expr {
      let is_url = match &*new.callee {
        Expr::Ident(id) => id.sym == atom!("URL") && is_unresolved(id, self.unresolved_mark),
        _ => false,
      };

      if !is_url {
        return None;
      }

      if let Some(args) = &new.args {
        let (specifier, span) = if let Some(arg) = args.first() {
          match_str(&arg.expr)?
        } else {
          return None;
        };

        if let Some(arg) = args.get(1)
          && self.is_import_meta_url(&arg.expr)
        {
          return Some((specifier, span));
        }
      }
    }

    // self reference, e.g. new Worker(import.meta.url)
    if self.is_import_meta_url(expr) {
      let filename = Path::new(&self.config.filename).file_name().unwrap();
      let specifier = format!("./{}", filename.to_string_lossy());
      let span = match expr {
        Expr::Member(member) => member.span,
        _ => unreachable!(),
      };
      return Some((specifier.into(), span));
    }

    None
  }

  #[allow(clippy::wrong_self_convention)]
  fn is_import_meta_url(&mut self, expr: &Expr) -> bool {
    match expr {
      Expr::Member(member) => {
        if !self.is_import_meta(&member.obj) {
          return false;
        }

        let name = match_property_name(member);

        if let Some((name, _)) = name {
          name == atom!("url")
        } else {
          false
        }
      }
      Expr::Bin(BinExpr {
        op: BinaryOp::Add,
        left,
        right,
        ..
      }) => {
        // Match "file:" + __filename
        let left = match_str(left);
        match (left, &**right) {
          (Some((left, _)), Expr::Ident(Ident { sym: right, .. })) => {
            &left == "file:" && right == "__filename"
          }
          _ => false,
        }
      }
      _ => false,
    }
  }

  #[allow(clippy::wrong_self_convention)]
  fn is_import_meta(&mut self, expr: &Expr) -> bool {
    match &expr {
      Expr::MetaProp(MetaPropExpr {
        kind: MetaPropKind::ImportMeta,
        span,
      }) => {
        if self.config.source_type == SourceType::Script {
          self.diagnostics.push(Diagnostic {
            message: "`import.meta` is not supported outside a module.".to_string(),
            code_highlights: Some(vec![CodeHighlight {
              message: None,
              loc: SourceLocation::from(&self.source_map, *span),
            }]),
            hints: None,
            show_environment: true,
            severity: DiagnosticSeverity::Error,
            documentation_url: Some(String::from(
              "https://parceljs.org/languages/javascript/#classic-scripts",
            )),
          })
        }
        true
      }
      _ => false,
    }
  }

  fn get_project_relative_filename(&self) -> String {
    if let Some(relative) = pathdiff::diff_paths(&self.config.filename, &self.config.project_root) {
      relative.to_slash_lossy()
    } else if let Some(filename) = Path::new(&self.config.filename).file_name() {
      String::from(filename.to_string_lossy())
    } else {
      String::from("unknown.js")
    }
  }

  fn get_import_meta_url(&mut self) -> Expr {
    Expr::Lit(Lit::Str(
      format!("file:///{}", self.get_project_relative_filename()).into(),
    ))
  }

  fn get_import_meta(&mut self) -> Expr {
    if let Some(decl) = &self.import_meta {
      if let Pat::Ident(name) = &decl.decls[0].name {
        Expr::Ident(name.id.clone())
      } else {
        unreachable!()
      }
    } else {
      // Declares a variable at the top of the module:
      // var import_meta = Object.assign(Object.create(null), {url: 'file:///src/foo.js'});
      let ident = Ident::new_private(
        format!("${}$import_meta", self.config.module_id).into(),
        DUMMY_SP,
      );
      self.import_meta = Some(VarDecl {
        kind: VarDeclKind::Var,
        declare: false,
        span: DUMMY_SP,
        decls: vec![VarDeclarator {
          name: Pat::Ident(BindingIdent::from(ident.clone())),
          init: Some(Box::new(Expr::Call(CallExpr {
            callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
              obj: Box::new(Expr::Ident(Ident::new_no_ctxt(atom!("Object"), DUMMY_SP))),
              prop: MemberProp::Ident(IdentName::new("assign".into(), DUMMY_SP)),
              span: DUMMY_SP,
            }))),
            args: vec![
              ExprOrSpread {
                expr: Box::new(Expr::Call(CallExpr {
                  callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                    obj: (Box::new(Expr::Ident(Ident::new_no_ctxt(atom!("Object"), DUMMY_SP)))),
                    prop: MemberProp::Ident(IdentName::new("create".into(), DUMMY_SP)),
                    span: DUMMY_SP,
                  }))),
                  args: vec![ExprOrSpread {
                    expr: Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))),
                    spread: None,
                  }],
                  span: DUMMY_SP,
                  ctxt: SyntaxContext::empty(),
                  type_args: None,
                })),
                spread: None,
              },
              ExprOrSpread {
                expr: Box::new(Expr::Object(ObjectLit {
                  props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Ident(IdentName::new(atom!("url"), DUMMY_SP)),
                    value: Box::new(self.get_import_meta_url()),
                  })))],
                  span: DUMMY_SP,
                })),
                spread: None,
              },
            ],
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            type_args: None,
          }))),
          span: DUMMY_SP,
          definite: false,
        }],
        ctxt: SyntaxContext::empty(),
      });
      Expr::Ident(ident)
    }
  }
}

// matches the `type: 'module'` option of workers
fn match_worker_type(expr: Option<&ExprOrSpread>) -> (SourceType, Option<ExprOrSpread>) {
  if let Some(expr_or_spread) = expr
    && let Expr::Object(obj) = &*expr_or_spread.expr
  {
    let mut source_type: Option<SourceType> = None;
    let props: Vec<PropOrSpread> = obj
      .props
      .iter()
      .filter(|key| {
        let prop = match key {
          PropOrSpread::Prop(prop) => prop,
          _ => return true,
        };

        let kv = match &**prop {
          Prop::KeyValue(kv) => kv,
          _ => return true,
        };

        match &kv.key {
          PropName::Ident(IdentName { sym, .. }) if sym == "type" => {}
          PropName::Str(Str { value, .. }) if value == "type" => {}
          _ => return true,
        };

        let v = if let Some((v, _)) = match_str(&kv.value) {
          v
        } else {
          return true;
        };

        source_type = Some(if v == "module" {
          SourceType::Module
        } else {
          SourceType::Script
        });

        false
      })
      .cloned()
      .collect();

    if let Some(source_type) = source_type {
      let e = if props.is_empty() {
        None
      } else {
        Some(ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Object(ObjectLit {
            props,
            span: obj.span,
          })),
        })
      };

      return (source_type, e);
    }
  }

  (SourceType::Script, expr.cloned())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::DependencyDescriptor;
  use atlaspack_core::types::DependencyKind;
  use atlaspack_swc_runner::test_utils::{RunContext, RunVisitResult, run_test_visit};
  use indoc::{formatdoc, indoc};
  use pretty_assertions::assert_eq;

  fn make_dependency_collector<'a>(
    context: RunContext,
    items: &'a mut Vec<DependencyDescriptor>,
    diagnostics: &'a mut Vec<Diagnostic>,
    config: &'a Config,
    conditions: &'a mut BTreeSet<Condition>,
  ) -> DependencyCollector<'a> {
    DependencyCollector {
      source_map: context.source_map.clone(),
      items,
      in_try: false,
      in_promise: false,
      require_node: None,
      ignore_mark: Mark::new(),
      unresolved_mark: context.unresolved_mark,
      config,
      diagnostics,
      import_meta: None,
      conditions,
    }
  }

  fn make_config() -> Config {
    let mut config = Config::default();
    config.is_browser = true;
    config.nested_promise_import_fix = true;
    config
  }

  fn make_placeholder_hash(specifier: &str, dependency_kind: DependencyKind) -> String {
    format!(
      "{:x}",
      hash!(format!("{}:{}:{}", "", specifier, dependency_kind))
    )
  }

  #[test]
  fn test_dynamic_import_dependency() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      const { x } = await import('other');
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash = make_placeholder_hash("other", DependencyKind::DynamicImport);
    let expected_code = formatdoc! {r#"
      const {{ x }} = await require("{hash}");
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::DynamicImport,
        specifier: "other".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: Some(hash),
        ..items[0].clone()
      }]
    );
  }

  #[test]
  fn test_dynamic_import_nested_promise() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let mut config = make_config();
    config.scope_hoist = true;

    let input_code = indoc! {r#"
      const dynamic = () => import('./dynamic');

      Promise.resolve().then(() => {
          Promise.resolve().then(() => console.log());
      });
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash = make_placeholder_hash("./dynamic", DependencyKind::DynamicImport);
    let expected_code = formatdoc! {r#"
      const dynamic = ()=>import("{hash}");
      Promise.resolve().then(()=>{{
          Promise.resolve().then(()=>console.log());
      }});
    "#};

    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::DynamicImport,
        specifier: "./dynamic".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: Some(hash),
        ..items[0].clone()
      }]
    );
    assert_eq!(output_code, expected_code);
  }

  #[test]
  fn test_dynamic_import_dependency_from_script() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let mut config = make_config();
    config.source_type = SourceType::Script;

    let input_code = r#"
      const { x } = await import('other');
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash = make_placeholder_hash("other", DependencyKind::DynamicImport);
    let expected_code = formatdoc! {r#"
      const {{ x }} = await __parcel__require__("{hash}");
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::DynamicImport,
        specifier: "other".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Script),
        placeholder: Some(hash),
        ..items[0].clone()
      }]
    );
  }

  #[test]
  fn test_import_dependency() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      import { x } from 'other';
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let expected_code = indoc! {r#"
      import { x } from 'other';
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::Import,
        specifier: "other".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: None,
        ..items[0].clone()
      }]
    );
  }

  #[test]
  fn test_export_dependency() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      export { x } from 'other';
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let expected_code = indoc! {r#"
      export { x } from 'other';
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::Export,
        specifier: "other".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: None,
        ..items[0].clone()
      }]
    );
  }

  #[test]
  fn test_export_star_dependency() {
    let mut conditions = BTreeSet::new();
    let mut items = vec![];
    let mut diagnostics = vec![];

    let config = make_config();
    let input_code = r#"
      export * from 'other';
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let expected_code = indoc! {r#"
      export * from 'other';
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::Export,
        specifier: "other".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: None,
        ..items[0].clone()
      }]
    );
  }

  #[test]
  fn test_require_dependency() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      const { x } = require('other');
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash = make_placeholder_hash("other", DependencyKind::Require);
    let expected_code = formatdoc! {r#"
      const {{ x }} = require("{hash}");
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::Require,
        specifier: "other".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: Some(hash),
        ..items[0].clone()
      }]
    );
  }

  #[test]
  fn test_optional_require_dependency() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      try {
        const { x } = require('other');
      } catch (err) {}
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash = make_placeholder_hash("other", DependencyKind::Require);
    let expected_code = formatdoc! {r#"
      try {{
          const {{ x }} = require("{hash}");
      }} catch (err) {{}}
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::Require,
        specifier: "other".into(),
        attributes: None,
        is_optional: true,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: Some(hash),
        ..items[0].clone()
      }]
    );
  }

  #[test]
  fn test_node_dependency() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      import { join } from 'node:path';

      async function main() {
        const { readFile } = require('node:fs/promises');
        const { Readable } = await import('node:stream');
      }
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let fs_hash = make_placeholder_hash("node:fs/promises", DependencyKind::Require);
    let stream_hash = make_placeholder_hash("node:stream", DependencyKind::DynamicImport);
    let expected_code = formatdoc! {r#"
      import {{ join }} from 'node:path';
      async function main() {{
          const {{ readFile }} = require("{fs_hash}");
          const {{ Readable }} = await require("{stream_hash}");
      }}
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [
        DependencyDescriptor {
          kind: DependencyKind::Import,
          specifier: "node:path".into(),
          attributes: None,
          is_optional: false,
          is_helper: false,
          source_type: Some(SourceType::Module),
          placeholder: None,
          ..items[0].clone()
        },
        DependencyDescriptor {
          kind: DependencyKind::Require,
          specifier: "node:fs/promises".into(),
          attributes: None,
          is_optional: false,
          is_helper: false,
          source_type: Some(SourceType::Module),
          placeholder: Some(fs_hash.clone()),
          ..items[1].clone()
        },
        DependencyDescriptor {
          kind: DependencyKind::DynamicImport,
          specifier: "node:stream".into(),
          attributes: None,
          is_optional: false,
          is_helper: false,
          source_type: Some(SourceType::Module),
          placeholder: Some(stream_hash.clone()),
          ..items[2].clone()
        }
      ]
    );
  }

  // Require is treated as dynamic import
  #[test]
  fn test_compiled_dynamic_imports() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = indoc! {r#"
      Promise.resolve().then(() => require('other'));
    "#};

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash = make_placeholder_hash("other", DependencyKind::DynamicImport);
    let expected_code = formatdoc! {r#"
      Promise.resolve().then(()=>require("{hash}"));
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::DynamicImport,
        specifier: "other".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: Some(hash),
        ..items[0].clone()
      }]
    );
  }

  // Require is treated as dynamic import
  #[test]
  fn test_compiled_dynamic_imports_with_chain() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      Promise.resolve().then(() => doSomething(require('other')));
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash = make_placeholder_hash("other", DependencyKind::DynamicImport);
    let expected_code = formatdoc! {r#"
      Promise.resolve().then(function() {{
          return require("{hash}");
      }}).then((res)=>doSomething(res));
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::DynamicImport,
        specifier: "other".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: Some(hash),
        ..items[0].clone()
      }]
    );
  }

  // Require is treated as dynamic import
  #[test]
  fn test_compiled_dynamic_imports_with_function_chain() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      Promise.resolve().then(function() { return doSomething(require('other')); });
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash = make_placeholder_hash("other", DependencyKind::DynamicImport);
    let expected_code = formatdoc! {r#"
      Promise.resolve().then(function() {{
          return require("{hash}");
      }}).then(function(res) {{
          return doSomething(res);
      }});
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::DynamicImport,
        specifier: "other".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: Some(hash),
        ..items[0].clone()
      }]
    );
  }

  // Require is treated as dynamic import
  #[test]
  fn test_new_promise_require_imports() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      new Promise((resolve) => resolve(require("other")));
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash = make_placeholder_hash("other", DependencyKind::DynamicImport);
    let expected_code = formatdoc! {r#"
      new Promise((resolve)=>resolve(require("{hash}")));
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::DynamicImport,
        specifier: "other".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: Some(hash),
        ..items[0].clone()
      }]
    );
  }

  // Require is treated as dynamic import
  #[test]
  fn test_new_promise_require_imports_with_function_expr() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      new Promise(function(resolve) { return resolve(require("other")) });
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash = make_placeholder_hash("other", DependencyKind::DynamicImport);
    let expected_code = formatdoc! {r#"
      new Promise(function(resolve) {{
          return resolve(require("{hash}"));
      }});
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::DynamicImport,
        specifier: "other".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: Some(hash),
        ..items[0].clone()
      }]
    );
  }

  // Require is treated as dynamic import
  #[test]
  fn test_promise_resolve_require_dynamic_import() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      Promise.resolve(require("other"));
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash = make_placeholder_hash("other", DependencyKind::DynamicImport);
    let expected_code = formatdoc! {r#"
        Promise.resolve(require("{hash}"));
    "#};
    let expected_code = expected_code.trim_start().trim_end_matches(' ');

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::DynamicImport,
        specifier: "other".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: Some(hash),
        ..items[0].clone()
      }]
    );
  }

  #[test]
  fn test_parcel_url_dependency() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      new __parcel__URL__("./other.js");
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let expected_code = formatdoc! {r#"
       new URL("./other.js", "file:" + __filename);
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(items.len(), 0);
  }

  #[test]
  fn test_esm_parcel_url_dependency() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let mut config = make_config();
    config.is_esm_output = true;

    let input_code = r#"
      new __parcel__URL__("./other.js");
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let expected_code = formatdoc! {r#"
       new URL("./other.js", import.meta.url);
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(items.len(), 0);
  }

  #[test]
  fn test_worker_dependency() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      new Worker(new URL('foo', import.meta.url));
      new Worker(new URL('bar', import.meta.url), {type: 'module'});
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let foo_hash = make_placeholder_hash("foo", DependencyKind::WebWorker);
    let bar_hash = make_placeholder_hash("bar", DependencyKind::WebWorker);
    let expected_code = formatdoc! {r#"
      new Worker(require("{foo_hash}"));
      new Worker(require("{bar_hash}"));
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [
        DependencyDescriptor {
          kind: DependencyKind::WebWorker,
          specifier: "foo".into(),
          attributes: None,
          is_optional: false,
          is_helper: false,
          source_type: Some(SourceType::Script),
          placeholder: Some(foo_hash),
          ..items[0].clone()
        },
        DependencyDescriptor {
          kind: DependencyKind::WebWorker,
          specifier: "bar".into(),
          attributes: None,
          is_optional: false,
          is_helper: false,
          source_type: Some(SourceType::Module),
          placeholder: Some(bar_hash),
          ..items[1].clone()
        }
      ]
    );
  }

  #[test]
  fn test_service_worker_dependency() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();
    let input_code = r#"
      navigator.serviceWorker.register(new URL('foo', import.meta.url));
      navigator.serviceWorker.register(new URL('bar', import.meta.url), {type: 'module'});
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let foo_hash = make_placeholder_hash("foo", DependencyKind::ServiceWorker);
    let bar_hash = make_placeholder_hash("bar", DependencyKind::ServiceWorker);
    let expected_code = formatdoc! {r#"
      navigator.serviceWorker.register(require("{foo_hash}"));
      navigator.serviceWorker.register(require("{bar_hash}"));
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [
        DependencyDescriptor {
          kind: DependencyKind::ServiceWorker,
          specifier: "foo".into(),
          attributes: None,
          is_optional: false,
          is_helper: false,
          source_type: Some(SourceType::Script),
          placeholder: Some(foo_hash),
          ..items[0].clone()
        },
        DependencyDescriptor {
          kind: DependencyKind::ServiceWorker,
          specifier: "bar".into(),
          attributes: None,
          is_optional: false,
          is_helper: false,
          source_type: Some(SourceType::Module),
          placeholder: Some(bar_hash),
          ..items[1].clone()
        }
      ]
    );
  }

  #[test]
  fn test_worklet_dependency() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];
    let config = make_config();
    let input_code = r#"
      CSS.paintWorklet.addModule(new URL('other', import.meta.url));
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash = make_placeholder_hash("other", DependencyKind::Worklet);
    let expected_code = formatdoc! {r#"
        CSS.paintWorklet.addModule(require("{hash}"));
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::Worklet,
        specifier: "other".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: Some(hash),
        ..items[0].clone()
      }]
    );
  }

  #[test]
  fn test_url_dependency() {
    let mut items = vec![];
    let mut diagnostics = vec![];
    let config = make_config();
    let input_code = r#"
      let img = document.createElement('img');
      img.src = new URL('hero.jpg', import.meta.url);
      document.body.appendChild(img);
    "#;

    let mut conditions = BTreeSet::new();

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash = make_placeholder_hash("hero.jpg", DependencyKind::Url);
    let expected_code = formatdoc! {r#"
      let img = document.createElement('img');
      img.src = new URL(require("{hash}"));
      document.body.appendChild(img);
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::Url,
        specifier: "hero.jpg".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: Some(SourceType::Module),
        placeholder: Some(hash),
        ..items[0].clone()
      }]
    );
  }

  #[test]
  fn test_import_cond_dependency() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let mut config = make_config();
    config.conditional_bundling = true;

    let input_code = r#"
      const x = importCond('condition', 'a', 'b');
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash_a = make_placeholder_hash("a", DependencyKind::ConditionalImport);
    let hash_b = make_placeholder_hash("b", DependencyKind::ConditionalImport);
    let expected_code = formatdoc! {r#"
      const x = require("{hash_b}", "{hash_a}").default;
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [
        DependencyDescriptor {
          kind: DependencyKind::ConditionalImport,
          specifier: "a".into(),
          attributes: None,
          is_optional: false,
          is_helper: false,
          source_type: Some(SourceType::Module),
          placeholder: Some(hash_a.clone()),
          ..items[0].clone()
        },
        DependencyDescriptor {
          kind: DependencyKind::ConditionalImport,
          specifier: "b".into(),
          attributes: None,
          is_optional: false,
          is_helper: false,
          source_type: Some(SourceType::Module),
          placeholder: Some(hash_b.clone()),
          ..items[1].clone()
        }
      ]
    );
    assert_eq!(
      conditions,
      BTreeSet::from([Condition {
        key: "condition".into(),
        if_true_placeholder: Some(hash_a),
        if_false_placeholder: Some(hash_b)
      }])
    );
  }

  #[test]
  fn test_import_cond_scope_hoisting_enabled_dependency() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let mut config = make_config();
    config.scope_hoist = true;
    config.conditional_bundling = true;

    let input_code = r#"
      const x = importCond('condition', 'a', 'b');
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let hash_a = make_placeholder_hash("a", DependencyKind::ConditionalImport);
    let hash_b = make_placeholder_hash("b", DependencyKind::ConditionalImport);
    let expected_code = formatdoc! {r#"
      const x = importCond("{hash_a}", "{hash_b}").default;
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [
        DependencyDescriptor {
          kind: DependencyKind::ConditionalImport,
          specifier: "a".into(),
          attributes: None,
          is_optional: false,
          is_helper: false,
          source_type: Some(SourceType::Module),
          placeholder: Some(hash_a),
          ..items[0].clone()
        },
        DependencyDescriptor {
          kind: DependencyKind::ConditionalImport,
          specifier: "b".into(),
          attributes: None,
          is_optional: false,
          is_helper: false,
          source_type: Some(SourceType::Module),
          placeholder: Some(hash_b),
          ..items[1].clone()
        }
      ]
    );
  }

  #[test]
  fn test_import_cond_invalid() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let mut config = make_config();
    config.scope_hoist = true;
    config.conditional_bundling = true;

    let input_code = r#"
      const x = importCond('condition', 'a');
    "#;

    run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
      diagnostics[0].message,
      "importCond requires three arguments"
    );
  }

  #[test]
  fn test_import_cond_same_deps() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let mut config = make_config();
    config.scope_hoist = true;
    config.conditional_bundling = true;

    let input_code = r#"
      const x = importCond('condition', 'a', 'a');
    "#;

    run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
      diagnostics[0].message,
      "importCond requires unique dependencies"
    );
  }

  #[test]
  fn test_parcel_require() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let config = make_config();

    let input_code = r#"
      const x = parcelRequire('foo');
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let expected_code = formatdoc! {r#"
      const x = parcelRequire('foo');
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(items, []);
    assert_eq!(conditions, BTreeSet::new(),);
  }

  #[test]
  fn test_parcel_require_with_hmr_improvements_ff_on() {
    let mut conditions = BTreeSet::new();
    let mut diagnostics = vec![];
    let mut items = vec![];

    let mut config = make_config();
    config.hmr_improvements = true;

    let input_code = r#"
      const x = parcelRequire('foo');
    "#;

    let RunVisitResult { output_code, .. } = run_test_visit(input_code, |context| {
      make_dependency_collector(
        context,
        &mut items,
        &mut diagnostics,
        &config,
        &mut conditions,
      )
    });

    let expected_code = formatdoc! {r#"
      const x = module.bundle.root('foo');
    "#};

    assert_eq!(output_code, expected_code);
    assert_eq!(diagnostics, []);
    assert_eq!(
      items,
      [DependencyDescriptor {
        kind: DependencyKind::Id,
        specifier: "foo".into(),
        attributes: None,
        is_optional: false,
        is_helper: false,
        source_type: None,
        placeholder: None,
        ..items[0].clone()
      },]
    );
    assert_eq!(conditions, BTreeSet::new(),);
  }
}
