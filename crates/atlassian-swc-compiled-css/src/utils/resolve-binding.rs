use std::any::Any;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use indexmap::IndexSet;
use oxc_resolver::{ResolveOptions, Resolver};
use serde_json::Value;
use swc_core::common::Spanned;
use swc_core::common::comments::{Comment, SingleThreadedComments};
use swc_core::common::sync::Lrc;
use swc_core::common::{FileName, SourceMap};
use swc_core::ecma::ast::{
  EsVersion, ExportSpecifier, Expr, ModuleDecl, ModuleExportName, ModuleItem, Program, Prop,
  PropName, PropOrSpread,
};
use swc_core::ecma::parser::lexer::Lexer;
use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax, TsSyntax};

use crate::constants::{DEFAULT_CODE_EXTENSIONS, DEFAULT_PARSER_BABEL_PLUGINS};
use crate::types::{
  CachedModule, Metadata, PluginOptions, TransformFile, TransformFileOptions, TransformState,
};
use crate::utils_create_result_pair::{ResultPair, create_result_pair};
use crate::utils_module_scope;
use crate::utils_traversers::{
  get_default_export, get_named_export, set_imported_compiled_imports,
};
use crate::utils_traversers_types::TraverserResult;
use crate::utils_types::{
  BindingPathKind, EvaluateExpression, ImportBindingKind, PartialBindingWithMeta,
};

fn expression_references_import(
  expr: &Expr,
  meta: &Metadata,
  visited: &mut IndexSet<String>,
  evaluate_expression: EvaluateExpression,
) -> bool {
  match expr {
    Expr::Ident(ident) => {
      if !visited.insert(ident.sym.to_string()) {
        return false;
      }

      if let Some(binding) = resolve_binding(ident.sym.as_ref(), meta.clone(), evaluate_expression)
      {
        if matches!(binding.source, crate::utils_types::BindingSource::Import) {
          return true;
        }

        if let Some(node) = &binding.node {
          return expression_references_import(node, &binding.meta, visited, evaluate_expression);
        }
      }

      false
    }
    Expr::Member(member) => {
      let obj_refs = expression_references_import(&member.obj, meta, visited, evaluate_expression);

      let prop_refs = match &member.prop {
        swc_core::ecma::ast::MemberProp::Ident(ident) => expression_references_import(
          &Expr::Ident(ident.clone().into()),
          meta,
          visited,
          evaluate_expression,
        ),
        swc_core::ecma::ast::MemberProp::Computed(comp) => {
          expression_references_import(&comp.expr, meta, visited, evaluate_expression)
        }
        swc_core::ecma::ast::MemberProp::PrivateName(_) => false,
      };

      obj_refs || prop_refs
    }
    Expr::Call(call) => {
      let callee_refs = match &call.callee {
        swc_core::ecma::ast::Callee::Expr(expr) => {
          expression_references_import(expr, meta, visited, evaluate_expression)
        }
        _ => false,
      };

      callee_refs
        || call
          .args
          .iter()
          .any(|arg| expression_references_import(&arg.expr, meta, visited, evaluate_expression))
    }
    Expr::Array(array) => array
      .elems
      .iter()
      .flatten()
      .any(|elem| expression_references_import(&elem.expr, meta, visited, evaluate_expression)),
    Expr::Object(object) => object.props.iter().any(|prop| match prop {
      PropOrSpread::Prop(prop) => match prop.as_ref() {
        Prop::KeyValue(kv) => {
          expression_references_import(&kv.value, meta, visited, evaluate_expression)
        }
        Prop::Assign(assign) => {
          expression_references_import(&assign.value, meta, visited, evaluate_expression)
        }
        Prop::Method(_) | Prop::Getter(_) | Prop::Setter(_) => false,
        _ => false,
      },
      PropOrSpread::Spread(spread) => {
        expression_references_import(&spread.expr, meta, visited, evaluate_expression)
      }
    }),
    Expr::Tpl(tpl) => tpl
      .exprs
      .iter()
      .any(|expr| expression_references_import(expr, meta, visited, evaluate_expression)),
    Expr::TaggedTpl(tagged) => {
      expression_references_import(&tagged.tag, meta, visited, evaluate_expression)
        || expression_references_import(
          &Expr::Tpl(*tagged.tpl.clone()),
          meta,
          visited,
          evaluate_expression,
        )
    }
    Expr::Bin(bin) => {
      expression_references_import(&bin.left, meta, visited, evaluate_expression)
        || expression_references_import(&bin.right, meta, visited, evaluate_expression)
    }
    Expr::Cond(cond) => {
      expression_references_import(&cond.test, meta, visited, evaluate_expression)
        || expression_references_import(&cond.cons, meta, visited, evaluate_expression)
        || expression_references_import(&cond.alt, meta, visited, evaluate_expression)
    }
    Expr::Paren(paren) => {
      expression_references_import(&paren.expr, meta, visited, evaluate_expression)
    }
    Expr::Unary(unary) => {
      expression_references_import(&unary.arg, meta, visited, evaluate_expression)
    }
    Expr::Assign(assign) => {
      expression_references_import(&assign.right, meta, visited, evaluate_expression)
    }
    Expr::Seq(seq) => seq
      .exprs
      .iter()
      .any(|expr| expression_references_import(expr, meta, visited, evaluate_expression)),
    Expr::New(new_expr) => {
      expression_references_import(&new_expr.callee, meta, visited, evaluate_expression)
        || new_expr
          .args
          .as_ref()
          .map(|args| {
            args.iter().any(|arg| {
              expression_references_import(&arg.expr, meta, visited, evaluate_expression)
            })
          })
          .unwrap_or(false)
    }
    _ => false,
  }
}

