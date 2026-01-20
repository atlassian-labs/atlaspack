use std::cell::RefCell;
use std::env;
use std::rc::Rc;

use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;
use regex::Regex;

use swc_core::common::comments::{Comment, CommentKind};
use swc_core::common::{DUMMY_SP, Span, Spanned, SyntaxContext};
use swc_core::ecma::ast::{
  BindingIdent, BlockStmt, ClassDecl, Decl, DefaultDecl, EmptyStmt, Expr, FnDecl, Ident,
  ImportDecl, ImportNamedSpecifier, ImportPhase, ImportSpecifier, ImportStarAsSpecifier, Lit,
  Module, ModuleDecl, ModuleExportName, ModuleItem, Null, Pat, Program, Stmt, Str, VarDecl,
  VarDeclKind, VarDeclarator,
};
use swc_core::ecma::visit::{VisitMut, VisitMutWith, noop_visit_mut_type};

use crate::class_names::visit_class_names;
use crate::constants::COMPILED_IMPORT;
use crate::css_map::{CssMapUsage, visit_css_map_path};
use crate::css_prop::visit_css_prop;
use crate::postcss::plugins::extract_stylesheets::normalize_block_value_spacing;
use crate::styled::{StyledVisitResult, visit_styled};
use crate::types::{
  CleanupAction, CompiledImports, Metadata, PathCleanup, PluginOptions, SharedScope,
  SharedTransformState, TransformFile, TransformMetadata, TransformState,
};
use crate::utils_append_runtime_imports::append_runtime_imports;
use crate::utils_ast::build_code_frame_error;
use crate::utils_is_compiled::{
  is_compiled_css_call_expression, is_compiled_css_tagged_template_expression,
  is_compiled_keyframes_call_expression, is_compiled_keyframes_tagged_template_expression,
  is_compiled_styled_call_expression, is_compiled_styled_tagged_template_expression,
};
use crate::utils_is_jsx_function::is_transformed_jsx_function;
use crate::utils_module_scope;
use crate::utils_normalize_props_usage::normalize_props_usage;
use crate::utils_preserve_leading_comments::preserve_leading_comments;
use crate::utils_types::{BindingPath, BindingSource, PartialBindingWithMeta};
use crate::xcss_prop::{visit_xcss_prop, visit_xcss_prop_on_element};

const PACKAGE_NAME: &str = "@compiled/babel-plugin";

fn insert_sheet_declarations(module: &mut Module, state: &mut TransformState) {
  if state.sheets.is_empty() {
    return;
  }

  let mut declarations = Vec::with_capacity(state.sheets.len());

  for (sheet, ident) in state.sheets.iter() {
    let binding = BindingIdent {
      id: ident.clone(),
      type_ann: None,
    };

    let declarator = VarDeclarator {
      span: DUMMY_SP,
      name: Pat::Ident(binding),
      init: Some(Box::new(Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: sheet.clone().into(),
        raw: None,
      })))),
      definite: false,
    };

    let var_decl = VarDecl {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      kind: VarDeclKind::Const,
      declare: false,
      decls: vec![declarator],
    };

    declarations.push(ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(var_decl)))));
  }

  let insert_index = module
    .body
    .iter()
    .position(|item| !matches!(item, ModuleItem::ModuleDecl(ModuleDecl::Import(_))))
    .unwrap_or(module.body.len());

  module
    .body
    .splice(insert_index..insert_index, declarations.into_iter());
}

/// Primary SWC transform that will eventually mirror `@compiled/babel-plugin`.
pub struct CompiledCssInJsTransform {
  state: SharedTransformState,
  metadata: TransformMetadata,
}

impl CompiledCssInJsTransform {
  pub fn new(options: PluginOptions) -> Self {
    let state = Rc::new(RefCell::new(TransformState::new(
      TransformFile::default(),
      options,
    )));

    Self {
      state,
      metadata: TransformMetadata::default(),
    }
  }

  pub fn into_metadata(self) -> TransformMetadata {
    let mut metadata = self.metadata;
    let state = self.state.borrow();

    if metadata.included_files.is_empty() {
      metadata.included_files = state.included_files.clone();
    }

    if metadata.style_rules.is_empty() {
      if !state.style_rules.is_empty() {
        metadata.style_rules = state
          .style_rules
          .iter()
          .map(|rule| normalize_block_value_spacing(rule))
          .collect();
      } else if !state.sheets.is_empty() {
        // Fallback: when the explicit style_rules set has not been populated,
        // derive style rules from the hoisted sheets map (preserves insertion order).
        metadata.style_rules = state
          .sheets
          .keys()
          .map(|rule| normalize_block_value_spacing(rule))
          .collect();
      }
    }

    metadata
  }

  pub fn state(&self) -> SharedTransformState {
    Rc::clone(&self.state)
  }

  pub fn metadata_mut(&mut self) -> &mut TransformMetadata {
    &mut self.metadata
  }
}

#[cfg(test)]
mod tests {
  use super::CompiledCssInJsTransform;
  use crate::types::{PluginOptions, TransformFile, TransformFileOptions, TransformState};
  use std::any::Any;
  use std::panic::{AssertUnwindSafe, catch_unwind};
  use swc_core::common::comments::{Comment, CommentKind};
  use swc_core::common::sync::Lrc;
  use swc_core::common::{BytePos, FileName, SourceFile, SourceMap, Span};
  use swc_core::ecma::ast::{
    BlockStmtOrExpr, Decl, Expr, ImportSpecifier, JSXAttrName, JSXAttrOrSpread, JSXElementName,
    Lit, Module, ModuleDecl, ModuleItem, Program, Stmt,
  };
  use swc_core::ecma::codegen::{Config, Emitter, text_writer::JsWriter};
  use swc_core::ecma::parser::lexer::Lexer;
  use swc_core::ecma::parser::{EsSyntax, Parser, StringInput, Syntax};
  use swc_core::ecma::visit::VisitMutWith;

