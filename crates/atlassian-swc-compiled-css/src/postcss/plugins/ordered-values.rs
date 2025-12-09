use std::collections::{HashMap, HashSet};

use once_cell::sync::Lazy;
use swc_core::css::ast::{ComponentValue, Declaration, Rule, Stylesheet};

use super::super::transform::{Plugin, TransformContext};
use crate::postcss::plugins::expand_shorthands::types::{
    declaration_property_name, parse_value_to_components, serialize_component_values,
};
use crate::postcss::value_parser::{
    new_node, parse_unit, parse_value, stringify_nodes, DivNode, Node, NodeData, ParsedValue,
    SpaceNode,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct OrderedValues;

impl Plugin for OrderedValues {
    fn name(&self) -> &'static str {
        "postcss-ordered-values"
    }

    fn run(&self, stylesheet: &mut Stylesheet, _ctx: &mut TransformContext<'_>) {
        let mut cache = HashMap::new();
        normalize_stylesheet(stylesheet, &mut cache);
    }
}

pub fn ordered_values() -> OrderedValues {
    OrderedValues
}

type ValueCache = HashMap<String, String>;

fn normalize_stylesheet(stylesheet: &mut Stylesheet, cache: &mut ValueCache) {
    normalize_rules(&mut stylesheet.rules, cache);
}

fn normalize_rules(rules: &mut Vec<Rule>, cache: &mut ValueCache) {
    for rule in rules.iter_mut() {
        match rule {
            Rule::QualifiedRule(rule) => {
                normalize_component_values(&mut rule.block.value, cache);
            }
            Rule::AtRule(at_rule) => {
                if let Some(block) = &mut at_rule.block {
                    normalize_component_values(&mut block.value, cache);
                }
            }
            Rule::ListOfComponentValues(list) => {
                normalize_component_values(&mut list.children, cache);
            }
        }
    }
}

fn normalize_component_values(values: &mut Vec<ComponentValue>, cache: &mut ValueCache) {
    for value in values.iter_mut() {
        match value {
            ComponentValue::Declaration(declaration) => {
                normalize_declaration(declaration, cache);
            }
            ComponentValue::QualifiedRule(rule) => {
                normalize_component_values(&mut rule.block.value, cache);
            }
            ComponentValue::AtRule(at_rule) => {
                if let Some(block) = &mut at_rule.block {
                    normalize_component_values(&mut block.value, cache);
                }
            }
            ComponentValue::SimpleBlock(block) => {
                normalize_component_values(&mut block.value, cache);
            }
            ComponentValue::ListOfComponentValues(list) => {
                normalize_component_values(&mut list.children, cache);
            }
            ComponentValue::Function(function) => {
                normalize_component_values(&mut function.value, cache);
            }
            ComponentValue::KeyframeBlock(block) => {
                normalize_component_values(&mut block.block.value, cache);
            }
            _ => {}
        }
    }
}

