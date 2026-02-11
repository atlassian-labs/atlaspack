use std::collections::HashSet;

use once_cell::sync::Lazy;
use parking_lot::Mutex;

static WARNED_MESSAGES: Lazy<Mutex<HashSet<String>>> = Lazy::new(|| Mutex::new(HashSet::new()));

/// Emit a warning message to stderr once per process, matching PostCSS's
/// developer warning semantics.
pub fn warn_once(message: &str) {
  let mut guard = WARNED_MESSAGES.lock();
  if guard.insert(message.to_string()) {
    eprintln!("{}", message);
  }
}