  struct EnvVarGuard(&'static str);

  impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
      unsafe {
        std::env::set_var(key, value);
      }
      EnvVarGuard(key)
    }
  }

  impl Drop for EnvVarGuard {
    fn drop(&mut self) {
      unsafe {
        std::env::remove_var(self.0);
      }
    }
  }

  fn module_items_without_noop(module: &Module) -> Vec<&ModuleItem> {
    module
      .body
      .iter()
      .filter(|item| !matches!(item, ModuleItem::Stmt(Stmt::Empty(_))))
      .collect()
  }

  fn parse_program(code: &str) -> (Program, Lrc<SourceMap>, Lrc<SourceFile>) {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
      Lrc::new(FileName::Custom("test.tsx".into())),
      code.to_string(),
    );
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
    let program = parser.parse_program().expect("failed to parse program");
    assert!(parser.take_errors().is_empty());

    (program, cm, fm)
  }

  fn parse_script(code: &str) -> (Program, Lrc<SourceMap>, Lrc<SourceFile>) {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
      Lrc::new(FileName::Custom("script.tsx".into())),
      code.to_string(),
    );
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
    let script = parser
      .parse_script()
      .expect("failed to parse script program");
    assert!(parser.take_errors().is_empty());

    (Program::Script(script), cm, fm)
  }

  fn print_module(cm: &Lrc<SourceMap>, module: &Module) -> String {
    let mut buf = Vec::new();
    {
      let writer = JsWriter::new(cm.clone(), "\n", &mut buf, None);
      let mut emitter = Emitter {
        cfg: Config::default(),
        comments: None,
        cm: cm.clone(),
        wr: writer,
      };
      emitter
        .emit_module(module)
        .expect("failed to emit transformed module");
    }

    String::from_utf8(buf).expect("module is not valid UTF-8")
  }

  fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
      return message.clone();
    }

    if let Some(message) = payload.downcast_ref::<&str>() {
      return (*message).to_string();
    }

    panic!("unexpected panic payload")
  }

  fn reset_transform_state(
    transform: &mut CompiledCssInJsTransform,
    cm: Lrc<SourceMap>,
    filename: &str,
  ) {
    let file = TransformFile::transform_compiled_with_options(
      cm,
      Vec::new(),
      TransformFileOptions {
        filename: Some(filename.into()),
        loc_filename: Some(filename.into()),
        ..TransformFileOptions::default()
      },
    );

    let shared_state = transform.state();
    let options = {
      let state_ref = shared_state.borrow();
      state_ref.opts.clone()
    };

    *shared_state.borrow_mut() = TransformState::new(file, options);
  }

  #[test]
  fn records_compiled_imports_and_removes_matched_specifiers() {
    let (mut program, _, _) =
      parse_program("import { styled, ClassNames } from '@compiled/react';");

    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());
    program.visit_mut_with(&mut transform);

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    assert!(matches!(
      module.body.first(),
      Some(ModuleItem::Stmt(Stmt::Empty(_)))
    ));

    let items = module_items_without_noop(module);
    assert_eq!(items.len(), 3);

    let ModuleItem::ModuleDecl(ModuleDecl::Import(forward_ref_import)) = items[0] else {
      panic!("expected forwardRef import");
    };
    assert_eq!(forward_ref_import.src.value.as_ref(), "react");
    assert_eq!(forward_ref_import.specifiers.len(), 1);
    let ImportSpecifier::Named(named) = &forward_ref_import.specifiers[0] else {
      panic!("expected named forwardRef specifier");
    };
    assert_eq!(named.local.sym.as_ref(), "forwardRef");

    let ModuleItem::ModuleDecl(ModuleDecl::Import(react_import)) = items[1] else {
      panic!("expected React namespace import");
    };
    assert_eq!(react_import.src.value.as_ref(), "react");
    assert_eq!(react_import.specifiers.len(), 1);
    assert!(matches!(
      react_import.specifiers[0],
      ImportSpecifier::Namespace(_)
    ));

    let ModuleItem::ModuleDecl(ModuleDecl::Import(runtime_import)) = items[2] else {
      panic!("expected runtime import");
    };
    assert_eq!(runtime_import.src.value.as_ref(), "@compiled/react/runtime");

    let state = transform.state();
    let state_ref = state.borrow();
    let imports = state_ref
      .compiled_imports
      .as_ref()
      .expect("compiled imports should be tracked");
    assert_eq!(imports.styled, vec!["styled".to_string()]);
    assert_eq!(imports.class_names, vec!["ClassNames".to_string()]);
    assert!(imports.css.is_empty());
    assert!(imports.keyframes.is_empty());
    assert!(imports.css_map.is_empty());
  }

  #[test]
  fn retains_unmatched_specifiers() {
    let (mut program, _, _) = parse_program("import { something } from '@compiled/react';");

    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());
    program.visit_mut_with(&mut transform);

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    assert!(matches!(
      module.body.first(),
      Some(ModuleItem::Stmt(Stmt::Empty(_)))
    ));

    let items = module_items_without_noop(module);
    assert_eq!(items.len(), 1);
    let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = items[0] else {
      panic!("expected retained import");
    };
    assert_eq!(import.specifiers.len(), 1);

    let state = transform.state();
    let state_ref = state.borrow();
    let imports = state_ref
      .compiled_imports
      .as_ref()
      .expect("compiled imports should be initialised");
    assert!(imports.styled.is_empty());
    assert!(imports.class_names.is_empty());
    assert!(imports.css.is_empty());
    assert!(imports.keyframes.is_empty());
    assert!(imports.css_map.is_empty());
  }

  #[test]
  fn panics_when_jsx_call_remains_after_transform() {
    let (mut program, cm, _) =
      parse_program("import { jsx } from '@compiled/react';\nconst value = jsx('div');");

    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());
    reset_transform_state(&mut transform, cm, "test.tsx");
    let result = catch_unwind(AssertUnwindSafe(|| {
      program.visit_mut_with(&mut transform);
    }));

    assert!(
      result.is_err(),
      "transform should panic when jsx call remains"
    );
    let message = panic_message(result.err().unwrap());
    assert!(
      message.contains("Found a `jsx` function call"),
      "unexpected panic message: {message}"
    );
  }

  #[test]
  fn allows_jsx_calls_without_compiled_imports() {
    let (mut program, cm, _) = parse_program("const value = jsx('div');");

    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());
    reset_transform_state(&mut transform, cm, "test.tsx");
    program.visit_mut_with(&mut transform);

    let state = transform.state();
    assert!(state.borrow().compiled_imports.is_none());
  }

  #[test]
  fn inserts_generated_comment_and_noop_statement() {
    let source = "\
            import { css } from '@compiled/react';\n\
            const styles = css({ color: 'red' });\n\
        ";

    let _guard = EnvVarGuard::set("TEST_PKG_VERSION", "0.0.0");

    let (mut program, _, _) = parse_program(source);
    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());
    program.visit_mut_with(&mut transform);

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    match module.body.first() {
      Some(ModuleItem::Stmt(Stmt::Empty(_))) => {}
      other => panic!("expected leading empty statement, found {other:?}"),
    }

    let state = transform.state();
    let state_ref = state.borrow();
    let comment = state_ref
      .file
      .comments
      .first()
      .expect("expected generated comment");
    assert_eq!(comment.kind, CommentKind::Block);
    assert_eq!(
      comment.text.as_ref(),
      " File generated by @compiled/babel-plugin v0.0.0 "
    );
  }

  #[test]
  fn enables_css_prop_via_jsx_import_source_pragma() {
    let source = "/** @jsxImportSource @compiled/react */\nconst element = <div />;";

    let (mut program, cm, fm) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());

    let comment_source = "/** @jsxImportSource @compiled/react */";
    let comment_start = source
      .find(comment_source)
      .expect("comment should be present") as u32;
    let comment_span = Span::new(
      BytePos(fm.start_pos.0 + comment_start),
      BytePos(fm.start_pos.0 + comment_start + comment_source.len() as u32),
    );
    let comment = Comment {
      kind: CommentKind::Block,
      span: comment_span,
      text: "* @jsxImportSource @compiled/react ".into(),
    };

    assert!(super::JSX_SOURCE_ANNOTATION_REGEX.is_match(comment.text.as_ref()));

    {
      let mut state = transform.state.borrow_mut();
      state.replace_file(TransformFile::transform_compiled_with_options(
        cm.clone(),
        vec![comment],
        TransformFileOptions {
          filename: Some("test.tsx".into()),
          ..TransformFileOptions::default()
        },
      ));
    }

    program.visit_mut_with(&mut transform);

    let state = transform.state();
    let state_ref = state.borrow();
    assert!(
      state_ref
        .import_sources
        .iter()
        .any(|source| source == "@compiled/react")
    );
    assert!(state_ref.pragma.jsx_import_source);
    assert!(state_ref.compiled_imports.is_some());
    assert_eq!(state_ref.file.comments.len(), 1);

    let generated_comment = &state_ref.file.comments[0];
    assert!(
      generated_comment
        .text
        .contains("generated by @compiled/babel-plugin")
    );
    assert!(!generated_comment.text.as_ref().contains("@jsxImportSource"));
  }

  #[test]
  fn enables_classic_jsx_pragma_when_imported_from_compiled() {
    let source = "import { jsx as compiledJsx } from '@compiled/react';\n/** @jsx compiledJsx */\nconst element = <div />;";

    let (mut program, cm, fm) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());

    let comment_source = "/** @jsx compiledJsx */";
    let comment_start = source
      .find(comment_source)
      .expect("comment should be present") as u32;
    let comment_span = Span::new(
      BytePos(fm.start_pos.0 + comment_start),
      BytePos(fm.start_pos.0 + comment_start + comment_source.len() as u32),
    );
    let comment = Comment {
      kind: CommentKind::Block,
      span: comment_span,
      text: "* @jsx compiledJsx ".into(),
    };

    assert!(super::JSX_ANNOTATION_REGEX.is_match(comment.text.as_ref()));

    {
      let mut state = transform.state.borrow_mut();
      state.replace_file(TransformFile::transform_compiled_with_options(
        cm.clone(),
        vec![comment],
        TransformFileOptions {
          filename: Some("test.tsx".into()),
          ..TransformFileOptions::default()
        },
      ));
    }

    program.visit_mut_with(&mut transform);

    let state = transform.state();
    let state_ref = state.borrow();
    assert!(state_ref.pragma.jsx);
    assert!(state_ref.compiled_imports.is_some());
    assert_eq!(state_ref.file.comments.len(), 1);

    let generated_comment = &state_ref.file.comments[0];
    assert!(
      generated_comment
        .text
        .contains("generated by @compiled/babel-plugin")
    );
    assert!(!generated_comment.text.as_ref().contains("@jsx"));
  }

  #[test]
  fn preserves_leading_comments_before_runtime_imports() {
    let source = "// @flow strict-local\nimport { styled } from '@compiled/react';\nconst Component = styled.div({ fontSize: 12 });";

    let (mut program, cm, fm) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());

    let comment_marker = "// @flow strict-local";
    let comment_start = source
      .find(comment_marker)
      .expect("comment should be present") as u32;
    let comment_span = Span::new(
      BytePos(fm.start_pos.0 + comment_start),
      BytePos(fm.start_pos.0 + comment_start + comment_marker.len() as u32),
    );
    let comment = Comment {
      kind: CommentKind::Line,
      span: comment_span,
      text: " @flow strict-local".into(),
    };

    {
      let mut state = transform.state.borrow_mut();
      state.replace_file(TransformFile::transform_compiled_with_options(
        cm.clone(),
        vec![comment],
        TransformFileOptions {
          filename: Some("test.tsx".into()),
          ..TransformFileOptions::default()
        },
      ));
    }

    program.visit_mut_with(&mut transform);

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    assert!(matches!(
      module.body.first(),
      Some(ModuleItem::Stmt(Stmt::Empty(_)))
    ));

    let state = transform.state();
    let state_ref = state.borrow();
    assert_eq!(state_ref.file.comments.len(), 2);
    assert!(
      state_ref.file.comments[0]
        .text
        .contains("generated by @compiled/babel-plugin")
    );
    assert!(
      state_ref.file.comments[1]
        .text
        .as_ref()
        .contains("@flow strict-local")
    );
  }

  #[test]
  fn preserves_multiple_leading_comments_in_order() {
    let source = "// first\n// second\nimport { styled } from '@compiled/react';\nconst Component = styled.div({ fontSize: 12 });";

    let (mut program, cm, fm) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());

    let comments = ["// first", "// second"];
    let mut stored = Vec::new();

    for marker in comments {
      let start = source.find(marker).expect("comment present") as u32;
      let span = Span::new(
        BytePos(fm.start_pos.0 + start),
        BytePos(fm.start_pos.0 + start + marker.len() as u32),
      );
      stored.push(Comment {
        kind: CommentKind::Line,
        span,
        text: format!(" {}", marker.trim_start_matches("//")).into(),
      });
    }

    {
      let mut state = transform.state.borrow_mut();
      state.replace_file(TransformFile::transform_compiled_with_options(
        cm.clone(),
        stored,
        TransformFileOptions {
          filename: Some("test.tsx".into()),
          ..TransformFileOptions::default()
        },
      ));
    }

    program.visit_mut_with(&mut transform);

    let state = transform.state();
    let state_ref = state.borrow();

    assert_eq!(state_ref.file.comments.len(), 3);
    assert!(
      state_ref.file.comments[0]
        .text
        .contains("generated by @compiled/babel-plugin")
    );
    assert!(state_ref.file.comments[1].text.as_ref().contains("first"));
    assert!(state_ref.file.comments[2].text.as_ref().contains("second"));
  }

  #[test]
  fn replaces_css_variable_initialiser_with_null() {
    let source = r#"
            import { css } from '@compiled/react';

            const styles = css`color: red;`;
        "#;

    let (mut program, _, _) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());
    program.visit_mut_with(&mut transform);

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    let var_decl = module
      .body
      .iter()
      .find_map(|item| match item {
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => Some(var),
        _ => None,
      })
      .expect("expected variable declaration");

    let Some(init) = var_decl.decls[0].init.as_ref() else {
      panic!("expected css init");
    };

    assert!(matches!(init.as_ref(), Expr::Lit(Lit::Null(_))));
  }

  #[test]
  fn replaces_keyframes_initialiser_with_null() {
    let source = r#"
            import { keyframes } from '@compiled/react';

            const fadeOut = keyframes`from { opacity: 1; } to { opacity: 0; }`;
        "#;

    let (mut program, _, _) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());
    program.visit_mut_with(&mut transform);

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    let var_decl = module
      .body
      .iter()
      .find_map(|item| match item {
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => Some(var),
        _ => None,
      })
      .expect("expected variable declaration");

    let Some(init) = var_decl.decls[0].init.as_ref() else {
      panic!("expected keyframes init");
    };

    assert!(matches!(init.as_ref(), Expr::Lit(Lit::Null(_))));
  }

  #[test]
  fn hoists_keyframes_sheet_before_nullifying_binding() {
    let source = r#"
            import { ClassNames, keyframes } from '@compiled/react';

            const fadeOut = keyframes({
              from: { opacity: 1 },
              to: { opacity: 0 }
            });

            const Component = () => (
              <ClassNames>
                {({ css }) => (
                  <div className={css({ animation: `${fadeOut} 2s ease-in-out` })} />
                )}
              </ClassNames>
            );
        "#;

    let (mut program, cm, _) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions {
      extract: Some(true),
      ..PluginOptions::default()
    });
    {
      let file = TransformFile::transform_compiled_with_options(
        cm.clone(),
        Vec::new(),
        TransformFileOptions {
          filename: Some("test.tsx".into()),
          ..TransformFileOptions::default()
        },
      );
      let state = transform.state();
      let mut state_ref = state.borrow_mut();
      state_ref.replace_file(file);
    }
    program.visit_mut_with(&mut transform);

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    let printed = print_module(&cm, module);
    assert!(
      printed.contains("const fadeOut = null;"),
      "expected fadeOut binding to be replaced with null"
    );

    let metadata = transform.into_metadata();
    let keyframes_rule = metadata
      .style_rules
      .iter()
      .find(|rule| rule.contains("@keyframes"))
      .cloned()
      .expect("expected keyframes style rule to be recorded in metadata");
    let animation_rule = metadata
      .style_rules
      .iter()
      .find(|rule| rule.contains("animation:"))
      .cloned()
      .expect("expected animation style rule to be recorded in metadata");

    let name_start = keyframes_rule
      .find("@keyframes ")
      .map(|index| index + "@keyframes ".len())
      .expect("expected keyframes rule to include animation name");
    let name_end = keyframes_rule[name_start..]
      .find('{')
      .map(|offset| name_start + offset)
      .expect("expected keyframes rule to include body");
    let keyframe_name = &keyframes_rule[name_start..name_end];
    let class_start = animation_rule
      .find('.')
      .map(|index| index + 1)
      .expect("expected animation rule to include class selector");
    let class_end = animation_rule[class_start..]
      .find('{')
      .map(|offset| class_start + offset)
      .expect("expected animation rule to include declarations");
    let class_name = &animation_rule[class_start..class_end];

    assert!(
      animation_rule.contains(keyframe_name),
      "expected animation rule to reference the keyframes name"
    );
    assert!(
      printed.contains(class_name),
      "expected transformed module to reference the animation class"
    );
  }

  #[test]
  fn hoists_keyframes_sheet_for_tagged_template_before_cleanup() {
    let source = r#"
            import { ClassNames, keyframes } from '@compiled/react';

            const fadeOut = keyframes`from { opacity: 1; } to { opacity: 0; }`;

            const Component = () => (
              <ClassNames>
                {({ css }) => (
                  <div className={css`animation: ${fadeOut} 2s ease-in-out;`} />
                )}
              </ClassNames>
            );
        "#;

    let (mut program, cm, _) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions {
      extract: Some(true),
      ..PluginOptions::default()
    });
    {
      let file = TransformFile::transform_compiled_with_options(
        cm.clone(),
        Vec::new(),
        TransformFileOptions {
          filename: Some("test.tsx".into()),
          ..TransformFileOptions::default()
        },
      );

      let state = transform.state();
      let mut state_ref = state.borrow_mut();
      state_ref.replace_file(file);
    }
    program.visit_mut_with(&mut transform);

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    let printed = print_module(&cm, module);
    assert!(
      printed.contains("const fadeOut = null;"),
      "expected fadeOut binding to be replaced with null"
    );

    let metadata = transform.into_metadata();
    let keyframes_rule = metadata
      .style_rules
      .iter()
      .find(|rule| rule.contains("@keyframes"))
      .cloned()
      .expect("expected keyframes style rule to be recorded in metadata");
    let animation_rule = metadata
      .style_rules
      .iter()
      .find(|rule| rule.contains("animation:"))
      .cloned()
      .expect("expected animation style rule to be recorded in metadata");

    let name_start = keyframes_rule
      .find("@keyframes ")
      .map(|index| index + "@keyframes ".len())
      .expect("expected keyframes rule to include animation name");
    let name_end = keyframes_rule[name_start..]
      .find('{')
      .map(|offset| name_start + offset)
      .expect("expected keyframes rule to include body");
    let keyframe_name = &keyframes_rule[name_start..name_end];
    let class_start = animation_rule
      .find('.')
      .map(|index| index + 1)
      .expect("expected animation rule to include class selector");
    let class_end = animation_rule[class_start..]
      .find('{')
      .map(|offset| class_start + offset)
      .expect("expected animation rule to include declarations");
    let class_name = &animation_rule[class_start..class_end];

    assert!(
      animation_rule.contains(keyframe_name),
      "expected animation rule to reference the keyframes name"
    );
    assert!(
      printed.contains(class_name),
      "expected transformed module to reference the animation class"
    );
  }

  #[test]
  fn replaces_css_call_expression_in_arguments_with_null() {
    let source = r#"
            import { css } from '@compiled/react';

            console.log(css({ color: 'red' }));
        "#;

    let (mut program, _, _) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());
    program.visit_mut_with(&mut transform);

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    let expr_stmt = module
      .body
      .iter()
      .filter_map(|item| match item {
        ModuleItem::Stmt(Stmt::Expr(expr)) => Some(expr),
        _ => None,
      })
      .next()
      .expect("expected expression statement");

    let Expr::Call(call) = &*expr_stmt.expr else {
      panic!("expected call expression");
    };

    assert_eq!(call.args.len(), 1);
    let arg = &call.args[0];
    assert!(matches!(arg.expr.as_ref(), Expr::Lit(Lit::Null(_))));
  }

  #[test]
  fn replaces_keyframes_tagged_template_with_null_in_arrays() {
    let source = r#"
            import { keyframes } from '@compiled/react';

            const animations = [keyframes`from { opacity: 1; } to { opacity: 0; }`];
        "#;

    let (mut program, _, _) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());
    program.visit_mut_with(&mut transform);

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    let var_decl = module
      .body
      .iter()
      .filter_map(|item| match item {
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => Some(var),
        _ => None,
      })
      .next()
      .expect("expected variable declaration");

    let Some(init) = var_decl.decls[0].init.as_ref() else {
      panic!("expected initializer");
    };

    let Expr::Array(array) = init.as_ref() else {
      panic!("expected array expression");
    };

    assert_eq!(array.elems.len(), 1);
    let Some(elem) = &array.elems[0] else {
      panic!("expected array element");
    };

    assert!(matches!(elem.expr.as_ref(), Expr::Lit(Lit::Null(_))));
  }

  #[test]
  fn transforms_css_prop_into_compiled_component() {
    let source = r#"
            import { css } from '@compiled/react';

            const Component = () => <div css={{ color: 'red' }} />;
        "#;

    let (mut program, cm, _) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());
    {
      let file = TransformFile::transform_compiled_with_options(
        cm.clone(),
        Vec::new(),
        TransformFileOptions {
          filename: Some("test.tsx".into()),
          ..TransformFileOptions::default()
        },
      );
      let mut state = transform.state.borrow_mut();
      state.replace_file(file);
    }
    program.visit_mut_with(&mut transform);

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    assert!(matches!(
      module.body.first(),
      Some(ModuleItem::Stmt(Stmt::Empty(_)))
    ));

    let items = module_items_without_noop(module);
    assert_eq!(
      items.len(),
      4,
      "expected 4 items: react import, runtime import, sheet declaration, and component"
    );

    let ModuleItem::ModuleDecl(ModuleDecl::Import(react_import)) = items[0] else {
      panic!("expected react import");
    };
    assert_eq!(react_import.src.value.as_ref(), "react");
    let ModuleItem::ModuleDecl(ModuleDecl::Import(runtime_import)) = items[1] else {
      panic!("expected runtime import");
    };
    assert_eq!(runtime_import.src.value.as_ref(), "@compiled/react/runtime");
    let specifiers: Vec<String> = runtime_import
      .specifiers
      .iter()
      .map(|specifier| match specifier {
        ImportSpecifier::Named(named) => named.local.sym.to_string(),
        ImportSpecifier::Default(default) => default.local.sym.to_string(),
        ImportSpecifier::Namespace(namespace) => namespace.local.sym.to_string(),
      })
      .collect();
    assert_eq!(specifiers, vec!["ax", "ix", "CC", "CS"]);

    // item[2] should be the hoisted sheet declaration
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(sheet_decl))) = items[2] else {
      panic!("expected sheet variable declaration");
    };
    assert_eq!(sheet_decl.decls.len(), 1);

    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) = items[3] else {
      panic!("expected component variable declaration");
    };
    assert_eq!(var_decl.decls.len(), 1);
    let declarator = &var_decl.decls[0];
    let Some(init) = &declarator.init else {
      panic!("expected initializer");
    };

    let Expr::Arrow(arrow) = &**init else {
      panic!("expected arrow expression");
    };
    let BlockStmtOrExpr::Expr(body_expr) = arrow.body.as_ref() else {
      panic!("expected expression body");
    };
    let Expr::JSXElement(element) = &**body_expr else {
      panic!("expected jsx element");
    };

    let JSXElementName::Ident(ident) = &element.opening.name else {
      panic!("expected CC identifier");
    };
    assert_eq!(ident.sym.as_ref(), "CC");
  }

  #[test]
  fn collects_style_rules_when_extract_enabled() {
    let source = r#"
            import { css } from '@compiled/react';

            const Component = () => <div css={{ color: 'red' }} />;
        "#;

    let (mut program, cm, _) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions {
      extract: Some(true),
      ..PluginOptions::default()
    });

    {
      let file = TransformFile::transform_compiled_with_options(
        cm.clone(),
        Vec::new(),
        TransformFileOptions {
          filename: Some("test.tsx".into()),
          ..TransformFileOptions::default()
        },
      );
      let mut state = transform.state.borrow_mut();
      state.replace_file(file);
    }

    program.visit_mut_with(&mut transform);

    let metadata = transform.into_metadata();
    assert_eq!(
      metadata.style_rules,
      vec!["._syaz5scu{color:red}".to_string()]
    );
  }

  #[test]
  fn processes_xcss_by_default() {
    let source = "export {};\nconst Component = () => <div xcss={{ color: 'red' }} />;";

    let (mut program, cm, _) = parse_program(source);
    let mut transform = CompiledCssInJsTransform::new(PluginOptions::default());
    reset_transform_state(&mut transform, cm.clone(), "test.tsx");

    program.visit_mut_with(&mut transform);

    {
      let state = transform.state();
      let state_ref = state.borrow();
      assert!(state_ref.uses_xcss, "xcss should be processed by default");
    }

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    let has_runtime_import = module_items_without_noop(module)
      .iter()
      .any(|item| match item {
        ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
          import.src.value.as_ref() == "@compiled/react/runtime"
        }
        _ => false,
      });

    assert!(
      has_runtime_import,
      "expected runtime import when xcss runs by default"
    );
  }

  #[test]
  fn processes_script_xcss_by_default() {
    let source = "const Component = () => <div xcss={{ color: 'red' }} />;";

    let (mut program, cm, _) = parse_script(source);
    let mut transform = CompiledCssInJsTransform::new(PluginOptions {
      extract: Some(true),
      ..PluginOptions::default()
    });
    reset_transform_state(&mut transform, cm.clone(), "script.tsx");

    program.visit_mut_with(&mut transform);

    match &program {
      Program::Module(module) => {
        let has_runtime_import = module.body.iter().any(|item| {
          matches!(
              item,
              ModuleItem::ModuleDecl(ModuleDecl::Import(import))
                  if import.src.value.as_ref() == "@compiled/react/runtime"
          )
        });

        assert!(
          has_runtime_import,
          "expected runtime import when transforming script program"
        );
      }
      Program::Script(_) => panic!("expected script program to be converted to a module"),
    }

    let metadata = transform.into_metadata();
    assert!(
      !metadata.style_rules.is_empty(),
      "expected style rules for script xcss transform"
    );
  }

  #[test]
  fn skips_xcss_when_disabled_via_option() {
    let source = "export {};\nconst Component = () => <div xcss={{ color: 'red' }} />;";

    let (mut program, cm, _) = parse_program(source);
    let mut transform = CompiledCssInJsTransform::new(PluginOptions {
      process_xcss: Some(false),
      ..PluginOptions::default()
    });
    reset_transform_state(&mut transform, cm.clone(), "test.tsx");

    program.visit_mut_with(&mut transform);

    {
      let state = transform.state();
      let state_ref = state.borrow();
      assert!(
        !state_ref.uses_xcss,
        "xcss should be skipped when explicitly disabled"
      );
    }

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    let has_runtime_import = module.body.iter().any(|item| match item {
      ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
        import.src.value.as_ref() == "@compiled/react/runtime"
      }
      _ => false,
    });

    assert!(
      !has_runtime_import,
      "runtime import should not be added when xcss is disabled"
    );

    let component_decl = module
      .body
      .iter()
      .find_map(|item| match item {
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => Some(var),
        _ => None,
      })
      .expect("expected component declaration");

    let Some(init) = component_decl.decls[0].init.as_ref() else {
      panic!("expected initializer");
    };

    let Expr::Arrow(arrow) = init.as_ref() else {
      panic!("expected arrow function initializer");
    };

    let body_expr = match arrow.body.as_ref() {
      BlockStmtOrExpr::Expr(expr) => expr,
      BlockStmtOrExpr::BlockStmt(block) => block
        .stmts
        .iter()
        .find_map(|stmt| match stmt {
          Stmt::Return(ret) => ret.arg.as_ref(),
          _ => None,
        })
        .expect("expected return argument in block body"),
    };

    let Expr::JSXElement(element) = body_expr.as_ref() else {
      panic!("expected JSX element expression");
    };

    let has_xcss_attr = element.opening.attrs.iter().any(|attr| match attr {
            JSXAttrOrSpread::JSXAttr(attr)
                if matches!(&attr.name, JSXAttrName::Ident(ident) if ident.sym.as_ref() == "xcss") =>
            {
                true
            }
            _ => false,
        });

    assert!(
      has_xcss_attr,
      "xcss attribute should be preserved when disabled"
    );
  }

  #[test]
  fn carries_included_files_into_metadata() {
    let transform = CompiledCssInJsTransform::new(PluginOptions::default());

    {
      let mut state = transform.state.borrow_mut();
      state.included_files.push("./a.tsx".into());
      state.included_files.push("./b.tsx".into());
    }

    let metadata = transform.into_metadata();
    assert_eq!(
      metadata.included_files,
      vec!["./a.tsx".to_string(), "./b.tsx".to_string()]
    );
  }

  #[test]
  fn skips_react_import_when_disabled() {
    let source = r#"
            import { css } from '@compiled/react';

            const Component = () => <div css={{ color: 'red' }} />;
        "#;

    let (mut program, cm, _) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions {
      import_react: Some(false),
      ..PluginOptions::default()
    });

    {
      let file = TransformFile::transform_compiled_with_options(
        cm.clone(),
        Vec::new(),
        TransformFileOptions {
          filename: Some("test.tsx".into()),
          ..TransformFileOptions::default()
        },
      );

      let mut state = transform.state.borrow_mut();
      state.replace_file(file);
    }

    program.visit_mut_with(&mut transform);

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    assert!(matches!(
      module.body.first(),
      Some(ModuleItem::Stmt(Stmt::Empty(_)))
    ));

    let items = module_items_without_noop(module);
    assert_eq!(
      items.len(),
      3,
      "expected 3 items: runtime import, sheet declaration, and component"
    );

    let ModuleItem::ModuleDecl(ModuleDecl::Import(runtime_import)) = items[0] else {
      panic!("expected runtime import");
    };
    assert_eq!(runtime_import.src.value.as_ref(), "@compiled/react/runtime");
  }

  #[test]
  fn transforms_styled_usage_and_inserts_display_name() {
    let source = r#"
            import { styled } from '@compiled/react';

            const Component = styled.div({ color: 'red' });
        "#;

    let (mut program, cm, _) = parse_program(source);

    let mut transform = CompiledCssInJsTransform::new(PluginOptions {
      add_component_name: Some(true),
      ..PluginOptions::default()
    });

    {
      let file = TransformFile::transform_compiled_with_options(
        cm.clone(),
        Vec::new(),
        TransformFileOptions {
          filename: Some("test.tsx".into()),
          ..TransformFileOptions::default()
        },
      );

      let mut state = transform.state.borrow_mut();
      state.replace_file(file);
    }

    program.visit_mut_with(&mut transform);

    let Program::Module(module) = &program else {
      panic!("expected module program");
    };

    assert!(matches!(
      module.body.first(),
      Some(ModuleItem::Stmt(Stmt::Empty(_)))
    ));

    let items = module_items_without_noop(module);
    assert_eq!(
      items.len(),
      6,
      "expected 6 items: forwardRef, react, runtime imports, sheet declaration, styled, and display name"
    );

    let ModuleItem::ModuleDecl(ModuleDecl::Import(forward_ref_import)) = items[0] else {
      panic!("expected forwardRef import");
    };
    assert_eq!(forward_ref_import.src.value.as_ref(), "react");

    let ModuleItem::ModuleDecl(ModuleDecl::Import(react_import)) = items[1] else {
      panic!("expected react import");
    };
    assert_eq!(react_import.src.value.as_ref(), "react");

    let ModuleItem::ModuleDecl(ModuleDecl::Import(runtime_import)) = items[2] else {
      panic!("expected runtime import");
    };
    assert_eq!(runtime_import.src.value.as_ref(), "@compiled/react/runtime");

    // item[3] should be the hoisted sheet declaration
    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(_sheet_decl))) = items[3] else {
      panic!("expected sheet variable declaration");
    };

    let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) = items[4] else {
      panic!("expected styled variable declaration");
    };
    assert_eq!(var_decl.decls.len(), 1);

    let ModuleItem::Stmt(Stmt::If(_)) = items[5] else {
      panic!("expected display name assignment");
    };
  }
}