fn normalize_declaration(declaration: &mut Declaration, cache: &mut ValueCache) {
    let Some(original_value) = serialize_value_with_spacing(&declaration.value) else {
        return;
    };

    if original_value.trim().is_empty() {
        return;
    }
    if let Some(cached) = cache.get(&original_value) {
        declaration.value = parse_value_to_components(cached);
        return;
    }

    let property = declaration_property_name(&declaration.name);
    let normalized_prop = vendor_unprefixed(&property.to_ascii_lowercase());
    let parsed = parse_value(&original_value);

    let abort = should_abort(&parsed);
    if parsed.len() < 2 || abort {
        cache.insert(original_value.clone(), original_value.clone());
        declaration.value = parse_value_to_components(&original_value);
        return;
    }

    let result = match processor_for_property(&normalized_prop) {
        Some(Processor::Animation) => Some(normalize_animation(&parsed)),
        Some(Processor::Border) => Some(normalize_border(&parsed)),
        Some(Processor::BoxShadow) => normalize_box_shadow(&parsed),
        Some(Processor::Columns) => Some(normalize_columns(&parsed)),
        Some(Processor::FlexFlow) => Some(normalize_flex_flow(&parsed)),
        Some(Processor::Transition) => Some(normalize_transition(&parsed)),
        Some(Processor::ListStyle) => Some(normalize_list_style(&parsed)),
        Some(Processor::GridAutoFlow) => Some(normalize_grid_auto_flow(&parsed)),
        Some(Processor::GridGap) => Some(normalize_grid_gap(&parsed)),
        Some(Processor::GridLine) => normalize_grid_line(&parsed),
        None => None,
    };

    let current_serialized = serialize_component_values(&declaration.value).unwrap_or_default();

    if let Some(output) = result {
        cache.insert(original_value.clone(), output.clone());
        // Always set parsed components from the normalized output to preserve explicit spaces
        declaration.value = parse_value_to_components(&output);
    } else {
        cache.insert(original_value.clone(), original_value.clone());
        if current_serialized != original_value {
            declaration.value = parse_value_to_components(&original_value);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Processor {
    Animation,
    Border,
    BoxShadow,
    Columns,
    FlexFlow,
    Transition,
    ListStyle,
    GridAutoFlow,
    GridGap,
    GridLine,
}

fn processor_for_property(property: &str) -> Option<Processor> {
    match property {
        "animation" => Some(Processor::Animation),
        "outline" => Some(Processor::Border),
        "box-shadow" => Some(Processor::BoxShadow),
        "flex-flow" => Some(Processor::FlexFlow),
        "list-style" => Some(Processor::ListStyle),
        "transition" => Some(Processor::Transition),
        "columns" => Some(Processor::Columns),
        "grid-auto-flow" => Some(Processor::GridAutoFlow),
        "grid-column-gap" | "grid-row-gap" => Some(Processor::GridGap),
        "grid-column" | "grid-row" | "grid-row-start" | "grid-row-end" | "grid-column-start"
        | "grid-column-end" => Some(Processor::GridLine),
        "border"
        | "border-block"
        | "border-inline"
        | "border-block-end"
        | "border-block-start"
        | "border-inline-end"
        | "border-inline-start"
        | "border-top"
        | "border-right"
        | "border-bottom"
        | "border-left"
        | "column-rule" => Some(Processor::Border),
        _ => None,
    }
}

fn vendor_unprefixed(value: &str) -> String {
    if let Some(rest) = value.strip_prefix('-') {
        if let Some(idx) = rest.find('-') {
            return rest[idx + 1..].to_string();
        }
    }
    value.to_string()
}

fn should_abort(parsed: &ParsedValue) -> bool {
    let mut abort = false;
    parsed.walk(|node| {
        if abort {
            return false;
        }
        match &*node.borrow() {
            NodeData::Comment(_) => {
                abort = true;
                false
            }
            NodeData::Function(func) => {
                let name = func.value.to_ascii_lowercase();
                if matches!(name.as_str(), "var" | "env" | "constant") {
                    abort = true;
                    false
                } else {
                    true
                }
            }
            NodeData::Word(word) => {
                if word.value.contains("___CSS_LOADER_IMPORT___") {
                    abort = true;
                    false
                } else {
                    true
                }
            }
            _ => true,
        }
    });
    abort
}

fn get_arguments(parsed: &ParsedValue) -> Vec<Vec<Node>> {
    let mut list: Vec<Vec<Node>> = vec![Vec::new()];
    for node in parsed.nodes() {
        if matches!(&*node.borrow(), NodeData::Div(_)) {
            list.push(Vec::new());
        } else {
            if let Some(current) = list.last_mut() {
                current.push(node.clone());
            }
        }
    }
    list
}

fn add_space_node() -> Node {
    new_node(NodeData::Space(SpaceNode {
        value: " ".to_string(),
        source_index: 0,
        source_end_index: 0,
    }))
}

struct AnimationState {
    name: Vec<Node>,
    duration: Vec<Node>,
    timing_function: Vec<Node>,
    delay: Vec<Node>,
    iteration_count: Vec<Node>,
    direction: Vec<Node>,
    fill_mode: Vec<Node>,
    play_state: Vec<Node>,
}

impl AnimationState {
    fn new() -> Self {
        Self {
            name: Vec::new(),
            duration: Vec::new(),
            timing_function: Vec::new(),
            delay: Vec::new(),
            iteration_count: Vec::new(),
            direction: Vec::new(),
            fill_mode: Vec::new(),
            play_state: Vec::new(),
        }
    }
}

static ANIMATION_FUNCTIONS: Lazy<HashSet<&'static str>> =
    Lazy::new(|| HashSet::from(["steps", "cubic-bezier", "frames"]));

static ANIMATION_KEYWORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "ease",
        "ease-in",
        "ease-in-out",
        "ease-out",
        "linear",
        "step-end",
        "step-start",
    ])
});

