use regex::Captures;

use super::types::{
  BasicMatchInfo, ComparisonOperator, LengthInfo, MatchComponents, ParsedAtRule, Property,
};

const REM_SIZE: f64 = 16.0;

fn convert_min_max_media_query(captures: &Captures<'_>) -> Option<(Property, ComparisonOperator)> {
  let property = captures.name("property")?.as_str();

  match property {
    "min-width" => Some((Property::Width, ComparisonOperator::GreaterEqual)),
    "min-device-width" => Some((Property::DeviceWidth, ComparisonOperator::GreaterEqual)),
    "max-width" => Some((Property::Width, ComparisonOperator::LessEqual)),
    "max-device-width" => Some((Property::DeviceWidth, ComparisonOperator::LessEqual)),
    "min-height" => Some((Property::Height, ComparisonOperator::GreaterEqual)),
    "min-device-height" => Some((Property::DeviceHeight, ComparisonOperator::GreaterEqual)),
    "max-height" => Some((Property::Height, ComparisonOperator::LessEqual)),
    "max-device-height" => Some((Property::DeviceHeight, ComparisonOperator::LessEqual)),
    _ => panic!(
      "Unexpected property '{}' found when sorting media queries",
      property
    ),
  }
}

fn reverse_operator(operator: ComparisonOperator) -> ComparisonOperator {
  operator.reverse()
}

fn get_property(captures: &Captures<'_>) -> Option<Property> {
  let property = captures.name("property")?.as_str();

  match property {
    "width" => Some(Property::Width),
    "height" => Some(Property::Height),
    "device-width" => Some(Property::DeviceWidth),
    "device-height" => Some(Property::DeviceHeight),
    _ => panic!(
      "Unexpected property '{}' found when sorting media queries.",
      property
    ),
  }
}

fn parse_operator(captures: &Captures<'_>, group: &str) -> Option<ComparisonOperator> {
  let operator = captures.name(group)?.as_str();

  match operator {
    "<=" => Some(ComparisonOperator::LessEqual),
    "=" => Some(ComparisonOperator::Equal),
    ">=" => Some(ComparisonOperator::GreaterEqual),
    "<" => Some(ComparisonOperator::Less),
    ">" => Some(ComparisonOperator::Greater),
    _ => panic!(
      "Unexpected comparison operator '{}' found when sorting media queries.",
      operator
    ),
  }
}

fn get_operator(captures: &Captures<'_>, reverse: bool, group: &str) -> Option<ComparisonOperator> {
  parse_operator(captures, group).map(|operator| {
    if reverse {
      reverse_operator(operator)
    } else {
      operator
    }
  })
}

fn get_length_info(captures: &Captures<'_>) -> Option<LengthInfo> {
  let length_match = captures.name("length")?;
  let length = length_match.as_str();

  if length == "0" {
    return Some(LengthInfo { length: 0.0 });
  }

  let unit = captures.name("lengthUnit")?.as_str();
  let value: f64 = length.parse().ok()?;

  let converted = match unit {
    "ch" | "ex" => value * 0.5 * REM_SIZE,
    "em" | "rem" => value * REM_SIZE,
    "px" => value,
    _ => panic!("Unrecognized length unit {}. This is a Compiled bug!", unit),
  };

  Some(LengthInfo { length: converted })
}

fn get_basic_match_info(captures: &Captures<'_>) -> Option<BasicMatchInfo> {
  let matched = captures.get(0)?;
  let start = matched.start();

  if start == 0 {
    return None;
  }

  Some(BasicMatchInfo {
    match_text: matched.as_str().to_string(),
    index: start,
  })
}

fn assemble_match(
  captures: &Captures<'_>,
  property: Property,
  comparison_operator: ComparisonOperator,
) -> Option<MatchComponents> {
  let match_info = get_basic_match_info(captures)?;
  let length = get_length_info(captures)?;

  Some(MatchComponents {
    property,
    comparison_operator,
    length: length.length,
    match_info,
  })
}

pub fn parse_min_max_syntax(captures: &Captures<'_>) -> Option<ParsedAtRule> {
  let (property, comparison_operator) = convert_min_max_media_query(captures)?;
  assemble_match(captures, property, comparison_operator).map(ParsedAtRule::from)
}

pub fn parse_reversed_range_syntax(captures: &Captures<'_>) -> Option<ParsedAtRule> {
  let property = get_property(captures)?;
  let comparison_operator = get_operator(captures, true, "operator")?;
  assemble_match(captures, property, comparison_operator).map(ParsedAtRule::from)
}

pub fn parse_range_syntax(captures: &Captures<'_>) -> Option<ParsedAtRule> {
  let property = get_property(captures)?;
  let comparison_operator = get_operator(captures, false, "operator")?;
  assemble_match(captures, property, comparison_operator).map(ParsedAtRule::from)
}
