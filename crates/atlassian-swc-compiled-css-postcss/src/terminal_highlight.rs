use std::env;

use crate::input::{Input, InputOptions, InputRef};
use crate::parse::{Token, TokenKind, Tokenizer};

fn colorize(code: &str, text: &str) -> String {
  format!("\u{1b}[{}m{}\u{1b}[0m", code, text)
}

fn yellow(text: &str) -> String {
  colorize("33", text)
}

fn cyan(text: &str) -> String {
  colorize("36", text)
}

fn gray(text: &str) -> String {
  colorize("90", text)
}

fn magenta(text: &str) -> String {
  colorize("35", text)
}

fn green(text: &str) -> String {
  colorize("32", text)
}

#[derive(Clone, Copy)]
enum Highlight {
  Yellow,
  Cyan,
  Gray,
  Magenta,
  Green,
}

impl Highlight {
  fn apply(self, value: &str) -> String {
    match self {
      Highlight::Yellow => yellow(value),
      Highlight::Cyan => cyan(value),
      Highlight::Gray => gray(value),
      Highlight::Magenta => magenta(value),
      Highlight::Green => green(value),
    }
  }
}

fn determine_highlight(token: &Token, tokenizer: &mut Tokenizer) -> Option<Highlight> {
  match token.kind {
    TokenKind::Semicolon | TokenKind::Colon => Some(Highlight::Yellow),
    TokenKind::OpenParenthesis | TokenKind::CloseParenthesis => Some(Highlight::Cyan),
    TokenKind::OpenSquare | TokenKind::CloseSquare => Some(Highlight::Yellow),
    TokenKind::OpenCurly | TokenKind::CloseCurly => Some(Highlight::Yellow),
    TokenKind::AtWord => Some(Highlight::Cyan),
    TokenKind::Brackets => Some(Highlight::Cyan),
    TokenKind::Comment => Some(Highlight::Gray),
    TokenKind::String => Some(Highlight::Green),
    TokenKind::Word => {
      if let Some(first) = token.value.chars().next() {
        if first == '.' {
          return Some(Highlight::Yellow);
        }
        if first == '#' {
          return Some(Highlight::Magenta);
        }
      }

      if !tokenizer.end_of_file() {
        if let Ok(Some(next)) = tokenizer.next_token(true) {
          let highlight = matches!(next.kind, TokenKind::Brackets | TokenKind::OpenParenthesis);
          tokenizer.back(next);
          if highlight {
            return Some(Highlight::Cyan);
          }
        }
      }

      None
    }
    _ => None,
  }
}

/// Apply PostCSS' terminal highlighting rules to a CSS string.
pub fn highlight(css: &str) -> String {
  let input = match Input::new(css.to_string(), InputOptions::default()) {
    Ok(input) => input,
    Err(_) => return css.to_string(),
  };
  let input_ref = InputRef::from(input);
  let mut tokenizer = Tokenizer::new(input_ref, true);
  let mut result = String::new();

  loop {
    match tokenizer.next_token(true) {
      Ok(Some(token)) => {
        if let Some(highlight) = determine_highlight(&token, &mut tokenizer) {
          result.push_str(&highlight.apply(&token.value));
        } else {
          result.push_str(&token.value);
        }
      }
      Ok(None) => break,
      Err(_) => return css.to_string(),
    }
  }

  result
}

/// Determine whether ANSI colors should be emitted by default.
pub fn is_color_supported() -> bool {
  if env::var_os("NO_COLOR").is_some() {
    return false;
  }
  if let Some(force) = env::var_os("FORCE_COLOR") {
    return force != "0";
  }
  true
}