static ANIMATION_DIRECTIONS: Lazy<HashSet<&'static str>> =
    Lazy::new(|| HashSet::from(["normal", "reverse", "alternate", "alternate-reverse"]));

static ANIMATION_FILL_MODES: Lazy<HashSet<&'static str>> =
    Lazy::new(|| HashSet::from(["none", "forwards", "backwards", "both"]));

static ANIMATION_PLAY_STATES: Lazy<HashSet<&'static str>> =
    Lazy::new(|| HashSet::from(["running", "paused"]));

static TIME_UNITS: Lazy<HashSet<&'static str>> = Lazy::new(|| HashSet::from(["ms", "s"]));

fn normalize_animation(parsed: &ParsedValue) -> String {
    let mut list: Vec<Vec<Node>> = Vec::new();

    for arg in get_arguments(parsed) {
        let mut state = AnimationState::new();

        for node in arg {
            match &*node.borrow() {
                NodeData::Space(_) => {}
                NodeData::Word(word) => {
                    let lower = word.value.to_ascii_lowercase();
                    if ANIMATION_KEYWORDS.contains(lower.as_str()) {
                        push_once(&mut state.timing_function, &node);
                    } else if ANIMATION_DIRECTIONS.contains(lower.as_str()) {
                        push_once(&mut state.direction, &node);
                    } else if ANIMATION_FILL_MODES.contains(lower.as_str()) {
                        push_once(&mut state.fill_mode, &node);
                    } else if ANIMATION_PLAY_STATES.contains(lower.as_str()) {
                        push_once(&mut state.play_state, &node);
                    } else if is_time_literal(&word.value) {
                        if state.duration.is_empty() {
                            push_once(&mut state.duration, &node);
                        } else {
                            push_once(&mut state.delay, &node);
                        }
                    } else if is_iteration_count(&word.value) {
                        push_once(&mut state.iteration_count, &node);
                    } else {
                        state.name.push(node.clone());
                        state.name.push(add_space_node());
                    }
                }
                NodeData::Function(func) => {
                    let lower = func.value.to_ascii_lowercase();
                    if ANIMATION_FUNCTIONS.contains(lower.as_str()) {
                        push_once(&mut state.timing_function, &node);
                    } else if is_time_literal(&stringify_nodes(&func.nodes)) {
                        if state.duration.is_empty() {
                            push_once(&mut state.duration, &node);
                        } else {
                            push_once(&mut state.delay, &node);
                        }
                    } else {
                        state.name.push(node.clone());
                        state.name.push(add_space_node());
                    }
                }
                _ => {
                    state.name.push(node.clone());
                    state.name.push(add_space_node());
                }
            }
        }

        list.push(
            [
                state.name,
                state.duration,
                state.timing_function,
                state.delay,
                state.iteration_count,
                state.direction,
                state.fill_mode,
                state.play_state,
            ]
            .into_iter()
            .flatten()
            .collect(),
        );
    }

    flatten_and_stringify(list)
}

fn normalize_transition(parsed: &ParsedValue) -> String {
    let mut list: Vec<Vec<Node>> = Vec::new();

    for arg in get_arguments(parsed) {
        let mut property: Vec<Node> = Vec::new();
        let mut time1: Vec<Node> = Vec::new();
        let mut time2: Vec<Node> = Vec::new();
        let mut timing: Vec<Node> = Vec::new();

        for node in arg {
            match &*node.borrow() {
                NodeData::Space(_) => {}
                NodeData::Word(word) => {
                    let lower = word.value.to_ascii_lowercase();
                    if is_time_literal(&word.value) {
                        if time1.is_empty() {
                            push_once(&mut time1, &node);
                        } else {
                            push_once(&mut time2, &node);
                        }
                    } else if TRANSITION_TIMING_KEYWORDS.contains(lower.as_str()) {
                        push_once(&mut timing, &node);
                    } else {
                        property.push(node.clone());
                        property.push(add_space_node());
                    }
                }
                NodeData::Function(func) => {
                    let lower = func.value.to_ascii_lowercase();
                    if TRANSITION_TIMING_FUNCTIONS.contains(lower.as_str()) {
                        push_once(&mut timing, &node);
                    } else {
                        property.push(node.clone());
                        property.push(add_space_node());
                    }
                }
                _ => {
                    property.push(node.clone());
                    property.push(add_space_node());
                }
            }
        }

        list.push(
            [property, time1, timing, time2]
                .into_iter()
                .flatten()
                .collect(),
        );
    }

    flatten_and_stringify(list)
}

