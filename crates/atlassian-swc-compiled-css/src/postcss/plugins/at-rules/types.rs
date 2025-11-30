use swc_core::css::ast::Rule;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Property {
  Width,
  Height,
  DeviceWidth,
  DeviceHeight,
}

impl Property {
  pub fn sort_order(self) -> i32 {
    match self {
      Property::Width => 1,
      Property::Height => 2,
      Property::DeviceWidth => 101,
      Property::DeviceHeight => 102,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
  LessEqual,
  Equal,
  GreaterEqual,
  Less,
  Greater,
}

impl ComparisonOperator {
  pub fn sort_order(self) -> i32 {
    match self {
      ComparisonOperator::Greater => 10,
      ComparisonOperator::GreaterEqual => 20,
      ComparisonOperator::Less => 30,
      ComparisonOperator::LessEqual => 40,
      ComparisonOperator::Equal => 50,
    }
  }

  pub fn includes_greater(self) -> bool {
    matches!(
      self,
      ComparisonOperator::Greater | ComparisonOperator::GreaterEqual
    )
  }

  pub fn reverse(self) -> Self {
    match self {
      ComparisonOperator::Less => ComparisonOperator::Greater,
      ComparisonOperator::Greater => ComparisonOperator::Less,
      ComparisonOperator::LessEqual => ComparisonOperator::GreaterEqual,
      ComparisonOperator::GreaterEqual => ComparisonOperator::LessEqual,
      ComparisonOperator::Equal => ComparisonOperator::Equal,
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LengthInfo {
  pub length: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicMatchInfo {
  pub match_text: String,
  pub index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedAtRule {
  pub property: Property,
  pub comparison_operator: ComparisonOperator,
  pub length: f64,
  pub match_info: BasicMatchInfo,
}

impl ParsedAtRule {
  pub fn sort_key(&self) -> i32 {
    self.property.sort_order() + self.comparison_operator.sort_order()
  }
}

#[derive(Debug, Clone)]
pub struct AtRuleInfo {
  pub parsed: Vec<ParsedAtRule>,
  pub node: Rule,
  pub at_rule_name: String,
  pub query: String,
}

#[derive(Debug, Clone, Copy)]
pub struct PropertyInfo {
  pub property: Property,
}

#[derive(Debug, Clone, Copy)]
pub struct OperatorInfo {
  pub comparison_operator: ComparisonOperator,
}

#[derive(Debug, Clone)]
pub struct MatchComponents {
  pub property: Property,
  pub comparison_operator: ComparisonOperator,
  pub length: f64,
  pub match_info: BasicMatchInfo,
}

impl From<MatchComponents> for ParsedAtRule {
  fn from(components: MatchComponents) -> Self {
    ParsedAtRule {
      property: components.property,
      comparison_operator: components.comparison_operator,
      length: components.length,
      match_info: components.match_info,
    }
  }
}
