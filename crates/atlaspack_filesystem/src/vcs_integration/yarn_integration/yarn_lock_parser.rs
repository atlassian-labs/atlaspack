//! On yarn v2, the lockfile is a YAML file.
//!
//! However, on yarn v1, the lockfile is a custom format. This module provides
//! a parser for the yarn.lock file format.
//!
//! Comments are intentionally skipped and not parsed. If we wanted to represent
//! them we'd make a minor change to the file format so that we list statements
//! at the top-level.
//!
//! This is not used at the moment because the yarn state file is missing on
//! this yarn version. In order to support V1, we need to do separate handling
//! of the diffs to expand them onto files, by manually resolving the changed
//! packages.

use std::collections::HashMap;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::complete::take_till;
use nom::bytes::complete::take_while;
use nom::combinator::opt;
use nom::multi::many0;
use nom::IResult;
use nom::Parser;

#[derive(Debug, PartialEq, Clone)]
pub enum YarnV1LockValue {
  String(String),
  Dict(Box<YarnV1LockDict>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct YarnV1Comment(String);

#[derive(Debug, PartialEq, Clone)]
pub struct YarnV1LockDict {
  pub value: HashMap<String, YarnV1LockValue>,
}

impl<T> From<T> for YarnV1LockDict
where
  T: Into<HashMap<String, YarnV1LockValue>>,
{
  fn from(iter: T) -> Self {
    YarnV1LockDict { value: iter.into() }
  }
}

#[derive(Debug, PartialEq, Clone)]
pub struct YarnV1LockFile {
  pub value: YarnV1LockDict,
}

fn yarn_v1_string(input: &str) -> IResult<&str, String> {
  // the strings can be quoted or unquoted
  // if they are unquoted we parse until whitespace or ':'
  // if they are quoted we parse until the closing quote

  fn quoted_string(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("\"")(input)?;
    let (input, s) = take_till(|s| s == '"')(input)?;
    let (input, _) = tag("\"")(input)?;
    Ok((input, s))
  }

  fn unquoted_string(input: &str) -> IResult<&str, &str> {
    if input.is_empty() {
      return Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Eof,
      )));
    }

    let (input, output) = take_till(|s| s == ' ' || s == '\n' || s == ':')(input)?;
    if output.is_empty() {
      return Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Eof,
      )));
    }

    Ok((input, output))
  }

  let (input, s) = alt((quoted_string, unquoted_string))(input)?;
  Ok((input, s.to_string()))
}

fn yarn_v1_pair(input: &str, indentation_level: usize) -> IResult<&str, (String, YarnV1LockValue)> {
  if input.is_empty() {
    return Err(nom::Err::Error(nom::error::Error::new(
      input,
      nom::error::ErrorKind::Eof,
    )));
  }

  let (input, consumed_spaces) = take_while(|c| c == ' ')(input)?;
  if consumed_spaces.len() != indentation_level {
    return Err(nom::Err::Error(nom::error::Error::new(
      input,
      nom::error::ErrorKind::Eof,
    )));
  }

  let (input, key) = yarn_v1_string(input)?;
  let (input, is_nested_dict) = opt(tag(":"))(input)?;
  if is_nested_dict.is_some() {
    let (input, _) = take_while(|c| c == ' ')(input)?;
    let (input, _) = take_while(|c| c == '\n')(input)?;

    // if we're parsing a dict, we'll look-ahead at the indentation level
    let (_input, spaces) = take_while(|c| c == ' ')(input)?;
    let indentation_level = spaces.len();

    let (input, dict) = yarn_v1_dict(input, indentation_level)?;
    Ok((input, (key, YarnV1LockValue::Dict(Box::new(dict)))))
  } else {
    let (input, _) = take_while(|c| c == ' ')(input)?;
    let (input, value) = yarn_v1_string(input)?;
    // consume the newline
    let (input, _) = take_while(|c| c == '\n')(input)?;

    Ok((input, (key, YarnV1LockValue::String(value))))
  }
}

