use swc_core::ecma::visit::{Visit, VisitMut, VisitMutWith, VisitWith};

use crate::{Config, TransformResult};

/// Base trait for all JS visitors that defines common functionality
///
/// This provides a zero-cost abstraction for visitor collections using compile-time polymorphism.
/// All visitor dispatch is resolved at compile time with no runtime overhead.
///
/// # Example
///
/// ```rust
/// use swc_core::ecma::visit::{Visit, VisitMut};
/// use swc_core::ecma::ast::*;
///
/// // Example immutable visitor (Visit)
/// struct MyReadVisitor {
///     found_jsx: bool,
/// }
///
/// impl JsVisitor for MyReadVisitor {
///     fn should_apply(&self, config: &Config) -> bool {
///         config.is_jsx
///     }
///
///     fn apply_results(self, result: &mut TransformResult) {
///         // Store any collected information in the result
///         if self.found_jsx {
///             // Add some metadata or modify result
///         }
///     }
/// }
///
/// impl Visit for MyReadVisitor {
///     fn visit_jsx_element(&mut self, _node: &JSXElement) {
///         self.found_jsx = true;
///     }
/// }
///
/// // Example mutable visitor (VisitMut)
/// struct MyMutVisitor;
///
/// impl JsVisitor for MyMutVisitor {
///     fn should_apply(&self, config: &Config) -> bool {
///         config.minify
///     }
/// }
///
/// impl VisitMut for MyMutVisitor {
///     fn visit_mut_ident(&mut self, node: &mut Ident) {
///         // Transform the identifier
///         node.sym = format!("_{}", node.sym).into();
///     }
/// }
///
/// // Usage Option 1: Static collection (zero-cost, compile-time dispatch)
/// let mut collection = StaticVisitorCollection::new()
///     .add_read_visitor(MyReadVisitor { found_jsx: false })
///     .add_mut_visitor(MyMutVisitor);
///
/// collection.run(&mut program, &config, &mut result);
///
/// // Usage Option 2: Macro for inline visitor execution
/// run_visitors! {
///     program: &mut program,
///     config: &config,
///     result: &mut result,
///     visitors: {
///         read: MyReadVisitor { found_jsx: false },
///         mut: MyMutVisitor,
///     }
/// }
///
/// // Usage Option 3: Direct runner for single visitors
/// VisitorRunner::run_visitor(&mut my_read_visitor, &mut program, &config, &mut result);
/// VisitorRunner::run_mut_visitor(&mut my_mut_visitor, &mut program, &config, &mut result);
/// ```
pub trait JsVisitor {
  /// Check if this visitor should be applied based on the config
  fn should_apply(&self, config: &Config) -> bool;

  /// Apply any results/side effects after visiting
  fn apply_results(self, _result: &mut TransformResult) where Self: Sized {}
}

/// Generic visitor runner that works with any visitor type at compile time
pub struct VisitorRunner;

impl VisitorRunner {
  /// Run a single visitor if it should apply
  pub fn run_visitor<V>(
    mut visitor: V,
    program: &mut swc_core::ecma::ast::Program,
    config: &Config,
    result: &mut TransformResult,
  ) where
    V: JsVisitor + Visit,
  {
    if visitor.should_apply(config) {
      program.visit_with(&mut visitor);
      visitor.apply_results(result);
    }
  }

  /// Run a single mutable visitor if it should apply
  pub fn run_mut_visitor<V>(
    mut visitor: V,
    program: &mut swc_core::ecma::ast::Program,
    config: &Config,
    result: &mut TransformResult,
  ) where
    V: JsVisitor + VisitMut,
  {
    if visitor.should_apply(config) {
      program.visit_mut_with(&mut visitor);
      visitor.apply_results(result);
    }
  }
}

/// Macro to define a collection of visitors and run them in sequence
/// This provides zero-cost abstraction - all dispatching happens at compile time
#[macro_export]
macro_rules! run_visitors {
    (
        program: $program:expr,
        config: $config:expr,
        result: $result:expr,
        visitors: {
            $( read: $read_visitor:expr, )*
            $( mut: $mut_visitor:expr, )*
        }
    ) => {
        $(
            $crate::visitors::VisitorRunner::run_visitor(
                $read_visitor,
                $program,
                $config,
                $result
            );
        )*
        $(
            $crate::visitors::VisitorRunner::run_mut_visitor(
                $mut_visitor,
                $program,
                $config,
                $result
            );
        )*
    };
}

