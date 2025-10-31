#![allow(clippy::many_single_char_names)]

/// Compute the murmurhash2_gc hash identical to the JavaScript implementation used by
/// `@compiled` packages. The resulting hash is returned as a base36 string to match the
/// Babel plugin output.
pub fn hash(input: &str, seed: u32) -> String {
  let units: Vec<u16> = input.encode_utf16().collect();
  if units.is_empty() {
    return to_base36(murmur2_gc(&units, seed));
  }
  let hash_value = murmur2_gc(&units, seed);
  to_base36(hash_value)
}

fn murmur2_gc(units: &[u16], seed: u32) -> u32 {
  let mut len = units.len();
  let mut h = seed ^ (len as u32);
  let mut index = 0usize;

  while len >= 4 {
    let mut k = (units[index] as u32 & 0xff)
      | (((units[index + 1] as u32) & 0xff) << 8)
      | (((units[index + 2] as u32) & 0xff) << 16)
      | (((units[index + 3] as u32) & 0xff) << 24);

    k = mix_k(k);
    h = mul_mix(h) ^ k;

    index += 4;
    len -= 4;
  }

  match len {
    3 => {
      h ^= ((units[index + 2] as u32) & 0xff) << 16;
      h ^= ((units[index + 1] as u32) & 0xff) << 8;
      h ^= (units[index] as u32) & 0xff;
      h = mul_mix(h);
    }
    2 => {
      h ^= ((units[index + 1] as u32) & 0xff) << 8;
      h ^= (units[index] as u32) & 0xff;
      h = mul_mix(h);
    }
    1 => {
      h ^= (units[index] as u32) & 0xff;
      h = mul_mix(h);
    }
    _ => {}
  }

  h ^= h >> 13;
  h = mul_mix(h);
  h ^= h >> 15;
  h
}

#[inline]
fn mix_k(value: u32) -> u32 {
  let mut v = mul_mix(value);
  v ^= v >> 24;
  mul_mix(v)
}

#[inline]
fn mul_mix(value: u32) -> u32 {
  let low = (value & 0xffff).wrapping_mul(0x5bd1e995);
  let high = ((value >> 16) & 0xffff).wrapping_mul(0x5bd1e995);
  low.wrapping_add(high << 16)
}

fn to_base36(mut value: u32) -> String {
  const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

  if value == 0 {
    return "0".to_string();
  }

  let mut buf = [0u8; 32];
  let mut idx = buf.len();
  while value > 0 {
    let rem = (value % 36) as usize;
    idx -= 1;
    buf[idx] = DIGITS[rem];
    value /= 36;
  }

  String::from_utf8(buf[idx..].to_vec()).expect("base36 string")
}

#[cfg(test)]
mod tests {
  use super::hash;

  #[test]
  fn matches_known_hashes() {
    assert_eq!(hash("compiled", 0), "3mvezc");
    assert_eq!(hash("css", 0), "12w0n9j");
    assert_eq!(hash("keyframes", 0), "1hp1jho");
    assert_eq!(hash("compiled", 1), "yzbs45");
  }

  #[test]
  fn hash_matches_babel_for_direct_is_selector() {
    let group_hash = hash("undefined& >:is(div,button)flex-shrink", 0);
    let value_hash = hash("0", 0);
    let class_name = format!("_{}{}", &group_hash[..4], &value_hash[..4]);
    assert_eq!(class_name, "_1puhidpf");
  }

  #[test]
  fn hash_matches_babel_for_child_star_selector() {
    let group_hash = hash("undefined& >*margin-top", 0);
    let value_hash = hash("0", 0);
    let class_name = format!("_{}{}", &group_hash[..4], &value_hash[..4]);
    assert_eq!(class_name, "_1mizidpf");
  }
}