fn skip_empty_lines(input: &str) -> IResult<&str, &str> {
  let (input, line) = take_while(|c| c == '\n')(input)?;
  if line.is_empty() {
    return Err(nom::Err::Error(nom::error::Error::new(
      input,
      nom::error::ErrorKind::Eof,
    )));
  }
  let (input, _) = opt(take_while(|c| c == ' '))(input)?;
  Ok((input, ""))
}

fn yarn_v1_dict(input: &str, indentation_level: usize) -> IResult<&str, YarnV1LockDict> {
  enum YarnStatement {
    Discard,
    Pair((String, YarnV1LockValue)),
  }

  let (input, pairs) = many0(alt((
    skip_empty_lines.map(|_| YarnStatement::Discard),
    yarn_v1_comment.map(|_| YarnStatement::Discard),
    (|input| yarn_v1_pair(input, indentation_level)).map(|p| YarnStatement::Pair(p)),
  )))(input)?;

  let mut dict = HashMap::new();
  for pair in pairs {
    match pair {
      YarnStatement::Pair((key, value)) => {
        dict.insert(key, value);
      }
      _ => {}
    }
  }

  Ok((input, YarnV1LockDict { value: dict }))
}

fn yarn_v1_comment(input: &str) -> IResult<&str, YarnV1Comment> {
  let (input, _) = tag("#")(input)?;
  let (input, comment) = take_till(|c| c == '\n')(input)?;
  Ok((input, YarnV1Comment(comment.to_string())))
}

