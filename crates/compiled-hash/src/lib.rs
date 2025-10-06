const M: u32 = 0x5bd1e995;

fn murmur_mul(a: u32, b: u32) -> u32 {
  // Replicate JavaScript's 32-bit multiplication behavior
  let a_low = a & 0xffff;
  let a_high = a >> 16;
  let b_low = b & 0xffff;
  let b_high = b >> 16;

  let result_low = a_low.wrapping_mul(b_low);
  let result_high = a_high
    .wrapping_mul(b_low)
    .wrapping_add(a_low.wrapping_mul(b_high));

  result_low.wrapping_add((result_high & 0xffff) << 16)
}

pub fn hash(key: &str, seed: u32) -> String {
  let str_bytes = key.as_bytes();
  let mut l = str_bytes.len();
  let mut h = seed ^ (l as u32);
  let mut i = 0;

  while l >= 4 {
    let k = (str_bytes[i] as u32)
      | ((str_bytes[i + 1] as u32) << 8)
      | ((str_bytes[i + 2] as u32) << 16)
      | ((str_bytes[i + 3] as u32) << 24);

    let mut k = murmur_mul(k, M);
    k ^= k >> 24;
    k = murmur_mul(k, M);
    h = murmur_mul(h, M) ^ k;

    l -= 4;
    i += 4;
  }

  // Handle remaining bytes
  match l {
    3 => {
      h ^= (str_bytes[i + 2] as u32) << 16;
      h ^= (str_bytes[i + 1] as u32) << 8;
      h ^= str_bytes[i] as u32;
      h = murmur_mul(h, M);
    }
    2 => {
      h ^= (str_bytes[i + 1] as u32) << 8;
      h ^= str_bytes[i] as u32;
      h = murmur_mul(h, M);
    }
    1 => {
      h ^= str_bytes[i] as u32;
      h = murmur_mul(h, M);
    }
    _ => {}
  }

  h ^= h >> 13;
  h = murmur_mul(h, M);
  h ^= h >> 15;

  // Convert to base36 string - equivalent to JavaScript's (h >>> 0).toString(36)
  fn to_base36(mut num: u32) -> String {
    if num == 0 {
      return "0".to_string();
    }

    let chars = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut result = Vec::new();

    while num > 0 {
      result.push(chars[(num % 36) as usize]);
      num /= 36;
    }

    result.reverse();
    String::from_utf8(result).unwrap()
  }

  to_base36(h)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_case_1() {
    assert_eq!(hash("undefined&font-size", 0), "1wyb1t4");
  }

  #[test]
  fn test_case_2() {
    assert_eq!(hash("undefined&color", 0), "syazsv");
  }

  #[test]
  fn test_case_3() {
    assert_eq!(hash("blue", 0), "13q2bts");
  }

  #[test]
  fn test_case_4() {
    assert_eq!(hash("undefined[data-look='h100']&display", 0), "mi0gz2");
  }

  #[test]
  fn test_case_5() {
    assert_eq!(hash("block", 0), "1ulexfb");
  }

  #[test]
  fn test_case_6() {
    assert_eq!(hash("12px", 0), "1fwxnve");
  }

  #[test]
  fn test_case_7() {
    assert_eq!(hash("undefined&user-select", 0), "uiztiz");
  }

  #[test]
  fn test_case_8() {
    assert_eq!(hash("none", 0), "glywfm");
  }

  #[test]
  fn test_case_9() {
    assert_eq!(hash("undefined&&display", 0), "if29fb");
  }

  #[test]
  fn test_case_10() {
    assert_eq!(hash("undefined&:hoveruser-select", 0), "180hq6f");
  }

  #[test]
  fn test_case_11() {
    assert_eq!(hash("undefined&:focususer-select", 0), "1j5pxr4");
  }

  #[test]
  fn test_case_12() {
    assert_eq!(hash("media(min-width: 30rem)&user-select", 0), "ufx4c2");
  }

  #[test]
  fn test_case_13() {
    assert_eq!(
      hash("media(min-width: 30rem)& divuser-select", 0),
      "195xxsm"
    );
  }

  #[test]
  fn test_case_14() {
    assert_eq!(
      hash(
        "media(min-width: 30rem)media(min-width: 20rem)&user-select",
        0
      ),
      "uf5eh2"
    );
  }

  #[test]
  fn test_case_15() {
    assert_eq!(hash("undefined&display", 0), "1e0ca89");
  }

  #[test]
  fn test_case_16() {
    assert_eq!(hash("undefined&text-align", 0), "y3gnw1");
  }

  #[test]
  fn test_case_17() {
    assert_eq!(hash("center", 0), "1h6ojuz");
  }

  #[test]
  fn test_case_18() {
    assert_eq!(hash("container(width > 300px)& h2color", 0), "eq983t");
  }

  #[test]
  fn test_case_19() {
    assert_eq!(hash("redtrue", 0), "1qpqmqh");
  }

  #[test]
  fn test_case_20() {
    assert_eq!(hash("undefined&font-size", 0), "1wyb1t4");
  }
}
