use csscolorparser::Color;
use swc_core::atoms::Atom;
use swc_core::css::ast::{ComponentValue, Declaration, Ident, QualifiedRule, Rule, Stylesheet, Token};
use swc_core::css::codegen::{writer::basic::BasicCssWriter, CodeGenerator, CodegenConfig, Emit};

use super::super::transform::{Plugin, TransformContext};

#[derive(Debug, Default, Clone, Copy)]
pub struct ColorMin;

impl Plugin for ColorMin {
    fn name(&self) -> &'static str {
        "postcss-colormin"
    }

    fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
        for rule in &mut stylesheet.rules {
            minimize_rule(rule);
        }
    }
}

pub fn colormin() -> ColorMin {
    ColorMin
}

fn minimize_rule(rule: &mut Rule) {
    match rule {
        Rule::QualifiedRule(rule) => minimize_declarations(&mut rule.block.value),
        Rule::AtRule(at) => {
            if let Some(block) = &mut at.block {
                minimize_declarations(&mut block.value);
            }
        }
        Rule::ListOfComponentValues(list) => minimize_components(&mut list.children),
    }
}

fn minimize_components(values: &mut [ComponentValue]) {
    for v in values {
        match v {
            ComponentValue::Declaration(decl) => minimize_declaration(decl),
            ComponentValue::QualifiedRule(rule) => minimize_declarations(&mut rule.block.value),
            ComponentValue::AtRule(at) => {
                if let Some(block) = &mut at.block {
                    minimize_declarations(&mut block.value);
                }
            }
            ComponentValue::SimpleBlock(block) => minimize_declarations(&mut block.value),
            ComponentValue::ListOfComponentValues(list) => minimize_components(&mut list.children),
            ComponentValue::Function(fun) => minimize_components(&mut fun.value),
            _ => {}
        }
    }
}

fn minimize_declarations(values: &mut Vec<ComponentValue>) {
    for v in values.iter_mut() {
        if let ComponentValue::Declaration(decl) = v {
            minimize_declaration(decl);
        }
    }
}

fn minimize_declaration(decl: &mut Declaration) {
    if decl.value.is_empty() { return; }
    // Walk tokens and replace color idents with shorter hex where beneficial
    for comp in decl.value.iter_mut() {
        if let ComponentValue::PreservedToken(tok) = comp {
            if let Token::Ident { value, .. } = &tok.token {
                let name = value.to_string();
                if let Ok(color) = Color::parse(&name) {
                    let hex = color_to_short_hex(&color);
                    if hex.len() < name.len() {
                        // Replace token with Ident node to ensure proper serialization
                        *comp = ComponentValue::Ident(Ident { span: decl.span, value: Atom::from(hex.as_str()), raw: None });
                    }
                }
            }
        }
    }
}

fn color_to_short_hex(color: &Color) -> String {
    let (r, g, b, _a) = color.to_rgba8();
    // Hex with lowercase
    if r % 17 == 0 && g % 17 == 0 && b % 17 == 0 {
        // #rgb shorthand
        return format!("#{:x}{:x}{:x}", r / 17, g / 17, b / 17);
    }
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}
