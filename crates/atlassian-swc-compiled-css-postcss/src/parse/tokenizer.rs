#![allow(
  clippy::collapsible_if,
  clippy::needless_return,
  clippy::result_large_err
)]

use crate::css_syntax_error::CssSyntaxError;
use crate::input::InputRef;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenKind {
  Space,
  OpenSquare,
  CloseSquare,
  OpenCurly,
  CloseCurly,
  Colon,
  Semicolon,
  OpenParenthesis,
  CloseParenthesis,
  Word,
  AtWord,
  String,
  Comment,
  Brackets,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
  pub kind: TokenKind,
  pub value: String,
  pub start: Option<usize>,
  pub end: Option<usize>,
}

impl Token {
  pub fn new(kind: TokenKind, value: String, start: Option<usize>, end: Option<usize>) -> Self {
    Self {
      kind,
      value,
      start,
      end,
    }
  }
}

const SINGLE_QUOTE: u8 = b'\'';
const DOUBLE_QUOTE: u8 = b'"';
const BACKSLASH: u8 = b'\\';
const SLASH: u8 = b'/';
const NEWLINE: u8 = b'\n';
const SPACE: u8 = b' ';
const FEED: u8 = 0x0C;
const TAB: u8 = b'\t';
const CR: u8 = b'\r';
const OPEN_SQUARE: u8 = b'[';
const CLOSE_SQUARE: u8 = b']';
const OPEN_PARENTHESES: u8 = b'(';
const CLOSE_PARENTHESES: u8 = b')';
const OPEN_CURLY: u8 = b'{';
const CLOSE_CURLY: u8 = b'}';
const SEMICOLON: u8 = b';';
const ASTERISK: u8 = b'*';
const COLON: u8 = b':';
const AT: u8 = b'@';

fn is_space(code: u8) -> bool {
  matches!(code, SPACE | NEWLINE | TAB | CR | FEED)
}

fn is_at_end_stop(code: u8) -> bool {
  matches!(
    code,
    b'\t'
      | b'\n'
      | b'\r'
      | b' '
      | b'"'
      | b'#'
      | b'\''
      | b'('
      | b')'
      | b'/'
      | b';'
      | 0x0c
      | b'['
      | b'\\'
      | b']'
      | b'{'
      | b'}'
  )
}

fn is_word_end_stop(code: u8) -> bool {
  matches!(
    code,
    b'\t'
      | b'\n'
      | b'\r'
      | b' '
      | b'!'
      | b'"'
      | b'#'
      | b'\''
      | b'('
      | b')'
      | b':'
      | b';'
      | 0x0c
      | b'@'
      | b'['
      | b'\\'
      | b']'
      | b'{'
      | b'}'
  )
}

fn is_bad_bracket_char(code: u8) -> bool {
  matches!(code, b'\r' | b'\n' | b'"' | b'\'' | b'(' | b'/' | b'\\')
}

fn is_hex_escape(ch: char) -> bool {
  ch.is_ascii_hexdigit()
}

pub struct Tokenizer {
  input: InputRef,
  css: String,
  bytes: Vec<u8>,
  pos: usize,
  returned: Vec<Token>,
  buffer: Vec<Token>,
  ignore_errors: bool,
}

impl Tokenizer {
  pub fn new(input: InputRef, ignore_errors: bool) -> Self {
    let css = input.css().to_string();
    let bytes = css.as_bytes().to_vec();
    Self {
      input,
      css,
      bytes,
      pos: 0,
      returned: Vec::new(),
      buffer: Vec::new(),
      ignore_errors,
    }
  }

  pub fn position(&self) -> usize {
    self.pos
  }

  pub fn end_of_file(&self) -> bool {
    self.returned.is_empty() && self.pos >= self.bytes.len()
  }

  fn error_at(&self, message: &str, offset: usize, end_offset: Option<usize>) -> CssSyntaxError {
    let start = self.input.from_offset(offset);
    let end = end_offset.map(|end| self.input.from_offset(end));
    self.input.error(message.to_string(), start, end)
  }

  fn unclosed(&self, what: &str) -> CssSyntaxError {
    self.error_at(&format!("Unclosed {}", what), self.pos, None)
  }

  pub fn back(&mut self, token: Token) {
    self.returned.push(token);
  }

  fn slice(&self, start: usize, end: usize) -> String {
    self.css[start..=end].to_string()
  }

