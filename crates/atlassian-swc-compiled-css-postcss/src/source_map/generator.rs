#![allow(clippy::collapsible_match)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use pathdiff::diff_paths;
use sourcemap::SourceMapBuilder;
use url::Url;

use crate::ast::nodes::{self, RootLike};
use crate::ast::NodeRef;
use crate::input::{Input, InputOptions, InputRef, Position};
use crate::processor::CustomStringifier;

use super::options::{MapAnnotation, MapOptions, MapSetting};
use super::previous_map::{PreviousMap, PreviousMapError};

pub struct MapGenerator<'a> {
  root: Option<RootLike>,
  css: Option<String>,
  map_setting: &'a MapSetting,
  map_opts: MapOptions,
  from: Option<&'a str>,
  to: Option<&'a str>,
  previous_maps: Option<Vec<PreviousMap>>,
  stringifier: Arc<dyn CustomStringifier>,
}

#[derive(Clone, Debug, Default)]
pub struct MapResult {
  pub css: String,
  pub map: Option<String>,
}

impl<'a> MapGenerator<'a> {
  pub fn new(
    root: Option<RootLike>,
    css: Option<String>,
    setting: &'a MapSetting,
    from: Option<&'a str>,
    to: Option<&'a str>,
    stringifier: Arc<dyn CustomStringifier>,
  ) -> Self {
    let map_opts = setting.options();

    Self {
      root,
      css,
      map_setting: setting,
      map_opts,
      from,
      to,
      previous_maps: None,
      stringifier,
    }
  }

  pub fn generate(&mut self) -> Result<MapResult, PreviousMapError> {
    let mut css = self.css.take().unwrap_or_default();
    let css_for_maps = if self.root.is_none() {
      Some(css.as_str())
    } else {
      None
    };
    self.ensure_previous_maps(css_for_maps)?;
    let previous_maps = self.previous_maps.as_ref().cloned().unwrap_or_default();
    if self.root.is_some() {
      css.clear();
    }

    self.clear_annotation(&mut css);

    let map_enabled = match self.map_setting {
      MapSetting::Disabled => false,
      MapSetting::Enabled(_) => true,
      MapSetting::Auto => !previous_maps.is_empty(),
    };
    let annotation_enabled = self.is_annotation_enabled(&previous_maps);

    if !map_enabled {
      if let Some(root) = &self.root {
        let mut builder = |chunk: &str, _node: Option<&NodeRef>, _ty: Option<&'static str>| {
          css.push_str(chunk);
        };
        self.stringifier.stringify(root, &mut builder);
      }
      return Ok(MapResult { css, map: None });
    }

    let output_file = self.output_file();
    let mut builder = SourceMapBuilder::new(output_file.as_deref());
    let mut sources = HashMap::<String, u32>::new();
    let mut source_inputs = HashMap::<u32, Option<InputRef>>::new();
    let mut css_output = String::new();

    let mut line: u32 = 1;
    let mut column: u32 = 1;
    let no_source = "<no source>".to_string();
    let base_dir = self.base_dir();

