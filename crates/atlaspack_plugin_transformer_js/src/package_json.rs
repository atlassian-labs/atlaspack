use nodejs_semver::Range;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct DependencyList {
  pub react: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
  pub dependencies: Option<DependencyList>,
  pub dev_dependencies: Option<DependencyList>,
  pub peer_dependencies: Option<DependencyList>,
}

pub fn depends_on_react(package_json: &PackageJson) -> bool {
  [
    package_json.dependencies.as_ref(),
    package_json.dev_dependencies.as_ref(),
    package_json.peer_dependencies.as_ref(),
  ]
  .iter()
  .any(|dependency_list| dependency_list.is_some_and(|d| d.react.is_some()))
}

pub fn supports_automatic_jsx_runtime(package_json: &PackageJson) -> bool {
  [
    package_json.dependencies.as_ref(),
    package_json.dev_dependencies.as_ref(),
    package_json.peer_dependencies.as_ref(),
  ]
  .iter()
  .any(|dependency_list| {
    dependency_list.is_some_and(|d| {
      d.react.as_ref().is_some_and(|r| {
        Range::parse(r).is_ok_and(|r| {
          r.min_version()
            .is_some_and(|v| v.major > 16 || (v.major == 16 && v.minor >= 14))
        })
      })
    })
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  mod package_json_struct {
    use super::*;

    use serde_json::from_str;

    #[test]
    fn deserializes_empty_dependencies() -> anyhow::Result<()> {
      assert_eq!(
        from_str::<PackageJson>(r#"{}"#)?,
        PackageJson {
          dependencies: None,
          dev_dependencies: None,
          peer_dependencies: None
        }
      );

      assert_eq!(
        from_str::<PackageJson>(
          r#"{
            "dependencies": {},
            "devDependencies": {},
            "peerDependencies": {}
          }"#
        )?,
        PackageJson {
          dependencies: Some(DependencyList { react: None }),
          dev_dependencies: Some(DependencyList { react: None }),
          peer_dependencies: Some(DependencyList { react: None }),
        }
      );

      Ok(())
    }

    #[test]
    fn deserializes_react_dependency() -> anyhow::Result<()> {
      assert_eq!(
        from_str::<PackageJson>(
          r#"{
            "dependencies": {
              "react": "1.0.0"
            },
            "devDependencies": {
              "react": "^1.0.0"
            },
            "peerDependencies": {
              "react": "> 1.0.0"
            }
          }"#
        )?,
        PackageJson {
          dependencies: Some(DependencyList {
            react: Some(String::from("1.0.0"))
          }),
          dev_dependencies: Some(DependencyList {
            react: Some(String::from("^1.0.0"))
          }),
          peer_dependencies: Some(DependencyList {
            react: Some(String::from("> 1.0.0"))
          }),
        }
      );

      Ok(())
    }

    #[test]
    fn deserializes_dependencies() -> anyhow::Result<()> {
      assert_eq!(
        from_str::<PackageJson>(
          r#"{
            "dependencies": {
              "foo": "^1.0.0"
            },
            "devDependencies": {
              "bar": "^1.0.0"
            },
            "peerDependencies": {
              "baz": "^1.0.0"
            }
          }"#
        )?,
        PackageJson {
          dependencies: Some(DependencyList { react: None }),
          dev_dependencies: Some(DependencyList { react: None }),
          peer_dependencies: Some(DependencyList { react: None }),
        }
      );

      Ok(())
    }

    #[test]
    fn deserializes_dependencies_with_react() -> anyhow::Result<()> {
      assert_eq!(
        from_str::<PackageJson>(
          r#"{
            "dependencies": {
              "foo": "0.0.0",
              "react": "^1.0.0"
            },
            "devDependencies": {
              "bar": "0.0.0",
              "react": "^2.0.0"
            },
            "peerDependencies": {
              "baz": "0.0.0",
              "react": "^3.0.0"
            },
            "meta": {}
          }"#
        )?,
        PackageJson {
          dependencies: Some(DependencyList {
            react: Some(String::from("^1.0.0"))
          }),
          dev_dependencies: Some(DependencyList {
            react: Some(String::from("^2.0.0"))
          }),
          peer_dependencies: Some(DependencyList {
            react: Some(String::from("^3.0.0"))
          }),
        }
      );

      Ok(())
    }
  }

  mod depends_on_react {
    use super::*;

    #[test]
    fn returns_false_when_react_is_not_present() {
      assert!(!depends_on_react(&PackageJson {
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
      }));
    }

    #[test]
    fn returns_true_when_react_is_present_in_dependencies() {
      assert!(depends_on_react(&PackageJson {
        dependencies: Some(DependencyList {
          react: Some(String::default())
        }),
        dev_dependencies: None,
        peer_dependencies: None
      }));
    }

    #[test]
    fn returns_true_when_react_is_present_in_dev_dependencies() {
      assert!(depends_on_react(&PackageJson {
        dependencies: None,
        dev_dependencies: Some(DependencyList {
          react: Some(String::default())
        }),
        peer_dependencies: None
      }));
    }

    #[test]
    fn returns_true_when_react_is_present_in_peer_dependencies() {
      assert!(depends_on_react(&PackageJson {
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: Some(DependencyList {
          react: Some(String::default())
        }),
      }));
    }

    #[test]
    fn returns_true_when_react_is_present_in_all_dependencies() {
      assert!(depends_on_react(&PackageJson {
        dependencies: Some(DependencyList {
          react: Some(String::default())
        }),
        dev_dependencies: Some(DependencyList {
          react: Some(String::default())
        }),
        peer_dependencies: Some(DependencyList {
          react: Some(String::default())
        }),
      }));
    }
  }

  mod supports_automatic_jsx_runtime {
    use super::*;

    #[test]
    fn returns_false_when_react_is_not_present() {
      assert!(!supports_automatic_jsx_runtime(&PackageJson {
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
      }));
    }

    #[test]
    fn returns_false_when_react_dependency_is_below_range() {
      for version in unsupported_versions() {
        assert!(!supports_automatic_jsx_runtime(&PackageJson {
          dependencies: Some(DependencyList {
            react: Some(version)
          }),
          dev_dependencies: None,
          peer_dependencies: None,
        }));
      }
    }

    #[test]
    fn returns_false_when_react_dev_dependency_is_below_range() {
      for version in unsupported_versions() {
        assert!(!supports_automatic_jsx_runtime(&PackageJson {
          dependencies: None,
          dev_dependencies: Some(DependencyList {
            react: Some(version)
          }),
          peer_dependencies: None,
        }));
      }
    }

    #[test]
    fn returns_false_when_react_peer_dependency_is_below_range() {
      for version in unsupported_versions() {
        assert!(!supports_automatic_jsx_runtime(&PackageJson {
          dependencies: None,
          dev_dependencies: None,
          peer_dependencies: Some(DependencyList {
            react: Some(version)
          }),
        }));
      }
    }

    #[test]
    fn returns_false_when_react_dependencies_are_below_range() {
      for version in unsupported_versions() {
        assert!(!supports_automatic_jsx_runtime(&PackageJson {
          dependencies: Some(DependencyList {
            react: Some(version.clone())
          }),
          dev_dependencies: Some(DependencyList {
            react: Some(version.clone())
          }),
          peer_dependencies: Some(DependencyList {
            react: Some(version)
          }),
        }));
      }
    }

    #[test]
    fn returns_true_when_react_dependency_is_in_range() {
      for version in supported_versions() {
        assert!(supports_automatic_jsx_runtime(&PackageJson {
          dependencies: Some(DependencyList {
            react: Some(version.clone())
          }),
          dev_dependencies: None,
          peer_dependencies: None,
        }));
      }
    }

    #[test]
    fn returns_true_when_react_dev_dependency_is_in_range() {
      for version in supported_versions() {
        assert!(supports_automatic_jsx_runtime(&PackageJson {
          dependencies: None,
          dev_dependencies: Some(DependencyList {
            react: Some(version.clone())
          }),
          peer_dependencies: None,
        }));
      }
    }

    #[test]
    fn returns_true_when_react_peer_dependency_is_in_range() {
      for version in supported_versions() {
        assert!(supports_automatic_jsx_runtime(&PackageJson {
          dependencies: None,
          dev_dependencies: None,
          peer_dependencies: Some(DependencyList {
            react: Some(version.clone())
          }),
        }));
      }
    }

    #[test]
    fn returns_true_when_react_dependencies_are_in_range() {
      for version in supported_versions() {
        assert!(supports_automatic_jsx_runtime(&PackageJson {
          dependencies: Some(DependencyList {
            react: Some(version.clone())
          }),
          dev_dependencies: Some(DependencyList {
            react: Some(version.clone())
          }),
          peer_dependencies: Some(DependencyList {
            react: Some(version)
          }),
        }));
      }
    }

    fn supported_versions() -> Vec<String> {
      vec![
        String::from("^16.14.0"),
        String::from("^18.0.0"),
        String::from("^17.0.0 || ^18.0.0"),
      ]
    }

    fn unsupported_versions() -> Vec<String> {
      vec![
        String::from("^16.0.0"),
        String::from("^16.13.9"),
        String::from("^16.13.9 || ^17.0.0 || ^18.0.0"),
      ]
    }
  }
}
