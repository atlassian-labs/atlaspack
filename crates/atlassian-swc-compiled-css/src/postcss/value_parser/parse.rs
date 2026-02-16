use super::{Node, ParsedValue};

pub fn parse(input: &str) -> ParsedValue {
  // Minimal port of postcss-value-parser parse.js sufficient for cssnano plugins.
  let value = input.to_string();
  let mut pos: usize = 0;
  let max = value.len();
  let bytes: Vec<u8> = value.as_bytes().to_vec();
  let mut stack: Vec<Vec<Node>> = vec![Vec::new()];

  while pos < max {
    let code = bytes[pos];
    // standalone parenthesis group "(...)" without a preceding ident -> treat as function with empty name
    if code == b'(' {
      let mut depth: usize = 1;
      let mut i = pos + 1;
      while i < max && depth > 0 {
        let c = bytes[i];
        if c == b'(' {
          depth += 1;
        } else if c == b')' {
          depth -= 1;
          if depth == 0 {
            break;
          }
        }
        i += 1;
      }
      let inner_start = pos + 1;
      let inner_end = if i < max { i } else { max.saturating_sub(1) };
      let inner = if inner_start <= inner_end {
        String::from_utf8(bytes[inner_start..inner_end].to_vec()).unwrap_or_default()
      } else {
        String::new()
      };
      let parsed_inner = parse(&inner);
      stack.last_mut().unwrap().push(Node::Function {
        value: String::new(),
        nodes: parsed_inner.nodes,
        before: String::new(),
        after: String::new(),
        unclosed: i >= max,
      });
      pos = if i < max { i + 1 } else { max };
      continue;
    }
    if code == b')' {
      // Preserve unbalanced ')' as Word node to match Babel behavior
      // This handles cases like extra parentheses in invalid CSS
      stack.last_mut().unwrap().push(Node::Word {
        value: ")".to_string(),
      });
      pos += 1;
      continue;
    }
    // whitespace (<= 32)
    if code <= 32 {
      let start = pos;
      let mut next = pos;
      while next < max && bytes[next] <= 32 {
        next += 1;
      }
      let token = String::from_utf8(bytes[start..next].to_vec()).unwrap_or_default();
      stack.last_mut().unwrap().push(Node::Space { value: token });
      pos = next;
      continue;
    }

    // comments /* ... */
    if code == b'/' && pos + 1 < max && bytes[pos + 1] == b'*' {
      let start = pos + 2;
      let mut end = start;
      let mut unclosed = false;
      while end + 1 < max && !(bytes[end] == b'*' && bytes[end + 1] == b'/') {
        end += 1;
      }
      if end + 1 >= max {
        unclosed = true;
      }
      let val = if unclosed {
        String::from_utf8(bytes[start..max].to_vec()).unwrap_or_default()
      } else {
        String::from_utf8(bytes[start..end].to_vec()).unwrap_or_default()
      };
      stack.last_mut().unwrap().push(Node::Comment {
        value: val,
        unclosed,
      });
      pos = if unclosed { max } else { end + 2 };
      continue;
    }

    // strings '...' or "..."
    if code == b'\'' || code == b'"' {
      let quote = code as char;
      let mut next = pos;
      let mut unclosed = false;
      loop {
        next = match value[next + 1..].find(quote) {
          Some(o) => next + 1 + o,
          None => {
            unclosed = true;
            max - 1
          }
        };
        // check escapes
        let mut escape = false;
        let mut escape_pos = next;
        while escape_pos > 0 && bytes[escape_pos - 1] == b'\\' {
          escape = !escape;
          escape_pos -= 1;
        }
        if !escape {
          break;
        }
        if next + 1 >= max {
          unclosed = true;
          break;
        }
      }
      let val = if pos + 1 <= next {
        String::from_utf8(bytes[pos + 1..next].to_vec()).unwrap_or_default()
      } else {
        String::new()
      };
      stack.last_mut().unwrap().push(Node::String {
        value: val,
        quote,
        unclosed,
      });
      pos = if unclosed { max } else { next + 1 };
      continue;
    }

    // functions name(...)
    if is_ident_start(code) {
      // read word or function
      let start = pos;
      let mut end = pos;
      while end < max && is_ident_continue(bytes[end]) {
        end += 1;
      }
      let name = String::from_utf8(bytes[start..end].to_vec()).unwrap_or_default();
      if end < max && bytes[end] == b'(' {
        // parse function

        // consume '('
        end += 1;
        let inner_start = end;
        // naive: find matching ')', non-nested handling by recursion on commas and simple strings/comments
        let mut depth = 1usize;
        let mut i = end;
        while i < max && depth > 0 {
          let c = bytes[i];
          if c == b'(' {
            depth += 1;
          } else if c == b')' {
            depth -= 1;
            if depth == 0 {
              break;
            }
          }
          i += 1;
        }
        let inner_end = i;
        let inner = if inner_start <= inner_end {
          String::from_utf8(bytes[inner_start..inner_end].to_vec()).unwrap_or_default()
        } else {
          String::new()
        };
        let parsed_inner = parse(&inner); // recurse
        let fn_nodes = parsed_inner.nodes;
        stack.last_mut().unwrap().push(Node::Function {
          value: name,
          nodes: fn_nodes,
          before: String::new(),
          after: String::new(),
          unclosed: inner_end >= max,
        });
        pos = if inner_end < max { inner_end + 1 } else { max };
        continue;
      } else {
        stack.last_mut().unwrap().push(Node::Word { value: name });
        pos = end;
        continue;
      }
    }

    // div punctuation: comma, colon, slash
    if code == b',' || code == b':' || code == b'/' {
      let val = (code as char).to_string();
      stack.last_mut().unwrap().push(Node::Div {
        value: val,
        before: String::new(),
        after: String::new(),
      });
      pos += 1;
      continue;
    }

    // default: read as word until whitespace or punctuation
    let start = pos;
    let mut end = pos;
    while end < max {
      let c = bytes[end];
      if c <= 32
        || c == b','
        || c == b':'
        || c == b'/'
        || c == b'('
        || c == b')'
        || c == b'\''
        || c == b'"'
      {
        break;
      }
      end += 1;
    }
    if end == start {
      // Safety: advance by one to avoid infinite loop on unexpected punctuation
      pos += 1;
      continue;
    } else {
      let word = String::from_utf8(bytes[start..end].to_vec()).unwrap_or_default();
      stack.last_mut().unwrap().push(Node::Word { value: word });
      pos = end;
    }
  }

  ParsedValue {
    nodes: stack.pop().unwrap_or_default(),
  }
}

