//! Hash utilities mirrored from `packages/utils/src/hash.ts`.

/// Compute a deterministic MurmurHash2 hash for the provided string using the
/// default seed of `0`. The return value matches the base-36 output from the
/// JavaScript implementation in `@compiled/utils`.
pub fn hash(value: &str) -> String {
  hash_with_seed(value, 0)
}

/// Compute a deterministic MurmurHash2 hash for the provided string with a
/// custom seed. This mirrors the behaviour of `hash(str, seed)` in
/// `packages/utils/src/hash.ts`.
pub fn hash_with_seed(value: &str, seed: u32) -> String {
  const M: u32 = 0x5bd1e995;
  const R: u32 = 24;

  let units: Vec<u16> = value.encode_utf16().collect();
  let mut len = units.len();
  let mut hash = seed ^ (len as u32);
  let mut index = 0usize;

  while len >= 4 {
    let mut k = u32::from(units[index] & 0xff)
      | (u32::from(units[index + 1] & 0xff) << 8)
      | (u32::from(units[index + 2] & 0xff) << 16)
      | (u32::from(units[index + 3] & 0xff) << 24);

    k = k.wrapping_mul(M);
    k ^= k >> R;
    k = k.wrapping_mul(M);

    hash = hash.wrapping_mul(M);
    hash ^= k;

    index += 4;
    len -= 4;
  }

  match len {
    3 => {
      hash ^= u32::from(units[index + 2] & 0xff) << 16;
      hash ^= u32::from(units[index + 1] & 0xff) << 8;
      hash ^= u32::from(units[index] & 0xff);
      hash = hash.wrapping_mul(M);
    }
    2 => {
      hash ^= u32::from(units[index + 1] & 0xff) << 8;
      hash ^= u32::from(units[index] & 0xff);
      hash = hash.wrapping_mul(M);
    }
    1 => {
      hash ^= u32::from(units[index] & 0xff);
      hash = hash.wrapping_mul(M);
    }
    _ => {}
  }

  hash ^= hash >> 13;
  hash = hash.wrapping_mul(M);
  hash ^= hash >> 15;

  to_base36(hash)
}

fn to_base36(mut value: u32) -> String {
  if value == 0 {
    return "0".to_string();
  }

  let mut buffer = Vec::new();
  while value > 0 {
    let digit = (value % 36) as u8;
    let byte = if digit < 10 {
      b'0' + digit
    } else {
      b'a' + (digit - 10)
    };
    buffer.push(byte);
    value /= 36;
  }

  buffer.reverse();
  String::from_utf8(buffer).expect("base36 conversion produced invalid utf8")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn matches_known_hashes() {
    assert_eq!(hash("color"), "1ylxx6h");
    assert_eq!(hash("margin"), "1py5azy");
    assert_eq!(hash("!important"), "pjhvf0");
  }

  #[test]
  fn matches_var_value_hash() {
    assert_eq!(
      hash("var(--space-200,4px)")
        .chars()
        .take(4)
        .collect::<String>(),
      "opcn"
    );
  }

  #[test]
  fn hashes_with_seed_match_js_reference() {
    assert_eq!(hash_with_seed("namespace----cacheKey", 0), "11sab8f");
    assert_eq!(hash_with_seed("namespace----cacheKey", 5), "wqqrxw");
  }
}