fn parse_current_module(meta: &Metadata) -> Option<Program> {
  let state = meta.state();
  let Some(path) = state.filename.clone() else {
    return None;
  };
  let options = state.opts.clone();
  drop(state);

  let code = fs::read_to_string(&path).ok()?;
  let (program, _cm, _comments) = parse_program(&path, &code, &options)?;
  Some(program)
}

fn find_local_name_for_named_export(meta: &Metadata, export_name: &str) -> Option<String> {
  let program = parse_current_module(meta)?;
  let Program::Module(module) = program else {
    return None;
  };

  for item in module.body.iter() {
    if let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(named)) = item {
      if named.src.is_none() {
        for spec in &named.specifiers {
          if let ExportSpecifier::Named(n) = spec {
            let exported_matches = n
              .exported
              .as_ref()
              .map(|e| match e {
                ModuleExportName::Ident(i) => i.sym.as_ref() == export_name,
                ModuleExportName::Str(s) => s.value.as_ref() == export_name,
              })
              .unwrap_or_else(|| match &n.orig {
                ModuleExportName::Ident(i) => i.sym.as_ref() == export_name,
                ModuleExportName::Str(s) => s.value.as_ref() == export_name,
              });

            if exported_matches {
              return Some(match &n.orig {
                ModuleExportName::Ident(i) => i.sym.to_string(),
                ModuleExportName::Str(s) => s.value.to_string(),
              });
            }
          }
        }
      }
    }
  }

  None
}

fn ensure_module_resolver(state: &mut TransformState) {
  if state.module_resolver.is_some() {
    return;
  }

  let mut options = ResolveOptions::default();
  let extensions = state.opts.extensions.clone().unwrap_or_else(|| {
    DEFAULT_CODE_EXTENSIONS
      .iter()
      .map(|ext| ext.to_string())
      .collect()
  });
  options.extensions = extensions;

  state.module_resolver = Some(Resolver::new(options));
}

fn resolve_request(state: &mut TransformState, filename: &str, request: &str) -> Option<String> {
  let resolver = state.module_resolver.as_ref()?;
  let base = Path::new(filename)
    .parent()
    .map(Path::to_path_buf)
    .unwrap_or_else(PathBuf::new);

  resolver
    .resolve(&base, request)
    .ok()
    .map(|resolution| resolution.into_path_buf().to_string_lossy().into_owned())
}

#[derive(Clone, Debug)]
struct ParserPluginConfig {
  use_typescript: bool,
  tsx: bool,
  tsx_explicit: bool,
  dts: bool,
  no_early_errors: bool,
  disallow_ambiguous_jsx_like: bool,
  decorators: bool,
  decorators_before_export: bool,
  jsx: bool,
  fn_bind: bool,
  export_default_from: bool,
  import_attributes: bool,
  allow_super_outside_method: bool,
  allow_return_outside_function: bool,
  auto_accessors: bool,
  explicit_resource_management: bool,
}

impl ParserPluginConfig {
  fn new(path: &str) -> Self {
    let lower_path = path.to_ascii_lowercase();
    let ts_like = lower_path.ends_with(".ts")
      || lower_path.ends_with(".tsx")
      || lower_path.ends_with(".mts")
      || lower_path.ends_with(".cts");
    let dts_like = lower_path.ends_with(".d.ts")
      || lower_path.ends_with(".d.tsx")
      || lower_path.ends_with(".d.mts")
      || lower_path.ends_with(".d.cts");
    let ends_with_x = lower_path.ends_with('x');

    Self {
      use_typescript: ts_like,
      tsx: ends_with_x,
      tsx_explicit: false,
      dts: dts_like,
      no_early_errors: false,
      disallow_ambiguous_jsx_like: false,
      decorators: false,
      decorators_before_export: false,
      jsx: false,
      fn_bind: false,
      export_default_from: false,
      import_attributes: false,
      allow_super_outside_method: false,
      allow_return_outside_function: false,
      auto_accessors: false,
      explicit_resource_management: false,
    }
  }

