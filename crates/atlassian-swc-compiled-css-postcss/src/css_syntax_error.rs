#![allow(
  clippy::manual_pattern_char_comparison,
  clippy::redundant_closure,
  clippy::too_many_arguments
)]

use std::error::Error;
use std::fmt;

use url::Url;

use crate::input::{InputRef, Position};
use crate::terminal_highlight;

#[derive(Clone, Debug)]
pub struct ErrorInput {
  pub line: Option<u32>,
  pub column: Option<u32>,
  pub end_line: Option<u32>,
  pub end_column: Option<u32>,
  pub source: String,
  pub file: Option<String>,
  pub url: Option<String>,
}

impl ErrorInput {
  fn from_input(input: &InputRef, start: &Position, end: Option<&Position>) -> Self {
    let line = Some(start.line);
    let column = Some(start.column);
    let end_line = end.map(|pos| pos.line);
    let end_column = end.map(|pos| pos.column);

    let file = input
      .file
      .clone()
      .or_else(|| input.from.clone())
      .or_else(|| input.id.clone());
    let url = file
      .as_ref()
      .and_then(|path| Url::from_file_path(path).ok())
      .map(|u| u.to_string());

    Self {
      line,
      column,
      end_line,
      end_column,
      source: input.css().to_string(),
      file,
      url,
    }
  }
}

#[derive(Clone, Debug)]
pub struct CssSyntaxError {
  pub name: &'static str,
  pub reason: String,
  pub file: Option<String>,
  pub source: Option<String>,
  pub plugin: Option<String>,
  pub line: Option<u32>,
  pub column: Option<u32>,
  pub end_line: Option<u32>,
  pub end_column: Option<u32>,
  pub input: Option<ErrorInput>,
  message: String,
}

impl CssSyntaxError {
  pub fn new(
    reason: impl Into<String>,
    line: Option<u32>,
    column: Option<u32>,
    end_line: Option<u32>,
    end_column: Option<u32>,
    source: Option<String>,
    file: Option<String>,
    plugin: Option<String>,
  ) -> Self {
    let mut error = Self {
      name: "CssSyntaxError",
      reason: reason.into(),
      file,
      source,
      plugin,
      line,
      column,
      end_line,
      end_column,
      input: None,
      message: String::new(),
    };
    error.set_message();
    error
  }

  pub fn from_input(
    reason: impl Into<String>,
    input: Option<InputRef>,
    start: Position,
    end: Option<Position>,
    plugin: Option<String>,
  ) -> Self {
    let (source, file, input_info) = if let Some(input_ref) = input {
      let info = ErrorInput::from_input(&input_ref, &start, end.as_ref());
      (Some(info.source.clone()), info.file.clone(), Some(info))
    } else {
      (None, None, None)
    };

    let mut error = CssSyntaxError::new(
      reason,
      Some(start.line),
      Some(start.column),
      end.as_ref().map(|pos| pos.line),
      end.as_ref().map(|pos| pos.column),
      source,
      file,
      plugin,
    );
    error.input = input_info;
    error
  }

  pub fn with_plugin(mut self, plugin: impl Into<String>) -> Self {
    self.plugin = Some(plugin.into());
    self.set_message();
    self
  }

  pub fn set_end_position(&mut self, pos: Position) {
    self.end_line = Some(pos.line);
    self.end_column = Some(pos.column);
    self.set_message();
    if let Some(input) = &mut self.input {
      input.end_line = Some(pos.line);
      input.end_column = Some(pos.column);
    }
  }

  pub fn file(&self) -> Option<&str> {
    self.file.as_deref()
  }

  fn set_message(&mut self) {
    let mut message = String::new();
    if let Some(plugin) = &self.plugin {
      message.push_str(plugin);
      message.push_str(": ");
    }

    if let Some(file) = &self.file {
      message.push_str(file);
    } else {
      message.push_str("<css input>");
    }

    if let (Some(line), Some(column)) = (self.line, self.column) {
      message.push(':');
      message.push_str(&line.to_string());
      message.push(':');
      message.push_str(&column.to_string());
    }

    message.push_str(": ");
    message.push_str(&self.reason);
    self.message = message;
  }

  pub fn show_source_code(&self, color: Option<bool>) -> String {
    let line = match self.line {
      Some(line) if line > 0 => line as usize,
      _ => return String::new(),
    };

    let source = if let Some(source) = &self.source {
      source.clone()
    } else if let Some(input) = &self.input {
      input.source.clone()
    } else {
      return String::new();
    };

    let color_enabled = color.unwrap_or_else(|| terminal_highlight::is_color_supported());
    let display_source = if color_enabled {
      terminal_highlight::highlight(&source)
    } else {
      source.clone()
    };

    let mark: Box<dyn Fn(&str) -> String>;
    let aside: Box<dyn Fn(&str) -> String>;
    if color_enabled {
      mark = Box::new(|text: &str| format!("\u{1b}[1;31m{}\u{1b}[0m", text));
      aside = Box::new(|text: &str| format!("\u{1b}[90m{}\u{1b}[0m", text));
    } else {
      mark = Box::new(|text: &str| text.to_string());
      aside = Box::new(|text: &str| text.to_string());
    }

    let display_lines: Vec<&str> = display_source.split(|c| c == '\n').collect();
    let plain_lines: Vec<&str> = source.split(|c| c == '\n').collect();
    if line == 0 || line > display_lines.len() {
      return String::new();
    }

    let start = line.saturating_sub(3);
    let end = (line + 2).min(display_lines.len());
    let max_width = end.to_string().len();

    let mut lines = Vec::new();
    for (index, display_line) in display_lines[start..end].iter().enumerate() {
      let number = start + index + 1;
      let gutter = format!(" {:>width$} | ", number, width = max_width);
      if number == line {
        let plain_line = plain_lines.get(number - 1).copied().unwrap_or("");
        let gutter_marker: String = gutter
          .chars()
          .map(|ch| if ch.is_ascii_digit() { ' ' } else { ch })
          .collect();
        let pointer_padding: String = plain_line
          .chars()
          .take(self.column.unwrap_or(1).saturating_sub(1) as usize)
          .map(|ch| if ch == '\t' { '\t' } else { ' ' })
          .collect();
        let spacing = format!("{}{}", aside(&gutter_marker), pointer_padding);
        let marker = mark("^");
        let line_indicator = mark(">");
        lines.push(format!(
          "{}{}{}\n {}{}{}",
          line_indicator,
          aside(&gutter),
          display_line,
          spacing,
          marker,
          ""
        ));
      } else {
        lines.push(format!(" {}{}{}", "", aside(&gutter), display_line));
      }
    }

    lines.join("\n")
  }
}

impl fmt::Display for CssSyntaxError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut output = format!("{}: {}", self.name, self.message);
    let code = self.show_source_code(None);
    if !code.is_empty() {
      output.push_str("\n\n");
      output.push_str(&code);
      output.push('\n');
    }
    write!(f, "{}", output)
  }
}

impl Error for CssSyntaxError {}