fn imported_name(specifier: &ImportNamedSpecifier) -> &str {
  match &specifier.imported {
    Some(ModuleExportName::Ident(ident)) => ident.sym.as_ref(),
    Some(ModuleExportName::Str(value)) => value.value.as_ref(),
    None => specifier.local.sym.as_ref(),
  }
}

fn normalized_join(base: &Path, segment: &str) -> PathBuf {
  base.join(segment).components().collect()
}

fn pattern_contains_ident(pat: &Pat, name: &str) -> bool {
  match pat {
    Pat::Ident(binding) => binding.id.sym.as_ref() == name,
    Pat::Array(array) => array
      .elems
      .iter()
      .flatten()
      .any(|elem| pattern_contains_ident(elem, name)),
    Pat::Object(object) => object.props.iter().any(|prop| match prop {
      swc_core::ecma::ast::ObjectPatProp::Assign(assign) => assign.key.sym.as_ref() == name,
      swc_core::ecma::ast::ObjectPatProp::KeyValue(kv) => pattern_contains_ident(&kv.value, name),
      swc_core::ecma::ast::ObjectPatProp::Rest(rest) => pattern_contains_ident(&rest.arg, name),
    }),
    Pat::Assign(assign) => pattern_contains_ident(&assign.left, name),
    Pat::Rest(rest) => pattern_contains_ident(&rest.arg, name),
    Pat::Expr(expr) => {
      matches!(expr.as_ref(), Expr::Ident(ident) if ident.sym.as_ref() == name)
    }
    _ => false,
  }
}

