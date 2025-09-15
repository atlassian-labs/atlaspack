use std::path::PathBuf;

use swc_core::common::comments::SingleThreadedComments;
use swc_core::common::sync::Lrc;
use swc_core::common::*;
use swc_core::ecma::ast::*;
use swc_core::ecma::parser::lexer::Lexer;
use swc_core::ecma::parser::*;
use swc_core::ecma::visit::VisitWith;

use crate::visitors::js_visitor::{JsVisitor, VisitorRunner};
use crate::{Config, TransformResult};
use super::MagicCommentsVisitor;

pub fn parse(code: &str) -> anyhow::Result<Program> {
  let source_map = Lrc::new(SourceMap::default());
  let source_file =
    source_map.new_source_file(Lrc::new(FileName::Real(PathBuf::new())), code.into());

  let comments = SingleThreadedComments::default();
  let syntax = {
    let mut tsconfig = TsSyntax::default();
    tsconfig.tsx = true;
    Syntax::Typescript(tsconfig)
  };

  let lexer = Lexer::new(
    syntax,
    EsVersion::latest(),
    StringInput::from(&*source_file),
    Some(&comments),
  );

  let mut parser = Parser::new_from(lexer);

  let program = match parser.parse_program() {
    Err(err) => anyhow::bail!("{:?}", err),
    Ok(program) => program,
  };

  Ok(program)
}

#[test]
fn it_should_not_set_chunk_name_if_code_does_not_contain_a_magic_comment() -> anyhow::Result<()> {
  let code = r#"import('./foo')"#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert_eq!(
    visitor.magic_comments.len(),
    0,
    "Expected no magic comments to be set"
  );

  Ok(())
}