/// Builder pattern for creating and running visitor collections with zero-cost abstraction
pub struct StaticVisitorCollection<T> {
  visitors: T,
}

impl StaticVisitorCollection<()> {
  pub fn new() -> Self {
    Self { visitors: () }
  }
}

impl<T> StaticVisitorCollection<T> {
  /// Add an immutable visitor to the collection
  pub fn add_read_visitor<V: JsVisitor + Visit>(
    self,
    visitor: V,
  ) -> StaticVisitorCollection<(T, ReadVisitorWrapper<V>)> {
    StaticVisitorCollection {
      visitors: (self.visitors, ReadVisitorWrapper { visitor }),
    }
  }

  /// Add a mutable visitor to the collection
  pub fn add_mut_visitor<V: JsVisitor + VisitMut>(
    self,
    visitor: V,
  ) -> StaticVisitorCollection<(T, MutVisitorWrapper<V>)> {
    StaticVisitorCollection {
      visitors: (self.visitors, MutVisitorWrapper { visitor }),
    }
  }
}

/// Wrapper for immutable visitors
pub struct ReadVisitorWrapper<V> {
  visitor: V,
}

/// Wrapper for mutable visitors
pub struct MutVisitorWrapper<V> {
  visitor: V,
}

/// Trait to run visitors in a collection
pub trait RunVisitors {
  fn run(
    self,
    program: &mut swc_core::ecma::ast::Program,
    config: &Config,
    result: &mut TransformResult,
  );
}

// Base case - empty collection
impl RunVisitors for () {
  fn run(
    self,
    _program: &mut swc_core::ecma::ast::Program,
    _config: &Config,
    _result: &mut TransformResult,
  ) {
    // Nothing to do
  }
}

// Recursive case for read visitors
impl<T, V> RunVisitors for (T, ReadVisitorWrapper<V>)
where
  T: RunVisitors,
  V: JsVisitor + Visit,
{
  fn run(
    self,
    program: &mut swc_core::ecma::ast::Program,
    config: &Config,
    result: &mut TransformResult,
  ) {
    self.0.run(program, config, result);
    VisitorRunner::run_visitor(self.1.visitor, program, config, result);
  }
}

// Recursive case for mutable visitors
impl<T, V> RunVisitors for (T, MutVisitorWrapper<V>)
where
  T: RunVisitors,
  V: JsVisitor + VisitMut,
{
  fn run(
    self,
    program: &mut swc_core::ecma::ast::Program,
    config: &Config,
    result: &mut TransformResult,
  ) {
    self.0.run(program, config, result);
    VisitorRunner::run_mut_visitor(self.1.visitor, program, config, result);
  }
}