fn module_has_binding(module: &Module, name: &str) -> bool {
  for item in &module.body {
    match item {
      ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
        if import.specifiers.iter().any(|specifier| match specifier {
          ImportSpecifier::Named(named) => named.local.sym.as_ref() == name,
          ImportSpecifier::Default(default) => default.local.sym.as_ref() == name,
          ImportSpecifier::Namespace(namespace) => namespace.local.sym.as_ref() == name,
        }) {
          return true;
        }
      }
      ModuleItem::Stmt(Stmt::Decl(decl)) => match decl {
        Decl::Var(var) => {
          if var
            .decls
            .iter()
            .any(|decl| pattern_contains_ident(&decl.name, name))
          {
            return true;
          }
        }
        Decl::Fn(FnDecl { ident, .. }) => {
          if ident.sym.as_ref() == name {
            return true;
          }
        }
        Decl::Class(ClassDecl { ident, .. }) => {
          if ident.sym.as_ref() == name {
            return true;
          }
        }
        _ => {}
      },
      _ => {}
    }
  }

  false
}

fn insert_react_import(module: &mut Module) {
  let import_decl = ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![ImportSpecifier::Namespace(ImportStarAsSpecifier {
      span: DUMMY_SP,
      local: Ident::new("React".into(), DUMMY_SP, SyntaxContext::empty()),
    })],
    src: Box::new(Str {
      span: DUMMY_SP,
      value: "react".into(),
      raw: None,
    }),
    type_only: false,
    with: None,
    phase: ImportPhase::Evaluation,
  };

  module
    .body
    .insert(0, ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)));
}

fn insert_forward_ref_import(module: &mut Module) {
  let import_decl = ImportDecl {
    span: DUMMY_SP,
    specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
      span: DUMMY_SP,
      local: Ident::new("forwardRef".into(), DUMMY_SP, SyntaxContext::empty()),
      // Match Babel emission: omit alias when local equals imported
      imported: None,
      is_type_only: false,
    })],
    src: Box::new(Str {
      span: DUMMY_SP,
      value: "react".into(),
      raw: None,
    }),
    type_only: false,
    with: None,
    phase: ImportPhase::Evaluation,
  };

  module
    .body
    .insert(0, ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)));
}

fn is_compiled_module(user_module: &str, state: &TransformState) -> bool {
  if state
    .import_sources
    .iter()
    .any(|origin| origin == user_module)
  {
    return true;
  }

  if !user_module.starts_with('.') {
    return false;
  }

  let Some(filename) = &state.filename else {
    return false;
  };

  let file_path = Path::new(filename);
  let base_dir = file_path.parent().unwrap_or_else(|| Path::new(""));
  let resolved = normalized_join(base_dir, user_module);

  state
    .import_sources
    .iter()
    .any(|origin| normalized_join(Path::new(""), origin) == resolved)
}