static TRANSITION_TIMING_FUNCTIONS: Lazy<HashSet<&'static str>> =
    Lazy::new(|| HashSet::from(["steps", "cubic-bezier"]));

static TRANSITION_TIMING_KEYWORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "ease",
        "linear",
        "ease-in",
        "ease-out",
        "ease-in-out",
        "step-start",
        "step-end",
    ])
});

fn normalize_box_shadow(parsed: &ParsedValue) -> Option<String> {
    let mut list: Vec<Vec<Node>> = Vec::new();
    let mut abort = false;

    for arg in get_arguments(parsed) {
        let mut inset: Vec<Node> = Vec::new();
        let mut lengths: Vec<Node> = Vec::new();
        let mut color: Vec<Node> = Vec::new();

        for node in arg {
            if abort {
                break;
            }
            match &*node.borrow() {
                NodeData::Space(_) => {}
                NodeData::Function(func) => {
                    let lower = vendor_unprefixed(&func.value.to_ascii_lowercase());
                    if MATH_FUNCTIONS.contains(lower.as_str()) {
                        abort = true;
                        break;
                    }
                    lengths.push(node.clone());
                    lengths.push(add_space_node());
                }
                NodeData::Word(word) => {
                    let lower = word.value.to_ascii_lowercase();
                    if lower == "inset" {
                        inset.push(node.clone());
                        inset.push(add_space_node());
                    } else if parse_unit(&word.value).is_some() {
                        lengths.push(node.clone());
                        lengths.push(add_space_node());
                    } else {
                        color.push(node.clone());
                        color.push(add_space_node());
                    }
                }
                _ => {
                    color.push(node.clone());
                    color.push(add_space_node());
                }
            }
        }

        if abort {
            break;
        }

        let mut combined = Vec::new();
        combined.extend(inset);
        combined.extend(lengths);
        combined.extend(color);
        list.push(combined);
    }

    if abort {
        return None;
    }

    let serialized = flatten_and_stringify(list);
    Some(collapse_function_commas(&serialized))
}

static MATH_FUNCTIONS: Lazy<HashSet<&'static str>> =
    Lazy::new(|| HashSet::from(["calc", "clamp", "max", "min"]));

fn normalize_border(parsed: &ParsedValue) -> String {
    let mut width = String::new();
    let mut style = String::new();
    let mut color = String::new();

    parsed.walk(|node| match &*node.borrow() {
        NodeData::Word(word) => {
            let lower = word.value.to_ascii_lowercase();
            if BORDER_STYLES.contains(lower.as_str()) {
                style = append_token(std::mem::take(&mut style), &word.value);
                return false;
            }
            if BORDER_WIDTH_KEYWORDS.contains(lower.as_str()) || parse_unit(&word.value).is_some() {
                width = append_token(std::mem::take(&mut width), &word.value);
                return false;
            }
            color = append_token(std::mem::take(&mut color), &word.value);
            false
        }
        NodeData::Function(func) => {
            let lower = func.value.to_ascii_lowercase();
            if MATH_FUNCTIONS.contains(lower.as_str()) {
                width = append_token(
                    std::mem::take(&mut width),
                    &stringify_nodes(&[node.clone()]),
                );
            } else {
                color = append_token(
                    std::mem::take(&mut color),
                    &stringify_nodes(&[node.clone()]),
                );
            }
            false
        }
        _ => true,
    });

    format!("{} {} {}", width.trim(), style.trim(), color.trim())
        .trim()
        .to_string()
}

static BORDER_WIDTH_KEYWORDS: Lazy<HashSet<&'static str>> =
    Lazy::new(|| HashSet::from(["thin", "medium", "thick"]));

static BORDER_STYLES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "none", "auto", "hidden", "dotted", "dashed", "solid", "double", "groove", "ridge",
        "inset", "outset",
    ])
});

fn normalize_columns(parsed: &ParsedValue) -> String {
    let mut widths: Vec<String> = Vec::new();
    let mut others: Vec<String> = Vec::new();

    parsed.walk(|node| {
        if let NodeData::Word(word) = &*node.borrow() {
            if let Some(unit) = parse_unit(&word.value) {
                if !unit.unit.is_empty() {
                    widths.push(word.value.clone());
                } else {
                    others.push(word.value.clone());
                }
            } else {
                others.push(word.value.clone());
            }
        }
        true
    });

    if widths.len() == 1 && others.len() == 1 {
        format!("{} {}", widths[0].trim_start(), others[0].trim_start())
    } else {
        parsed.to_string()
    }
}