    if let Some(root) = &self.root {
      let mut builder = |chunk: &str, node: Option<&NodeRef>, string_type: Option<&'static str>| {
        if let Some(node_ref) = node {
          let (source_name, input_ref, start_pos) =
            node_start_metadata(node_ref, &self.map_opts, base_dir.as_ref(), self.to);
          if string_type != Some("end") {
            let (source_id, original_line, original_column) = if let Some(start) = start_pos {
              let id = ensure_source(
                &mut builder,
                &mut sources,
                &mut source_inputs,
                &source_name.clone().unwrap_or_else(|| no_source.clone()),
                input_ref.clone(),
              );
              (
                id,
                start.line.saturating_sub(1),
                start.column.saturating_sub(1),
              )
            } else {
              let id = ensure_source(
                &mut builder,
                &mut sources,
                &mut source_inputs,
                &no_source,
                None,
              );
              (id, 0, 0)
            };
            builder.add_raw(
              line.saturating_sub(1),
              column.saturating_sub(1),
              original_line,
              original_column,
              Some(source_id),
              None,
              false,
            );
          }
        }

        css_output.push_str(chunk);

        update_position(chunk, &mut line, &mut column);

        if let Some(node_ref) = node {
          if string_type != Some("start") {
            if let Some((source_name, input_ref, end_pos)) =
              node_end_metadata(node_ref, &self.map_opts, base_dir.as_ref(), self.to)
            {
              if let Some(end) = end_pos {
                let id = ensure_source(
                  &mut builder,
                  &mut sources,
                  &mut source_inputs,
                  &source_name.unwrap_or_else(|| no_source.clone()),
                  input_ref,
                );
                builder.add_raw(
                  line.saturating_sub(1),
                  column.saturating_sub(1),
                  end.line.saturating_sub(1),
                  end.column.saturating_sub(1),
                  Some(id),
                  None,
                  false,
                );
              }
            }
          }
        }
      };
      self.stringifier.stringify(root, &mut builder);
      css = css_output;
    }

    if self.should_set_sources_content() {
      for (id, input) in &source_inputs {
        if let Some(input_ref) = input {
          builder.set_source_contents(*id, Some(input_ref.css()));
        }
      }
    }

    let mut map = builder.into_sourcemap();

    if !previous_maps.is_empty() {
      for prev in previous_maps.iter().rev() {
        let mut combined = (*prev.consumer()).clone();
        combined.adjust_mappings(&map);
        map = combined;
      }
    }

    let sourcemap = {
      let mut buf = Vec::new();
      map
        .to_writer(&mut buf)
        .map_err(|_| PreviousMapError("Failed to serialize source map".to_string()))?;
      String::from_utf8(buf)
        .map_err(|_| PreviousMapError("Serialized source map is not valid UTF-8".to_string()))?
    };

    if annotation_enabled {
      if self.is_inline() {
        self.add_annotation(&mut css, Some(&sourcemap));
        return Ok(MapResult { css, map: None });
      } else {
        self.add_annotation(&mut css, None);
        return Ok(MapResult {
          css,
          map: Some(sourcemap),
        });
      }
    }

    Ok(MapResult {
      css,
      map: Some(sourcemap),
    })
  }
  fn ensure_previous_maps(&mut self, css: Option<&str>) -> Result<(), PreviousMapError> {
    if self.previous_maps.is_none() {
      let mut maps = Vec::new();
      let mut seen = HashSet::new();
      if let Some(root) = &self.root {
        let mut collect = |node: NodeRef, _| {
          let maybe_map = {
            let borrowed = node.borrow();
            borrowed
              .source
              .input
              .as_ref()
              .and_then(|input| input.map.clone())
          };
          if let Some(prev) = maybe_map {
            let key = previous_map_key(&prev);
            if seen.insert(key) {
              maps.push(prev);
            }
          }
          true
        };
        walk_root_like(root, &mut collect);
      } else if let Some(css_text) = css {
        let input = Input::new(
          css_text.to_string(),
          InputOptions {
            from: self.from.map(|f| f.to_string()),
            map: self.map_setting.clone(),
          },
        )?;
        if let Some(prev) = input.map {
          let key = previous_map_key(&prev);
          if seen.insert(key) {
            maps.push(prev);
          }
        }
      }
      self.previous_maps = Some(maps);
    }
    Ok(())
  }

  fn is_inline(&self) -> bool {
    self.map_opts.inline.unwrap_or(false)
  }

  fn is_annotation_enabled(&self, previous_maps: &[PreviousMap]) -> bool {
    if self.is_inline() {
      return true;
    }
    match &self.map_opts.annotation {
      MapAnnotation::Disabled => false,
      MapAnnotation::String(_) | MapAnnotation::Callback(_) => true,
      MapAnnotation::Default => {
        if !previous_maps.is_empty() {
          previous_maps.iter().any(|map| map.annotation.is_some())
        } else {
          true
        }
      }
    }
  }

  fn should_set_sources_content(&self) -> bool {
    self.map_opts.sources_content.unwrap_or(true)
  }

  fn base_dir(&self) -> Option<PathBuf> {
    self
      .to
      .and_then(|to| Path::new(to).parent().map(|p| p.to_path_buf()))
      .or_else(|| {
        self
          .from
          .and_then(|from| Path::new(from).parent().map(|p| p.to_path_buf()))
      })
  }

  fn output_file(&self) -> Option<String> {
    self
      .to
      .map(|to| to.to_string())
      .or_else(|| self.from.map(|from| from.to_string()))
  }

  fn clear_annotation(&self, css: &mut String) {
    if let Some(root) = &self.root {
      let node = root.to_node();
      let mut inner = node.borrow_mut();
      let mut index = inner.nodes.len();
      while index > 0 {
        index -= 1;
        if let Some(comment) = nodes::as_comment(&inner.nodes[index]) {
          if comment
            .text()
            .trim_start()
            .starts_with("# sourceMappingURL=")
          {
            inner.nodes.remove(index);
          }
        }
      }
    } else if !css.is_empty() {
      if let Some(pos) = css.rfind("/*# sourceMappingURL=") {
        let trimmed = css[..pos].trim_end().to_string();
        *css = trimmed;
      }
    }
  }

  fn add_annotation(&self, css: &mut String, map: Option<&str>) {
    let content = if self.is_inline() {
      let encoded = BASE64.encode(map.unwrap_or("{}").as_bytes());
      format!("data:application/json;base64,{}", encoded)
    } else {
      match &self.map_opts.annotation {
        MapAnnotation::String(value) => value.clone(),
        MapAnnotation::Callback(callback) => callback(self.to, self.root.clone()),
        MapAnnotation::Default => {
          format!("{}{}", self.output_file().unwrap_or_default(), ".map")
        }
        MapAnnotation::Disabled => return,
      }
    };

    let eol = if css.contains("\r\n") { "\r\n" } else { "\n" };
    css.push_str(eol);
    css.push_str("/*# sourceMappingURL=");
    css.push_str(&content);
    css.push_str(" */");
  }
}