fn record_compiled_import(imports: &mut CompiledImports, name: &str, local: &str) -> bool {
  match name {
    "styled" => {
      imports.styled.push(local.to_string());
      true
    }
    "ClassNames" => {
      imports.class_names.push(local.to_string());
      true
    }
    "css" => {
      imports.css.push(local.to_string());
      true
    }
    "keyframes" => {
      imports.keyframes.push(local.to_string());
      true
    }
    "cssMap" => {
      imports.css_map.push(local.to_string());
      true
    }
    _ => false,
  }
}

fn has_active_compiled_imports(imports: &CompiledImports) -> bool {
  !(imports.class_names.is_empty()
    && imports.css.is_empty()
    && imports.keyframes.is_empty()
    && imports.styled.is_empty()
    && imports.css_map.is_empty())
}

static JSX_SOURCE_ANNOTATION_REGEX: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"\*?\s*@jsxImportSource\s+([^\s]+)").expect("jsx import source regex should compile")
});

static JSX_ANNOTATION_REGEX: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"\*?\s*@jsx\s+([^\s]+)").expect("jsx pragma regex should compile"));

impl VisitMut for CompiledCssInJsTransform {
  noop_visit_mut_type!();

  fn visit_mut_program(&mut self, program: &mut Program) {
    match program {
      Program::Module(module) => self.transform_module_program(module),
      Program::Script(script) => {
        let body = std::mem::take(&mut script.body);
        let shebang = script.shebang.take();
        let mut module = Module {
          span: script.span,
          body: body.into_iter().map(ModuleItem::Stmt).collect(),
          shebang,
        };

        self.transform_module_program(&mut module);
        *program = Program::Module(module);
      }
    }
  }
}

impl CompiledCssInJsTransform {
  fn transform_module_program(&mut self, module: &mut Module) {
    self.remove_jsx_imports(module);
    utils_module_scope::populate_module_scope(&self.state(), module);
    self.process_jsx_pragmas();
    self.visit_mut_module(module);

    let (should_import_react, has_styled_import) = {
      let mut state = self.state.borrow_mut();
      let has_compiled_imports = state
        .compiled_imports
        .as_ref()
        .map(has_active_compiled_imports)
        .unwrap_or(false);
      let should_append_runtime =
        has_compiled_imports || state.uses_xcss || state.uses_runtime_wrappers;

      if std::env::var("COMPILED_CLI_TRACE").is_ok() {
        eprintln!(
          "[transform] decision file={} has_compiled_imports={} uses_xcss={} uses_runtime_wrappers={} should_append_runtime={}",
          state.filename.clone().unwrap_or_default(),
          has_compiled_imports,
          state.uses_xcss,
          state.uses_runtime_wrappers,
          should_append_runtime
        );
      }

      if should_append_runtime {
        preserve_leading_comments(&module.body, &mut state);
        append_runtime_imports(module, &mut state);
      }

      // Always insert sheet declarations when sheets have been hoisted (e.g., via css prop),
      // even if extract mode is disabled. The runtime component needs these declarations and strip
      // babel plugin will handle the extraction.
      if !state.sheets.is_empty() {
        insert_sheet_declarations(module, &mut state);
      }

      let has_styled_import = state
        .compiled_imports
        .as_ref()
        .map(|imports| !imports.styled.is_empty())
        .unwrap_or(false);

      let should_import_react =
        should_append_runtime && (state.pragma.jsx || state.opts.import_react.unwrap_or(true));

      (should_import_react, has_styled_import)
    };

    if should_import_react && !module_has_binding(module, "React") {
      insert_react_import(module);
    }

    if has_styled_import && !module_has_binding(module, "forwardRef") {
      insert_forward_ref_import(module);
    }

    self.maybe_insert_generated_comment_for_module(module);
  }

  fn generated_comment_text(&self) -> Option<String> {
    let (filename, version) = {
      let state = self.state.borrow();

      if state.compiled_imports.is_none() && !state.uses_xcss && !state.uses_runtime_wrappers {
        return None;
      }

      let filename = state
        .filename
        .as_deref()
        .and_then(|value| Path::new(value).file_name())
        .and_then(|name| name.to_str())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| "File".to_string());

      let version = env::var("TEST_PKG_VERSION")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

      (filename, version)
    };

    Some(format!(
      " {filename} generated by {PACKAGE_NAME} v{version} "
    ))
  }

  fn push_generated_comment(&mut self, comment_text: String) {
    let mut state = self.state.borrow_mut();
    state.file.comments.insert(
      0,
      Comment {
        kind: CommentKind::Block,
        span: DUMMY_SP,
        text: comment_text.into(),
      },
    );
  }

  fn maybe_insert_generated_comment_for_module(&mut self, module: &mut Module) {
    let Some(comment) = self.generated_comment_text() else {
      return;
    };

    module.body.insert(
      0,
      ModuleItem::Stmt(Stmt::Empty(EmptyStmt { span: DUMMY_SP })),
    );
    self.push_generated_comment(comment);
  }

  fn remove_jsx_imports(&mut self, module: &mut Module) {
    let mut state = self.state.borrow_mut();

    let mut index = 0;
    let mut needs_react_jsx_import = false;
    while index < module.body.len() {
      let ModuleItem::ModuleDecl(ModuleDecl::Import(import)) = &mut module.body[index] else {
        index += 1;
        continue;
      };

      if !is_compiled_module(import.src.value.as_ref(), &state) {
        index += 1;
        continue;
      }

      let mut remaining = Vec::with_capacity(import.specifiers.len());
      let mut removed_jsx = false;

      for specifier in import.specifiers.drain(..) {
        match specifier {
          ImportSpecifier::Named(named) => {
            if imported_name(&named) == "jsx" {
              state.pragma.classic_jsx_pragma_is_compiled = true;
              state.pragma.classic_jsx_pragma_local_name = Some(named.local.sym.to_string());
              removed_jsx = true;
              needs_react_jsx_import = true;
              continue;
            }

            remaining.push(ImportSpecifier::Named(named));
          }
          other => remaining.push(other),
        }
      }

      if removed_jsx {
        state
          .compiled_imports
          .get_or_insert_with(CompiledImports::default);
      }

      import.specifiers = remaining;

      if import.specifiers.is_empty() {
        module.body.remove(index);
        continue;
      }

      index += 1;
    }

    // Check if we should import React based on the import_react option
    let should_import_react = state.opts.import_react.unwrap_or(true);

    // Drop the mutable borrow before inserting imports
    drop(state);

    // Add jsx import from react if we removed it from @compiled/react
    if needs_react_jsx_import && !module_has_binding(module, "jsx") {
      let import_decl = ImportDecl {
        span: DUMMY_SP,
        specifiers: vec![ImportSpecifier::Named(ImportNamedSpecifier {
          span: DUMMY_SP,
          local: Ident::new("jsx".into(), DUMMY_SP, SyntaxContext::empty()),
          imported: None,
          is_type_only: false,
        })],
        src: Box::new(Str {
          span: DUMMY_SP,
          value: "react/jsx-runtime".into(),
          raw: None,
        }),
        type_only: false,
        with: None,
        phase: ImportPhase::Evaluation,
      };

      module
        .body
        .insert(0, ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)));

      // Only add React import if it doesn't already exist and import_react option allows it
      if should_import_react && !module_has_binding(module, "React") {
        let react_import_decl = ImportDecl {
          span: DUMMY_SP,
          specifiers: vec![ImportSpecifier::Namespace(ImportStarAsSpecifier {
            span: DUMMY_SP,
            local: Ident::new("React".into(), DUMMY_SP, SyntaxContext::empty()),
          })],
          src: Box::new(Str {
            span: DUMMY_SP,
            value: "react".into(),
            raw: None,
          }),
          type_only: false,
          with: None,
          phase: ImportPhase::Evaluation,
        };

        module.body.insert(
          0,
          ModuleItem::ModuleDecl(ModuleDecl::Import(react_import_decl)),
        );
      }
    }
  }

  fn process_jsx_pragmas(&mut self) {
    let mut state = self.state.borrow_mut();

    if state.file.comments.is_empty() {
      return;
    }

    let mut matched_index: Option<usize> = None;
    let comments = state.file.comments.clone();

    for (idx, comment) in comments.iter().enumerate() {
      let text = comment.text.as_ref();

      if let Some(captures) = JSX_SOURCE_ANNOTATION_REGEX.captures(text) {
        let origin = captures.get(1).map(|m| m.as_str()).unwrap_or("");
        if state.import_sources.iter().any(|source| source == origin) {
          state
            .compiled_imports
            .get_or_insert_with(CompiledImports::default);
          state.pragma.jsx_import_source = true;
          matched_index = Some(idx);
        }
      }

      if state.pragma.classic_jsx_pragma_is_compiled {
        if let Some(captures) = JSX_ANNOTATION_REGEX.captures(text) {
          let Some(local_name) = &state.pragma.classic_jsx_pragma_local_name else {
            continue;
          };

          let matched = captures.get(1).map(|m| m.as_str()).unwrap_or("");
          if matched == local_name {
            state
              .compiled_imports
              .get_or_insert_with(CompiledImports::default);
            state.pragma.jsx = true;
            matched_index = Some(idx);
          }
        }
      }
    }

    if let Some(index) = matched_index {
      state.file.comments.remove(index);
    }
  }

  fn visit_mut_module(&mut self, module: &mut Module) {
    let mut index = 0;

    while index < module.body.len() {
      let keep = match &mut module.body[index] {
        ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => self.visit_mut_import_decl(import),
        item => {
          item.visit_mut_with(self);
          true
        }
      };

      if keep {
        index += 1;
      } else {
        module.body.remove(index);
      }
    }

    let (
      css_prop_enabled,
      has_css_import,
      has_styled_import,
      has_class_names_import,
      has_css_map_import,
      has_keyframes_import,
      process_xcss,
    ) = {
      let state = self.state.borrow();
      let imports = state.compiled_imports.clone();
      (
        imports.is_some(),
        imports
          .as_ref()
          .map(|imports| !imports.css.is_empty())
          .unwrap_or(false),
        imports
          .as_ref()
          .map(|imports| !imports.styled.is_empty())
          .unwrap_or(false),
        imports
          .as_ref()
          .map(|imports| !imports.class_names.is_empty())
          .unwrap_or(false),
        imports
          .as_ref()
          .map(|imports| !imports.css_map.is_empty())
          .unwrap_or(false),
        imports
          .as_ref()
          .map(|imports| !imports.keyframes.is_empty())
          .unwrap_or(false),
        state.opts.process_xcss.unwrap_or(true),
      )
    };

    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      let state = self.state.borrow();
      let styles_len = state.style_rules.len();
      let sheets_len = state.sheets.len();
      let generated_comment = state.file.comments.first().map(|c| c.text.to_string());
      let has_css_alias = state.imported_compiled_imports.css.is_some();
      let current_file = state.filename.clone().unwrap_or_default();
      let uses_wrappers = state.uses_runtime_wrappers;
      drop(state);

      eprintln!(
        "[transform] imports file={} css_prop_enabled={} css={} styled={} classNames={} cssMap={} keyframes={} process_xcss={} uses_runtime_wrappers={} style_rules={} sheets={} comment_present={} css_alias_present={}",
        current_file,
        css_prop_enabled,
        has_css_import,
        has_styled_import,
        has_class_names_import,
        has_css_map_import,
        has_keyframes_import,
        process_xcss,
        uses_wrappers,
        styles_len,
        sheets_len,
        generated_comment.is_some(),
        has_css_alias
      );
    }

    {
      let metadata = Metadata::new(self.state());
      let mut guard = JsxFunctionGuard::new(metadata);
      module.visit_mut_with(&mut guard);
    }

    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      let state = self.state.borrow();
      eprintln!(
        "[transform] post-guard style_rules={} sheets={} uses_runtime_wrappers={} css_imports_present={}",
        state.style_rules.len(),
        state.sheets.len(),
        state.uses_runtime_wrappers,
        state
          .compiled_imports
          .as_ref()
          .map(|imports| !imports.css.is_empty())
          .unwrap_or(false)
      );
    }

    if has_styled_import {
      let metadata = Metadata::new(self.state());
      if std::env::var("COMPILED_CLI_TRACE").is_ok() {
        eprintln!("[transform] StyledVisitor active");
      }
      let mut visitor = StyledVisitor::new(metadata);
      module.visit_mut_with(&mut visitor);
      visitor.insert_display_names(module);
    }

    if has_css_map_import {
      let metadata = Metadata::new(self.state());
      let mut visitor = CssMapVisitor::new(metadata);
      module.visit_mut_with(&mut visitor);
    }

    if css_prop_enabled {
      let metadata = Metadata::new(self.state());
      if std::env::var("COMPILED_CLI_TRACE").is_ok() {
        eprintln!("[transform] CssPropVisitor active");
      }
      let mut visitor = CssPropVisitor::new(metadata);
      module.visit_mut_with(&mut visitor);
    }

    if has_class_names_import {
      let metadata = Metadata::new(self.state());
      let mut visitor = ClassNamesVisitor::new(metadata);
      module.visit_mut_with(&mut visitor);
    }

    if process_xcss {
      let metadata = Metadata::new(self.state());
      let mut visitor = XcssVisitor::new(metadata);
      module.visit_mut_with(&mut visitor);
    }

    if has_css_import || has_keyframes_import {
      let metadata = Metadata::new(self.state());
      {
        let mut recorder = CompiledUtilRecorder::new(metadata.clone());
        module.visit_mut_with(&mut recorder);
      }

      let actions = {
        let mut state = self.state.borrow_mut();
        std::mem::take(&mut state.paths_to_cleanup)
      };

      if !actions.is_empty() {
        let mut applier = CleanupApplier::new(actions);
        module.visit_mut_with(&mut applier);
      }
    }
  }

  fn visit_mut_import_decl(&mut self, import: &mut ImportDecl) -> bool {
    let module_name = import.src.value.to_string();

    let mut state = self.state.borrow_mut();
    if !is_compiled_module(&module_name, &state) {
      return true;
    }

    let mut remaining = Vec::with_capacity(import.specifiers.len());
    let mut css_alias: Option<String> = None;

    {
      let compiled_imports = state
        .compiled_imports
        .get_or_insert_with(CompiledImports::default);

      for specifier in import.specifiers.drain(..) {
        match specifier {
          ImportSpecifier::Named(named) => {
            let imported = imported_name(&named).to_string();
            let local = named.local.sym.to_string();

            if record_compiled_import(compiled_imports, &imported, &local) {
              if imported == "css" && css_alias.is_none() {
                css_alias = Some(local);
              }
              continue;
            }

            remaining.push(ImportSpecifier::Named(named));
          }
          other => {
            remaining.push(other);
          }
        }
      }
    }

    import.specifiers = remaining;

    if let Some(alias) = css_alias {
      state.imported_compiled_imports.css = Some(alias);
    }

    !import.specifiers.is_empty()
  }
}

