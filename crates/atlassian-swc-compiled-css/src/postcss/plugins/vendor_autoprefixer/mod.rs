use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use swc_core::css::ast::{
  AtRuleName, ComponentValue, Declaration, DeclarationName, QualifiedRule, Rule, Stylesheet,
};

use super::super::transform::{Plugin, TransformContext};
use crate::postcss::plugins::vendor_prefixing_lite::make_ident;

pub(crate) static PREFIXES_JSON: Lazy<Option<&'static str>> = Lazy::new(|| {
  Some(include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/autoprefixer_data/prefixes.json"
  )))
});

pub(crate) static AGENTS_JSON: Lazy<Option<&'static str>> = Lazy::new(|| {
  Some(include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/autoprefixer_data/agents.json"
  )))
});

#[derive(Debug, Deserialize)]
struct PrefixEntry {
  #[serde(default)]
  browsers: Vec<String>,
  #[serde(default)]
  mistakes: Vec<String>,
  #[serde(default)]
  feature: Option<String>,
  #[serde(default)]
  props: Option<Vec<String>>,
  #[serde(default)]
  selector: bool,
}

#[derive(Debug, Deserialize)]
struct AgentsMap(HashMap<String, Agent>);

#[derive(Debug, Deserialize)]
struct Agent {
  prefix: String,
  #[serde(default, rename = "prefix_exceptions")]
  prefix_exceptions: Option<HashMap<String, String>>,
  #[serde(default)]
  versions: Vec<Option<String>>,
}

#[derive(Debug, Default)]
pub struct PrefixDB {
  // name -> entry
  entries: HashMap<String, PrefixEntry>,
  agents: HashMap<String, Agent>,
}

#[derive(Clone, Debug)]
pub(crate) struct ValueRule {
  pub(crate) keyword: String,
  pub(crate) prefixes: Vec<String>,
  kind: ValueKind,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ValueKind {
  Plain,
  FitContent,
  Stretch,
}

impl PrefixDB {
  pub fn load() -> Option<Self> {
    let pjson = PREFIXES_JSON.as_ref().and_then(|s| Some(*s))?;
    let aj = AGENTS_JSON.as_ref().and_then(|s| Some(*s))?;
    let entries: HashMap<String, PrefixEntry> = match serde_json::from_str(pjson) {
      Ok(v) => v,
      Err(err) => {
        if std::env::var("COMPILED_CLI_TRACE").is_ok() {
          eprintln!("[autoprefixer] failed to parse prefixes.json: {err}");
        }
        return None;
      }
    };
    let agents: HashMap<String, Agent> = match serde_json::from_str::<AgentsMap>(aj) {
      Ok(v) => v.0,
      Err(err) => {
        if std::env::var("COMPILED_CLI_TRACE").is_ok() {
          eprintln!("[autoprefixer] failed to parse agents.json: {err}");
        }
        return None;
      }
    };
    Some(PrefixDB { entries, agents })
  }

  #[allow(dead_code)]
  pub fn load_from_str(prefixes_json: &str, agents_json: &str) -> Option<Self> {
    let entries: HashMap<String, PrefixEntry> = serde_json::from_str(prefixes_json).ok()?;
    let agents: HashMap<String, Agent> = serde_json::from_str::<AgentsMap>(agents_json).ok()?.0;
    Some(PrefixDB { entries, agents })
  }

  fn prefix_for_browser(&self, browser: &str) -> Option<String> {
    let (name, version) = browser.split_once(' ')?;
    let agent = self.agents.get(name)?;
    if let Some(map) = &agent.prefix_exceptions {
      if let Some(p) = map.get(version) {
        return Some(format!("-{}-", p));
      }
    }
    Some(format!("-{}-", agent.prefix))
  }

