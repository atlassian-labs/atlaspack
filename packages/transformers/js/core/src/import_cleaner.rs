use std::collections::HashSet;

use swc_core::ecma::ast::*;
use swc_core::ecma::atoms::JsWord;
use swc_core::ecma::visit::{Visit, VisitMut, VisitWith};

/// Transformer that removes unused imports from ES modules.
///
/// This transform enables SSR compatibility by cleaning up unused imports
/// while preserving specific imports that are required:
/// - `react`: Always kept as it may be used implicitly by JSX
/// - `@emotion/react`: Always kept for JSX pragma imports
/// - `@atlaskit/css-reset`: Always kept for side effects
/// - Any import from packages containing `rxjs`: Always kept for side effects
///
/// For other imports, the transformer checks if any specifiers from the import
/// are actually referenced in the code. If all specifiers are unused, the entire
/// import is removed. If only some specifiers are unused, those specifiers are
/// removed while keeping the import statement with the used specifiers.
///
/// # Example
///
/// Input:
/// ```js
/// import { used, unused } from 'module';
/// import { completelyUnused } from 'other';
/// import React from 'react';
/// console.log(used);
/// ```
///
/// Output:
/// ```js
/// import { used } from 'module';
/// import React from 'react';
/// console.log(used);
/// ```
///
/// # Scope Awareness
///
/// The transformer correctly identifies references vs declarations, so local
/// variables with the same names are not affected.
///
/// # Implementation Details
///
/// The transformer works in two passes:
/// 1. Collects all identifier references in the module (excluding import declarations)
/// 2. Filters imports based on whether their specifiers appear in the collected references
pub struct ImportCleaner {
  /// Import sources that must always be kept, matched exactly.
  pub strict_whitelist: HashSet<JsWord>,

  /// Import sources that must be kept if the source contains any of these strings.
  pub loose_whitelist: HashSet<JsWord>,

  /// Set of all identifier bindings that are actually referenced in the code.
  /// Populated during the first pass, then used to filter imports.
  used_bindings: HashSet<Id>,
}

impl Default for ImportCleaner {
  fn default() -> Self {
    Self {
      strict_whitelist: HashSet::from([
        "react".into(),
        "@emotion/react".into(),
        "@atlaskit/css-reset".into(),
      ]),
      loose_whitelist: HashSet::from(["rxjs".into()]),
      used_bindings: HashSet::new(),
    }
  }
}

/// Visitor that collects all identifier references in a module.
///
/// This visitor traverses the AST and records every identifier it encounters,
/// except those declared in import statements. This allows us to determine
/// which imported symbols are actually used in the code.
struct BindingCollector {
  /// Set of all identifiers referenced in the module (excluding import declarations).
  used_bindings: HashSet<Id>,
}

impl Visit for BindingCollector {
  /// Records every identifier encountered during traversal.
  fn visit_ident(&mut self, node: &Ident) {
    self.used_bindings.insert(node.to_id());
  }

  /// Prevents traversal into import declarations.
  /// We only want to track identifiers that are *used* in the code, not just *declared* as imports.
  fn visit_import_decl(&mut self, _node: &ImportDecl) {
    // Intentionally empty - stops traversal into import declarations
  }
}

impl VisitMut for ImportCleaner {
  fn visit_mut_module(&mut self, node: &mut Module) {
    // First pass: Collect all identifier references in the module
    let mut collector = BindingCollector {
      used_bindings: HashSet::new(),
    };
    node.visit_with(&mut collector);
    self.used_bindings = collector.used_bindings;

    // Second pass: Filter imports based on collected references
    node.body = node
      .body
      .drain(..)
      .filter_map(|item| match item {
        ModuleItem::ModuleDecl(ModuleDecl::Import(mut import)) => {
          // Always keep whitelisted imports
          if self.strict_whitelist.contains(&import.src.value)
            || self
              .loose_whitelist
              .iter()
              .any(|w| import.src.value.contains(&**w))
          {
            return Some(ModuleItem::ModuleDecl(ModuleDecl::Import(import)));
          }

          // Filter specifiers to only those that are actually used
          import.specifiers.retain(|specifier| {
            let local_id = match specifier {
              ImportSpecifier::Named(named) => &named.local,
              ImportSpecifier::Default(default) => &default.local,
              ImportSpecifier::Namespace(namespace) => &namespace.local,
            };

            self.used_bindings.contains(&local_id.to_id())
          });

          // Keep the import only if it has at least one used specifier
          if !import.specifiers.is_empty() {
            return Some(ModuleItem::ModuleDecl(ModuleDecl::Import(import)));
          }

          None
        }
        other => Some(other),
      })
      .collect();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_swc_runner::test_utils::{RunVisitResult, run_test_visit};
  use indoc::indoc;

  #[test]
  fn test_removes_unused_imports() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import { foo, bar } from 'module';
        import React from 'react';
        console.log(foo);
      "#},
      |_| ImportCleaner::default(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        import { foo } from 'module';
        import React from 'react';
        console.log(foo);
      "#}
    );
  }

  #[test]
  fn test_keeps_react_import() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import React from 'react';
        import { unused } from 'other';
      "#},
      |_| ImportCleaner::default(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        import React from 'react';
      "#}
    );
  }

  #[test]
  fn test_keeps_emotion_import() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import { jsx } from '@emotion/react';
        import { unused } from 'other';
      "#},
      |_| ImportCleaner::default(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        import { jsx } from '@emotion/react';
      "#}
    );
  }

  #[test]
  fn test_keeps_rxjs_import() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import { Observable } from 'rxjs';
        import { map } from 'rxjs/operators';
        import { unused } from 'other';
      "#},
      |_| ImportCleaner::default(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        import { Observable } from 'rxjs';
        import { map } from 'rxjs/operators';
      "#}
    );
  }

  #[test]
  fn test_keeps_atlaskit_css_reset_import() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import '@atlaskit/css-reset';
        import { unused } from 'other';
      "#},
      |_| ImportCleaner::default(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        import '@atlaskit/css-reset';
      "#}
    );
  }

  #[test]
  fn test_keeps_imports_with_used_specifiers() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import { used, unused } from 'module';
        console.log(used);
      "#},
      |_| ImportCleaner::default(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        import { used } from 'module';
        console.log(used);
      "#}
    );
  }

  #[test]
  fn test_removes_completely_unused_imports() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import { unused1, unused2 } from 'module';
        console.log('test');
      "#},
      |_| ImportCleaner::default(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        console.log('test');
      "#}
    );
  }

  #[test]
  fn test_keeps_default_imports_when_used() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import defaultExport from 'module';
        console.log(defaultExport);
      "#},
      |_| ImportCleaner::default(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        import defaultExport from 'module';
        console.log(defaultExport);
      "#}
    );
  }

  #[test]
  fn test_removes_default_imports_when_unused() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import defaultExport from 'module';
        console.log('test');
      "#},
      |_| ImportCleaner::default(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        console.log('test');
      "#}
    );
  }

  #[test]
  fn test_keeps_namespace_imports_when_used() {
    let RunVisitResult { output_code, .. } = run_test_visit(
      indoc! {r#"
        import * as all from 'module';
        console.log(all.foo);
      "#},
      |_| ImportCleaner::default(),
    );

    assert_eq!(
      output_code,
      indoc! {r#"
        import * as all from 'module';
        console.log(all.foo);
      "#}
    );
  }
}