  fn apply_plugin(&mut self, name: &str, raw_options: Option<&Value>) {
    match name {
      "typescript" => {
        self.use_typescript = true;

        if let Some(Value::Object(options)) = raw_options {
          if let Some(value) = options.get("isTSX").and_then(Value::as_bool) {
            self.tsx = value;
            self.tsx_explicit = true;
          }

          if let Some(value) = options
            .get("disallowAmbiguousJSXLike")
            .and_then(Value::as_bool)
          {
            self.disallow_ambiguous_jsx_like = value;
          }

          if let Some(value) = options.get("dts").and_then(Value::as_bool) {
            self.dts = value;
          }

          if let Some(value) = options.get("noEarlyErrors").and_then(Value::as_bool) {
            self.no_early_errors = value;
          }
        }

        if !self.tsx_explicit && self.jsx {
          self.tsx = true;
        }
      }
      "jsx" => {
        self.jsx = true;

        if self.use_typescript && !self.tsx_explicit {
          self.tsx = true;
        }
      }
      "decorators" => {
        self.decorators = true;

        if let Some(Value::Object(options)) = raw_options {
          if let Some(value) = options
            .get("decoratorsBeforeExport")
            .and_then(Value::as_bool)
          {
            self.decorators_before_export = value;
          }
        }
      }
      "decorators-legacy" => {
        self.decorators = true;
        self.decorators_before_export = false;
      }
      "functionBind" => {
        self.fn_bind = true;
      }
      "exportDefaultFrom" => {
        self.export_default_from = true;
      }
      "importAssertions" | "importAttributes" => {
        self.import_attributes = true;
      }
      "allowSuperOutsideMethod" => {
        self.allow_super_outside_method = true;
      }
      "allowReturnOutsideFunction" => {
        self.allow_return_outside_function = true;
      }
      "autoAccessors" => {
        self.auto_accessors = true;
      }
      "explicitResourceManagement" => {
        self.explicit_resource_management = true;
      }
      _ => {}
    }
  }

  fn from_options(path: &str, options: &PluginOptions) -> Self {
    let mut config = Self::new(path);

    if let Some(plugins) = &options.parser_babel_plugins {
      for plugin in plugins {
        match plugin {
          Value::String(name) => config.apply_plugin(name, None),
          Value::Array(items) => {
            if let Some(Value::String(name)) = items.get(0) {
              config.apply_plugin(name, items.get(1));
            }
          }
          _ => {}
        }
      }
    } else {
      for name in DEFAULT_PARSER_BABEL_PLUGINS {
        config.apply_plugin(name, None);
      }
    }

    config
  }
}

fn parse_program(
  path: &str,
  code: &str,
  options: &PluginOptions,
) -> Option<(Program, Lrc<SourceMap>, Vec<Comment>)> {
  let source_map: Lrc<SourceMap> = Default::default();
  let file_name: FileName = FileName::Real(PathBuf::from(path).into());
  let file = source_map.new_source_file(file_name.into(), code.to_string());
  let comments = SingleThreadedComments::default();

  let parser_config = ParserPluginConfig::from_options(path, options);
  let syntax = if parser_config.use_typescript {
    Syntax::Typescript(TsSyntax {
      tsx: parser_config.tsx,
      decorators: parser_config.decorators,
      dts: parser_config.dts,
      no_early_errors: parser_config.no_early_errors,
      disallow_ambiguous_jsx_like: parser_config.disallow_ambiguous_jsx_like,
    })
  } else {
    Syntax::Es(EsSyntax {
      jsx: parser_config.jsx,
      decorators: parser_config.decorators,
      decorators_before_export: parser_config.decorators_before_export,
      fn_bind: parser_config.fn_bind,
      export_default_from: parser_config.export_default_from,
      import_attributes: parser_config.import_attributes,
      allow_super_outside_method: parser_config.allow_super_outside_method,
      allow_return_outside_function: parser_config.allow_return_outside_function,
      auto_accessors: parser_config.auto_accessors,
      explicit_resource_management: parser_config.explicit_resource_management,
      ..Default::default()
    })
  };

  let lexer = Lexer::new(
    syntax,
    EsVersion::Es2022,
    StringInput::from(&*file),
    Some(&comments),
  );

  let mut parser = Parser::new_from(lexer);
  let program = parser.parse_program().ok()?;

  if !parser.take_errors().is_empty() {
    return None;
  }

  let mut collected: Vec<Comment> = Vec::new();
  let (leading, trailing) = comments.take_all();

  {
    let mut leading = leading.borrow_mut();
    for (_, mut list) in leading.drain() {
      collected.append(&mut list);
    }
  }

  {
    let mut trailing = trailing.borrow_mut();
    for (_, mut list) in trailing.drain() {
      collected.append(&mut list);
    }
  }

  Some((program, source_map, collected))
}