  pub fn select_add_remove(
    &self,
    selected_browsers: &[String],
  ) -> (HashMap<String, Vec<String>>, HashMap<String, Vec<String>>) {
    let normalized_targets = self.normalize_targets(selected_browsers);
    let mut add: HashMap<String, Vec<String>> = HashMap::new();
    let mut remove: HashMap<String, Vec<String>> = HashMap::new();
    let sel: BTreeSet<String> = normalized_targets.into_iter().collect();

    for (name, data) in &self.entries {
      let mut entries: Vec<(String, Option<String>)> = Vec::new();
      for br in &data.browsers {
        let mut parts = br.split_whitespace();
        let browser = match (parts.next(), parts.next()) {
          (Some(name), Some(version)) => {
            let mut browser = String::from(name);
            browser.push(' ');
            browser.push_str(version);
            browser
          }
          _ => br.clone(),
        };
        let note = parts.next().map(|first| {
          let mut note = String::from(first);
          for rest in parts {
            note.push(' ');
            note.push_str(rest);
          }
          note
        });
        entries.push((browser, note));
      }

      let mut note_prefixes: Vec<String> = Vec::new();
      for (browser, note) in &entries {
        if let Some(note) = note {
          if let Some(prefix) = self.prefix_for_browser(browser) {
            note_prefixes.push(format!("{} {}", prefix, note));
          }
        }
      }
      note_prefixes.sort();
      note_prefixes.dedup();

      let mut needed: Vec<String> = Vec::new();
      for (browser, note) in &entries {
        if sel.contains(browser) {
          if let Some(prefix) = self.prefix_for_browser(browser) {
            if let Some(note) = note {
              needed.push(format!("{} {}", prefix, note));
            } else {
              needed.push(prefix);
            }
          }
        }
      }
      needed.sort();
      needed.dedup();
      let need_set: BTreeSet<String> = needed.iter().cloned().collect();

      let mut all_prefixes: BTreeSet<String> = BTreeSet::new();
      for br in &data.browsers {
        if let Some(pref) = self.prefix_for_browser(br) {
          all_prefixes.insert(pref);
        }
      }
      for m in &data.mistakes {
        all_prefixes.insert(m.clone());
      }
      for note in &note_prefixes {
        all_prefixes.insert(note.clone());
      }

      if !needed.is_empty() {
        let mut prefixes = needed.clone();
        sort_prefixes(&mut prefixes);
        add.insert(name.clone(), prefixes);
        let rem: Vec<String> = all_prefixes.difference(&need_set).cloned().collect();
        if !rem.is_empty() {
          remove.insert(name.clone(), rem);
        }
      } else if !all_prefixes.is_empty() {
        remove.insert(name.clone(), all_prefixes.into_iter().collect());
      }
    }
    (add, remove)
  }
}

fn sort_prefixes(prefixes: &mut Vec<String>) {
  fn remove_note(value: &str) -> &str {
    value.split_once(' ').map(|(p, _)| p).unwrap_or(value)
  }
  prefixes.sort_by(|a, b| {
    let a_clean = remove_note(a);
    let b_clean = remove_note(b);
    if a_clean.len() == b_clean.len() {
      b.len().cmp(&a.len())
    } else {
      b_clean.len().cmp(&a_clean.len())
    }
  });
}

fn parse_leading_numeric(value: &str) -> Option<f64> {
  let mut end = 0;
  for (idx, ch) in value.char_indices() {
    if ch.is_ascii_digit() || ch == '.' {
      end = idx + ch.len_utf8();
    } else {
      break;
    }
  }
  if end == 0 {
    return None;
  }
  value[..end].parse::<f64>().ok()
}

impl Agent {
  fn normalize_version(&self, version: &str) -> Option<String> {
    if self
      .versions
      .iter()
      .filter_map(|v| v.as_ref())
      .any(|v| v == version)
    {
      return Some(version.to_string());
    }
    let target = parse_leading_numeric(version)?;
    let mut best_value = None;
    let mut best_version = None;
    for known in self.versions.iter().filter_map(|v| v.as_ref()) {
      if known.eq(version) {
        return Some(known.clone());
      }
      if let Some(val) = parse_leading_numeric(known) {
        if val <= target {
          let replace = match best_value {
            Some(current) => val > current,
            None => true,
          };
          if replace {
            best_value = Some(val);
            best_version = Some(known.clone());
          }
        }
      }
    }
    best_version
  }
}

impl PrefixDB {
  fn normalize_targets(&self, targets: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(targets.len());
    for target in targets {
      if let Some((browser, version)) = target.split_once(' ') {
        if let Some(agent) = self.agents.get(browser) {
          if let Some(norm) = agent.normalize_version(version) {
            out.push(format!("{} {}", browser, norm));
            continue;
          }
        }
      }
      out.push(target.clone());
    }
    out
  }
}

#[derive(Clone, Debug)]
pub struct AutoprefixerData {
  pub add: HashMap<String, Vec<String>>,
  pub remove: HashMap<String, Vec<String>>,
  pub value_map: HashMap<String, Vec<ValueRule>>,
  pub selector_map: HashMap<String, Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct PrefixedDecl {
  pub property: String,
  pub value: String,
}

impl AutoprefixerData {
  pub fn load_with_targets() -> Option<(Self, Vec<String>)> {
    let db = PrefixDB::load()?;
    let targets = resolve_browserslist_targets();
    let data = Self::from_db(db, targets.clone());
    Some((data, targets))
  }

