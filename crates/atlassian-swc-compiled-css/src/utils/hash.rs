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
  to_base36(murmur2(value, seed))
}

/// Computes the raw 32-bit MurmurHash2 value for a string.
///
/// This is the shared core used by every encoding variant (`hash`,
/// `hash_base62`). Extracted so all hashing shares a single algorithm — the only
/// difference between variants is how the resulting u32 is encoded. Mirrors
/// `murmur2` in `packages/utils/src/hash.ts`.
fn murmur2(value: &str, seed: u32) -> u32 {
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

  hash
}

/// Base-62 character set: digits, lowercase, uppercase. Matches the AFM
/// in-sourced `ap_compiled_css` crate's `hash_base62` and the babel plugin's
/// `hashBase62` so class names are identical across all three implementations,
/// avoiding version-skew mismatches.
const BASE62_CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// The number of characters used for the group hash portion of an atomic class
/// name. 6 chars in base-62 encodes 62^6 = 56.8B values, covering the full
/// 32-bit hash space (4.3B) with zero truncation — eliminating the
/// leading-character bias of the previous base-36 encoding.
pub const ATOMIC_GROUP_HASH_LENGTH: usize = 6;

/// The number of characters used for the value hash portion of an atomic class
/// name. 4 chars in base-62 encodes 62^4 = 14.8M values, sufficient for value
/// deduplication. Fixed-width so `ax()` can extract the group key with a fast,
/// fixed-offset slice.
pub const ATOMIC_VALUE_HASH_LENGTH: usize = 4;

/// Hashes a string and encodes it in base-62 (0-9, a-z, A-Z), zero-padded to a
/// fixed width. Used for collision-resistant atomic class name generation.
///
/// Mirrors `hashBase62` in `packages/utils/src/hash.ts`.
pub fn hash_base62(value: &str, length: usize) -> String {
  let mut v = murmur2(value, 0);
  let mut buffer = vec![b'0'; length];
  for slot in buffer.iter_mut().rev() {
    *slot = BASE62_CHARS[(v % 62) as usize];
    v /= 62;
  }
  // `buffer` only ever contains bytes from `BASE62_CHARS` (all ASCII), so each
  // byte is a valid single-byte UTF-8 char. Build the string directly via
  // `push` to avoid both `.expect()` and `unsafe`.
  let mut out = String::with_capacity(length);
  for byte in buffer {
    out.push(byte as char);
  }
  out
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
  // `buffer` only ever contains ASCII bytes (`0-9`, `a-z`), so each byte is a
  // valid single-byte UTF-8 char. Build the string directly via `push` to avoid
  // both `.expect()` and `unsafe`.
  let mut out = String::with_capacity(buffer.len());
  for byte in buffer {
    out.push(byte as char);
  }
  out
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

  #[test]
  fn atomic_class_hash_lengths_produce_an_11_char_class() {
    // An atomic class is `_` + group + value. With a 6-char group and 4-char
    // value that is exactly 11 characters — the shape `ax()` relies on.
    assert_eq!(ATOMIC_GROUP_HASH_LENGTH, 6);
    assert_eq!(ATOMIC_VALUE_HASH_LENGTH, 4);
    assert_eq!(1 + ATOMIC_GROUP_HASH_LENGTH + ATOMIC_VALUE_HASH_LENGTH, 11);
  }

  #[test]
  fn hash_base62_output_is_always_exactly_the_requested_length() {
    // Fixed-width (zero-padded) output is required so `ax()` can extract the
    // group key with a fixed-offset slice. Even small hashes must not shrink.
    for input in ["a", "color", "margin", "scrollbar-width", "text-anchor"] {
      assert_eq!(
        hash_base62(input, ATOMIC_GROUP_HASH_LENGTH).len(),
        ATOMIC_GROUP_HASH_LENGTH,
        "group hash for {input:?} was not zero-padded to a fixed width"
      );
      assert_eq!(
        hash_base62(input, ATOMIC_VALUE_HASH_LENGTH).len(),
        ATOMIC_VALUE_HASH_LENGTH,
        "value hash for {input:?} was not zero-padded to a fixed width"
      );
    }
  }

  #[test]
  fn hash_base62_only_emits_base62_characters() {
    // Class names must be valid CSS identifiers. base-62 (0-9, a-z, A-Z)
    // deliberately excludes `-`/`_` which would break `ax()` and CSS parsing.
    let out = hash_base62("some-arbitrary-input-value", 6);
    assert!(
      out
        .chars()
        .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase() || c.is_ascii_uppercase()),
      "unexpected non-base62 char in output: {out}"
    );
  }

  #[test]
  fn hash_base62_is_deterministic_for_the_same_input() {
    // The same input must always hash to the same class name across builds.
    assert_eq!(hash_base62("color", 6), hash_base62("color", 6));
  }

  #[test]
  fn hash_base62_known_good_values_cross_implementation_parity() {
    // These values are verified against compiled/packages/utils/src/hash.ts
    // (hashBase62) and the AFM in-sourced ap_compiled_css crate's hash.rs
    // (hash_base62). All three must produce identical output for the same input —
    // any divergence causes version-skew class-name mismatches across prod/CI
    // (babel plugin), Confluence local dev (AFM crate), and Jira local dev
    // (this crate). If this test fails after touching hash.rs, re-verify
    // parity with the other two implementations before landing.
    assert_eq!(hash_base62("color", 6), "4EWkA1");
    assert_eq!(hash_base62("color", 4), "WkA1");
    assert_eq!(hash_base62("margin", 6), "45uXpk");
    assert_eq!(hash_base62("margin", 4), "uXpk");
    assert_eq!(hash_base62("display", 6), "4i4Gny");
    assert_eq!(hash_base62("display", 4), "4Gny");
    assert_eq!(hash_base62("padding", 6), "4BPML4");
    assert_eq!(hash_base62("padding", 4), "PML4");
    assert_eq!(hash_base62("font-size", 6), "2HlGlw");
    assert_eq!(hash_base62("font-size", 4), "lGlw");
  }
}
