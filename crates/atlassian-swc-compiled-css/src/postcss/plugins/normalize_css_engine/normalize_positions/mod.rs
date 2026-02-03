use crate::postcss::value_parser as vp;
use postcss as pc;

/// Direction keywords for background-position.
const DIRECTION_KEYWORDS: [&str; 5] = ["top", "right", "bottom", "left", "center"];

/// Check if a value is a direction keyword.
fn is_direction_keyword(keyword: &str) -> bool {
  DIRECTION_KEYWORDS.contains(&keyword.to_ascii_lowercase().as_str())
}

/// Check if a keyword is a horizontal position (left/right).
fn is_horizontal_keyword(keyword: &str) -> bool {
  matches!(keyword.to_ascii_lowercase().as_str(), "left" | "right")
}

/// Check if a keyword is a vertical position (top/bottom).
fn is_vertical_keyword(keyword: &str) -> bool {
  matches!(keyword.to_ascii_lowercase().as_str(), "top" | "bottom")
}

/// Convert horizontal keywords to percentage equivalents.
/// This follows the postcss-normalize-positions behavior.
fn horizontal_to_percentage(keyword: &str) -> Option<&'static str> {
  match keyword.to_ascii_lowercase().as_str() {
    "left" => Some("0"),
    "right" => Some("100%"),
    "center" => Some("50%"),
    _ => None,
  }
}

/// Convert vertical keywords to percentage equivalents.
fn vertical_to_percentage(keyword: &str) -> Option<&'static str> {
  match keyword.to_ascii_lowercase().as_str() {
    "top" => Some("0"),
    "bottom" => Some("100%"),
    _ => None,
  }
}

/// Normalize a zero value with unit to just "0".
/// Only affects values that are exactly "0" with a unit (e.g., "0px" -> "0"),
/// NOT values that contain "0px" as a substring (e.g., "300px" stays "300px").
fn normalize_zero_value(value: &str) -> String {
  // Use the value parser's unit extraction to check if this is a zero value
  if let Some(parsed) = vp::unit::unit(value) {
    // If the numeric part is "0" (possibly with leading zeros like "00" or "0.0"),
    // strip the unit
    let num: f64 = parsed.number.parse().unwrap_or(f64::NAN);
    if num == 0.0 {
      return "0".to_string();
    }
  }
  value.to_string()
}

/// Check if a value is a dimension (number with unit).
fn is_dimension(value: &str) -> bool {
  if let Some(parsed) = vp::unit::unit(value) {
    return !parsed.unit.is_empty();
  }
  false
}

/// Check if a value is a number (without unit).
fn is_number(value: &str) -> bool {
  value.parse::<f64>().is_ok()
}

/// Check if a value is a position keyword (direction keyword, dimension, or number).
fn is_position_keyword(value: &str) -> bool {
  is_direction_keyword(value) || is_dimension(value) || is_number(value)
}

/// Normalize a single position value in a comma-separated list.
/// This follows the postcss-normalize-positions algorithm exactly:
///
/// 1. Single value or second value is 'center':
///    - Collapse to just the first value, converted using horizontal map
///    - `left center` → `0`, `right center` → `100%`, `center center` → `50%` (→ `center`)
///
/// 2. First value is 'center' and second is a direction keyword:
///    - Collapse to just the second value
///    - If second is horizontal (left/right), convert it: `center left` → `0`, `center right` → `100%`
///    - If second is vertical (top/bottom), keep as keyword: `center top` → `top`, `center bottom` → `bottom`
///
/// 3. Horizontal + vertical keyword pairs:
///    - If first is horizontal and second is vertical: convert both to percentages
///    - If first is vertical and second is horizontal: swap and convert
/// Check if a string looks like a bare unit (e.g., "%", "px", "em")
fn is_bare_unit(value: &str) -> bool {
  matches!(
    value.to_ascii_lowercase().as_str(),
    "%"
      | "px"
      | "em"
      | "rem"
      | "vh"
      | "vw"
      | "vmin"
      | "vmax"
      | "deg"
      | "rad"
      | "grad"
      | "turn"
      | "s"
      | "ms"
      | "cm"
      | "mm"
      | "in"
      | "pt"
      | "pc"
      | "ex"
      | "ch"
      | "fr"
  )
}

