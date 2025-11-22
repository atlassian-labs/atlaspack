use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::ModuleStaticResult;
use crate::hash::hash;

#[derive(Clone)]
pub(crate) struct ModuleFingerprint {
  pub file_hash: String,
  pub tree_hash: String,
  pub dependencies: Vec<PathBuf>,
}

#[derive(Clone)]
pub(crate) struct ModuleStaticData {
  pub result: ModuleStaticResult,
  pub fingerprint: ModuleFingerprint,
}

#[derive(Default)]
struct ModuleCache {
  entries: HashMap<PathBuf, ModuleStaticData>,
}

static GLOBAL_MODULE_CACHE: Lazy<Mutex<ModuleCache>> =
  Lazy::new(|| Mutex::new(ModuleCache::default()));

impl ModuleCache {
  fn validate(&mut self, path: &Path, visited: &mut HashSet<PathBuf>) -> Option<ModuleStaticData> {
    if !visited.insert(path.to_path_buf()) {
      return self.entries.get(path).cloned();
    }

    let entry = match self.entries.get(path) {
      Some(entry) => entry.clone(),
      None => return None,
    };

    let source = match fs::read_to_string(path) {
      Ok(source) => source,
      Err(_) => {
        self.entries.remove(path);
        return None;
      }
    };

    let current_file_hash = hash(&source, 0);
    let mut dependency_hashes: Vec<(PathBuf, String)> = Vec::new();

    for dep_path in &entry.fingerprint.dependencies {
      match self.validate(dep_path, visited) {
        Some(dep_entry) => {
          dependency_hashes.push((dep_path.clone(), dep_entry.fingerprint.tree_hash.clone()))
        }
        None => {
          self.entries.remove(path);
          return None;
        }
      }
    }

    if dependency_hashes.len() > 1 {
      dependency_hashes.sort_by(|a, b| a.0.cmp(&b.0));
    }

    let current_tree_hash = compute_tree_hash(&current_file_hash, &dependency_hashes);
    if current_tree_hash == entry.fingerprint.tree_hash
      && current_file_hash == entry.fingerprint.file_hash
    {
      Some(entry)
    } else {
      self.entries.remove(path);
      None
    }
  }

  fn get(&mut self, path: &Path) -> Option<ModuleStaticData> {
    let mut visited = HashSet::new();
    self.validate(path, &mut visited)
  }

  fn insert(&mut self, path: PathBuf, data: ModuleStaticData) {
    self.entries.insert(path, data);
  }
}

pub(crate) fn get_cached_module(path: &Path) -> Option<ModuleStaticData> {
  GLOBAL_MODULE_CACHE
    .lock()
    .expect("module cache poisoned")
    .get(path)
}

pub(crate) fn store_cached_module(path: PathBuf, data: ModuleStaticData) {
  GLOBAL_MODULE_CACHE
    .lock()
    .expect("module cache poisoned")
    .insert(path, data);
}

pub(crate) fn compute_tree_hash(file_hash: &str, dependencies: &[(PathBuf, String)]) -> String {
  let mut sorted: Vec<(PathBuf, String)> = dependencies.iter().cloned().collect();
  if sorted.len() > 1 {
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
  }

  let mut buffer = String::new();
  buffer.push_str(file_hash);

  for (path, dep_hash) in sorted {
    buffer.push('|');
    buffer.push_str(path.to_string_lossy().as_ref());
    buffer.push('=');
    buffer.push_str(&dep_hash);
  }

  hash(&buffer, 0)
}
