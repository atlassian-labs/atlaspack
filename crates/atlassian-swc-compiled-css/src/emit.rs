use std::borrow::Cow;
use std::hash::Hasher;

use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::*;
use xxhash_rust::xxh3::Xxh3Builder;

use crate::CompiledCssInJsCollector;

pub struct RuleInput<'a> {
  pub prop_kebab: &'a str,
  pub value_text: Cow<'a, str>,
  pub suffix: &'a str,
  pub wrappers: &'a [String],
}

impl CompiledCssInJsCollector {
  pub fn ensure_const_for_rule<F>(&mut self, rule_key: u64, css_builder: F) -> Ident
  where
    F: FnOnce() -> String,
  {
    if let Some(id) = self.rule_key_to_ident.get(&rule_key) {
      return id.clone();
    }
    let mut candidate: String;
    loop {
      self.next_id += 1;
      candidate = if self.next_id == 1 {
        "_".to_string()
      } else {
        format!("_{}", self.next_id)
      };
      if !self.used_idents.contains(&candidate) {
        break;
      }
    }
    self.used_idents.insert(candidate.clone());
    let ident = Ident::new(candidate.into(), DUMMY_SP, SyntaxContext::empty());
    let css_text = css_builder();
    self
      .css_rules_for_emission
      .push((rule_key, css_text.clone()));
    self.style_rules.insert(css_text.clone());
    let var_decl = VarDecl {
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
          value: css_text.into(),
          raw: None,
        })))),
        definite: false,
      }],
    };
    let decl = ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(var_decl))));
    self.hoisted.push(decl);
    self.rule_key_to_ident.insert(rule_key, ident.clone());

    // Track CSS constant for CS component generation
    if self.emit_cc_cs_components {
      self.css_constants_for_cs.push(ident.clone());
    }

    ident
  }

  pub fn emit_rule_input<'a>(&mut self, input: RuleInput<'a>, out_classes: &mut Vec<String>) {
    // Skip empty values (except content: "")
    if input.prop_kebab != "content" && input.value_text.as_ref().is_empty() {
      return;
    }

    let value_text = self.normalise_css_property_value(input.value_text.as_ref());

    // Selectors don't work yet, so default to "&" which works css without selectors
    let class_name = self.hash_class_name(input.prop_kebab, &value_text, "&", None, false);
    let mut hasher = Xxh3Builder::new().build();
    hasher.write(input.suffix.as_bytes());
    hasher.write(&[0]);
    hasher.write(input.prop_kebab.as_bytes());
    hasher.write(&[0]);
    hasher.write(value_text.as_bytes());
    hasher.write(&[0]);
    for w in input.wrappers {
      hasher.write(w.as_bytes());
      hasher.write(&[0]);
    }
    let rule_key = hasher.finish();
    let mut value_text_cow: Cow<'a, str> = value_text.into();
    if input.prop_kebab == "content" && value_text_cow.as_ref().is_empty() {
      value_text_cow = Cow::Owned("\"\"".to_string());
    }

    if input.wrappers.is_empty() {
      // If we've already emitted this rule, avoid building the selector string
      if self.rule_key_to_ident.contains_key(&rule_key) {
        out_classes.push(class_name);
        return;
      }
      // Build the selector only when needed
      let selector_rule = if !input.suffix.is_empty()
        && (input.suffix.starts_with('.')
          || input.suffix.starts_with('#')
          || input.suffix.contains('&'))
      {
        if input.suffix.contains('&')
          && (input.suffix.starts_with('.') || input.suffix.starts_with('#'))
        {
          let mut self_sel = String::with_capacity(1 + class_name.len());
          self_sel.push('.');
          self_sel.push_str(&class_name);
          let replaced = input.suffix.replace('&', &self_sel);
          let mut s = String::with_capacity(
            replaced.len() + 1 + input.prop_kebab.len() + 1 + value_text_cow.len() + 1,
          );
          s.push_str(&replaced);
          s.push('{');
          s.push_str(input.prop_kebab);
          s.push(':');
          s.push_str(value_text_cow.as_ref());
          s.push('}');
          s
        } else {
          let mut s = String::with_capacity(
            1 + class_name.len()
              + input.suffix.len()
              + 1
              + input.prop_kebab.len()
              + 1
              + value_text_cow.len()
              + 1,
          );
          s.push('.');
          s.push_str(&class_name);
          s.push_str(input.suffix);
          s.push('{');
          s.push_str(input.prop_kebab);
          s.push(':');
          s.push_str(value_text_cow.as_ref());
          s.push('}');
          s
        }
      } else {
        let mut s = String::with_capacity(
          1 + class_name.len()
            + input.suffix.len()
            + 1
            + input.prop_kebab.len()
            + 1
            + value_text_cow.len()
            + 1,
        );
        s.push('.');
        s.push_str(&class_name);
        if !input.suffix.is_empty() {
          s.push_str(input.suffix);
        }
        s.push('{');
        s.push_str(input.prop_kebab);
        s.push(':');
        s.push_str(value_text_cow.as_ref());
        s.push('}');
        s
      };
      let selector_rule_clone = selector_rule.clone();
      self.ensure_const_for_rule(rule_key, move || selector_rule_clone);
    } else {
      let key = input.wrappers.join("\x1f");
      let seen = self.wrapper_key_to_seen.entry(key.clone()).or_default();
      if !seen.contains(&rule_key) {
        seen.insert(rule_key);
        // Build the selector only if we will store it
        let selector_rule = if !input.suffix.is_empty()
          && (input.suffix.starts_with('.')
            || input.suffix.starts_with('#')
            || input.suffix.contains('&'))
        {
          if input.suffix.contains('&')
            && (input.suffix.starts_with('.') || input.suffix.starts_with('#'))
          {
            let mut self_sel = String::with_capacity(1 + class_name.len());
            self_sel.push('.');
            self_sel.push_str(&class_name);
            let replaced = input.suffix.replace('&', &self_sel);
            let mut s = String::with_capacity(
              replaced.len() + 1 + input.prop_kebab.len() + 1 + value_text_cow.len() + 1,
            );
            s.push_str(&replaced);
            s.push('{');
            s.push_str(input.prop_kebab);
            s.push(':');
            s.push_str(value_text_cow.as_ref());
            s.push('}');
            s
          } else {
            let mut s = String::with_capacity(
              1 + class_name.len()
                + input.suffix.len()
                + 1
                + input.prop_kebab.len()
                + 1
                + value_text_cow.len()
                + 1,
            );
            s.push('.');
            s.push_str(&class_name);
            s.push_str(input.suffix);
            s.push('{');
            s.push_str(input.prop_kebab);
            s.push(':');
            s.push_str(value_text_cow.as_ref());
            s.push('}');
            s
          }
        } else {
          let mut s = String::with_capacity(
            1 + class_name.len()
              + input.suffix.len()
              + 1
              + input.prop_kebab.len()
              + 1
              + value_text_cow.len()
              + 1,
          );
          s.push('.');
          s.push_str(&class_name);
          if !input.suffix.is_empty() {
            s.push_str(input.suffix);
          }
          s.push('{');
          s.push_str(input.prop_kebab);
          s.push(':');
          s.push_str(value_text_cow.as_ref());
          s.push('}');
          s
        };
        self
          .wrapper_key_to_rules
          .entry(key.clone())
          .or_default()
          .push(selector_rule);
        self
          .wrapper_key_to_stack
          .entry(key)
          .or_insert_with(|| input.wrappers.to_vec());
      }
    }
    out_classes.push(class_name);
  }
}
