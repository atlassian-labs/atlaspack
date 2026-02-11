use std::sync::Arc;

use csscolorparser::Color;
use swc_core::atoms::Atom;
use swc_core::common::{FileName, SourceMap, Span, input::StringInput};
use swc_core::css::ast::{
  Angle, ComponentValue, Declaration, DeclarationName, Dimension, Flex, FunctionName, IdSelector,
  Ident, Length, Resolution, Rule, Stylesheet, Time, Token, TokenAndSpan, UnknownDimension,
};
use swc_core::css::codegen::{CodeGenerator, CodegenConfig, Emit, writer::basic::BasicCssWriter};
use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

#[derive(Clone, Debug, PartialEq)]
pub enum LonghandDeclaration {
  Replace {
    prop: String,
    value: Vec<ComponentValue>,
  },
  KeepOriginal,
}

impl LonghandDeclaration {
  pub fn replace(prop: impl Into<String>, value: Vec<ComponentValue>) -> Self {
    LonghandDeclaration::Replace {
      prop: prop.into(),
      value,
    }
  }

  pub fn keep_original() -> Self {
    LonghandDeclaration::KeepOriginal
  }
}

#[derive(Clone, Debug, Default)]
pub struct ValuesRoot {
  pub nodes: Vec<ValueNode>,
}

impl ValuesRoot {
  pub fn from_components(values: &[ComponentValue]) -> Self {
    let mut nodes: Vec<ValueNode> = Vec::new();

    for component in values {
      if is_whitespace_component(component) {
        continue;
      }

      nodes.push(ValueNode::new(vec![component.clone()]));
    }

    ValuesRoot { nodes }
  }

  pub fn contains_var_function(&self) -> bool {
    self.nodes.iter().any(|node| node.is_var_function())
  }

  pub fn is_empty(&self) -> bool {
    self.nodes.is_empty()
  }
}

#[derive(Clone, Debug)]
pub struct ValueNode {
  components: Vec<ComponentValue>,
  kind: ValueNodeKind,
}

impl ValueNode {
  pub fn new(components: Vec<ComponentValue>) -> Self {
    let kind = determine_value_kind(&components);
    ValueNode { components, kind }
  }

  pub fn clone_components(&self) -> Vec<ComponentValue> {
    self.components.clone()
  }

  pub fn to_css_string(&self) -> Option<String> {
    serialize_component_values(&self.components)
  }

  pub fn is_var_function(&self) -> bool {
    matches!(
        &self.kind,
        ValueNodeKind::Function { name } if name.eq_ignore_ascii_case("var")
    )
  }

  pub fn as_word(&self) -> Option<&str> {
    match &self.kind {
      ValueNodeKind::Word(word) => Some(word.as_str()),
      _ => None,
    }
  }

  pub fn equals_word(&self, expected: &str) -> bool {
    match &self.kind {
      ValueNodeKind::Word(word) => word == expected,
      _ => false,
    }
  }

  pub fn is_word_in(&self, values: &[&str]) -> bool {
    values.iter().any(|value| self.equals_word(value))
  }

  pub fn is_word(&self) -> bool {
    matches!(self.kind, ValueNodeKind::Word(_))
  }

  pub fn is_numeric_without_unit(&self) -> bool {
    matches!(
      self.kind,
      ValueNodeKind::Number { .. } | ValueNodeKind::Integer { .. }
    )
  }

  pub fn is_unitless_zero(&self) -> bool {
    match &self.kind {
      ValueNodeKind::Number { raw, value } => {
        raw.as_deref() == Some("0") || (raw.is_none() && *value == 0.0)
      }
      ValueNodeKind::Integer { raw, value } => {
        raw.as_deref() == Some("0") || (raw.is_none() && *value == 0)
      }
      _ => false,
    }
  }

  pub fn numeric_string(&self) -> Option<String> {
    match &self.kind {
      ValueNodeKind::Number { raw, value } => Some(raw.clone().unwrap_or_else(|| {
        if value.fract() == 0.0 {
          format!("{:.0}", value)
        } else {
          value.to_string()
        }
      })),
      ValueNodeKind::Integer { raw, value } => {
        Some(raw.clone().unwrap_or_else(|| value.to_string()))
      }
      _ => None,
    }
  }

  pub fn is_function(&self) -> bool {
    matches!(self.kind, ValueNodeKind::Function { .. })
  }

  pub fn function_name(&self) -> Option<&str> {
    match &self.kind {
      ValueNodeKind::Function { name } => Some(name.as_str()),
      _ => None,
    }
  }

  pub fn is_color(&self) -> bool {
    match &self.kind {
      ValueNodeKind::Color => true,
      ValueNodeKind::Word(word) => is_color_keyword(word),
      ValueNodeKind::Function { name } => is_color_function(name),
      _ => false,
    }
  }

