#[derive(Clone, Debug, PartialEq)]
pub struct NumberUnit {
  pub number: String,
  pub unit: String,
}

pub fn unit(value: &str) -> Option<NumberUnit> {
  fn like_number(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
      return false;
    }
    let first = bytes[0];
    if first == b'+' || first == b'-' {
      if bytes.len() < 2 {
        return false;
      }
      let next = bytes[1];
      if next.is_ascii_digit() {
        return true;
      }
      if next == b'.' && bytes.len() >= 3 && bytes[2].is_ascii_digit() {
        return true;
      }
      return false;
    }
    if first == b'.' {
      return bytes.get(1).map_or(false, |c| c.is_ascii_digit());
    }
    first.is_ascii_digit()
  }

  if value.is_empty() {
    return None;
  }
  let bytes = value.as_bytes();
  if !like_number(bytes) {
    return None;
  }
  let mut pos = 0usize;
  let len = bytes.len();
  if matches!(bytes[pos], b'+' | b'-') {
    pos += 1;
  }
  while pos < len && bytes[pos].is_ascii_digit() {
    pos += 1;
  }
  if pos < len && bytes[pos] == b'.' {
    if pos + 1 < len && bytes[pos + 1].is_ascii_digit() {
      pos += 2;
      while pos < len && bytes[pos].is_ascii_digit() {
        pos += 1;
      }
    }
  }
  if pos < len && (bytes[pos] == b'e' || bytes[pos] == b'E') {
    let mut idx = pos + 1;
    if idx < len {
      if bytes[idx] == b'+' || bytes[idx] == b'-' {
        idx += 1;
      }
      let digit_start = idx;
      while idx < len && bytes[idx].is_ascii_digit() {
        idx += 1;
      }
      if idx > digit_start {
        pos = idx;
      }
    }
  }

  let (num, uni) = value.split_at(pos);
  if num.is_empty() {
    return None;
  }
  Some(NumberUnit {
    number: num.to_string(),
    unit: uni.to_string(),
  })
}