  pub fn next_token(&mut self, ignore_unclosed: bool) -> Result<Option<Token>, CssSyntaxError> {
    if let Some(token) = self.returned.pop() {
      return Ok(Some(token));
    }
    if self.pos >= self.bytes.len() {
      return Ok(None);
    }

    let mut code = self.bytes[self.pos];
    let mut next;
    let mut escaped;
    let mut escape_pos;
    let current_token;

    match code {
      NEWLINE | SPACE | TAB | CR | FEED => {
        let start = self.pos;
        next = self.pos;
        while next + 1 < self.bytes.len() && is_space(self.bytes[next + 1]) {
          next += 1;
        }
        let value = self.slice(start, next);
        current_token = Token::new(TokenKind::Space, value, Some(start), Some(next));
        self.pos = next + 1;
        return Ok(Some(current_token));
      }
      OPEN_SQUARE => {
        let start = self.pos;
        current_token = Token::new(
          TokenKind::OpenSquare,
          "[".to_string(),
          Some(start),
          Some(start),
        );
        self.pos += 1;
        return Ok(Some(current_token));
      }
      CLOSE_SQUARE => {
        let start = self.pos;
        current_token = Token::new(
          TokenKind::CloseSquare,
          "]".to_string(),
          Some(start),
          Some(start),
        );
        self.pos += 1;
        return Ok(Some(current_token));
      }
      OPEN_CURLY => {
        let start = self.pos;
        current_token = Token::new(
          TokenKind::OpenCurly,
          "{".to_string(),
          Some(start),
          Some(start),
        );
        self.pos += 1;
        return Ok(Some(current_token));
      }
      CLOSE_CURLY => {
        let start = self.pos;
        current_token = Token::new(
          TokenKind::CloseCurly,
          "}".to_string(),
          Some(start),
          Some(start),
        );
        self.pos += 1;
        return Ok(Some(current_token));
      }
      COLON => {
        let start = self.pos;
        current_token = Token::new(TokenKind::Colon, ":".to_string(), Some(start), Some(start));
        self.pos += 1;
        return Ok(Some(current_token));
      }
      SEMICOLON => {
        let start = self.pos;
        current_token = Token::new(
          TokenKind::Semicolon,
          ";".to_string(),
          Some(start),
          Some(start),
        );
        self.pos += 1;
        return Ok(Some(current_token));
      }
      CLOSE_PARENTHESES => {
        let start = self.pos;
        current_token = Token::new(
          TokenKind::CloseParenthesis,
          ")".to_string(),
          Some(start),
          Some(start),
        );
        self.pos += 1;
        return Ok(Some(current_token));
      }
      OPEN_PARENTHESES => {
        let prev = if let Some(last) = self.buffer.last() {
          last.value.clone()
        } else {
          String::new()
        };
        let next_code = if self.pos + 1 < self.bytes.len() {
          self.bytes[self.pos + 1]
        } else {
          0
        };
        if prev == "url"
          && next_code != SINGLE_QUOTE
          && next_code != DOUBLE_QUOTE
          && !is_space(next_code)
          && next_code != CR
        {
          next = self.pos;
          loop {
            let mut escaped_flag = false;
            if let Some(idx) = self.css[next + 1..].find(')') {
              next += idx + 1;
            } else if self.ignore_errors || ignore_unclosed {
              next = self.pos;
              break;
            } else {
              return Err(self.unclosed("bracket"));
            }
            escape_pos = next;
            while escape_pos > 0 && self.bytes.get(escape_pos - 1) == Some(&BACKSLASH) {
              escape_pos -= 1;
              escaped_flag = !escaped_flag;
            }
            if !escaped_flag {
              break;
            }
          }
          let start = self.pos;
          let value = self.slice(start, next);
          current_token = Token::new(TokenKind::Brackets, value, Some(start), Some(next));
          self.pos = next + 1;
          return Ok(Some(current_token));
        } else {
          if let Some(idx) = self.css[self.pos + 1..].find(')') {
            next = self.pos + idx + 1;
            let content = &self.css[self.pos..=next];
            if content.len() < 2
              || content
                .as_bytes()
                .iter()
                .skip(1)
                .any(|&b| is_bad_bracket_char(b))
            {
              let start = self.pos;
              current_token = Token::new(
                TokenKind::OpenParenthesis,
                "(".to_string(),
                Some(start),
                Some(start),
              );
            } else {
              let start = self.pos;
              current_token = Token::new(
                TokenKind::Brackets,
                content.to_string(),
                Some(start),
                Some(next),
              );
              self.pos = next + 1;
              return Ok(Some(current_token));
            }
          } else {
            let start = self.pos;
            current_token = Token::new(
              TokenKind::OpenParenthesis,
              "(".to_string(),
              Some(start),
              Some(start),
            );
          }
          self.pos += 1;
          return Ok(Some(current_token));
        }
      }
      SINGLE_QUOTE | DOUBLE_QUOTE => {
        let quote = if code == SINGLE_QUOTE { '\'' } else { '"' };
        next = self.pos;
        loop {
          let slice = &self.css[next + 1..];
          if let Some(idx) = slice.find(quote) {
            next += idx + 1;
          } else if self.ignore_errors || ignore_unclosed {
            next = self.pos + 1;
            break;
          } else {
            return Err(self.unclosed("string"));
          }
          let mut escape_index = next;
          escaped = false;
          while escape_index > 0 && self.bytes.get(escape_index - 1) == Some(&BACKSLASH) {
            escape_index -= 1;
            escaped = !escaped;
          }
          if !escaped {
            break;
          }
        }
        let start = self.pos;
        let value = self.slice(start, next);
        current_token = Token::new(TokenKind::String, value, Some(start), Some(next));
        self.pos = next + 1;
        return Ok(Some(current_token));
      }
      AT => {
        next = self.pos + 1;
        while next < self.bytes.len() {
          let code = self.bytes[next];
          if is_at_end_stop(code) {
            break;
          }
          next += 1;
        }
        let start = self.pos;
        let value = self.css[start..next].to_string();
        current_token = Token::new(TokenKind::AtWord, value, Some(start), Some(next - 1));
        self.pos = next;
        return Ok(Some(current_token));
      }
      BACKSLASH => {
        next = self.pos;
        let mut escape = true;
        while next + 1 < self.bytes.len() && self.bytes[next + 1] == BACKSLASH {
          next += 1;
          escape = !escape;
        }
        if next + 1 < self.bytes.len() {
          code = self.bytes[next + 1];
          if escape && !is_space(code) && code != SLASH {
            next += 1;
            if let Some(ch) = self.css[next..].chars().next() {
              if is_hex_escape(ch) {
                let mut consumed = next + ch.len_utf8();
                for c in self.css[consumed..].chars() {
                  if is_hex_escape(c) {
                    consumed += c.len_utf8();
                  } else {
                    break;
                  }
                }
                next = consumed - 1;
                if self.bytes.get(next + 1) == Some(&SPACE) {
                  next += 1;
                }
              }
            }
          }
        }
        let value = self.slice(self.pos, next);
        current_token = Token::new(TokenKind::Word, value, Some(self.pos), Some(next));
        self.pos = next + 1;
        return Ok(Some(current_token));
      }
      _ => {
        if code == SLASH && self.bytes.get(self.pos + 1) == Some(&ASTERISK) {
          let mut end = None;
          if let Some(idx) = self.css[self.pos + 2..].find("*/") {
            end = Some(self.pos + 2 + idx + 1);
          }
          if end.is_none() {
            if self.ignore_errors || ignore_unclosed {
              end = Some(self.bytes.len().saturating_sub(1));
            } else {
              return Err(self.unclosed("comment"));
            }
          }
          let next_index = end.unwrap_or(self.pos);
          let value = self.slice(self.pos, next_index);
          current_token = Token::new(TokenKind::Comment, value, Some(self.pos), Some(next_index));
          self.pos = next_index + 1;
          return Ok(Some(current_token));
        } else {
          next = self.pos + 1;
          while next < self.bytes.len() {
            let code = self.bytes[next];
            if is_word_end_stop(code) {
              if code == SLASH {
                if self.bytes.get(next + 1) == Some(&ASTERISK) {
                  break;
                }
              }
              break;
            }
            next += 1;
          }
          let value = self.css[self.pos..next].to_string();
          current_token = Token::new(TokenKind::Word, value, Some(self.pos), Some(next - 1));
          self.buffer.push(current_token.clone());
          self.pos = next;
          return Ok(Some(current_token));
        }
      }
    }
  }
}