fn normalize_flex_flow(parsed: &ParsedValue) -> String {
    let mut direction = String::new();
    let mut wrap = String::new();

    parsed.walk(|node| {
        if let NodeData::Word(word) = &*node.borrow() {
            let lower = word.value.to_ascii_lowercase();
            if FLEX_DIRECTIONS.contains(lower.as_str()) {
                direction = word.value.clone();
            } else if FLEX_WRAP.contains(lower.as_str()) {
                wrap = word.value.clone();
            }
        }
        true
    });

    format!("{} {}", direction.trim(), wrap.trim())
        .trim()
        .to_string()
}

static FLEX_DIRECTIONS: Lazy<HashSet<&'static str>> =
    Lazy::new(|| HashSet::from(["row", "row-reverse", "column", "column-reverse"]));

static FLEX_WRAP: Lazy<HashSet<&'static str>> =
    Lazy::new(|| HashSet::from(["nowrap", "wrap", "wrap-reverse"]));

fn normalize_grid_auto_flow(parsed: &ParsedValue) -> String {
    let mut front = String::new();
    let mut back = String::new();
    let mut should = false;

    parsed.walk(|node| {
        if let NodeData::Word(word) = &*node.borrow() {
            let lower = word.value.trim().to_ascii_lowercase();
            if lower == "dense" {
                should = true;
                back = word.value.clone();
            } else if lower == "row" || lower == "column" {
                should = true;
                front = word.value.clone();
            } else {
                should = false;
            }
        }
        true
    });

    if should {
        format!("{} {}", front.trim(), back.trim())
    } else {
        parsed.to_string()
    }
}

fn normalize_grid_gap(parsed: &ParsedValue) -> String {
    let mut front = String::new();
    let mut back = String::new();
    let mut should = false;

    parsed.walk(|node| {
        if let NodeData::Word(word) = &*node.borrow() {
            if word.value == "normal" {
                should = true;
                front = word.value.clone();
            } else {
                back.push(' ');
                back.push_str(&word.value);
            }
        }
        true
    });

    if should {
        format!("{} {}", front.trim(), back.trim())
    } else {
        parsed.to_string()
    }
}

fn normalize_grid_line(parsed: &ParsedValue) -> Option<String> {
    let value = parsed.to_string();
    let parts: Vec<&str> = value.split('/').collect();
    if parts.len() > 1 {
        let normalized: Vec<String> = parts
            .into_iter()
            .map(|segment| normalize_grid_segment(segment))
            .collect();
        Some(join_grid_values(&normalized))
    } else {
        let normalized: Vec<String> = parts
            .into_iter()
            .map(|segment| normalize_grid_segment(segment))
            .collect();
        Some(normalized.join(","))
    }
}

fn normalize_grid_segment(segment: &str) -> String {
    let mut front = String::new();
    let mut back = String::new();

    for part in segment.trim().split_whitespace() {
        if part == "span" {
            front = part.to_string();
        } else {
            if !back.is_empty() {
                back.push(' ');
            }
            back.push_str(part);
        }
    }

    format!("{} {}", front.trim(), back.trim())
        .trim()
        .to_string()
}

fn join_grid_values(values: &[String]) -> String {
    values.join(" / ").trim().to_string()
}

fn normalize_list_style(parsed: &ParsedValue) -> String {
    let mut r#type: Vec<String> = Vec::new();
    let mut position: Vec<String> = Vec::new();
    let mut image: Vec<String> = Vec::new();
    let mut has_none = false;

    parsed.walk(|node| match &*node.borrow() {
        NodeData::Word(word) => {
            let value = word.value.clone();
            if LIST_STYLE_TYPES.contains(value.as_str()) {
                r#type.push(value);
            } else if LIST_STYLE_POSITIONS.contains(value.as_str()) {
                position.push(value);
            } else if value == "none" {
                if has_none {
                    image.push(value);
                } else {
                    has_none = true;
                    r#type.push("none".into());
                }
            } else {
                r#type.push(value);
            }
            true
        }
        NodeData::Function(_) => {
            image.push(stringify_nodes(&[node.clone()]));
            false
        }
        _ => true,
    });

    format!(
        "{} {} {}",
        r#type.join(" ").trim(),
        position.join(" ").trim(),
        image.join(" ").trim()
    )
    .trim()
    .to_string()
}