  pub fn is_width(&self) -> bool {
    match &self.kind {
      ValueNodeKind::Dimension { unit } => is_width_unit(unit),
      ValueNodeKind::Percentage => true,
      ValueNodeKind::Word(word) => is_width_keyword(word),
      ValueNodeKind::Function { .. } => true,
      _ => false,
    }
  }

  pub fn width_string(&self) -> Option<String> {
    match &self.kind {
      ValueNodeKind::Dimension { .. }
      | ValueNodeKind::Percentage
      | ValueNodeKind::Word(_)
      | ValueNodeKind::Function { .. } => self.to_css_string(),
      _ => None,
    }
  }
}

#[derive(Clone, Debug)]
enum ValueNodeKind {
  Word(String),
  Number { raw: Option<String>, value: f64 },
  Integer { raw: Option<String>, value: i64 },
  Dimension { unit: String },
  Percentage,
  Function { name: String },
  Color,
  Other,
}

fn determine_value_kind(components: &[ComponentValue]) -> ValueNodeKind {
  if components.len() != 1 {
    return ValueNodeKind::Other;
  }

  match &components[0] {
    ComponentValue::Ident(ident) => ValueNodeKind::Word(ident.value.to_string()),
    ComponentValue::DashedIdent(ident) => ValueNodeKind::Word(ident.value.to_string()),
    ComponentValue::Str(value) => ValueNodeKind::Word(value.value.to_string()),
    ComponentValue::Number(number) => ValueNodeKind::Number {
      raw: number.raw.as_ref().map(|raw| raw.to_string()),
      value: number.value,
    },
    ComponentValue::Integer(integer) => ValueNodeKind::Integer {
      raw: integer.raw.as_ref().map(|raw| raw.to_string()),
      value: integer.value,
    },
    ComponentValue::Dimension(dimension) => ValueNodeKind::Dimension {
      unit: extract_dimension_unit(dimension),
    },
    ComponentValue::Percentage(_) => ValueNodeKind::Percentage,
    ComponentValue::Function(function) => match &function.name {
      FunctionName::Ident(ident) => ValueNodeKind::Function {
        name: ident.value.to_string(),
      },
      FunctionName::DashedIdent(ident) => ValueNodeKind::Function {
        name: ident.value.to_string(),
      },
    },
    ComponentValue::Color(_) => ValueNodeKind::Color,
    ComponentValue::IdSelector(id) => id_selector_to_word(id),
    _ => ValueNodeKind::Other,
  }
}

fn id_selector_to_word(id: &IdSelector) -> ValueNodeKind {
  let value = format!("#{}", id.text.value);
  ValueNodeKind::Word(value)
}

fn extract_dimension_unit(dimension: &Dimension) -> String {
  match dimension {
    Dimension::Length(Length { unit, .. })
    | Dimension::Angle(Angle { unit, .. })
    | Dimension::Time(Time { unit, .. }) => unit.value.to_string(),
    Dimension::Frequency(freq) => freq.unit.value.to_string(),
    Dimension::Resolution(Resolution { unit, .. }) => unit.value.to_string(),
    Dimension::Flex(Flex { unit, .. }) => unit.value.to_string(),
    Dimension::UnknownDimension(UnknownDimension { unit, .. }) => unit.value.to_string(),
  }
}

fn is_color_keyword(word: &str) -> bool {
  const SPECIAL: [&str; 2] = ["transparent", "currentcolor"];
  if SPECIAL
    .iter()
    .any(|candidate| candidate.eq_ignore_ascii_case(word))
  {
    return true;
  }

  Color::from_html(word).is_ok()
}

fn is_color_function(name: &str) -> bool {
  matches!(
    name.to_ascii_lowercase().as_str(),
    "rgb"
      | "rgba"
      | "hsl"
      | "hsla"
      | "hwb"
      | "lab"
      | "lch"
      | "color"
      | "oklab"
      | "oklch"
      | "device-cmyk"
  )
}

fn is_width_unit(unit: &str) -> bool {
  matches!(
    unit,
    "%"
      | "cap"
      | "ch"
      | "cm"
      | "em"
      | "ex"
      | "fr"
      | "ic"
      | "in"
      | "lh"
      | "mm"
      | "pc"
      | "pt"
      | "px"
      | "Q"
      | "rem"
      | "rlh"
      | "vb"
      | "vh"
      | "vi"
      | "vmax"
      | "vmin"
      | "vw"
  )
}

fn is_width_keyword(word: &str) -> bool {
  matches!(
    word,
    "auto"
      | "min-content"
      | "max-content"
      | "fit-content"
      | "inherit"
      | "initial"
      | "unset"
      | "revert"
      | "revert-layer"
  )
}