pub fn yarn_v1_parser(input: &str) -> IResult<&str, YarnV1LockFile> {
  let (input, dict) = yarn_v1_dict(input, 0)?;
  Ok((input, YarnV1LockFile { value: dict }))
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_parse_comment() {
    let result = yarn_v1_comment("# this is a comment\n").unwrap();
    assert_eq!(result.1, YarnV1Comment(" this is a comment".to_string()));
  }

  #[test]
  fn test_parse_string() {
    let result = yarn_v1_string("hello\n").unwrap();
    assert_eq!(result.1, "hello".to_string());
    let result = yarn_v1_string("hello: other stuff").unwrap();
    assert_eq!(result.1, "hello".to_string());
    let result = yarn_v1_string("hello other stuff").unwrap();
    assert_eq!(result.1, "hello".to_string());
  }

  #[test]
  fn test_parse_quoted_string() {
    let result = yarn_v1_string("\"hello\": other stuff").unwrap();
    assert_eq!(result.1, "hello".to_string());
  }

  #[test]
  fn test_parse_key_pair() {
    let (_, result) = yarn_v1_pair(r#"key value"#, 0).unwrap();
    assert_eq!(
      result,
      (
        "key".to_string(),
        YarnV1LockValue::String("value".to_string())
      )
    );
  }

  #[test]
  fn test_parse_key_values() {
    let (_, result) = yarn_v1_dict(
      r#"
key value
"key1" "value"
"#,
      0,
    )
    .unwrap();
    assert_eq!(
      result,
      YarnV1LockDict {
        value: HashMap::from([
          (
            "key".to_string(),
            YarnV1LockValue::String("value".to_string())
          ),
          (
            "key1".to_string(),
            YarnV1LockValue::String("value".to_string())
          )
        ])
      }
    );
  }

  #[test]
  fn test_parse_dict_key_value() {
    let (_, result) = yarn_v1_dict(
      r#"
key:
  "key1" "value"
"#,
      0,
    )
    .unwrap();
    assert_eq!(
      result,
      YarnV1LockDict {
        value: HashMap::from([(
          "key".to_string(),
          YarnV1LockValue::Dict(Box::new(YarnV1LockDict::from([(
            "key1".to_string(),
            YarnV1LockValue::String("value".to_string())
          )])))
        ),])
      }
    );
  }

  #[test]
  fn test_parse_dict_key_followed_by_key() {
    let (_, result) = yarn_v1_dict(
      r#"
key:
  "key1" "value"
other here
"#,
      0,
    )
    .unwrap();
    assert_eq!(
      result,
      YarnV1LockDict {
        value: HashMap::from([
          (
            "key".to_string(),
            YarnV1LockValue::Dict(Box::new(YarnV1LockDict::from([(
              "key1".to_string(),
              YarnV1LockValue::String("value".to_string())
            )])))
          ),
          (
            "other".to_string(),
            YarnV1LockValue::String("here".to_string())
          )
        ])
      }
    );
  }

  #[test]
  fn test_parse_key_nested_values() {
    let (_, result) = yarn_v1_dict(
      r#"
root:
  key1 value
  key2 value
  key3:
    key4 value
    key5 value
other here
"#,
      0,
    )
    .unwrap();
    assert_eq!(
      result,
      YarnV1LockDict {
        value: HashMap::from([
          (
            "root".to_string(),
            YarnV1LockValue::Dict(Box::new(YarnV1LockDict {
              value: HashMap::from([
                (
                  "key1".to_string(),
                  YarnV1LockValue::String("value".to_string())
                ),
                (
                  "key2".to_string(),
                  YarnV1LockValue::String("value".to_string())
                ),
                (
                  "key3".to_string(),
                  YarnV1LockValue::Dict(Box::new(YarnV1LockDict {
                    value: HashMap::from([
                      (
                        "key4".to_string(),
                        YarnV1LockValue::String("value".to_string())
                      ),
                      (
                        "key5".to_string(),
                        YarnV1LockValue::String("value".to_string())
                      )
                    ])
                  }))
                )
              ])
            }))
          ),
          (
            "other".to_string(),
            YarnV1LockValue::String("here".to_string())
          )
        ])
      }
    );
  }

  #[test]
  fn test_parse_yarn_v1_lock() {
    let sample = r#"
# THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.
# yarn lockfile v1


"@ampproject/remapping@^2.2.0":
  version "2.2.1"
  resolved "https://registry.yarnpkg.com/@ampproject/remapping/-/remapping-2.2.1.tgz#99e8e11851128b8702cd57c33684f1d0f260b630"
  integrity sha512-lFMjJTrFL3j7L9yBxwYfCq2k6qqwHyzuUl/XBnif78PWTJYyL/dfowQHWE3sp6U6ZzqWiiIZnpTMO96zhkjwtg==
  dependencies:
    "@jridgewell/gen-mapping" "^0.3.0"
    "@jridgewell/trace-mapping" "^0.3.9"
    "#;

    let (_, result) = yarn_v1_parser(sample).unwrap();

    assert_eq!(
      result.value,
      YarnV1LockDict::from([(
        "@ampproject/remapping@^2.2.0".to_string(),
        YarnV1LockValue::Dict(Box::new(YarnV1LockDict::from([
          (
            "version".to_string(),
            YarnV1LockValue::String("2.2.1".to_string())
          ),
          (
            "resolved".to_string(),
            YarnV1LockValue::String("https://registry.yarnpkg.com/@ampproject/remapping/-/remapping-2.2.1.tgz#99e8e11851128b8702cd57c33684f1d0f260b630".to_string())
          ),
          (
            "integrity".to_string(),
            YarnV1LockValue::String("sha512-lFMjJTrFL3j7L9yBxwYfCq2k6qqwHyzuUl/XBnif78PWTJYyL/dfowQHWE3sp6U6ZzqWiiIZnpTMO96zhkjwtg==".to_string())
          ),
          (
            "dependencies".to_string(),
            YarnV1LockValue::Dict(Box::new(YarnV1LockDict::from([
              (
                "@jridgewell/gen-mapping".to_string(),
                YarnV1LockValue::String("^0.3.0".to_string())
              ),
              (
                "@jridgewell/trace-mapping".to_string(),
                YarnV1LockValue::String("^0.3.9".to_string())
              )
            ])))
          )
        ])))
      )])
    );
  }
}