  pub fn load() -> Option<Self> {
    Self::load_with_targets().map(|(data, _)| data)
  }

  pub fn from_db(db: PrefixDB, targets: Vec<String>) -> Self {
    let (add, remove) = db.select_add_remove(&targets);
    let value_map = build_value_prefix_map(&db, &add);
    let selector_map = build_selector_prefix_map(&db, &add);
    AutoprefixerData {
      add,
      remove,
      value_map,
      selector_map,
    }
  }

  pub fn property_prefixes(&self, prop: &str) -> Option<&Vec<String>> {
    self.add.get(prop)
  }

  pub fn selector_prefixes(&self, selector: &str) -> Option<&Vec<String>> {
    self.selector_map.get(selector)
  }

  pub fn prefixed_value_rules(&self, prop: &str) -> Option<&Vec<ValueRule>> {
    self.value_map.get(prop)
  }

  pub fn prefixed_decls(&self, prop: &str, value: &str) -> Vec<PrefixedDecl> {
    let mut out: Vec<PrefixedDecl> = Vec::new();
    if let Some(prefixes) = self.property_prefixes(prop) {
      for prefix in prefixes {
        out.push(PrefixedDecl {
          property: format!("{}{}", prefix, prop),
          value: value.to_string(),
        });
      }
    }
    out.extend(self.prefixed_value_decls(prop, value));
    out
  }

  pub fn prefixed_value_decls(&self, prop: &str, value: &str) -> Vec<PrefixedDecl> {
    let mut out: Vec<PrefixedDecl> = Vec::new();
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rules) = self.prefixed_value_rules(prop) {
      for rule in rules {
        if rule.matches_keyword(&lower) {
          for prefix in &rule.prefixes {
            if let Some(pref_value) = rule.prefixed_value(prefix) {
              out.push(PrefixedDecl {
                property: prop.to_string(),
                value: pref_value,
              });
            }
          }
        }
      }
    } else if matches!(prop, "width" | "min-width" | "max-width")
      && (trimmed.eq_ignore_ascii_case("fit-content") || lower.contains("fit-content"))
    {
      out.push(PrefixedDecl {
        property: prop.to_string(),
        value: "-moz-fit-content".to_string(),
      });
    }

    out
  }

  pub fn placeholder_selector_variants(&self, selector: &str) -> Vec<String> {
    if !selector.contains("::placeholder") {
      return Vec::new();
    }
    let mut out = Vec::new();
    if let Some(prefixes) = self.selector_prefixes("::placeholder") {
      for prefix in prefixes {
        for variant in placeholder_variants(prefix) {
          let replaced = selector.replace("::placeholder", &variant);
          if replaced != selector {
            out.push(replaced);
          }
        }
      }
    }
    out
  }
}

#[derive(Debug, Default)]
pub struct VendorAutoprefixer;

pub fn vendor_autoprefixer() -> VendorAutoprefixer {
  VendorAutoprefixer
}

impl Plugin for VendorAutoprefixer {
  fn name(&self) -> &'static str {
    "autoprefixer"
  }

  fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
    let Some((config, targets)) = AutoprefixerData::load_with_targets() else {
      return;
    };
    if std::env::var("COMPILED_CLI_TRACE").is_ok() {
      eprintln!("[autoprefixer] plugin start targets={}", targets.join(", "));
    }
    let tracing = std::env::var("COMPILED_CLI_TRACE").is_ok();
    if tracing {
      eprintln!(
        "[autoprefixer] targets={} add_keys={} remove_keys={}",
        targets.join(", "),
        config.add.len(),
        config.remove.len()
      );
      eprintln!(
        "[autoprefixer] trace includes user-select? {} placeholder? {}",
        config.add.contains_key("user-select"),
        config.add.contains_key("::placeholder")
      );
    }

    // Pass 1: @keyframes and @viewport
    let mut new_rules: Vec<Rule> = Vec::new();
    for rule in std::mem::take(&mut stylesheet.rules) {
      match rule {
        Rule::AtRule(at) => {
          if let Some(name) = at_rule_name(&at.name) {
            if name == "keyframes" {
              if let Some(prefixes) = config.add.get("@keyframes") {
                for pref in prefixes {
                  let mut cloned = (*at.clone()).clone();
                  cloned.name = at_rule_name_from_str(&format!("{}keyframes", pref));
                  new_rules.push(Rule::AtRule(Box::new(cloned)));
                }
              }
            }
          }
          new_rules.push(Rule::AtRule(at));
        }
        Rule::QualifiedRule(mut qr) => {
          apply_decl_prefixing(&mut qr, &config);
          new_rules.push(Rule::QualifiedRule(qr));
        }
        other => new_rules.push(other),
      }
    }
    stylesheet.rules = new_rules;
  }
}

pub fn resolve_browserslist_targets() -> Vec<String> {
  // Use oxc_browserslist to resolve defaults (no repo-specific options requested)
  let mut out = Vec::new();
  let opts = oxc_browserslist::Opts::default();
  match oxc_browserslist::execute(&opts) {
    Ok(list) => {
      for item in list {
        out.push(item.to_string());
      }
    }
    Err(_) => {}
  }
  out
}

fn at_rule_name(name: &AtRuleName) -> Option<&str> {
  match name {
    AtRuleName::Ident(i) => Some(&i.value),
    AtRuleName::DashedIdent(i) => Some(&i.value),
  }
}

fn at_rule_name_from_str(name: &str) -> AtRuleName {
  AtRuleName::Ident(swc_core::css::ast::Ident {
    value: name.into(),
    raw: None,
    span: Default::default(),
  })
}

fn decl_prop(name: &DeclarationName) -> &str {
  match name {
    DeclarationName::Ident(i) => &i.value,
    DeclarationName::DashedIdent(i) => &i.value,
  }
}

fn clone_decl_with_prop(decl: &Declaration, prop: String) -> Declaration {
  let mut d = decl.clone();
  d.name = DeclarationName::Ident(swc_core::css::ast::Ident {
    value: prop.into(),
    raw: None,
    span: Default::default(),
  });
  d
}