pub(crate) fn load_or_parse_module(meta: &Metadata, source: &str) -> Option<CachedModule> {
  let (module_path, code, options, resolver_clone, cwd, root, shared_module_cache) = {
    let mut state = meta.state_mut();
    let filename = state.filename.clone()?;

    ensure_module_resolver(&mut state);
    let resolved = resolve_request(&mut state, &filename, source)?;

    if !state
      .opts
      .extensions
      .clone()
      .unwrap_or_else(|| {
        DEFAULT_CODE_EXTENSIONS
          .iter()
          .map(|ext| ext.to_string())
          .collect()
      })
      .iter()
      .any(|ext| resolved.ends_with(ext))
    {
      return None;
    }

    if !state.included_files.contains(&resolved) {
      state.included_files.push(resolved.clone());
    }

    if let Some(cached) = state.module_cache.borrow().get(&resolved) {
      return Some(cached.clone());
    }

    let resolver_clone = state
      .module_resolver
      .as_ref()
      .map(|resolver| resolver.clone_with_options(resolver.options().clone()));
    let options = state.opts.clone();
    let cwd = state.cwd.clone();
    let root = state.root.clone();
    let code_value = {
      let mut cache = state
        .cache
        .lock()
        .expect("cache lock should not be poisoned");
      cache.load(Some("read-file"), &resolved, || {
        Value::String(fs::read_to_string(&resolved).expect("module should read"))
      })
    };
    let code = code_value
      .as_str()
      .map(|value| value.to_string())
      .unwrap_or_else(|| fs::read_to_string(&resolved).expect("module should read"));

    let shared_module_cache = state.module_cache.clone();

    (
      resolved,
      code,
      options,
      resolver_clone,
      cwd,
      root,
      shared_module_cache,
    )
  };

  let (program, source_map, comments) = parse_program(&module_path, &code, &options)?;

  let transform_file = TransformFile::transform_compiled_with_options(
    source_map.clone(),
    comments,
    TransformFileOptions {
      filename: Some(module_path.clone()),
      cwd: Some(cwd.clone()),
      root: Some(root.clone()),
      loc_filename: None,
    },
  );

  let shared_state = Rc::new(RefCell::new(TransformState::new(transform_file, options)));

  {
    let mut module_state = shared_state.borrow_mut();
    module_state.module_cache = shared_module_cache.clone();
  }

  {
    let mut module_state = shared_state.borrow_mut();
    module_state.module_resolver = resolver_clone;
  }

  if let Program::Module(module) = &program {
    utils_module_scope::populate_module_scope(&shared_state, module);
  }

  {
    let mut module_state = shared_state.borrow_mut();
    set_imported_compiled_imports(&program, &mut module_state);
  }

  let cached = CachedModule {
    program: program.clone(),
    state: shared_state.clone(),
  };

  {
    let state = meta.state_mut();
    state
      .module_cache
      .borrow_mut()
      .insert(module_path, cached.clone());
  }

  Some(cached)
}

fn get_scoped_binding(reference_name: &str, meta: &Metadata) -> Option<PartialBindingWithMeta> {
  if let Some(scope) = meta.own_scope() {
    if let Some(binding) = scope.borrow().get(reference_name) {
      if std::env::var("STACK_DEBUG_BINDING").is_ok() {
        eprintln!(
          "[resolve_binding] own scope hit ref='{}' node_present={}",
          reference_name,
          binding.node.is_some()
        );
      }
      return Some(binding.clone());
    }
  }

  meta.parent_scope().borrow().get(reference_name).cloned()
}

fn describe_expr(expr: &Expr) -> &'static str {
  use swc_core::ecma::ast::Expr::*;
  match expr {
    Ident(_) => "Ident",
    Member(_) => "Member",
    Object(_) => "Object",
    Array(_) => "Array",
    Lit(_) => "Lit",
    Tpl(_) => "Tpl",
    Call(_) => "Call",
    Fn(_) | Arrow(_) => "Function",
    _ => "Other",
  }
}

fn extract_property_value(
  expr: &Expr,
  meta: Metadata,
  segment: &str,
  visited: &mut IndexSet<String>,
  evaluate_expression: EvaluateExpression,
) -> Option<ResultPair> {
  match expr {
    Expr::Object(object) => object.props.iter().find_map(|prop| match prop {
      PropOrSpread::Prop(prop) => match prop.as_ref() {
        Prop::KeyValue(kv) => {
          let key_matches = match &kv.key {
            PropName::Ident(ident) => ident.sym.as_ref() == segment,
            PropName::Str(value) => value.value.as_ref() == segment,
            PropName::Num(value) => value.value.to_string() == segment,
            PropName::BigInt(value) => value.value.to_string() == segment,
            PropName::Computed(_) => false,
          };

          if key_matches {
            Some(create_result_pair(*kv.value.clone(), meta.clone()))
          } else {
            None
          }
        }
        Prop::Assign(assign) => {
          if assign.key.sym.as_ref() == segment {
            Some(create_result_pair(*assign.value.clone(), meta.clone()))
          } else {
            None
          }
        }
        _ => None,
      },
      _ => None,
    }),
    Expr::Ident(ident) => {
      if !visited.insert(ident.sym.to_string()) {
        return None;
      }

      let binding = resolve_binding(ident.sym.as_ref(), meta.clone(), evaluate_expression)?;

      if let Some(node) = &binding.node {
        extract_property_value(
          node,
          binding.meta.clone(),
          segment,
          visited,
          evaluate_expression,
        )
      } else {
        Some(create_result_pair(expr.clone(), binding.meta.clone()))
      }
    }
    Expr::Paren(paren) => {
      extract_property_value(&paren.expr, meta, segment, visited, evaluate_expression)
    }
    Expr::Member(_member) => {
      let pair = evaluate_expression(expr, meta.clone());
      if pair.value == *expr {
        None
      } else {
        extract_property_value(
          &pair.value,
          pair.meta,
          segment,
          visited,
          evaluate_expression,
        )
      }
    }
    _ => {
      let pair = evaluate_expression(expr, meta.clone());
      if pair.value == *expr {
        None
      } else {
        extract_property_value(
          &pair.value,
          pair.meta,
          segment,
          visited,
          evaluate_expression,
        )
      }
    }
  }
}

