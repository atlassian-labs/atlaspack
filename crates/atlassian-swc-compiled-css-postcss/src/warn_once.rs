use std::collections::HashSet;
use std::sync::Mutex;

use once_cell::sync::Lazy;

static WARNED_MESSAGES: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

/// Emit a warning message to stderr once per process, matching PostCSS's
/// developer warning semantics.
pub fn warn_once(message: &str) {
  let mut guard = WARNED_MESSAGES
    .lock()
    .expect("warn_once lock should not be poisoned");
  if guard.insert(message.to_string()) {
    eprintln!("{}", message);
  }
}
