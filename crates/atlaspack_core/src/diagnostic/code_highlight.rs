use serde::Deserialize;
use serde::Serialize;

use crate::types::Location;
use crate::types::SourceLocation;

/// Represents a snippet of code to highlight
#[derive(Serialize, Default, Deserialize, Debug, PartialEq, Clone)]
pub struct CodeHighlight {
  /// The start location to highlight
  pub start: Location,

  /// The end location to highlight
  pub end: Location,

  /// An optional message to display around the source-code range
  pub message: Option<String>,
}

impl From<[usize; 2]> for CodeHighlight {
  fn from(loc: [usize; 2]) -> Self {
    CodeHighlight {
      message: None,
      start: Location {
        line: loc[0],
        column: loc[1],
      },
      end: Location {
        line: loc[0] + 1,
        column: 1,
      },
    }
  }
}

impl From<SourceLocation> for CodeHighlight {
  fn from(loc: SourceLocation) -> Self {
    CodeHighlight {
      message: None,
      start: loc.start.clone(),
      end: Location {
        line: loc.end.line,
        column: loc.end.column - 1,
      },
    }
  }
}
