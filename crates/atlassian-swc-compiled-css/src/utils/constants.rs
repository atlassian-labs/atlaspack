#![allow(dead_code)]

//! Miscellaneous shared constants from the Babel implementation.

/// Selector appended when increasing specificity during CSS transforms.
pub const INCREASE_SPECIFICITY_SELECTOR: &str = ":not(#\\#)";

/// Property keys visited when traversing conditional expressions.
pub const CONDITIONAL_PATHS: [&str; 2] = ["consequent", "alternate"];
