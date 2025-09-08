use std::fs;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

pub trait JsonSerde: Serialize + DeserializeOwned {
  fn read_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
    let path = path.as_ref();

    if !fs::exists(path)? {
      return Err(anyhow::anyhow!("Failed to find file {:?}", path));
    }

    Ok(serde_json::from_slice::<Self>(&std::fs::read(path)?)?)
  }

  fn write_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
    Ok(fs::write(
      path,
      serde_json::to_string_pretty::<Self>(self)?,
    )?)
  }
}