fn is_ident_start(c: u8) -> bool {
  (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z') || c == b'_' || c == b'-'
}

fn is_ident_continue(c: u8) -> bool {
  is_ident_start(c) || (c >= b'0' && c <= b'9') || c == b'.'
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::postcss::value_parser::stringify;

  #[test]
  fn test_preserves_extra_closing_paren() {
    // Babel preserves extra/unbalanced parentheses, SWC should too
    // This matches the source bug in: min(100%, calc(...)))
    let input = "min(100%, calc(var(--a, 2000)*1px - var(--b, 40px))))";
    let parsed = parse(input);
    let output = stringify(&parsed.nodes);
    assert_eq!(output, input, "extra closing paren should be preserved");
  }

  #[test]
  fn test_preserves_stray_closing_paren() {
    // Stray closing parens should be preserved as Word nodes
    let input = "foo)bar";
    let parsed = parse(input);
    let output = stringify(&parsed.nodes);
    assert_eq!(output, input, "stray closing paren should be preserved");
  }

  #[test]
  fn test_preserves_multiple_stray_closing_parens() {
    // Multiple stray closing parens should all be preserved
    let input = "foo)))";
    let parsed = parse(input);
    let output = stringify(&parsed.nodes);
    assert_eq!(
      output, input,
      "multiple stray closing parens should be preserved"
    );
  }
}
