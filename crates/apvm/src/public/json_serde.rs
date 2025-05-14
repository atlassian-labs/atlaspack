use std::{fs, path::Path};

use serde::{de::DeserializeOwned, Serialize};

pub trait JsonSerde: Serialize + DeserializeOwned {
  fn read_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
    Ok(serde_json::from_slice::<Self>(&std::fs::read(path)?)?)
  }

  fn write_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
    Ok(fs::write(
      path,
      serde_json::to_string_pretty::<Self>(self)?,
    )?)
  }
}