static LIST_STYLE_TYPES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    HashSet::from([
        "afar",
        "amharic",
        "amharic-abegede",
        "arabic-indic",
        "armenian",
        "asterisks",
        "bengali",
        "binary",
        "cambodian",
        "circle",
        "cjk-decimal",
        "cjk-earthly-branch",
        "cjk-heavenly-stem",
        "cjk-ideographic",
        "decimal",
        "decimal-leading-zero",
        "devanagari",
        "disc",
        "disclosure-closed",
        "disclosure-open",
        "ethiopic",
        "ethiopic-abegede",
        "ethiopic-abegede-am-et",
        "ethiopic-abegede-gez",
        "ethiopic-abegede-ti-er",
        "ethiopic-abegede-ti-et",
        "ethiopic-halehame",
        "ethiopic-halehame-aa-er",
        "ethiopic-halehame-aa-et",
        "ethiopic-halehame-am",
        "ethiopic-halehame-am-et",
        "ethiopic-halehame-gez",
        "ethiopic-halehame-om-et",
        "ethiopic-halehame-sid-et",
        "ethiopic-halehame-so-et",
        "ethiopic-halehame-ti-er",
        "ethiopic-halehame-ti-et",
        "ethiopic-halehame-tig",
        "ethiopic-numeric",
        "footnotes",
        "georgian",
        "gujarati",
        "gurmukhi",
        "hangul",
        "hangul-consonant",
        "hebrew",
        "hiragana",
        "hiragana-iroha",
        "japanese-formal",
        "japanese-informal",
        "kannada",
        "katakana",
        "katakana-iroha",
        "khmer",
        "korean-hangul-formal",
        "korean-hanja-formal",
        "korean-hanja-informal",
        "lao",
        "lower-alpha",
        "lower-armenian",
        "lower-greek",
        "lower-hexadecimal",
        "lower-latin",
        "lower-norwegian",
        "lower-roman",
        "malayalam",
        "mongolian",
        "myanmar",
        "octal",
        "oriya",
        "oromo",
        "persian",
        "sidama",
        "simp-chinese-formal",
        "simp-chinese-informal",
        "somali",
        "square",
        "string",
        "symbols",
        "tamil",
        "telugu",
        "thai",
        "tibetan",
        "tigre",
        "tigrinya-er",
        "tigrinya-er-abegede",
        "tigrinya-et",
        "tigrinya-et-abegede",
        "trad-chinese-formal",
        "trad-chinese-informal",
        "upper-alpha",
        "upper-armenian",
        "upper-greek",
        "upper-hexadecimal",
        "upper-latin",
        "upper-norwegian",
        "upper-roman",
        "urdu",
    ])
});

static LIST_STYLE_POSITIONS: Lazy<HashSet<&'static str>> =
    Lazy::new(|| HashSet::from(["inside", "outside"]));

fn push_once(target: &mut Vec<Node>, node: &Node) {
    if target.is_empty() {
        target.push(node.clone());
        target.push(add_space_node());
    }
}

fn append_token(mut existing: String, value: &str) -> String {
    if !existing.is_empty() {
        existing.push(' ');
    }
    existing.push_str(value);
    existing
}

fn serialize_value_with_spacing(components: &[ComponentValue]) -> Option<String> {
    let mut result = String::new();

    for component in components {
        let piece = serialize_component_values(&[component.clone()])?;
        if piece.is_empty() {
            continue;
        }

        if needs_space_between(&result, &piece) {
            result.push(' ');
        }

        result.push_str(&piece);
    }

    Some(result)
}

fn needs_space_between(existing: &str, next: &str) -> bool {
    if existing.is_empty() {
        return false;
    }

    let prev = existing.chars().rev().find(|ch| !ch.is_whitespace());
    let next_char = next.chars().find(|ch| !ch.is_whitespace());

    match (prev, next_char) {
        (Some(','), Some(_)) => true,
        (Some('/'), Some(_)) => true,
        (Some(prev), Some(next)) => {
            let prev_word = is_word_like_char(prev);
            let next_word = is_word_like_char(next);
            (prev_word && next_word) || (matches!(prev, ')' | '%') && next_word)
        }
        _ => false,
    }
}

fn is_word_like_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')
}