fn resolve_variable_binding(
  binding: PartialBindingWithMeta,
  path: &[String],
  default: &Option<Expr>,
  evaluate_expression: EvaluateExpression,
) -> Option<PartialBindingWithMeta> {
  let Some(base) = binding.node.as_ref() else {
    return Some(binding);
  };

  let mut pair = evaluate_expression(base, binding.meta.clone());
  if std::env::var("STACK_DEBUG_BINDING").is_ok() || std::env::var("STACK_DEBUG_SHARED").is_ok() {
    eprintln!(
      "[resolve_binding] variable base expr_type={} path={:?}",
      describe_expr(&pair.value),
      path
    );
  }
  let mut visited = IndexSet::new();

  for (index, segment) in path.iter().enumerate() {
    match extract_property_value(
      &pair.value,
      pair.meta.clone(),
      segment,
      &mut visited,
      evaluate_expression,
    ) {
      Some(next_pair) => {
        pair = next_pair;
        if std::env::var("STACK_DEBUG_BINDING").is_ok() {
          eprintln!(
            "[resolve_binding] segment='{}' -> expr_type={} span={:?}",
            segment,
            describe_expr(&pair.value),
            pair.value.span()
          );
        }
      }
      None => {
        if index + 1 == path.len() {
          if let Some(default_expr) = default {
            pair = evaluate_expression(default_expr, binding.meta.clone());
            break;
          }
        }

        return None;
      }
    }
  }

  let mut resolved = binding.clone();
  resolved.node = Some(pair.value);
  resolved.meta = pair.meta;
  // COMPAT: Babel avoids inlining string/template literals when they are derived from imported
  // values (e.g., padding shorthands built from imported constants). Preserve that behaviour by
  // only skipping inlining when the expression references an import.
  let mut resolved = resolved;
  if matches!(
    resolved.node.as_ref(),
    Some(
      swc_core::ecma::ast::Expr::Lit(swc_core::ecma::ast::Lit::Str(_))
        | swc_core::ecma::ast::Expr::Tpl(_)
    )
  ) {
    let mut visited = IndexSet::new();
    if expression_references_import(base, &binding.meta, &mut visited, evaluate_expression) {
      // Mirror Babel: treat import-derived string/template bindings as dynamic so they are not
      // eagerly inlined, but still preserve the node for consumers (e.g., styled tagged
      // templates) that need the original template literal to build CSS.
      resolved.constant = false;
    }
  }

  Some(resolved)
}

fn resolve_import_binding(
  binding: PartialBindingWithMeta,
  source: &str,
  kind: &ImportBindingKind,
  meta: Metadata,
) -> Option<PartialBindingWithMeta> {
  // COMPAT: Babel does not eagerly resolve Compiled entrypoints (including the Atlaskit
  // wrapper) when tracking bindings. Attempting to parse these modules can pull in large
  // dependency graphs and surface parsing errors for types-only exports that the runtime
  // transform never evaluates. If the import source is already recognised as a Compiled
  // entrypoint, short-circuit to the placeholder binding.
  if crate::constants::DEFAULT_IMPORT_SOURCES
    .iter()
    .any(|s| s == &source)
  {
    return Some(binding);
  }

  let cached = load_or_parse_module(&meta, source)?;

  let resolved = match kind {
    ImportBindingKind::Namespace => Some(binding),
    ImportBindingKind::Default => {
      let result = get_default_export(&cached.program)?;
      Some(build_import_binding(binding, result, cached.state))
    }
    ImportBindingKind::Named(name) => {
      let result = get_named_export(&cached.program, name)?;
      Some(build_import_binding(binding, result, cached.state))
    }
  }?;

  Some(resolved)
}

fn build_import_binding(
  mut binding: PartialBindingWithMeta,
  traversed: TraverserResult<Expr>,
  state: Rc<RefCell<TransformState>>,
) -> PartialBindingWithMeta {
  let metadata = Metadata::new(state).with_parent_span(Some(traversed.span));
  binding.node = Some(traversed.node);
  binding.meta = metadata;
  binding
}