#[test]
fn it_should_set_chunk_name_if_code_contains_magic_comment() -> anyhow::Result<()> {
  let code = r#"import(/* webpackChunkName: "foo-chunk" */ "./foo")"#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert_eq!(
    visitor.magic_comments.get("./foo"),
    Some(&"foo-chunk".to_string()),
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn it_should_set_chunk_name_if_code_contains_multiple_magic_comment() -> anyhow::Result<()> {
  let code = r#"
    import(/* webpackChunkName: "foo-chunk" */ "./foo")
    import(/* webpackChunkName: "bar-chunk" */ "./bar")
  "#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert_eq!(
    visitor.magic_comments.get("./foo"),
    Some(&"foo-chunk".to_string()),
    "Expected magic comment to be set"
  );

  assert_eq!(
    visitor.magic_comments.get("./bar"),
    Some(&"bar-chunk".to_string()),
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn it_should_not_set_chunk_name_if_code_contains_multiple_imports() -> anyhow::Result<()> {
  let code = r#"
    import(/* webpackChunkName: "foo-chunk" */ "./foo")
    import("./bar")
  "#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert_eq!(
    visitor.magic_comments.get("./foo"),
    Some(&"foo-chunk".to_string()),
    "Expected magic comment to be set"
  );

  assert_eq!(
    visitor.magic_comments.get("./bar"),
    None,
    "Expected magic comment to not be set"
  );

  Ok(())
}

#[test]
fn it_should_work_with_current_dir_import() -> anyhow::Result<()> {
  let code = r#"import(/* webpackChunkName: "foo-chunk" */ ".");"#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert_eq!(
    visitor.magic_comments.get("."),
    Some(&"foo-chunk".to_string()),
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn it_should_work_with_current_dir_import_2() -> anyhow::Result<()> {
  let code = r#"import(/* webpackChunkName: "foo-chunk" */ "./");"#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert_eq!(
    visitor.magic_comments.get("./"),
    Some(&"foo-chunk".to_string()),
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn it_should_work_with_parent_dir_import() -> anyhow::Result<()> {
  let code = r#"import(/* webpackChunkName: "foo-chunk" */ "..");"#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert_eq!(
    visitor.magic_comments.get(".."),
    Some(&"foo-chunk".to_string()),
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn it_should_work_with_parent_dir_import_2() -> anyhow::Result<()> {
  let code = r#"import(/* webpackChunkName: "foo-chunk" */ "../");"#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert_eq!(
    visitor.magic_comments.get("../"),
    Some(&"foo-chunk".to_string()),
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn it_parses_lazy_imports() -> anyhow::Result<()> {
  let code = r#"
    const LazyPermalinkButton: ComponentType<Props> = lazyAfterPaint(
      () =>
        import(/* webpackChunkName: "async-issue-view-permalink-button" */ './view').then(
          (exportedModule) => exportedModule.PermalinkButton,
        ),
        {
          ssr: false,
        },
    );
  "#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert_eq!(
    visitor.magic_comments.get("./view"),
    Some(&"async-issue-view-permalink-button".to_string()),
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn it_handles_comments_and_imports_above() -> anyhow::Result<()> {
  let code = r#"
    /* Some comments */
    import { something } from './some-import.tsx';

    const Lazy = lazyForPaint(() => import( /* webpackChunkName: "the-chunk" */'@scope/package'), {
      ...params
    });

    // Some more comments
    // Some more comments
    // Some more comments
    const Lazy2 = lazyForPaint(() => import( /* webpackChunkName: "the-chunk-2" */'my-package'), {
      ...params
    });
  "#;

  let mut visitor = MagicCommentsVisitor::new(code);
  parse(code)?.visit_with(&mut visitor);

  assert_eq!(visitor.magic_comments.len(), 2);
  assert_eq!(
    visitor.magic_comments.get("@scope/package"),
    Some(&"the-chunk".to_string()),
    "Expected magic comment to be set"
  );
  assert_eq!(
    visitor.magic_comments.get("my-package"),
    Some(&"the-chunk-2".to_string()),
    "Expected magic comment to be set"
  );

  Ok(())
}

#[test]
fn test_js_visitor_api_integration() -> anyhow::Result<()> {
  let code = r#"import(/* webpackChunkName: "test-chunk" */ './test-module')"#;
  
  let mut visitor = MagicCommentsVisitor::new(code);
  let mut program = parse(code)?;
  
  // Test should_apply with magic_comments enabled
  let mut config = Config::default();
  config.magic_comments = true;
  assert!(visitor.should_apply(&config), "Visitor should apply when magic_comments is enabled and code contains magic comments");
  
  // Test should_apply with magic_comments disabled
  config.magic_comments = false;
  assert!(!visitor.should_apply(&config), "Visitor should not apply when magic_comments is disabled");
  
  // Test the actual visitor run with js_visitor API
  config.magic_comments = true;
  let mut result = TransformResult::default();
  
  VisitorRunner::run_visitor(visitor, &mut program, &config, &mut result);
  
  // Verify that magic comments were extracted and applied to result
  assert_eq!(result.magic_comments.len(), 1);
  assert_eq!(
    result.magic_comments.get("./test-module"),
    Some(&"test-chunk".to_string()),
    "Magic comment should be extracted and applied to result"
  );
  
  Ok(())
}

#[test]
fn test_js_visitor_api_should_not_apply_without_magic_comments() -> anyhow::Result<()> {
  let code = r#"import('./regular-import')"#; // No magic comments
  
  let mut visitor = MagicCommentsVisitor::new(code);
  let mut program = parse(code)?;
  
  let mut config = Config::default();
  config.magic_comments = true; // Even with flag enabled, should not apply without magic comments
  
  assert!(!visitor.should_apply(&config), "Visitor should not apply when code has no magic comments");
  
  let mut result = TransformResult::default();
  VisitorRunner::run_visitor(visitor, &mut program, &config, &mut result);
  
  // Verify that no magic comments were extracted
  assert_eq!(result.magic_comments.len(), 0, "No magic comments should be extracted from code without magic comments");
  
  Ok(())
}