fn apply_decl_prefixing(rule: &mut QualifiedRule, config: &AutoprefixerData) {
  let mut new_block: Vec<ComponentValue> = Vec::new();
  let push_decl = |vec: &mut Vec<ComponentValue>, decl: Declaration| {
    vec.push(ComponentValue::Declaration(Box::new(decl)));
    // Mirror PostCSS codegen: declarations inside a rule are separated by a semicolon delimiter.
    vec.push(ComponentValue::Delimiter(Box::new(
      swc_core::css::ast::Delimiter {
        span: Default::default(),
        value: swc_core::css::ast::DelimiterValue::Semicolon,
      },
    )));
  };
  for node in std::mem::take(&mut rule.block.value) {
    if let ComponentValue::Declaration(decl_box) = &node {
      let decl = &**decl_box;
      let prop = decl_prop(&decl.name).to_string();

      // Property-level: add prefixed properties
      if let Some(prefixes) = config.property_prefixes(&prop) {
        for pref in prefixes {
          let prefixed_prop = format!("{}{}", pref, prop);
          if std::env::var("COMPILED_CLI_TRACE").is_ok() {
            eprintln!("[autoprefixer] add-prop {}", prefixed_prop);
          }
          push_decl(&mut new_block, clone_decl_with_prop(decl, prefixed_prop));
        }
      }

      // Value-level prefixes for intrinsic keywords or known values.
      let value_prefixed = maybe_prefix_value(&prop, &decl.value, config);
      'inject: for v in value_prefixed {
        if let DeclarationName::Ident(id) = &v.name {
          let target_prop = id.value.to_string();
          // Serialize v.value to string for simple equality compare
          let already = new_block.iter().any(|c| match c {
            ComponentValue::Declaration(existing) => {
              let ep = decl_prop(&existing.name);
              if ep != target_prop {
                return false;
              }
              // Look for -moz-fit-content exact ident
              if existing.value.len() == 1 {
                if let ComponentValue::Ident(i2) = &existing.value[0] {
                  if i2.value.eq("-moz-fit-content") {
                    return true;
                  }
                }
              }
              false
            }
            _ => false,
          });
          if already {
            continue 'inject;
          }
        }
        push_decl(&mut new_block, v);
      }

      // Removal of old props (basic): skip emitting outdated prefixed properties
      if let Some(rem) = config.remove.get(&prop) {
        let n = decl_prop(&decl.name);
        if n.starts_with("-") && rem.iter().any(|p| n.starts_with(p)) {
          if std::env::var("COMPILED_CLI_TRACE").is_ok() {
            eprintln!("[autoprefixer] drop-old-prop {}", n);
          }
          // skip this node (old prefixed property slated for removal)
          continue;
        }
      }
    }
    new_block.push(node);
  }
  rule.block.value = new_block;
}

fn maybe_prefix_value(
  prop: &str,
  value: &Vec<ComponentValue>,
  config: &AutoprefixerData,
) -> Vec<Declaration> {
  let mut out: Vec<Declaration> = Vec::new();
  if let Some(rules) = config.prefixed_value_rules(prop) {
    if value.len() == 1 {
      if let ComponentValue::Ident(ident) = &value[0] {
        let lower = ident.value.to_ascii_lowercase();
        for rule in rules {
          if rule.matches_keyword(&lower) {
            for prefix in &rule.prefixes {
              if let Some(pref_value) = rule.prefixed_value(prefix) {
                out.push(Declaration {
                  name: DeclarationName::Ident(swc_core::css::ast::Ident {
                    value: prop.into(),
                    raw: None,
                    span: Default::default(),
                  }),
                  value: vec![make_ident(&pref_value)],
                  important: None,
                  span: Default::default(),
                });
              }
            }
          }
        }
      }
    }
  } else {
    // Special-case: width/min/max fit-content -> -moz-fit-content
    if matches!(prop, "width" | "min-width" | "max-width") {
      if value.len() == 1 {
        if let ComponentValue::Ident(i) = &value[0] {
          let low = i.value.to_ascii_lowercase();
          if i.value.trim().eq_ignore_ascii_case("fit-content") || low.contains("fit-content") {
            let d = Declaration {
              name: DeclarationName::Ident(swc_core::css::ast::Ident {
                value: prop.into(),
                raw: None,
                span: Default::default(),
              }),
              value: vec![make_ident("-moz-fit-content")],
              important: None,
              span: Default::default(),
            };
            out.push(d);
          }
        }
      }
    }
  }

  // display:flex and inline-flex basic prefixes. Mirror Babel/PostCSS autoprefixer:
  // it emits two additional declarations before the unprefixed one.
  if prop == "display" && !value.is_empty() {
    if let ComponentValue::Ident(i) = &value[0] {
      let v = i.value.to_ascii_lowercase();
      // Only add display prefixes when autoprefixer data says to.
      let needs_flex = config.add.get("display-flex");
      if needs_flex.is_some() {
        match v.as_str() {
          "flex" => {
            // 2009 spec
            out.push(Declaration {
              name: DeclarationName::Ident(swc_core::css::ast::Ident {
                value: prop.into(),
                raw: None,
                span: Default::default(),
              }),
              value: vec![make_ident("-webkit-box")],
              important: None,
              span: Default::default(),
            });
            // 2012 spec
            out.push(Declaration {
              name: DeclarationName::Ident(swc_core::css::ast::Ident {
                value: prop.into(),
                raw: None,
                span: Default::default(),
              }),
              value: vec![make_ident("-ms-flexbox")],
              important: None,
              span: Default::default(),
            });
          }
          "inline-flex" => {
            out.push(Declaration {
              name: DeclarationName::Ident(swc_core::css::ast::Ident {
                value: prop.into(),
                raw: None,
                span: Default::default(),
              }),
              value: vec![make_ident("-webkit-inline-box")],
              important: None,
              span: Default::default(),
            });
            out.push(Declaration {
              name: DeclarationName::Ident(swc_core::css::ast::Ident {
                value: prop.into(),
                raw: None,
                span: Default::default(),
              }),
              value: vec![make_ident("-ms-inline-flexbox")],
              important: None,
              span: Default::default(),
            });
          }
          _ => {}
        }
      }
    }
  }

  // TODO: Full value-level prefixing (gradients, grid, imageset, cross-fade, etc.)
  let _ = config; // silence unused parameter until full port lands
  out
}

