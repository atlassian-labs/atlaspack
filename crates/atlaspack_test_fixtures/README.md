# Atlaspack Test Fixtures

A clean, simple API for creating test fixtures with inline file definitions, similar to the JS `fsFixture` API.

## Features

- ✅ **Inline File Definitions**: Define filesystem structure directly in tests
- ✅ **InMemoryFileSystem Compatible**: Works with Atlaspack's filesystem abstraction
- ✅ **Built-in `indoc`**: Use `{}` for automatic clean indentation of multiline strings
- ✅ **Simple API**: Single macro, you control the dirname

## Usage

### Basic Example

```rust
use atlaspack_test_fixtures::test_fixture;
use std::path::PathBuf;

#[test]
fn test_config_loading() {
    let dirname = PathBuf::from("/test");
    let fs = test_fixture! {
        dirname.clone(),
        ".parcelrc" => r#"{"extends": "@atlaspack/config-default"}"#,
        "package.json" => r#"{"name": "test"}"#
    };

    // Use fs with your config loader
    let result = load_config(&fs, &dirname);
    assert!(result.is_ok());
}
```

### Multi-line Configs with Built-in `indoc`

Use curly braces `{}` for automatic clean indentation:

```rust
use atlaspack_test_fixtures::test_fixture;
use std::path::PathBuf;

#[test]
fn test_complex_config() {
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
                }
            }
        "#}
    };

    // The content will have proper indentation, no leading whitespace
}
```

### Multi-File Setups

```rust
use atlaspack_test_fixtures::test_fixture;
use std::path::PathBuf;

#[test]
fn test_config_inheritance() {
    let dirname = PathBuf::from("/test");
    let fs = test_fixture! {
        dirname,
        // Base config
        "/base/.parcelrc" => {r#"
            {
                "transformers": {
                    "*.svg": ["@atlaspack/transformer-svg"]
                }
            }
        "#},
        
        // User config that extends base
        "/user/.parcelrc" => {r#"
            {
                "extends": "../base/.parcelrc",
                "transformers": {
                    "*.svg": ["./custom-svg-transformer.js"]
                }
            }
        "#}
    };

    // Test config inheritance logic
}
```

## API Reference

### Macro: `test_fixture!`

```rust
test_fixture!(dirname, file_path => content, ...)
```

**Parameters:**
- `dirname: PathBuf` - The project root directory path
- `file_path => content` - File path and content pairs

**Content Formats:**
- `"path" => "content"` - Regular string literal
- `"path" => {r#"content"#}` - Auto-indoc multiline string (recommended for configs)

**Returns:** 
- `FileSystemRef` - The in-memory filesystem for use with Atlaspack APIs