struct CssPropVisitor {
  meta: Metadata,
  scope_stack: Vec<SharedScope>,
  recursion_depth: usize,
  max_recursion_depth: usize,
}

impl CssPropVisitor {
  fn new(meta: Metadata) -> Self {
    let parent_scope = meta.parent_scope();
    Self {
      meta,
      scope_stack: vec![parent_scope],
      recursion_depth: 0,
      max_recursion_depth: 500, // Prevent stack overflow from infinite recursion
    }
  }

  fn current_scope(&self) -> SharedScope {
    self
      .scope_stack
      .last()
      .cloned()
      .unwrap_or_else(|| self.meta.parent_scope())
  }

  fn push_scope(&mut self) {
    let scope = self.meta.allocate_own_scope();
    self.scope_stack.push(scope);
  }

  fn pop_scope(&mut self) {
    self.scope_stack.pop();
  }

  fn prop_name_to_string(name: &swc_core::ecma::ast::PropName) -> Option<String> {
    use swc_core::ecma::ast::PropName;

    match name {
      PropName::Ident(ident) => Some(ident.sym.to_string()),
      PropName::Str(value) => Some(value.value.to_string()),
      PropName::Num(value) => Some(value.value.to_string()),
      PropName::BigInt(value) => Some(value.value.to_string()),
      PropName::Computed(_) => None,
    }
  }

  fn insert_binding(
    &mut self,
    name: &str,
    init: Option<Expr>,
    path: Option<BindingPath>,
    constant: bool,
  ) {
    // Use the merged scope so bindings retain visibility of outer scopes (mirrors Babel's
    // scope chain resolution when evaluating identifiers).
    let binding_meta = self.meta.with_parent_scope(self.merged_scope());
    let binding =
      PartialBindingWithMeta::new(init, path, constant, binding_meta, BindingSource::Module);

    self
      .current_scope()
      .borrow_mut()
      .insert(name.to_string(), binding);
  }

  fn register_pattern(
    &mut self,
    pattern: &Pat,
    init: Option<&Expr>,
    constant: bool,
    path: Vec<String>,
    default_value: Option<Expr>,
    span: Option<Span>,
  ) {
    match pattern {
      Pat::Ident(binding) => {
        let name = binding.id.sym.as_ref();
        let span = span.unwrap_or(binding.id.span);
        // Always store variable bindings with the `Variable` path kind (even when there is
        // no nested destructuring) to mirror Babel's metadata and allow downstream helpers
        // to apply variable-specific compatibility logic.
        let binding_path = Some(BindingPath::variable(span.into(), path, default_value));

        self.insert_binding(name, init.cloned(), binding_path, constant);
      }
      Pat::Assign(assign) => {
        let assign_default = Some(*assign.right.clone()).or(default_value);

        self.register_pattern(
          &assign.left,
          init,
          constant,
          path,
          assign_default,
          Some(assign.span),
        );
      }
      Pat::Object(object) => {
        for prop in &object.props {
          match prop {
            swc_core::ecma::ast::ObjectPatProp::KeyValue(key_value) => {
              if let Some(key) = Self::prop_name_to_string(&key_value.key) {
                let mut next_path = path.clone();
                next_path.push(key);

                self.register_pattern(
                  &key_value.value,
                  init,
                  constant,
                  next_path,
                  None,
                  Some(key_value.value.span()),
                );
              }
            }
            swc_core::ecma::ast::ObjectPatProp::Assign(assign) => {
              let mut next_path = path.clone();
              next_path.push(assign.key.sym.to_string());
              let default_expr = assign.value.as_ref().map(|expr| expr.as_ref().clone());
              let span = Some(assign.key.span);
              let binding_path = BindingPath::variable(span.into(), next_path, default_expr);

              self.insert_binding(
                assign.key.sym.as_ref(),
                init.cloned(),
                Some(binding_path),
                constant,
              );
            }
            swc_core::ecma::ast::ObjectPatProp::Rest(rest) => {
              self.register_pattern(&rest.arg, init, constant, Vec::new(), None, Some(rest.span));
            }
          }
        }
      }
      _ => {}
    }
  }

  fn register_var_decl(&mut self, decl: &VarDecl) {
    let constant = matches!(decl.kind, VarDeclKind::Const);

    for declarator in &decl.decls {
      let init = declarator.init.as_deref();
      self.register_pattern(
        &declarator.name,
        init,
        constant,
        Vec::new(),
        None,
        Some(declarator.span),
      );
    }
  }

  fn merged_scope(&self) -> SharedScope {
    let merged = self.meta.allocate_own_scope();
    {
      let mut out = merged.borrow_mut();
      for scope in &self.scope_stack {
        for (key, value) in scope.borrow().iter() {
          out.insert(key.clone(), value.clone());
        }
      }
    }
    merged
  }

  fn scoped_meta(&self) -> Metadata {
    self.meta.with_parent_scope(self.merged_scope())
  }
}

impl VisitMut for CssPropVisitor {
  noop_visit_mut_type!();

  fn visit_mut_var_decl(&mut self, decl: &mut VarDecl) {
    self.register_var_decl(decl);
    decl.visit_mut_children_with(self);
  }

  fn visit_mut_function(&mut self, function: &mut swc_core::ecma::ast::Function) {
    self.push_scope();
    function.visit_mut_children_with(self);
    self.pop_scope();
  }

  fn visit_mut_arrow_expr(&mut self, expr: &mut swc_core::ecma::ast::ArrowExpr) {
    self.push_scope();
    expr.visit_mut_children_with(self);
    self.pop_scope();
  }

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    // Check recursion depth to prevent stack overflow from infinite loops
    if self.recursion_depth > self.max_recursion_depth {
      let error = build_code_frame_error(
        "AST traversal depth exceeded 500 levels - possible infinite recursion in visitor",
        Some(expr.span()),
        &self.meta,
      );
      panic!("{error}");
    }