pub(crate) fn build_value_prefix_map(
  db: &PrefixDB,
  add: &HashMap<String, Vec<String>>,
) -> HashMap<String, Vec<ValueRule>> {
  let mut map: HashMap<String, Vec<ValueRule>> = HashMap::new();
  for (name, entry) in &db.entries {
    let Some(props) = &entry.props else {
      continue;
    };
    let Some(kind) = classify_value_kind(name) else {
      continue;
    };
    let Some(prefixes) = add.get(name) else {
      continue;
    };
    let rule = ValueRule {
      keyword: name.clone(),
      prefixes: prefixes.clone(),
      kind,
    };
    for prop in props {
      map.entry(prop.clone()).or_default().push(rule.clone());
    }
  }
  map
}

fn classify_value_kind(name: &str) -> Option<ValueKind> {
  match name {
    "min-content" | "max-content" => Some(ValueKind::Plain),
    "fit-content" => Some(ValueKind::FitContent),
    "fill" | "fill-available" | "stretch" => Some(ValueKind::Stretch),
    _ => None,
  }
}

fn strip_note_prefix(prefix: &str) -> &str {
  prefix.split_once(' ').map(|(p, _)| p).unwrap_or(prefix)
}

impl ValueRule {
  pub(crate) fn matches_keyword(&self, value: &str) -> bool {
    value.eq_ignore_ascii_case(&self.keyword)
  }

  pub(crate) fn prefixed_value(&self, prefix: &str) -> Option<String> {
    let base = strip_note_prefix(prefix);
    match self.kind {
      ValueKind::Plain => Some(format!("{}{}", base, self.keyword)),
      ValueKind::FitContent => {
        if base == "-moz-" {
          Some("-moz-fit-content".to_string())
        } else {
          None
        }
      }
      ValueKind::Stretch => {
        if base == "-moz-" {
          Some("-moz-available".to_string())
        } else if base == "-webkit-" {
          Some("-webkit-fill-available".to_string())
        } else {
          None
        }
      }
    }
  }
}

pub(crate) fn build_selector_prefix_map(
  db: &PrefixDB,
  add: &HashMap<String, Vec<String>>,
) -> HashMap<String, Vec<String>> {
  let mut map = HashMap::new();
  for (name, entry) in &db.entries {
    if entry.selector {
      if let Some(prefixes) = add.get(name) {
        map.insert(name.clone(), prefixes.clone());
      }
    }
  }
  map
}

pub(crate) fn placeholder_variants(prefix: &str) -> Vec<String> {
  let base = strip_note_prefix(prefix);
  let is_old = prefix.contains("old");
  if base == "-webkit-" {
    vec!["::-webkit-input-placeholder".to_string()]
  } else if base == "-moz-" {
    if is_old {
      vec![":-moz-placeholder".to_string()]
    } else {
      vec!["::-moz-placeholder".to_string()]
    }
  } else if base == "-ms-" {
    if is_old {
      vec![":-ms-input-placeholder".to_string()]
    } else {
      vec!["::-ms-input-placeholder".to_string()]
    }
  } else {
    let trimmed = base.trim_matches('-');
    vec![format!("::{}placeholder", trimmed)]
  }
}
