use base64::Engine;
use sha1::Sha1;
use sha2::Digest;
use sha2::Sha256;
use sha2::Sha512;

#[derive(Debug, PartialEq)]
pub enum Integrity {
  Sha512(String),
  Sha256(String),
  Sha1(String),
}

impl Integrity {
  pub fn parse(input: impl AsRef<str>) -> anyhow::Result<Self> {
    let input = input.as_ref();
    log::info!("parse:hash {}", input);

    let Some((tag, hash)) = input.split_once("-") else {
      return Err(anyhow::anyhow!("Unable to parse hash"));
    };
    match tag {
      "sha512" => Ok(Self::Sha512(hash.to_string())),
      "sha256" => Ok(Self::Sha256(hash.to_string())),
      "sha1" => Ok(Self::Sha1(hash.to_string())),
      _ => Err(anyhow::anyhow!("Unsupported hash algorithm {}", tag)),
    }
  }

  pub fn sha512(bytes: &[u8]) -> Self {
    let result = Sha512::digest(bytes);
    let b64 = format!("{}", base64::prelude::BASE64_STANDARD.encode(result));
    log::info!("encode:hash:sha512 {}", b64);
    Self::Sha512(b64)
  }

  pub fn sha256(bytes: &[u8]) -> Self {
    let result = Sha256::digest(bytes);
    let b64 = format!("{}", base64::prelude::BASE64_STANDARD.encode(result));
    log::info!("encode:hash:sha256 {}", b64);
    Self::Sha256(b64)
  }

  pub fn sha1(bytes: &[u8]) -> Self {
    let result = Sha1::digest(bytes);
    let b64 = format!("{}", base64::prelude::BASE64_STANDARD.encode(result));
    log::info!("encode:hash:sha1 {}", b64);
    Self::Sha1(b64)
  }

  pub fn eq(&self, bytes: &[u8]) -> bool {
    match self {
      Integrity::Sha512(hash) => Self::sha512(bytes).b64() == hash,
      Integrity::Sha256(hash) => Self::sha256(bytes).b64() == hash,
      Integrity::Sha1(hash) => Self::sha1(bytes).b64() == hash,
    }
  }

  pub fn b64(&self) -> &str {
    match self {
      Integrity::Sha512(hash) => hash,
      Integrity::Sha256(hash) => hash,
      Integrity::Sha1(hash) => hash,
    }
  }
}