fn is_whitespace_component(component: &ComponentValue) -> bool {
  matches!(
      component,
      ComponentValue::PreservedToken(token)
          if matches!(token.token, Token::WhiteSpace { .. })
  )
}

pub fn serialize_component_values(values: &[ComponentValue]) -> Option<String> {
  let mut output = String::new();
  {
    let writer = BasicCssWriter::new(&mut output, None, Default::default());
    let mut generator = CodeGenerator::new(writer, CodegenConfig { minify: false });
    for component in values {
      if generator.emit(component).is_err() {
        return None;
      }
    }
  }
  Some(output)
}

pub fn parse_value_to_components(value: &str) -> Vec<ComponentValue> {
  if value.trim().is_empty() {
    return Vec::new();
  }

  let css = format!("a{{a:{value};}}");
  let cm: Arc<SourceMap> = Default::default();
  let fm = cm.new_source_file(FileName::Custom("value.css".into()).into(), css);
  let mut errors = vec![];

  if let Ok(mut stylesheet) = parse_string_input::<Stylesheet>(
    StringInput::from(&*fm),
    None,
    ParserConfig::default(),
    &mut errors,
  ) {
    if let Some(Rule::QualifiedRule(mut rule)) = stylesheet.rules.pop() {
      if let Some(ComponentValue::Declaration(mut declaration)) = rule.block.value.pop() {
        let mut result: Vec<ComponentValue> = Vec::new();
        let mut cursor = 0usize;
        let value_str = value;
        let value_len = value_str.len();

        for component in declaration.value.drain(..) {
          // Capture leading whitespace between tokens so round-tripping preserves spacing.
          let whitespace_start = cursor;
          while cursor < value_len {
            let ch = value_str[cursor..].chars().next().unwrap();
            if ch.is_whitespace() {
              cursor += ch.len_utf8();
            } else {
              break;
            }
          }

          if cursor > whitespace_start {
            let whitespace = &value_str[whitespace_start..cursor];
            if !whitespace.is_empty() {
              result.push(ComponentValue::PreservedToken(Box::new(TokenAndSpan {
                span: Span::default(),
                token: Token::WhiteSpace {
                  value: Atom::from(whitespace),
                },
              })));
            }
          }

          let component_text = serialize_component_values(&[component.clone()]).unwrap_or_default();
          if !component_text.is_empty() {
            let safe_cursor = cursor.min(value_len);
            if let Some(position) = value_str[safe_cursor..].find(&component_text) {
              cursor = safe_cursor + position + component_text.len();
            } else {
              cursor = safe_cursor
                .saturating_add(component_text.len())
                .min(value_len);
            }
          }

          result.push(component);
        }

        // Trailing whitespace at the end of the value string.
        let whitespace_start = cursor;
        while cursor < value_len {
          let ch = value_str[cursor..].chars().next().unwrap();
          if ch.is_whitespace() {
            cursor += ch.len_utf8();
          } else {
            break;
          }
        }

        if cursor > whitespace_start {
          let whitespace = &value_str[whitespace_start..cursor];
          if !whitespace.is_empty() {
            result.push(ComponentValue::PreservedToken(Box::new(TokenAndSpan {
              span: Span::default(),
              token: Token::WhiteSpace {
                value: Atom::from(whitespace),
              },
            })));
          }
        }

        return result;
      }
    }
  }

  Vec::new()
}

pub fn declaration_name_from(prop: &str) -> DeclarationName {
  if prop.starts_with("--") {
    DeclarationName::DashedIdent(swc_core::css::ast::DashedIdent {
      span: Span::default(),
      value: Atom::from(prop),
      raw: None,
    })
  } else {
    DeclarationName::Ident(Ident {
      span: Span::default(),
      value: Atom::from(prop),
      raw: None,
    })
  }
}

pub fn declaration_property_name(name: &DeclarationName) -> String {
  match name {
    DeclarationName::Ident(ident) => ident.value.to_string(),
    DeclarationName::DashedIdent(ident) => ident.value.to_string(),
  }
}

pub fn clone_with_new_name(
  declaration: &Declaration,
  prop: &str,
  value: Vec<ComponentValue>,
) -> Declaration {
  let mut cloned = declaration.clone();
  cloned.name = declaration_name_from(prop);
  cloned.value = value;
  cloned
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn splits_whitespace_into_nodes() {
    let components = parse_value_to_components("10px 20px");
    let root = ValuesRoot::from_components(&components);
    assert_eq!(root.nodes.len(), 2);
    assert_eq!(root.nodes[0].to_css_string().unwrap(), "10px");
    assert_eq!(root.nodes[1].to_css_string().unwrap(), "20px");
  }
}