fn flatten_and_stringify(values: Vec<Vec<Node>>) -> String {
    let mut nodes: Vec<Node> = Vec::new();

    let total = values.len();

    for (index, mut arg) in values.into_iter().enumerate() {
        if index != total - 1 {
            if let Some(last) = arg.last_mut() {
                *last.borrow_mut() = NodeData::Div(DivNode {
                    before: String::new(),
                    value: ",".to_string(),
                    after: String::new(),
                    source_index: 0,
                    source_end_index: 0,
                });
            }
        } else if matches!(arg.last(), Some(node) if matches!(&*node.borrow(), NodeData::Space(_)))
        {
            arg.pop();
        }

        nodes.extend(arg);
    }

    stringify_nodes(&nodes)
}

fn collapse_function_commas(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut depth = 0usize;
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '(' => {
                depth += 1;
                result.push(ch);
            }
            ')' => {
                if depth > 0 {
                    depth -= 1;
                }
                result.push(ch);
            }
            ',' if depth > 0 => {
                result.push(ch);
                while let Some(next) = chars.peek() {
                    if next.is_whitespace() {
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            _ => result.push(ch),
        }
    }

    result
}

fn is_time_literal(value: &str) -> bool {
    if let Some(unit) = parse_unit(value) {
        return TIME_UNITS.contains(unit.unit.as_str());
    }
    false
}

fn is_iteration_count(value: &str) -> bool {
    if value == "infinite" {
        return true;
    }
    parse_unit(value)
        .map(|parsed| parsed.unit.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use swc_core::common::{input::StringInput, FileName, SourceMap};
    use swc_core::css::parser::{parse_string_input, parser::ParserConfig};

    use crate::postcss::transform::{TransformContext, TransformCssOptions};

    fn parse_stylesheet(css: &str) -> Stylesheet {
        let cm: Arc<SourceMap> = Default::default();
        let fm = cm.new_source_file(FileName::Custom("inline.css".into()).into(), css.into());
        let mut errors = vec![];
        parse_string_input::<Stylesheet>(
            StringInput::from(&*fm),
            None,
            ParserConfig::default(),
            &mut errors,
        )
        .expect("stylesheet")
    }

    fn first_declaration_value(stylesheet: &Stylesheet) -> Option<String> {
        for rule in &stylesheet.rules {
            if let Rule::QualifiedRule(rule) = rule {
                for value in &rule.block.value {
                    if let ComponentValue::Declaration(declaration) = value {
                        return serialize_component_values(&declaration.value);
                    }
                }
            }
        }
        None
    }

    fn run_plugin(css: &str) -> Stylesheet {
        let mut stylesheet = parse_stylesheet(css);
        let options = TransformCssOptions::default();
        let mut ctx = TransformContext::new(&options);
        OrderedValues.run(&mut stylesheet, &mut ctx);
        stylesheet
    }

    #[test]
    fn reorders_border_values() {
        let stylesheet = run_plugin(".a { border: solid red 1px; }");
        let value = first_declaration_value(&stylesheet).expect("value");
        assert_eq!(value, "1px solid red");
    }

    #[test]
    fn reorders_transition_values() {
        let stylesheet = run_plugin(".a { transition: opacity ease-in 1s 2s; }");
        let value = first_declaration_value(&stylesheet).expect("value");
        assert_eq!(value, "opacity 1s ease-in 2s");
    }

    #[test]
    fn reorders_animation_values() {
        let stylesheet = run_plugin(".a { animation: ease-in 1s slide 2; }");
        let value = first_declaration_value(&stylesheet).expect("value");
        assert_eq!(value, "slide 1s ease-in 2");
    }

    #[test]
    fn normalizes_box_shadow() {
        let stylesheet = run_plugin(".a { box-shadow: inset 10px 20px rgba(0,0,0,0.5); }");
        let value = first_declaration_value(&stylesheet).expect("value");
        assert_eq!(value, "inset 10px 20px rgba(0, 0, 0, 0.5)");
    }

    #[test]
    fn normalizes_columns() {
        let stylesheet = run_plugin(".a { columns: 200px auto; }");
        let value = first_declaration_value(&stylesheet).expect("value");
        assert_eq!(value, "200px auto");
    }

    #[test]
    fn normalizes_list_style() {
        let stylesheet = run_plugin(".a { list-style: inside none url(image.png); }");
        let value = first_declaration_value(&stylesheet).expect("value");
        assert_eq!(value, "none inside url(image.png)");
    }
}
