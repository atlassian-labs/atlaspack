use once_cell::sync::Lazy;
use regex::Regex;

use super::parsers::{parse_min_max_syntax, parse_range_syntax, parse_reversed_range_syntax};
use super::types::ParsedAtRule;

const COMPARISON_OPERATORS: &str = r"(?P<operator>(?:<=?)|(?:>=?)|=)\s*";
const PROPERTY: &str = r"(?:(?P<property>((?:min|max)-)?(?:device-)?(?:width|height))\s*)";
const COLON: &str = r"(?P<colon>:\s*)";
const LENGTH: &str = r"(?P<length>-?\d*\.?\d+)(?P<lengthUnit>ch|em|ex|px|rem)?\s*";

static SITUATION_ONE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(&format!("{}{}{}", PROPERTY, COLON, LENGTH)).expect("invalid situation one regex")
});
static SITUATION_TWO: Lazy<Regex> = Lazy::new(|| {
  Regex::new(&format!("{}{}{}", LENGTH, COMPARISON_OPERATORS, PROPERTY))
    .expect("invalid situation two regex")
});
static SITUATION_THREE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(&format!("{}{}{}", PROPERTY, COMPARISON_OPERATORS, LENGTH))
    .expect("invalid situation three regex")
});

pub fn parse_media_query(params: &str) -> Vec<ParsedAtRule> {
  let mut parsed_matches: Vec<ParsedAtRule> = Vec::new();

  for captures in SITUATION_ONE.captures_iter(params) {
    if let Some(parsed) = parse_min_max_syntax(&captures) {
      parsed_matches.push(parsed);
    }
  }

  for captures in SITUATION_TWO.captures_iter(params) {
    if let Some(parsed) = parse_reversed_range_syntax(&captures) {
      parsed_matches.push(parsed);
    }
  }

  for captures in SITUATION_THREE.captures_iter(params) {
    if let Some(parsed) = parse_range_syntax(&captures) {
      parsed_matches.push(parsed);
    }
  }

  parsed_matches.sort_by(|a, b| a.match_info.index.cmp(&b.match_info.index));
  parsed_matches
}