/// Pre-process nodes to merge number + bare unit pairs that were incorrectly split.
/// For example: ["-100", "%"] -> ["-100%"]
fn merge_number_unit_pairs(nodes: &[vp::Node]) -> Vec<vp::Node> {
  let mut result = Vec::new();
  let mut i = 0;

  while i < nodes.len() {
    if let vp::Node::Word { value: num_val } = &nodes[i] {
      // Check if this looks like a number (including negative numbers)
      let looks_like_number = num_val.parse::<f64>().is_ok();

      // Check if next node is a bare unit
      if looks_like_number && i + 1 < nodes.len() {
        if let vp::Node::Word { value: unit_val } = &nodes[i + 1] {
          if is_bare_unit(unit_val) {
            // Merge them
            result.push(vp::Node::Word {
              value: format!("{}{}", num_val, unit_val),
            });
            i += 2;
            continue;
          }
        }
      }
    }

    result.push(nodes[i].clone());
    i += 1;
  }

  result
}

fn normalize_single_position(value: &str) -> String {
  let parsed = vp::parse(value);
  // Pre-process to merge incorrectly split number+unit pairs (e.g., "-100" + "%" -> "-100%")
  let merged_nodes = merge_number_unit_pairs(&parsed.nodes);
  let mut words: Vec<(usize, String)> = Vec::new();

  // Collect word nodes with their indices
  for (idx, node) in merged_nodes.iter().enumerate() {
    if let vp::Node::Word { value: word_val } = node {
      let v = normalize_zero_value(word_val);
      if is_position_keyword(&v) {
        words.push((idx, v));
      }
    }
  }

  // If no position keywords found or more than 2, just normalize zeros and return
  if words.is_empty() || words.len() > 2 {
    let mut result = String::new();
    for node in &merged_nodes {
      match node {
        vp::Node::Word { value: v } => {
          result.push_str(&normalize_zero_value(v));
        }
        _ => {
          result.push_str(&vp::stringify(&[node.clone()]));
        }
      }
    }
    return result;
  }

  let first = &words[0].1;
  let first_lower = first.to_ascii_lowercase();

  // Case 1: Single value
  if words.len() == 1 {
    // Use horizontal map (left -> 0, right -> 100%, center -> 50%)
    // Note: center IS converted to 50% to match Babel's cssnano behavior
    if let Some(pct) = horizontal_to_percentage(&first_lower) {
      return pct.to_string();
    }
    return normalize_zero_value(first);
  }

  // Two values
  let second = &words[1].1;
  let second_lower = second.to_ascii_lowercase();

  // Case 2: Second value is 'center' - collapse to just first value
  // Note: center center becomes 50% to match Babel's cssnano behavior
  if second_lower == "center" {
    if let Some(pct) = horizontal_to_percentage(&first_lower) {
      return pct.to_string();
    }
    return normalize_zero_value(first);
  }

  // Case 3: First value is 'center' and second is a direction keyword
  if first_lower == "center" && is_direction_keyword(&second_lower) {
    // If second is horizontal, convert it
    if is_horizontal_keyword(&second_lower) {
      if let Some(pct) = horizontal_to_percentage(&second_lower) {
        return pct.to_string();
      }
    }
    // If second is vertical (top/bottom), keep it as keyword (NOT converted to percentage)
    // This matches the original postcss-normalize-positions behavior
    return second.to_string();
  }

  // Case 4: Horizontal + vertical pairs
  if is_horizontal_keyword(&first_lower) && is_vertical_keyword(&second_lower) {
    // Already in correct order: horizontal vertical -> convert both
    let x = horizontal_to_percentage(&first_lower).unwrap_or(first);
    let y = vertical_to_percentage(&second_lower).unwrap_or(second);
    return format!("{} {}", x, y);
  }

  if is_vertical_keyword(&first_lower) && is_horizontal_keyword(&second_lower) {
    // Swap: vertical horizontal -> horizontal vertical
    let x = horizontal_to_percentage(&second_lower).unwrap_or(second);
    let y = vertical_to_percentage(&first_lower).unwrap_or(first);
    return format!("{} {}", x, y);
  }

  // Default: normalize zeros and return both values
  let v1 = normalize_zero_value(first);
  let v2 = normalize_zero_value(second);
  format!("{} {}", v1, v2)
}