fn previous_map_key(map: &PreviousMap) -> String {
  let file = map
    .file
    .as_ref()
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_default();
  format!("{}::{}", file, map.text.as_str())
}

fn update_position(chunk: &str, line: &mut u32, column: &mut u32) {
  for ch in chunk.chars() {
    if ch == '\n' {
      *line += 1;
      *column = 1;
    } else {
      *column += 1;
    }
  }
}

fn ensure_source(
  builder: &mut SourceMapBuilder,
  sources: &mut HashMap<String, u32>,
  contents: &mut HashMap<u32, Option<InputRef>>,
  name: &str,
  input: Option<InputRef>,
) -> u32 {
  if let Some(id) = sources.get(name) {
    return *id;
  }
  let id = builder.add_source(name);
  sources.insert(name.to_string(), id);
  contents.insert(id, input);
  id
}

fn node_start_metadata(
  node: &NodeRef,
  opts: &MapOptions,
  base_dir: Option<&PathBuf>,
  to: Option<&str>,
) -> (Option<String>, Option<InputRef>, Option<Position>) {
  let borrowed = node.borrow();
  let input = borrowed.source.input.clone();
  let start = borrowed.source.start.clone();
  drop(borrowed);
  let path = input
    .as_ref()
    .and_then(|input_ref| source_path(input_ref, opts, base_dir, to));
  (path, input, start)
}

fn node_end_metadata(
  node: &NodeRef,
  opts: &MapOptions,
  base_dir: Option<&PathBuf>,
  to: Option<&str>,
) -> Option<(Option<String>, Option<InputRef>, Option<Position>)> {
  let borrowed = node.borrow();
  let input = borrowed.source.input.clone();
  let end = borrowed.source.end.clone();
  drop(borrowed);
  let path = input
    .as_ref()
    .and_then(|input_ref| source_path(input_ref, opts, base_dir, to));
  Some((path, input, end))
}

fn source_path(
  input: &InputRef,
  opts: &MapOptions,
  base_dir: Option<&PathBuf>,
  to: Option<&str>,
) -> Option<String> {
  let file_path = input
    .file
    .as_ref()
    .or_else(|| input.from.as_ref())
    .cloned()
    .or_else(|| input.id.clone());
  let file_path = file_path?;
  if opts.absolute {
    if let Ok(url) = Url::from_file_path(&file_path) {
      return Some(url.to_string());
    }
  }
  if let Some(base) = base_dir {
    if let Some(relative) = diff_paths(&file_path, base) {
      return Some(relative.to_string_lossy().replace('\\', "/"));
    }
  }
  if let Some(target) = to {
    if let Some(dir) = Path::new(target).parent() {
      if let Some(relative) = diff_paths(&file_path, dir) {
        return Some(relative.to_string_lossy().replace('\\', "/"));
      }
    }
  }
  Some(file_path.replace('\\', "/"))
}
fn walk_root_like<F>(root: &RootLike, callback: &mut F) -> bool
where
  F: FnMut(NodeRef, usize) -> bool,
{
  match root {
    RootLike::Root(root) => root.walk(callback),
    RootLike::Document(document) => document.walk(callback),
  }
}
