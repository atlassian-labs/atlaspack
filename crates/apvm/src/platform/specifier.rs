use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Specifier {
  Npm { version: String },
  Git { branch: String },
  Local,
}

impl Specifier {
  pub fn parse<S: AsRef<str>>(input: S) -> anyhow::Result<Self> {
    let specifier = input.as_ref();

    if let Some((origin, specifier)) = specifier.split_once(":") {
      return match origin {
        "npm" => Ok(Self::Npm {
          version: specifier.to_string(),
        }),
        "git" => Ok(Self::Git {
          branch: specifier.to_string(),
        }),
        "local" => Ok(Self::Local),
        _ => Err(anyhow::anyhow!("Cannot parse version specifier")),
      };
    };

    if specifier == "local" {
      return Ok(Self::Local);
    }

    Ok(Self::Npm {
      version: specifier.to_string(),
    })
  }
}

impl From<Specifier> for String {
  fn from(value: Specifier) -> Self {
    format!("{}", value)
  }
}

impl std::fmt::Display for Specifier {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Specifier::Npm { version } => write!(f, "{}", version),
      Specifier::Git { branch } => write!(f, "git:{}", branch),
      Specifier::Local => write!(f, "local"),
    }
  }
}