/// Normalize the entire background-position value, handling comma-separated lists.
fn normalize_pair(value: &str) -> String {
  // Split by commas and normalize each position individually
  let parsed = vp::parse(value);

  // Collect positions separated by commas
  let mut positions: Vec<String> = Vec::new();
  let mut current_pos: Vec<vp::Node> = Vec::new();

  for node in &parsed.nodes {
    if let vp::Node::Div { value: div_val, .. } = node {
      if div_val == "," {
        // End of current position, normalize it
        if !current_pos.is_empty() {
          let pos_str = vp::stringify(&current_pos);
          positions.push(normalize_single_position(&pos_str));
          current_pos.clear();
        }
        continue;
      }
    }
    current_pos.push(node.clone());
  }

  // Don't forget the last position
  if !current_pos.is_empty() {
    let pos_str = vp::stringify(&current_pos);
    positions.push(normalize_single_position(&pos_str));
  }

  // If no commas, just normalize the whole thing
  if positions.is_empty() {
    return normalize_single_position(value);
  }

  // Join with ", " (comma-space) to match Babel's cssnano postcss-normalize-positions behavior.
  // This preserves the original whitespace pattern for consistent hashing.
  positions.join(", ")
}

/// Transform a background-position value for hash computation.
/// This ensures consistent hashing between Babel and SWC by normalizing
/// position keywords to their percentage equivalents.
pub fn transform_value_for_hash(value: &str) -> String {
  normalize_pair(value)
}

