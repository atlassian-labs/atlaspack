use std::cmp::Ordering;

use super::types::{AtRuleInfo, ParsedAtRule};

pub fn sort_at_rules(first: &AtRuleInfo, second: &AtRuleInfo) -> Ordering {
  let name_cmp = first.at_rule_name.cmp(&second.at_rule_name);
  if name_cmp != Ordering::Equal {
    return name_cmp;
  }

  let limit = first.parsed.len().min(second.parsed.len());
  for idx in 0..limit {
    let a = &first.parsed[idx];
    let b = &second.parsed[idx];

    let key_cmp = a.sort_key().cmp(&b.sort_key());
    if key_cmp != Ordering::Equal {
      return key_cmp;
    }

    if a.length != b.length {
      return if a.comparison_operator.includes_greater() {
        a.length.partial_cmp(&b.length).unwrap_or(Ordering::Equal)
      } else {
        b.length.partial_cmp(&a.length).unwrap_or(Ordering::Equal)
      };
    }
  }

  if (first.parsed.len() + second.parsed.len() > 0) && first.parsed.len() != second.parsed.len() {
    return first.parsed.len().cmp(&second.parsed.len());
  }

  first.query.cmp(&second.query)
}

pub fn get_sort_key(rule: &ParsedAtRule) -> i32 {
  rule.sort_key()
}
