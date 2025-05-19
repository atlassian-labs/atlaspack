use serde::de::DeserializeOwned;

pub fn download_bytes<S: AsRef<str>>(url: S) -> anyhow::Result<Vec<u8>> {
  let url = url.as_ref();
  let response = reqwest::blocking::get(url)?;
  if response.status() != 200 {
    return Err(anyhow::anyhow!("Unable to download {}", url));
  }
  Ok(response.bytes()?.to_vec())
}

pub fn download_string<S: AsRef<str>>(url: S) -> anyhow::Result<String> {
  let result = download_bytes(url)?;
  Ok(String::from_utf8(result)?)
}

pub fn download_serde<D: DeserializeOwned>(url: impl AsRef<str>) -> anyhow::Result<D> {
  let result = download_bytes(url)?;
  Ok(serde_json::from_slice(&result)?)
}