pub fn plugin() -> pc::BuiltPlugin {
  pc::plugin("postcss-normalize-positions")
    .decl_filter("background-position", |decl, _| {
      let current = decl.value();
      let next = normalize_pair(&current);
      if next != current {
        decl.set_value(next);
      }
      Ok(())
    })
    .build()
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn test_normalize_single_position_keywords() {
    // Single keywords use horizontal map - center IS converted to 50% to match Babel
    assert_eq!(normalize_pair("left"), "0");
    assert_eq!(normalize_pair("right"), "100%");
    assert_eq!(normalize_pair("center"), "50%");
    // Note: single "top" and "bottom" are not converted by postcss-normalize-positions
    // because they're not in the horizontal map used for single values
    assert_eq!(normalize_pair("top"), "top");
    assert_eq!(normalize_pair("bottom"), "bottom");
  }

  #[test]
  fn test_normalize_center_center() {
    // center center collapses to 50% to match Babel's cssnano behavior
    assert_eq!(normalize_pair("center center"), "50%");
  }

  #[test]
  fn test_normalize_center_collapsing() {
    // When second value is 'center', collapse to just the first value
    assert_eq!(normalize_pair("left center"), "0");
    assert_eq!(normalize_pair("right center"), "100%");
    // When first value is 'center' and second is direction keyword:
    // - If second is horizontal, convert it
    assert_eq!(normalize_pair("center left"), "0");
    assert_eq!(normalize_pair("center right"), "100%");
    // - If second is vertical, keep it as keyword (NOT converted)
    assert_eq!(normalize_pair("center top"), "top");
    assert_eq!(normalize_pair("center bottom"), "bottom");
  }

  #[test]
  fn test_normalize_position_pairs() {
    assert_eq!(normalize_pair("left top"), "0 0");
    assert_eq!(normalize_pair("right bottom"), "100% 100%");
    assert_eq!(normalize_pair("50% 50%"), "50% 50%");
  }

  #[test]
  fn test_normalize_position_keyword_ordering() {
    // CSS requires keywords to be in x y (horizontal vertical) order.
    // When vertical keyword comes first, it should be swapped.

    // "bottom left" = vertical horizontal -> should become "left bottom" = "0 100%"
    assert_eq!(normalize_pair("bottom left"), "0 100%");

    // "top right" = vertical horizontal -> should become "right top" = "100% 0"
    assert_eq!(normalize_pair("top right"), "100% 0");

    // "left bottom" = horizontal vertical -> already correct order = "0 100%"
    assert_eq!(normalize_pair("left bottom"), "0 100%");

    // "right top" = horizontal vertical -> already correct order = "100% 0"
    assert_eq!(normalize_pair("right top"), "100% 0");

    // "top left" = vertical horizontal -> should become "left top" = "0 0"
    assert_eq!(normalize_pair("top left"), "0 0");

    // "bottom right" = vertical horizontal -> should become "right bottom" = "100% 100%"
    assert_eq!(normalize_pair("bottom right"), "100% 100%");
  }

  #[test]
  fn test_normalize_position_preserves_non_zero_px_values() {
    // 300px 0 should NOT have px stripped from 300px
    // Only 0px should become 0
    assert_eq!(normalize_pair("300px 0"), "300px 0");
    assert_eq!(normalize_pair("0px 0"), "0 0");
    assert_eq!(normalize_pair("100px 50px"), "100px 50px");
    assert_eq!(normalize_pair("0px 100px"), "0 100px");
  }

  #[test]
  fn test_normalize_comma_separated_positions() {
    // Each comma-separated position should be normalized individually
    // This is the exact test case from the failing file:
    // Original: left center, left center, right center, right center, center top, 0px 52px, center bottom, center bottom
    // Expected: 0, 0, 100%, 100%, top, 0 52px, bottom, bottom (with spaces after commas to match Babel)
    assert_eq!(
      normalize_pair(
        "left center,left center,right center,right center,center top,0px 52px,center bottom,center bottom"
      ),
      "0, 0, 100%, 100%, top, 0 52px, bottom, bottom"
    );
  }

  #[test]
  fn test_normalize_comma_separated_simple() {
    // Simpler comma-separated test
    // Note: output has spaces after commas to match Babel's cssnano behavior
    assert_eq!(
      normalize_pair("0,0,100%,100%,top,0 52px,bottom,bottom"),
      "0, 0, 100%, 100%, top, 0 52px, bottom, bottom"
    );
  }

  #[test]
  fn test_normalize_mixed_values() {
    // Dimension + dimension pairs
    assert_eq!(normalize_pair("0 52px"), "0 52px");
    assert_eq!(normalize_pair("10px 20px"), "10px 20px");
  }

  #[test]
  fn test_preserve_negative_percentage() {
    // Negative percentages should be preserved
    assert_eq!(normalize_pair("-100%"), "-100%");
    assert_eq!(normalize_pair("100%"), "100%");
    assert_eq!(normalize_pair("-50%"), "-50%");
  }

  #[test]
  fn test_comma_space_for_babel_compatibility() {
    // IMPORTANT: Output must have ", " (comma + space) between positions to match Babel's
    // cssnano postcss-normalize-positions behavior. The hash is computed from this value
    // (with spaces), and then whitespace is minified separately for the final CSS output.
    //
    // Example from link-datasource/content-container/index.tsx:
    // - Input: 'left center, left center, right center, right center, center top, 0px 52px, center bottom, center bottom'
    // - After normalize_positions: '0, 0, 100%, 100%, top, 0 52px, bottom, bottom' (with spaces)
    // - Hash computed from: '0, 0, 100%, 100%, top, 0 52px, bottom, bottom' -> 'bac2'
    // - Final CSS after whitespace minification: '0,0,100%,100%,top,0 52px,bottom,bottom' (no spaces after commas)
    assert_eq!(
      normalize_pair(
        "left center, left center, right center, right center, center top, 0px 52px, center bottom, center bottom"
      ),
      "0, 0, 100%, 100%, top, 0 52px, bottom, bottom"
    );
    assert!(
      normalize_pair("a, b").contains(", "),
      "output must have space after comma"
    );
  }
}