    if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
      eprintln!(
        "[CssPropVisitor.visit_mut_expr] START depth={}",
        self.recursion_depth
      );
    }

    self.recursion_depth += 1;
    expr.visit_mut_children_with(self);
    self.recursion_depth -= 1;

    if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
      eprintln!("[CssPropVisitor.visit_mut_expr] children visited");
    }

    if matches!(expr, Expr::JSXElement(_)) {
      if std::env::var("COMPILED_CLI_TRACE").is_ok() {
        let state = self.meta.state();
        eprintln!(
          "[css-prop] state css_map keys={:?}",
          state.css_map.keys().collect::<Vec<&String>>()
        );
      }
      if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
        eprintln!("[CssPropVisitor.visit_mut_expr] calling visit_css_prop");
      }
      let meta = self
        .scoped_meta()
        .with_parent_expr(Some(expr))
        .with_own_span(Some(expr.span()));
      visit_css_prop(expr, &meta);
      if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
        eprintln!("[CssPropVisitor.visit_mut_expr] visit_css_prop completed");
      }
    }

    if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
      eprintln!("[CssPropVisitor.visit_mut_expr] END");
    }
  }

  fn visit_mut_jsx_element(&mut self, element: &mut swc_core::ecma::ast::JSXElement) {
    // Check recursion depth to prevent stack overflow from infinite loops
    if self.recursion_depth > self.max_recursion_depth {
      let error = build_code_frame_error(
        "JSX element nesting depth exceeded 500 levels - possible infinite recursion in visitor",
        Some(element.opening.span),
        &self.meta,
      );
      panic!("{error}");
    }

    if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
      use swc_core::ecma::ast::JSXElementName;
      let name = match &element.opening.name {
        JSXElementName::Ident(id) => id.sym.as_ref().to_string(),
        _ => String::from("<complex>"),
      };
      eprintln!(
        "[CssPropVisitor.visit_mut_jsx_element] START element={} depth={}",
        name, self.recursion_depth
      );
    }

    // First traverse children so nested elements are handled depth-first
    self.recursion_depth += 1;
    if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
      eprintln!("[CssPropVisitor.visit_mut_jsx_element] visiting children");
    }
    element.visit_mut_children_with(self);
    self.recursion_depth -= 1;
    if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
      eprintln!("[CssPropVisitor.visit_mut_jsx_element] children done");
    }

    // Then attempt css prop transform on this element directly
    if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
      eprintln!("[CssPropVisitor.visit_mut_jsx_element] calling visit_css_prop_on_element");
    }
    let meta = self.scoped_meta();
    crate::css_prop::visit_css_prop_on_element(element, &meta);
    if std::env::var("DEBUG_VISITOR_TRACE").is_ok() {
      eprintln!("[CssPropVisitor.visit_mut_jsx_element] visit_css_prop_on_element done");
    }
  }
}

struct JsxFunctionGuard {
  meta: Metadata,
}

impl JsxFunctionGuard {
  fn new(meta: Metadata) -> Self {
    Self { meta }
  }

  fn handle_expr(&self, expr: &Expr, meta: &Metadata) {
    let is_jsx = {
      let state = meta.state();
      is_transformed_jsx_function(expr, &state)
    };

    if !is_jsx {
      return;
    }

    let message = format!(
      "Found a `jsx` function call in the Babel output where one should not have been generated. Was Compiled not set up correctly?\n\nReasons this might happen:\n\n[Likely] Importing `jsx` from a library other than Compiled CSS-in-JS - please only import from `{COMPILED_IMPORT}`.\n\n[Less likely] If you are using `@babel/preset-react` (or `@babel/plugin-transform-react-jsx`) in your Babel configuration, and you are using `runtime: classic`, make sure you do not use the `pragma` option. Please use the /** @jsx jsx */ syntax instead, or switch to `runtime: automatic`"
    );
    let meta_with_span = meta.with_own_span(Some(expr.span()));
    let error = build_code_frame_error(&message, Some(expr.span()), &meta_with_span);
    panic!("{error}");
  }
}

impl VisitMut for JsxFunctionGuard {
  noop_visit_mut_type!();

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    let previous_meta = self.meta.clone();
    let child_meta = previous_meta
      .with_parent_span(Some(expr.span()))
      .with_parent_expr(Some(expr));

    self.meta = child_meta;
    expr.visit_mut_children_with(self);

    self.meta = previous_meta.clone();
    self.handle_expr(expr, &previous_meta);

    self.meta = previous_meta;
  }
}

struct StyledVisitor {
  meta: Metadata,
  display_names: Vec<DisplayNameInsertion>,
}

impl StyledVisitor {
  fn new(meta: Metadata) -> Self {
    Self {
      meta,
      display_names: Vec::new(),
    }
  }

  fn process_expr(&mut self, expr: &mut Expr, variable_name: Option<&str>) -> StyledVisitResult {
    let should_normalize = {
      let state = self.meta.state();
      is_compiled_styled_call_expression(expr, &state)
        || is_compiled_styled_tagged_template_expression(expr, &state)
    };

    if should_normalize {
      normalize_props_usage(expr);
    }

    expr.visit_mut_children_with(self);

    let meta = self
      .meta
      .with_parent_expr(Some(expr))
      .with_own_span(Some(expr.span()));
    visit_styled(expr, &meta, variable_name)
  }

  fn insert_display_names(&mut self, module: &mut Module) {
    if self.display_names.is_empty() {
      return;
    }

    let mut insertions = std::mem::take(&mut self.display_names);
    insert_display_names_in_items(&mut module.body, &mut insertions);
  }
}

impl VisitMut for StyledVisitor {
  noop_visit_mut_type!();

  fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
    declarator.name.visit_mut_with(self);

    let Some(init) = declarator.init.as_mut() else {
      return;
    };

    let variable_name = match &declarator.name {
      Pat::Ident(binding) => Some(binding.id.sym.as_ref()),
      _ => None,
    };

    let result = self.process_expr(init, variable_name);

    if let Some(display_name) = result.display_name {
      self.display_names.push(DisplayNameInsertion {
        declarator_span: declarator.span,
        stmt: display_name,
      });
    }
  }

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    self.process_expr(expr, None);
  }
}

struct ClassNamesVisitor {
  meta: Metadata,
  scope_stack: Vec<SharedScope>,
}

impl ClassNamesVisitor {
  fn new(meta: Metadata) -> Self {
    let parent_scope = meta.parent_scope();
    Self {
      meta,
      scope_stack: vec![parent_scope],
    }
  }

  fn current_scope(&self) -> SharedScope {
    self
      .scope_stack
      .last()
      .cloned()
      .unwrap_or_else(|| self.meta.parent_scope())
  }

  fn push_scope(&mut self) {
    let scope = self.meta.allocate_own_scope();
    self.scope_stack.push(scope);
  }

  fn pop_scope(&mut self) {
    self.scope_stack.pop();
  }

  fn prop_name_to_string(name: &swc_core::ecma::ast::PropName) -> Option<String> {
    use swc_core::ecma::ast::PropName;

    match name {
      PropName::Ident(ident) => Some(ident.sym.to_string()),
      PropName::Str(value) => Some(value.value.to_string()),
      PropName::Num(value) => Some(value.value.to_string()),
      PropName::BigInt(value) => Some(value.value.to_string()),
      PropName::Computed(_) => None,
    }
  }

  fn insert_binding(
    &mut self,
    name: &str,
    init: Option<Expr>,
    path: Option<BindingPath>,
    constant: bool,
  ) {
    let binding_meta = self.meta.with_parent_scope(self.current_scope());
    let binding =
      PartialBindingWithMeta::new(init, path, constant, binding_meta, BindingSource::Module);

    self
      .current_scope()
      .borrow_mut()
      .insert(name.to_string(), binding);
  }

  fn register_pattern(
    &mut self,
    pattern: &Pat,
    init: Option<&Expr>,
    constant: bool,
    path: Vec<String>,
    default_value: Option<Expr>,
    span: Option<Span>,
  ) {
    match pattern {
      Pat::Ident(binding) => {
        let name = binding.id.sym.as_ref();
        let span = span.unwrap_or(binding.id.span);
        let binding_path = if !path.is_empty() || default_value.is_some() {
          Some(BindingPath::variable(span.into(), path, default_value))
        } else {
          Some(BindingPath::new(span.into()))
        };

        self.insert_binding(name, init.cloned(), binding_path, constant);
      }
      Pat::Assign(assign) => {
        let assign_default = Some(*assign.right.clone()).or(default_value);

        self.register_pattern(
          &assign.left,
          init,
          constant,
          path,
          assign_default,
          Some(assign.span),
        );
      }
      Pat::Object(object) => {
        for prop in &object.props {
          match prop {
            swc_core::ecma::ast::ObjectPatProp::KeyValue(key_value) => {
              if let Some(key) = Self::prop_name_to_string(&key_value.key) {
                let mut next_path = path.clone();
                next_path.push(key);

                self.register_pattern(
                  &key_value.value,
                  init,
                  constant,
                  next_path,
                  None,
                  Some(key_value.value.span()),
                );
              }
            }
            swc_core::ecma::ast::ObjectPatProp::Assign(assign) => {
              let mut next_path = path.clone();
              next_path.push(assign.key.sym.to_string());
              let default_expr = assign.value.as_ref().map(|expr| expr.as_ref().clone());
              let span = Some(assign.key.span);
              let binding_path = BindingPath::variable(span.into(), next_path, default_expr);

              self.insert_binding(
                assign.key.sym.as_ref(),
                init.cloned(),
                Some(binding_path),
                constant,
              );
            }
            swc_core::ecma::ast::ObjectPatProp::Rest(rest) => {
              self.register_pattern(&rest.arg, init, constant, Vec::new(), None, Some(rest.span));
            }
          }
        }
      }
      _ => {}
    }
  }

  fn register_var_decl(&mut self, decl: &VarDecl) {
    let constant = matches!(decl.kind, VarDeclKind::Const);

    for declarator in &decl.decls {
      let init = declarator.init.as_deref();
      self.register_pattern(
        &declarator.name,
        init,
        constant,
        Vec::new(),
        None,
        Some(declarator.span),
      );
    }
  }

  fn merged_scope(&self) -> SharedScope {
    let merged = self.meta.allocate_own_scope();
    {
      let mut out = merged.borrow_mut();
      for scope in &self.scope_stack {
        for (key, value) in scope.borrow().iter() {
          out.insert(key.clone(), value.clone());
        }
      }
    }
    merged
  }

  fn scoped_meta(&self) -> Metadata {
    self.meta.with_parent_scope(self.merged_scope())
  }
}

impl VisitMut for ClassNamesVisitor {
  noop_visit_mut_type!();

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    expr.visit_mut_children_with(self);

    if matches!(expr, Expr::JSXElement(_)) {
      let meta = self
        .scoped_meta()
        .with_parent_expr(Some(expr))
        .with_own_span(Some(expr.span()));
      visit_class_names(expr, &meta);
    }
  }

  fn visit_mut_function(&mut self, function: &mut swc_core::ecma::ast::Function) {
    self.push_scope();
    function.visit_mut_children_with(self);
    self.pop_scope();
  }

  fn visit_mut_arrow_expr(&mut self, expr: &mut swc_core::ecma::ast::ArrowExpr) {
    self.push_scope();
    expr.visit_mut_children_with(self);
    self.pop_scope();
  }

  fn visit_mut_var_decl(&mut self, decl: &mut VarDecl) {
    self.register_var_decl(decl);
    decl.visit_mut_children_with(self);
  }
}

struct CssMapVisitor {
  meta: Metadata,
}

impl CssMapVisitor {
  fn new(meta: Metadata) -> Self {
    Self { meta }
  }

  fn is_css_map_ident(&self, ident: &Ident) -> bool {
    self
      .meta
      .state()
      .compiled_imports
      .as_ref()
      .map(|imports| {
        imports
          .css_map
          .iter()
          .any(|name| name == ident.sym.as_ref())
      })
      .unwrap_or(false)
  }
}