pub fn resolve_binding(
  reference_name: &str,
  meta: Metadata,
  evaluate_expression: EvaluateExpression,
) -> Option<PartialBindingWithMeta> {
  let shared_filter = std::env::var("STACK_DEBUG_SHARED").ok();
  let debug_shared = match shared_filter.as_deref() {
    Some("*") => true,
    Some(name) if name == reference_name => true,
    _ => false,
  };
  let debug_bind = std::env::var("STACK_DEBUG_BINDING").is_ok() || debug_shared;
  if debug_bind {
    eprintln!(
      "[resolve_binding] start ref='{}' file='{:?}'",
      reference_name,
      meta.state().file().filename
    );
  }
  let binding = get_scoped_binding(reference_name, &meta)?;
  let is_import_binding = matches!(
    binding.path.as_ref().map(|path| &path.kind),
    Some(BindingPathKind::Import { .. })
  );
  if std::env::var("STACK_DEBUG_BINDING").is_ok() && reference_name == "styles" {
    eprintln!(
      "[resolve_binding] scoped binding for 'styles' kind={:?} has_node={} file={:?}",
      binding.path.as_ref().map(|p| &p.kind),
      binding.node.is_some(),
      meta.state().file().filename
    );
  }
  if debug_shared {
    eprintln!(
      "[resolve_binding] ref='{}' path={:?} node_present={} constant={}",
      reference_name,
      binding.path.as_ref().map(|p| &p.kind),
      binding.node.is_some(),
      binding.constant
    );
  }

  // COMPAT: Babel resolves local export aliases by mapping the exported name
  // back to its local before proceeding. If our scoped binding is a
  // placeholder (no node captured), try to resolve a local name from a
  // `export { local as reference_name }` declaration in this module and then
  // resolve that local instead.
  if binding.node.is_none() && !is_import_binding {
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!(
        "[resolve_binding] placeholder for '{}', checking local export alias",
        reference_name
      );
    }
    if let Some(local_name) = find_local_name_for_named_export(&meta, reference_name) {
      if std::env::var("COMPILED_CLI_TRACE").is_ok() {
        eprintln!(
          "[resolve_binding] alias '{}' -> local '{}'",
          reference_name, local_name
        );
      }
      if let Some(resolved) = resolve_binding(&local_name, meta.clone(), evaluate_expression) {
        if debug_bind {
          eprintln!(
            "[resolve_binding] alias ref='{}' -> local='{}' node={:?}",
            reference_name,
            local_name,
            resolved.node.as_ref().map(|n| format!("{:?}", n.type_id()))
          );
        }
        return Some(resolved);
      }
    }
  }
  let path_kind = binding.path.clone().map(|path| path.kind);

  match path_kind {
    Some(BindingPathKind::Variable { path, default }) => {
      resolve_variable_binding(binding, &path, &default, evaluate_expression)
    }
    Some(BindingPathKind::Import { source, kind }) => {
      let imported = resolve_import_binding(binding, &source, &kind, meta);
      if debug_bind || debug_shared {
        eprintln!(
          "[resolve_binding] import ref='{}' source='{}' kind={:?} -> {:?}",
          reference_name,
          source,
          kind,
          imported
            .as_ref()
            .and_then(|b| b.node.as_ref())
            .map(|n| format!("{:?}", n.type_id()))
        );
      }
      imported
    }
    _ => Some(binding),
  }
}

