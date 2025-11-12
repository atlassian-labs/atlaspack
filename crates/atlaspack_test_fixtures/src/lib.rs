use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use atlaspack_filesystem::{FileSystemRef, in_memory_file_system::InMemoryFileSystem};

// Re-export indoc for convenience in tests
pub use indoc::indoc;

/// A test fixture that mimics the JS fsFixture API
#[derive(Clone)]
pub struct TestFixture {
  pub fs: FileSystemRef,
  pub in_memory_fs: Arc<InMemoryFileSystem>, // Keep a direct reference for writing
  pub dirname: PathBuf,
}

impl TestFixture {
  pub fn with_dirname(dirname: PathBuf) -> Self {
    let in_memory_fs = Arc::new(InMemoryFileSystem::default());

    Self {
      fs: in_memory_fs.clone() as FileSystemRef,
      in_memory_fs,
      dirname: dirname,
    }
  }

  /// Set the current working directory
  pub fn with_cwd(self, cwd: PathBuf) -> Self {
    // InMemoryFileSystem doesn't have set_current_working_directory,
    // but we can work with absolute paths instead
    Self {
      fs: self.fs,
      in_memory_fs: self.in_memory_fs,
      dirname: cwd,
    }
  }

  /// Write files from a HashMap structure
  pub fn write_files(&self, files: HashMap<&str, &str>) -> &Self {
    for (path, content) in files {
      let full_path = if Path::new(path).is_absolute() {
        PathBuf::from(path)
      } else {
        self.dirname.join(path)
      };
      self
        .in_memory_fs
        .write_file(&full_path, content.to_string());
    }
    self
  }

  /// Write a single file
  pub fn write_file(&self, path: &str, content: &str) -> &Self {
    let full_path = if Path::new(path).is_absolute() {
      PathBuf::from(path)
    } else {
      self.dirname.join(path)
    };
    self
      .in_memory_fs
      .write_file(&full_path, content.to_string());
    self
  }

  /// Create a directory structure
  pub fn mkdir(&self, path: &str) -> &Self {
    let full_path = if Path::new(path).is_absolute() {
      PathBuf::from(path)
    } else {
      self.dirname.join(path)
    };
    self.fs.create_directory(&full_path).ok();
    self
  }

  /// Chain file operations fluently
  pub fn file(self, path: &str, content: &str) -> Self {
    self.write_file(path, content);
    self
  }

  /// Check if a file exists
  pub fn file_exists(&self, path: &Path) -> bool {
    self.fs.is_file(path)
  }
}

/// Macro to create test files in an InMemoryFileSystem
/// Supports both regular strings and multi-line strings with automatic indoc processing
/// Returns the FileSystemRef for use in tests
#[macro_export]
macro_rules! test_fixture {
    ($dirname:expr, $($path:literal => {$content:literal}),* $(,)?) => {{
        let fixture = $crate::TestFixture::with_dirname($dirname);
        $(
            fixture.write_file($path, $crate::indoc!($content));
        )*
        fixture.fs
    }};

    ($dirname:expr, $($path:literal => $content:expr),* $(,)?) => {{
        let fixture = $crate::TestFixture::with_dirname($dirname);
        $(
            fixture.write_file($path, &$content);
        )*
        fixture.fs
    }};
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_test_fixture_macro_with_simple_strings() {
    let dirname = PathBuf::from("/test");
    let fs = test_fixture! {
        dirname.clone(),
        ".parcelrc" => r#"{"extends": "@atlaspack/config-default"}"#,
        "package.json" => r#"{"name": "test"}"#
    };

    // Verify file content is correct
    assert_eq!(
      fs.read_to_string(&dirname.join(".parcelrc")).unwrap(),
      r#"{"extends": "@atlaspack/config-default"}"#
    );
    assert_eq!(
      fs.read_to_string(&dirname.join("package.json")).unwrap(),
      r#"{"name": "test"}"#
    );
  }

  #[test]
  fn test_built_in_indoc_with_curly_braces() {
    let dirname = PathBuf::from("/test");
    let fs = test_fixture! {
        dirname.clone(),
        ".parcelrc" => {r#"
                {
                    "extends": "@atlaspack/config-default",
                    "transformers": {
                        "*.js": ["@atlaspack/transformer-js"],
                        "*.ts": [
                            "@atlaspack/transformer-typescript",
                            "..."
                        ]
                    },
                    "optimizers": {
                        "*.{js,ts}": ["@atlaspack/optimizer-terser"]
                    }
                }
            "#}
    };

    // Verify the exact content after indoc processing
    let expected_content = indoc! {r#"
            {
                "extends": "@atlaspack/config-default",
                "transformers": {
                    "*.js": ["@atlaspack/transformer-js"],
                    "*.ts": [
                        "@atlaspack/transformer-typescript",
                        "..."
                    ]
                },
                "optimizers": {
                    "*.{js,ts}": ["@atlaspack/optimizer-terser"]
                }
            }
        "#};

    let actual_content = fs.read_to_string(&dirname.join(".parcelrc")).unwrap();
    assert_eq!(actual_content, expected_content);
  }

  #[test]
  fn test_complex_multi_file_setup() {
    let dirname = PathBuf::from("/test");
    let fs = test_fixture! {
        dirname,
        "/base/.parcelrc" => {r#"
                {
                    "transformers": {
                        "*.svg": ["@atlaspack/transformer-svg"]
                    }
                }
            "#},

        "/user/.parcelrc" => {r#"
                {
                    "extends": "../base/.parcelrc",
                    "transformers": {
                        "*.svg": ["./custom-svg-transformer.js"]
                    }
                }
            "#}
    };

    // Verify exact base config content
    let expected_base_content = indoc! {r#"
            {
                "transformers": {
                    "*.svg": ["@atlaspack/transformer-svg"]
                }
            }
        "#};
    let actual_base_content = fs
      .read_to_string(&PathBuf::from("/base/.parcelrc"))
      .unwrap();
    assert_eq!(actual_base_content, expected_base_content);

    // Verify exact user config content
    let expected_user_content = indoc! {r#"
            {
                "extends": "../base/.parcelrc",
                "transformers": {
                    "*.svg": ["./custom-svg-transformer.js"]
                }
            }
        "#};
    let actual_user_content = fs
      .read_to_string(&PathBuf::from("/user/.parcelrc"))
      .unwrap();
    assert_eq!(actual_user_content, expected_user_content);
  }
}
