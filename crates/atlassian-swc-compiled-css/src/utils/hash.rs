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

/// Encode a 32-bit hash value in base-62 (digits 0–9, lower a–z, upper A–Z).
/// This mirrors `hashBase62` in `packages/utils/src/hash.ts`.
pub fn hash_base62(value: &str) -> String {
  // Reuse the same MurmurHash2 algorithm with seed=0, just encode differently.
  let raw = hash_raw(value);
  to_base62(raw)
}

/// Returns the raw u32 MurmurHash2 value (seed = 0), reusing `hash_with_seed` internals.
fn hash_raw(value: &str) -> u32 {
  // We reuse the same algorithm as hash_with_seed but need the raw u32 before base-36 encoding.
  // Since hash_with_seed returns a String, we replicate the computation inline here.
  // This keeps hash_base62 consistent with hash() by construction.
  const M: u32 = 0x5bd1e995;
  const R: u32 = 24;

  let units: Vec<u16> = value.encode_utf16().collect();
  let mut len = units.len();
  let mut h = 0u32 ^ (len as u32);
  let mut index = 0usize;

  while len >= 4 {
    let mut k = u32::from(units[index] & 0xff)
      | (u32::from(units[index + 1] & 0xff) << 8)
      | (u32::from(units[index + 2] & 0xff) << 16)
      | (u32::from(units[index + 3] & 0xff) << 24);

    k = k.wrapping_mul(M);
    k ^= k >> R;
    k = k.wrapping_mul(M);

    h = h.wrapping_mul(M);
    h ^= k;

    index += 4;
    len -= 4;
  }

  match len {
    3 => {
      h ^= u32::from(units[index + 2] & 0xff) << 16;
      h ^= u32::from(units[index + 1] & 0xff) << 8;
      h ^= u32::from(units[index] & 0xff);
      h = h.wrapping_mul(M);
    }
    2 => {
      h ^= u32::from(units[index + 1] & 0xff) << 8;
      h ^= u32::from(units[index] & 0xff);
      h = h.wrapping_mul(M);
    }
    1 => {
      h ^= u32::from(units[index] & 0xff);
      h = h.wrapping_mul(M);
    }
    _ => {}
  }

  h ^= h >> 13;
  h = h.wrapping_mul(M);
  h ^= h >> 15;
  h
}

fn to_base62(mut value: u32) -> String {
  const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
  if value == 0 {
    return "0".to_string();
  }
  let mut buffer = Vec::new();
  while value > 0 {
    buffer.push(CHARS[(value % 62) as usize]);
    value /= 62;
  }
  buffer.reverse();
  String::from_utf8(buffer).expect("base62 conversion produced invalid utf8")
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
      hash("var(--ds-space-0, 0)")
        .chars()
        .take(4)
        .collect::<String>(),
      "ze3t"
    );
  }

  #[test]
  fn hashes_with_seed_match_js_reference() {
    assert_eq!(hash_with_seed("namespace----cacheKey", 0), "11sab8f");
    assert_eq!(hash_with_seed("namespace----cacheKey", 5), "wqqrxw");
  }

  // --- hash_base62 tests ---
  // Expected values verified against the TypeScript `hashBase62` implementation.

  #[test]
  fn hash_base62_matches_js_reference() {
    assert_eq!(hash_base62("color"), "4EWkA1");
    assert_eq!(hash_base62("margin"), "45uXpk");
  }

  #[test]
  fn hash_base62_4_char_group_matches_js_reference() {
    // Enhanced strategy: take first 4 chars of base-62 hash.
    assert_eq!(
      hash_base62("color").chars().take(4).collect::<String>(),
      "4EWk"
    );
    assert_eq!(
      hash_base62("margin").chars().take(4).collect::<String>(),
      "45uX"
    );
    // Simulate a realistic group seed used by atomicify-rules:
    // prefix='' + at_rule='undefined' + normalized_selector='&' + prop='color'
    assert_eq!(
      hash_base62("undefined&color")
        .chars()
        .take(4)
        .collect::<String>(),
      "1UtD"
    );
  }

  #[test]
  fn hash_base62_6_char_group_matches_js_reference() {
    // Max strategy: take first 6 chars of base-62 hash (full 32-bit hash).
    assert_eq!(
      hash_base62("color").chars().take(6).collect::<String>(),
      "4EWkA1"
    );
    assert_eq!(
      hash_base62("margin").chars().take(6).collect::<String>(),
      "45uXpk"
    );
    // Simulate a realistic group seed used by atomicify-rules:
    // prefix='' + at_rule='undefined' + normalized_selector='&' + prop='color'
    assert_eq!(
      hash_base62("undefined&color")
        .chars()
        .take(6)
        .collect::<String>(),
      "1UtDYz"
    );
  }

  #[test]
  fn hash_base62_and_hash_produce_same_raw_value() {
    // hash() and hash_base62() must hash the same input to the same raw u32,
    // just encoded differently. Verify by cross-checking a known value.
    // base36("1ylxx6h") == base62("4EWkA1") == same raw u32 for "color".
    assert_eq!(hash("color"), "1ylxx6h"); // base-36
    assert_eq!(hash_base62("color"), "4EWkA1"); // base-62
  }
}