impl VisitMut for CssMapVisitor {
  noop_visit_mut_type!();

  fn visit_mut_var_declarator(&mut self, declarator: &mut VarDeclarator) {
    declarator.name.visit_mut_with(self);

    let Some(init) = declarator.init.as_mut() else {
      return;
    };

    let Some(binding_ident) = (match &declarator.name {
      Pat::Ident(binding) => Some(binding.id.clone()),
      _ => None,
    }) else {
      init.visit_mut_with(self);
      return;
    };

    match init.as_mut() {
      Expr::Call(call) => {
        let Some(callee_ident) = (match &call.callee {
          swc_core::ecma::ast::Callee::Expr(expr) => match expr.as_ref() {
            Expr::Ident(ident) => Some(ident.clone()),
            _ => None,
          },
          _ => None,
        }) else {
          call.visit_mut_children_with(self);
          return;
        };

        if !self.is_css_map_ident(&callee_ident) {
          call.visit_mut_children_with(self);
          return;
        }

        let call_expr = call.clone();
        let init_expr = Expr::Call(call_expr.clone());
        let init_span = call_expr.span;

        let meta = self
          .meta
          .with_parent_expr(Some(&init_expr))
          .with_parent_span(Some(declarator.span))
          .with_own_span(Some(init_span));

        let object = visit_css_map_path(CssMapUsage::Call(&call_expr), Some(&binding_ident), &meta);

        *init = Expr::Object(object).into();
      }
      Expr::TaggedTpl(tagged) => {
        if let Expr::Ident(tag_ident) = &*tagged.tag {
          if self.is_css_map_ident(tag_ident) {
            let tagged_tpl = tagged.clone();
            let init_expr = Expr::TaggedTpl(tagged_tpl.clone());
            let init_span = tagged_tpl.span;
            let meta = self
              .meta
              .with_parent_expr(Some(&init_expr))
              .with_parent_span(Some(declarator.span))
              .with_own_span(Some(init_span));

            let object = visit_css_map_path(
              CssMapUsage::TaggedTemplate(&tagged_tpl),
              Some(&binding_ident),
              &meta,
            );

            *init = Expr::Object(object).into();
            return;
          }
        }

        tagged.visit_mut_children_with(self);
      }
      _ => {
        init.visit_mut_with(self);
      }
    }
  }
}

struct XcssVisitor {
  meta: Metadata,
}

impl XcssVisitor {
  fn new(meta: Metadata) -> Self {
    Self { meta }
  }
}

impl VisitMut for XcssVisitor {
  noop_visit_mut_type!();

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    expr.visit_mut_children_with(self);

    if matches!(expr, Expr::JSXElement(_)) {
      let meta = self
        .meta
        .with_parent_expr(Some(expr))
        .with_own_span(Some(expr.span()));
      visit_xcss_prop(expr, &meta);
    }
  }

  fn visit_mut_jsx_element(&mut self, element: &mut swc_core::ecma::ast::JSXElement) {
    // Walk children first so nested nodes are handled depth-first.
    element.visit_mut_children_with(self);

    let meta = self.meta.clone();
    visit_xcss_prop_on_element(element, &meta);
  }
}

struct CompiledUtilRecorder {
  meta: Metadata,
}

impl CompiledUtilRecorder {
  fn new(meta: Metadata) -> Self {
    Self { meta }
  }

  fn should_record(&self, expr: &Expr) -> bool {
    let state = self.meta.state.borrow();

    is_compiled_css_call_expression(expr, &state)
      || is_compiled_css_tagged_template_expression(expr, &state)
      || is_compiled_keyframes_call_expression(expr, &state)
      || is_compiled_keyframes_tagged_template_expression(expr, &state)
  }
}

impl VisitMut for CompiledUtilRecorder {
  noop_visit_mut_type!();

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    expr.visit_mut_children_with(self);

    if self.should_record(expr) {
      let span = expr.span();
      let mut state = self.meta.state_mut();
      state.enqueue_cleanup(CleanupAction::Replace, span);
    }
  }
}

struct CleanupApplier {
  actions: Vec<PathCleanup>,
}

impl CleanupApplier {
  fn new(actions: Vec<PathCleanup>) -> Self {
    Self { actions }
  }

  fn take_action(&mut self, span: Span) -> Option<CleanupAction> {
    if let Some(index) = self.actions.iter().position(|entry| entry.span == span) {
      let entry = self.actions.remove(index);
      Some(entry.action)
    } else {
      None
    }
  }
}

impl VisitMut for CleanupApplier {
  noop_visit_mut_type!();

  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    expr.visit_mut_children_with(self);

    if let Some(action) = self.take_action(expr.span()) {
      match action {
        CleanupAction::Replace => {
          let span = expr.span();
          *expr = Expr::Lit(Lit::Null(Null { span }));
        }
        CleanupAction::Remove => {}
      }
    }
  }
}

#[derive(Clone)]
struct DisplayNameInsertion {
  declarator_span: Span,
  stmt: Stmt,
}

fn insert_display_names_in_items(
  items: &mut Vec<ModuleItem>,
  insertions: &mut Vec<DisplayNameInsertion>,
) {
  let mut index = 0;

  while index < items.len() {
    let extras = match &mut items[index] {
      ModuleItem::Stmt(stmt) => insert_display_names_in_stmt(stmt, insertions)
        .into_iter()
        .map(ModuleItem::Stmt)
        .collect::<Vec<_>>(),
      ModuleItem::ModuleDecl(decl) => insert_display_names_in_module_decl(decl, insertions),
    };

    if !extras.is_empty() {
      let count = extras.len();
      items.splice(index + 1..index + 1, extras);
      index += 1 + count;
    } else {
      index += 1;
    }
  }
}

fn insert_display_names_in_module_decl(
  decl: &mut ModuleDecl,
  insertions: &mut Vec<DisplayNameInsertion>,
) -> Vec<ModuleItem> {
  match decl {
    ModuleDecl::ExportDecl(export_decl) => match &mut export_decl.decl {
      Decl::Var(var_decl) => collect_display_names_from_var_decl(var_decl, insertions)
        .into_iter()
        .map(ModuleItem::Stmt)
        .collect(),
      _ => Vec::new(),
    },
    ModuleDecl::ExportDefaultDecl(default_decl) => {
      if let DefaultDecl::Fn(function) = &mut default_decl.decl {
        if let Some(body) = function.function.body.as_mut() {
          insert_display_names_in_stmts(&mut body.stmts, insertions);
        }
      }

      Vec::new()
    }
    _ => Vec::new(),
  }
}

fn insert_display_names_in_stmts(
  stmts: &mut Vec<Stmt>,
  insertions: &mut Vec<DisplayNameInsertion>,
) {
  let mut index = 0;

  while index < stmts.len() {
    let extras = insert_display_names_in_stmt(&mut stmts[index], insertions);

    if !extras.is_empty() {
      let count = extras.len();
      stmts.splice(index + 1..index + 1, extras);
      index += 1 + count;
    } else {
      index += 1;
    }
  }
}

fn insert_display_names_in_stmt(
  stmt: &mut Stmt,
  insertions: &mut Vec<DisplayNameInsertion>,
) -> Vec<Stmt> {
  match stmt {
    Stmt::Decl(Decl::Var(var_decl)) => collect_display_names_from_var_decl(var_decl, insertions),
    Stmt::Block(block) => {
      insert_display_names_in_stmts(&mut block.stmts, insertions);
      Vec::new()
    }
    Stmt::If(if_stmt) => {
      let cons_stmts = ensure_block(&mut if_stmt.cons);
      insert_display_names_in_stmts(cons_stmts, insertions);

      if let Some(alt) = if_stmt.alt.as_mut() {
        let alt_stmts = ensure_block(alt);
        insert_display_names_in_stmts(alt_stmts, insertions);
      }

      Vec::new()
    }
    Stmt::While(while_stmt) => {
      let body = ensure_block(&mut while_stmt.body);
      insert_display_names_in_stmts(body, insertions);
      Vec::new()
    }
    Stmt::DoWhile(do_while_stmt) => {
      let body = ensure_block(&mut do_while_stmt.body);
      insert_display_names_in_stmts(body, insertions);
      Vec::new()
    }
    Stmt::For(for_stmt) => {
      let body = ensure_block(&mut for_stmt.body);
      insert_display_names_in_stmts(body, insertions);
      Vec::new()
    }
    Stmt::ForIn(for_in_stmt) => {
      let body = ensure_block(&mut for_in_stmt.body);
      insert_display_names_in_stmts(body, insertions);
      Vec::new()
    }
    Stmt::ForOf(for_of_stmt) => {
      let body = ensure_block(&mut for_of_stmt.body);
      insert_display_names_in_stmts(body, insertions);
      Vec::new()
    }
    Stmt::Switch(switch_stmt) => {
      for case in &mut switch_stmt.cases {
        insert_display_names_in_stmts(&mut case.cons, insertions);
      }
      Vec::new()
    }
    Stmt::Try(try_stmt) => {
      insert_display_names_in_stmts(&mut try_stmt.block.stmts, insertions);

      if let Some(handler) = try_stmt.handler.as_mut() {
        insert_display_names_in_stmts(&mut handler.body.stmts, insertions);
      }

      if let Some(finalizer) = try_stmt.finalizer.as_mut() {
        insert_display_names_in_stmts(&mut finalizer.stmts, insertions);
      }

      Vec::new()
    }
    Stmt::Labeled(labeled_stmt) => {
      let body = ensure_block(&mut labeled_stmt.body);
      insert_display_names_in_stmts(body, insertions);
      Vec::new()
    }
    Stmt::With(with_stmt) => {
      let body = ensure_block(&mut with_stmt.body);
      insert_display_names_in_stmts(body, insertions);
      Vec::new()
    }
    _ => Vec::new(),
  }
}

fn collect_display_names_from_var_decl(
  var_decl: &VarDecl,
  insertions: &mut Vec<DisplayNameInsertion>,
) -> Vec<Stmt> {
  let mut extras = Vec::new();

  for declarator in &var_decl.decls {
    if let Some(index) = insertions
      .iter()
      .position(|insertion| insertion.declarator_span == declarator.span)
    {
      let insertion = insertions.remove(index);
      extras.push(insertion.stmt);
    }
  }

  extras
}

fn ensure_block(stmt: &mut Stmt) -> &mut Vec<Stmt> {
  match stmt {
    Stmt::Block(block) => &mut block.stmts,
    _ => {
      let span = stmt.span();
      let original = std::mem::replace(stmt, Stmt::Empty(EmptyStmt { span }));
      *stmt = Stmt::Block(BlockStmt {
        span,
        ctxt: Default::default(),
        stmts: vec![original],
      });

      match stmt {
        Stmt::Block(block) => &mut block.stmts,
        _ => unreachable!(),
      }
    }
  }
}