#[cfg(test)]
mod tests {
  use super::resolve_binding;
  use crate::types::{
    Metadata, PluginOptions, TransformFile, TransformFileOptions, TransformState,
  };
  use crate::utils_create_result_pair::create_result_pair;
  use crate::utils_evaluate_expression;
  use crate::utils_types::{
    BindingPath, BindingSource, EvaluateExpression, ImportBindingKind, PartialBindingWithMeta,
  };
  use serde_json::Value;
  use std::cell::RefCell;
  use std::fs;
  use std::rc::Rc;
  use swc_core::common::sync::Lrc;
  use swc_core::common::{DUMMY_SP, FileName, SourceMap};
  use swc_core::ecma::ast::{Expr, Lit, Str};
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};
  use tempfile::tempdir;

  fn create_metadata() -> Metadata {
    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::new(cm, Vec::new());
    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));

    Metadata::new(state)
  }

  fn string_literal(value: &str) -> Expr {
    Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: value.into(),
      raw: None,
    }))
  }

  fn identity_evaluate(expr: &Expr, meta: Metadata) -> crate::utils_create_result_pair::ResultPair {
    create_result_pair(expr.clone(), meta)
  }

  fn parse_expression(code: &str) -> Expr {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("expr.tsx".into()).into(), code.into());
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
    *parser.parse_expr().expect("parse expression")
  }

  #[test]
  fn resolves_binding_from_parent_scope() {
    let meta = create_metadata();
    let binding_expr = string_literal("blue");
    let binding = PartialBindingWithMeta::new(
      Some(binding_expr.clone()),
      Some(BindingPath::new(Some(DUMMY_SP))),
      true,
      meta.clone(),
      BindingSource::Module,
    );

    meta.insert_parent_binding("color", binding.clone());

    let result = resolve_binding(
      "color",
      meta.clone(),
      identity_evaluate as EvaluateExpression,
    )
    .expect("binding");

    assert!(result.constant);
    assert_eq!(result.node, Some(binding_expr));
    assert_eq!(result.source, BindingSource::Module);
  }

  #[test]
  fn prefers_binding_from_own_scope() {
    let meta = create_metadata();
    let parent_binding = PartialBindingWithMeta::new(
      Some(string_literal("parent")),
      None,
      true,
      meta.clone(),
      BindingSource::Module,
    );
    meta.insert_parent_binding("value", parent_binding);

    let own_scope = meta.allocate_own_scope();
    let scoped_meta = meta.with_own_scope(Some(own_scope.clone()));
    let own_binding = PartialBindingWithMeta::new(
      Some(string_literal("own")),
      None,
      true,
      scoped_meta.clone(),
      BindingSource::Module,
    );
    own_scope
      .borrow_mut()
      .insert("value".into(), own_binding.clone());

    let result = resolve_binding(
      "value",
      scoped_meta,
      identity_evaluate as EvaluateExpression,
    )
    .expect("binding");

    assert_eq!(result.node, Some(string_literal("own")));
  }

  #[test]
  fn resolves_destructured_binding() {
    let meta = create_metadata();
    let object_expr = parse_expression("({ color: 'red' })");
    let binding = PartialBindingWithMeta::new(
      Some(object_expr),
      Some(BindingPath::variable(
        Some(DUMMY_SP),
        vec!["color".into()],
        None,
      )),
      true,
      meta.clone(),
      BindingSource::Module,
    );
    meta.insert_parent_binding("color", binding);

    let result =
      resolve_binding("color", meta, identity_evaluate as EvaluateExpression).expect("binding");

    let Expr::Lit(Lit::Str(str_lit)) = result.node.expect("resolved literal") else {
      panic!("expected string literal");
    };

    assert_eq!(str_lit.value, "red");
  }

  #[test]
  fn resolves_named_import_binding() {
    let dir = tempdir().expect("temp directory");
    let entry_path = dir.path().join("entry.tsx");
    fs::write(&entry_path, "").expect("write entry");

    let module_path = dir.path().join("colors.ts");
    fs::write(&module_path, "export const blue = 'blue';").expect("write module");

    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      TransformFileOptions {
        filename: Some(entry_path.to_string_lossy().into_owned()),
        cwd: Some(dir.path().to_path_buf()),
        root: Some(dir.path().to_path_buf()),
        loc_filename: None,
      },
    );

    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));
    let meta = Metadata::new(state.clone());

    let binding = PartialBindingWithMeta::new(
      None,
      Some(BindingPath::import(
        Some(DUMMY_SP),
        "./colors".into(),
        ImportBindingKind::Named("blue".into()),
      )),
      true,
      meta.clone(),
      BindingSource::Import,
    );
    meta.insert_parent_binding("blue", binding);

    let result =
      resolve_binding("blue", meta, identity_evaluate as EvaluateExpression).expect("binding");

    let Expr::Lit(Lit::Str(str_lit)) = result.node.expect("resolved literal") else {
      panic!("expected string literal");
    };

    assert_eq!(str_lit.value, "blue");
  }

  #[test]
  fn resolves_reexported_import_binding() {
    let dir = tempdir().expect("temp directory");
    let entry_path = dir.path().join("entry.tsx");
    fs::write(&entry_path, "").expect("write entry");

    let colors_path = dir.path().join("colors.ts");
    fs::write(&colors_path, "export const blue = 'blue';").expect("write colors module");

    let gateway_path = dir.path().join("gateway.ts");
    fs::write(&gateway_path, "export { blue } from './colors';").expect("write gateway module");

    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      TransformFileOptions {
        filename: Some(entry_path.to_string_lossy().into_owned()),
        cwd: Some(dir.path().to_path_buf()),
        root: Some(dir.path().to_path_buf()),
        loc_filename: None,
      },
    );

    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));
    let meta = Metadata::new(state.clone());

    let binding = PartialBindingWithMeta::new(
      None,
      Some(BindingPath::import(
        Some(DUMMY_SP),
        "./gateway".into(),
        ImportBindingKind::Named("blue".into()),
      )),
      true,
      meta.clone(),
      BindingSource::Import,
    );
    meta.insert_parent_binding("blue", binding);

    let result = resolve_binding(
      "blue",
      meta,
      utils_evaluate_expression::evaluate_expression as EvaluateExpression,
    )
    .expect("binding");

    let node = result.node.expect("resolved node");
    let evaluated = utils_evaluate_expression::evaluate_expression(&node, result.meta.clone());

    let Expr::Lit(Lit::Str(str_lit)) = evaluated.value else {
      panic!("expected string literal");
    };

    assert_eq!(str_lit.value, "blue");
  }

  #[test]
  fn resolves_binding_using_parser_plugins() {
    let dir = tempdir().expect("temp directory");
    let entry_path = dir.path().join("entry.tsx");
    fs::write(&entry_path, "").expect("write entry");

    let module_path = dir.path().join("colors.js");
    fs::write(&module_path, "export const blue: string = 'blue';").expect("write module");

    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      TransformFileOptions {
        filename: Some(entry_path.to_string_lossy().into_owned()),
        cwd: Some(dir.path().to_path_buf()),
        root: Some(dir.path().to_path_buf()),
        loc_filename: None,
      },
    );

    let mut options = PluginOptions::default();
    options.parser_babel_plugins = Some(vec![Value::String("typescript".into())]);

    let state = Rc::new(RefCell::new(TransformState::new(file, options)));
    let meta = Metadata::new(state.clone());

    let binding = PartialBindingWithMeta::new(
      None,
      Some(BindingPath::import(
        Some(DUMMY_SP),
        "./colors".into(),
        ImportBindingKind::Named("blue".into()),
      )),
      true,
      meta.clone(),
      BindingSource::Import,
    );
    meta.insert_parent_binding("blue", binding);

    let result = resolve_binding(
      "blue",
      meta,
      utils_evaluate_expression::evaluate_expression as EvaluateExpression,
    )
    .expect("binding");

    let Expr::Lit(Lit::Str(str_lit)) = result.node.expect("resolved literal") else {
      panic!("expected string literal");
    };

    assert_eq!(str_lit.value, "blue");
  }

  #[test]
  fn resolves_binding_with_decorators_plugin() {
    let dir = tempdir().expect("temp directory");
    let entry_path = dir.path().join("entry.js");
    fs::write(&entry_path, "").expect("write entry");

    let module_path = dir.path().join("decorators.js");
    fs::write(
      &module_path,
      r#"
            function dec(target) {
              return target;
            }

            @dec
            class Foo {}

            export { Foo };
            export const blue = 'blue';
            "#,
    )
    .expect("write module");

    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      TransformFileOptions {
        filename: Some(entry_path.to_string_lossy().into_owned()),
        cwd: Some(dir.path().to_path_buf()),
        root: Some(dir.path().to_path_buf()),
        loc_filename: None,
      },
    );

    let mut options = PluginOptions::default();
    let mut decorator_options = serde_json::Map::new();
    decorator_options.insert("decoratorsBeforeExport".into(), Value::Bool(true));

    options.parser_babel_plugins = Some(vec![Value::Array(vec![
      Value::String("decorators".into()),
      Value::Object(decorator_options),
    ])]);

    let state = Rc::new(RefCell::new(TransformState::new(file, options)));
    let meta = Metadata::new(state.clone());

    let binding = PartialBindingWithMeta::new(
      None,
      Some(BindingPath::import(
        Some(DUMMY_SP),
        "./decorators".into(),
        ImportBindingKind::Named("blue".into()),
      )),
      true,
      meta.clone(),
      BindingSource::Import,
    );
    meta.insert_parent_binding("blue", binding);

    let result = resolve_binding(
      "blue",
      meta,
      utils_evaluate_expression::evaluate_expression as EvaluateExpression,
    )
    .expect("binding");

    let Expr::Lit(Lit::Str(str_lit)) = result.node.expect("resolved literal") else {
      panic!("expected string literal");
    };

    assert_eq!(str_lit.value, "blue");
  }

  #[test]
  fn resolves_binding_with_auto_accessors_plugin() {
    let dir = tempdir().expect("temp directory");
    let entry_path = dir.path().join("entry.js");
    fs::write(&entry_path, "").expect("write entry");

    let module_path = dir.path().join("auto-accessors.js");
    fs::write(
      &module_path,
      r#"
            export class Widget {
              accessor size = 10;
            }

            export const blue = 'blue';
            "#,
    )
    .expect("write module");

    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      TransformFileOptions {
        filename: Some(entry_path.to_string_lossy().into_owned()),
        cwd: Some(dir.path().to_path_buf()),
        root: Some(dir.path().to_path_buf()),
        loc_filename: None,
      },
    );

    let mut options = PluginOptions::default();
    options.parser_babel_plugins = Some(vec![Value::String("autoAccessors".into())]);

    let state = Rc::new(RefCell::new(TransformState::new(file, options)));
    let meta = Metadata::new(state.clone());

    let binding = PartialBindingWithMeta::new(
      None,
      Some(BindingPath::import(
        Some(DUMMY_SP),
        "./auto-accessors".into(),
        ImportBindingKind::Named("blue".into()),
      )),
      true,
      meta.clone(),
      BindingSource::Import,
    );
    meta.insert_parent_binding("blue", binding);

    let result = resolve_binding(
      "blue",
      meta,
      utils_evaluate_expression::evaluate_expression as EvaluateExpression,
    )
    .expect("binding");

    let Expr::Lit(Lit::Str(str_lit)) = result.node.expect("resolved literal") else {
      panic!("expected string literal");
    };

    assert_eq!(str_lit.value, "blue");
  }

  #[test]
  fn resolves_jsx_dependency_with_default_plugins() {
    let dir = tempdir().expect("temp directory");
    let entry_path = dir.path().join("entry.tsx");
    fs::write(&entry_path, "").expect("write entry");

    let module_path = dir.path().join("component.js");
    fs::write(
      &module_path,
      "export const element = <div data-testid={'hi'} />;",
    )
    .expect("write module");

    let cm: Lrc<SourceMap> = Default::default();
    let file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      TransformFileOptions {
        filename: Some(entry_path.to_string_lossy().into_owned()),
        cwd: Some(dir.path().to_path_buf()),
        root: Some(dir.path().to_path_buf()),
        loc_filename: None,
      },
    );

    let state = Rc::new(RefCell::new(TransformState::new(
      file,
      PluginOptions::default(),
    )));
    let meta = Metadata::new(state.clone());

    let binding = PartialBindingWithMeta::new(
      None,
      Some(BindingPath::import(
        Some(DUMMY_SP),
        "./component".into(),
        ImportBindingKind::Named("element".into()),
      )),
      true,
      meta.clone(),
      BindingSource::Import,
    );
    meta.insert_parent_binding("element", binding);

    let result =
      resolve_binding("element", meta, identity_evaluate as EvaluateExpression).expect("binding");

    match result.node {
      Some(Expr::JSXElement(_)) => {}
      other => panic!("expected JSXElement, found {other:?}"),
    }
  }
}