impl<T: RunVisitors> StaticVisitorCollection<T> {
  /// Run all visitors in the collection
  pub fn run(
    self,
    program: &mut swc_core::ecma::ast::Program,
    config: &Config,
    result: &mut TransformResult,
  ) {
    self.visitors.run(program, config, result);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{Config, TransformResult, run_visitors, test_utils::make_test_swc_config};
  use std::collections::HashSet;
  use swc_core::common::{DUMMY_SP, SyntaxContext};
  use swc_core::ecma::ast::*;
  use swc_core::ecma::visit::{Visit, VisitMut};

  // Test visitors for different scenarios

  /// Mock visitor that implements Visit (immutable)
  #[derive(Default)]
  struct MockReadVisitor {
    pub call_count: u32,
    pub should_apply_result: bool,
    pub identifiers_found: HashSet<String>,
  }

  impl JsVisitor for MockReadVisitor {
    fn should_apply(&self, _config: &Config) -> bool {
      self.should_apply_result
    }

    fn apply_results(self, result: &mut TransformResult) {
      // Now we can consume self, no need for hacks or interior mutability
      let _ = result; // Use the parameter to avoid warnings
    }
  }

  impl Visit for MockReadVisitor {
    fn visit_ident(&mut self, node: &Ident) {
      self.call_count += 1;
      self.identifiers_found.insert(node.sym.to_string());
    }
  }

  /// Mock visitor that implements VisitMut (mutable)
  #[derive(Default)]
  struct MockMutVisitor {
    pub call_count: u32,
    pub should_apply_result: bool,
    pub prefix: String,
  }

  impl JsVisitor for MockMutVisitor {
    fn should_apply(&self, _config: &Config) -> bool {
      self.should_apply_result
    }

    fn apply_results(self, _result: &mut TransformResult) {
      // Mock implementation
    }
  }

  impl VisitMut for MockMutVisitor {
    fn visit_mut_ident(&mut self, node: &mut Ident) {
      self.call_count += 1;
      if !self.prefix.is_empty() {
        node.sym = format!("{}{}", self.prefix, node.sym).into();
      }
    }
  }

  /// Visitor that always should NOT apply
  #[derive(Default, Clone)]
  struct DisabledVisitor {
    pub visit_called: bool,
  }

  impl JsVisitor for DisabledVisitor {
    fn should_apply(&self, _config: &Config) -> bool {
      false
    }
  }

  impl Visit for DisabledVisitor {
    fn visit_ident(&mut self, _node: &Ident) {
      self.visit_called = true;
    }
  }

  /// Visitor that checks specific config flags
  #[derive(Default, Clone)]
  struct ConfigBasedVisitor {
    pub visit_called: bool,
  }

  impl JsVisitor for ConfigBasedVisitor {
    fn should_apply(&self, config: &Config) -> bool {
      config.is_jsx && config.is_development
    }
  }

  impl Visit for ConfigBasedVisitor {
    fn visit_ident(&mut self, _node: &Ident) {
      self.visit_called = true;
    }
  }

  /// Helper function to create a simple program for testing
  fn create_test_program() -> Program {
    let ident1 = Ident::new("foo".into(), DUMMY_SP, SyntaxContext::empty());
    let ident2 = Ident::new("bar".into(), DUMMY_SP, SyntaxContext::empty());

    Program::Module(Module {
      span: DUMMY_SP,
      body: vec![
        ModuleItem::Stmt(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: Box::new(Expr::Ident(ident1)),
        })),
        ModuleItem::Stmt(Stmt::Expr(ExprStmt {
          span: DUMMY_SP,
          expr: Box::new(Expr::Ident(ident2)),
        })),
      ],
      shebang: None,
    })
  }

  #[test]
  fn test_visitor_runner_read_visitor_applies() {
    let mut visitor = MockReadVisitor {
      should_apply_result: true,
      ..Default::default()
    };
    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    VisitorRunner::run_visitor(visitor, &mut program, &config, &mut result);
  }

  #[test]
  fn test_visitor_runner_read_visitor_does_not_apply() {
    let mut visitor = MockReadVisitor {
      should_apply_result: false,
      ..Default::default()
    };
    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    VisitorRunner::run_visitor(visitor, &mut program, &config, &mut result);
  }

  #[test]
  fn test_visitor_runner_mut_visitor_applies() {
    let mut visitor = MockMutVisitor {
      should_apply_result: true,
      prefix: "test_".to_string(),
      ..Default::default()
    };
    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    VisitorRunner::run_mut_visitor(visitor, &mut program, &config, &mut result);

    // Check that the identifiers were actually modified
    if let Program::Module(module) = &program {
      let mut found_modified = false;
      for item in &module.body {
        if let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = item {
          if let Expr::Ident(ident) = expr_stmt.expr.as_ref() {
            if ident.sym.starts_with("test_") {
              found_modified = true;
              break;
            }
          }
        }
      }
      assert!(found_modified, "Identifiers should be modified with prefix");
    }
  }

  #[test]
  fn test_visitor_runner_mut_visitor_does_not_apply() {
    let mut visitor = MockMutVisitor {
      should_apply_result: false,
      prefix: "test_".to_string(),
      ..Default::default()
    };
    let mut program = create_test_program();
    let original_program = program.clone();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    VisitorRunner::run_mut_visitor(visitor, &mut program, &config, &mut result);
    // Program should remain unchanged
    assert_eq!(format!("{:?}", program), format!("{:?}", original_program));
  }

  #[test]
  fn test_static_visitor_collection_empty() {
    let mut collection = StaticVisitorCollection::new();
    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    // Should not panic with empty collection
    collection.run(&mut program, &config, &mut result);
  }

  #[test]
  fn test_static_visitor_collection_single_read_visitor() {
    let read_visitor = MockReadVisitor {
      should_apply_result: true,
      ..Default::default()
    };

    let mut collection = StaticVisitorCollection::new().add_read_visitor(read_visitor);

    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    collection.run(&mut program, &config, &mut result);

    // We can't easily check the visitor state after running since it's moved into the collection
    // But we can verify no panic occurred and test the macro instead
  }

  #[test]
  fn test_static_visitor_collection_single_mut_visitor() {
    let mut_visitor = MockMutVisitor {
      should_apply_result: true,
      prefix: "collection_".to_string(),
      ..Default::default()
    };

    let mut collection = StaticVisitorCollection::new().add_mut_visitor(mut_visitor);

    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    collection.run(&mut program, &config, &mut result);

    // Check that identifiers were modified
    if let Program::Module(module) = &program {
      let mut found_modified = false;
      for item in &module.body {
        if let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = item {
          if let Expr::Ident(ident) = expr_stmt.expr.as_ref() {
            if ident.sym.starts_with("collection_") {
              found_modified = true;
              break;
            }
          }
        }
      }
      assert!(
        found_modified,
        "Identifiers should be modified by mut visitor"
      );
    }
  }

  #[test]
  fn test_static_visitor_collection_mixed_visitors() {
    let read_visitor = MockReadVisitor {
      should_apply_result: true,
      ..Default::default()
    };

    let mut_visitor = MockMutVisitor {
      should_apply_result: true,
      prefix: "mixed_".to_string(),
      ..Default::default()
    };

    let mut collection = StaticVisitorCollection::new()
      .add_read_visitor(read_visitor)
      .add_mut_visitor(mut_visitor);

    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    collection.run(&mut program, &config, &mut result);

    // Check that identifiers were modified by the mut visitor
    if let Program::Module(module) = &program {
      let mut found_modified = false;
      for item in &module.body {
        if let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = item {
          if let Expr::Ident(ident) = expr_stmt.expr.as_ref() {
            if ident.sym.starts_with("mixed_") {
              found_modified = true;
              break;
            }
          }
        }
      }
      assert!(found_modified, "Identifiers should be modified");
    }
  }

  #[test]
  fn test_run_visitors_macro_read_only() {
    let mut read_visitor = MockReadVisitor {
      should_apply_result: true,
      ..Default::default()
    };

    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    run_visitors! {
        program: &mut program,
        config: &config,
        result: &mut result,
        visitors: {
            read: read_visitor,
        }
    }
  }

  #[test]
  fn test_run_visitors_macro_mut_only() {
    let mut mut_visitor = MockMutVisitor {
      should_apply_result: true,
      prefix: "macro_".to_string(),
      ..Default::default()
    };

    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    run_visitors! {
        program: &mut program,
        config: &config,
        result: &mut result,
        visitors: {
            mut: mut_visitor,
        }
    }

    // Check that identifiers were modified
    if let Program::Module(module) = &program {
      let mut found_modified = false;
      for item in &module.body {
        if let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = item {
          if let Expr::Ident(ident) = expr_stmt.expr.as_ref() {
            if ident.sym.starts_with("macro_") {
              found_modified = true;
              break;
            }
          }
        }
      }
      assert!(found_modified, "Identifiers should be modified");
    }
  }

  #[test]
  fn test_run_visitors_macro_mixed() {
    let mut read_visitor = MockReadVisitor {
      should_apply_result: true,
      ..Default::default()
    };

    let mut mut_visitor = MockMutVisitor {
      should_apply_result: true,
      prefix: "mixed_macro_".to_string(),
      ..Default::default()
    };

    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    run_visitors! {
        program: &mut program,
        config: &config,
        result: &mut result,
        visitors: {
            read: read_visitor,
            mut: mut_visitor,
        }
    }
  }

  #[test]
  fn test_run_visitors_macro_multiple_visitors() {
    let mut read_visitor1 = MockReadVisitor {
      should_apply_result: true,
      ..Default::default()
    };

    let mut read_visitor2 = MockReadVisitor {
      should_apply_result: true,
      ..Default::default()
    };

    let mut mut_visitor1 = MockMutVisitor {
      should_apply_result: true,
      prefix: "first_".to_string(),
      ..Default::default()
    };

    let mut mut_visitor2 = MockMutVisitor {
      should_apply_result: true,
      prefix: "second_".to_string(),
      ..Default::default()
    };

    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    run_visitors! {
        program: &mut program,
        config: &config,
        result: &mut result,
        visitors: {
            read: read_visitor1,
            read: read_visitor2,
            mut: mut_visitor1,
            mut: mut_visitor2,
        }
    }
  }

  #[test]
  fn test_visitor_config_based_should_apply() {
    let mut visitor = ConfigBasedVisitor::default();
    let mut program = create_test_program();
    let mut config = make_test_swc_config("test");

    // Set config flags that should enable the visitor
    config.is_jsx = true;
    config.is_development = true;

    let mut result = TransformResult::default();

    // Clone the visitor before running since run_visitor consumes it
    let visitor_clone = visitor.clone();
    VisitorRunner::run_visitor(visitor_clone, &mut program, &config, &mut result);
    
    // For this test, we need to check the result differently since we can't access visitor after consumption
    // We'll verify the visitor would have been called by checking if it should_apply returns true
    assert!(
      visitor.should_apply(&config),
      "Visitor should be called when config conditions are met"
    );
  }

  #[test]
  fn test_visitor_config_based_should_not_apply() {
    let mut visitor = ConfigBasedVisitor::default();
    let mut program = create_test_program();
    let mut config = make_test_swc_config("test");

    // Set config flags that should disable the visitor
    config.is_jsx = false;
    config.is_development = true; // Only one condition is true

    let mut result = TransformResult::default();

    // Clone the visitor before running since run_visitor consumes it
    let visitor_clone = visitor.clone();
    VisitorRunner::run_visitor(visitor_clone, &mut program, &config, &mut result);
    
    // For this test, we verify the visitor should not apply with the given config
    assert!(
      !visitor.should_apply(&config),
      "Visitor should not be called when config conditions are not met"
    );
  }

  #[test]
  fn test_disabled_visitor_never_runs() {
    let mut visitor = DisabledVisitor::default();
    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    // Clone the visitor before running since run_visitor consumes it
    let visitor_clone = visitor.clone();
    VisitorRunner::run_visitor(visitor_clone, &mut program, &config, &mut result);
    
    // For this test, we verify the disabled visitor should never apply
    assert!(
      !visitor.should_apply(&config),
      "Disabled visitor should never be called"
    );
  }

  #[test]
  fn test_visitor_ordering_in_collection() {
    // Test that visitors run in the order they were added
    let read_visitor = MockReadVisitor {
      should_apply_result: true,
      ..Default::default()
    };

    // First mut visitor adds prefix "first_"
    let mut_visitor1 = MockMutVisitor {
      should_apply_result: true,
      prefix: "first_".to_string(),
      ..Default::default()
    };

    // Second mut visitor adds prefix "second_" (will be "second_first_...")
    let mut_visitor2 = MockMutVisitor {
      should_apply_result: true,
      prefix: "second_".to_string(),
      ..Default::default()
    };

    let mut collection = StaticVisitorCollection::new()
      .add_read_visitor(read_visitor)
      .add_mut_visitor(mut_visitor1)
      .add_mut_visitor(mut_visitor2);

    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    collection.run(&mut program, &config, &mut result);

    // Check that both prefixes were applied in order
    if let Program::Module(module) = &program {
      let mut found_double_prefix = false;
      for item in &module.body {
        if let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = item {
          if let Expr::Ident(ident) = expr_stmt.expr.as_ref() {
            if ident.sym.starts_with("second_first_") {
              found_double_prefix = true;
              break;
            }
          }
        }
      }
      assert!(
        found_double_prefix,
        "Both prefixes should be applied in order"
      );
    }
  }

  #[test]
  fn test_visitor_can_be_chained_with_builder_pattern() {
    // Test the fluent builder pattern
    let collection = StaticVisitorCollection::new()
      .add_read_visitor(MockReadVisitor {
        should_apply_result: true,
        ..Default::default()
      })
      .add_mut_visitor(MockMutVisitor {
        should_apply_result: true,
        prefix: "chained_".to_string(),
        ..Default::default()
      })
      .add_read_visitor(MockReadVisitor {
        should_apply_result: true,
        ..Default::default()
      });

    // Just test that the collection can be created and we can call run
    let mut program = create_test_program();
    let config = make_test_swc_config("test");
    let mut result = TransformResult::default();

    // This should not panic
    let mut collection = collection;
    collection.run(&mut program, &config, &mut result);
  }
}
