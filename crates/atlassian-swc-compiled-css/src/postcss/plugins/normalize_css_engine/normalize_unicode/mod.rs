use postcss as pc;
use regex::Regex;

fn merge_range_bounds(left: &str, right: &str) -> Option<String> {
  let lchars: Vec<char> = left.chars().collect();
  let rchars: Vec<char> = right.chars().collect();
  if lchars.len() != rchars.len() {
    return None;
  }
  let mut question = 0usize;
  let mut group = String::from("u+");
  for i in 0..lchars.len() {
    let lc = lchars[i];
    let rc = rchars[i];
    if lc == rc && question == 0 {
      group.push(lc);
    } else if lc == '0' && rc == 'f' {
      question += 1;
      group.push('?');
    } else {
      return None;
    }
  }
  if question < 6 {
    Some(group)
  } else {
    None
  }
}

fn normalize_single_range(range: &str) -> String {
  // input like u+abcd or u+00-ff or already with wildcards
  let r = range.to_lowercase();
  let mut parts = r[2..].splitn(2, '-');
  let a = parts.next().unwrap_or("");
  if let Some(b) = parts.next() {
    if let Some(merged) = merge_range_bounds(a, b) {
      return merged;
    }
  }
  r
}

pub fn plugin() -> pc::BuiltPlugin {
  let is_legacy = false;
  let re = Regex::new(r"(?i)u\+[0-9a-f?]+(?:-[0-9a-f?]+)?").unwrap();
  pc::plugin("postcss-normalize-unicode")
    .once_exit(move |css, _| {
      let process_decl = |decl: postcss::ast::nodes::Declaration| {
        if decl.prop().eq_ignore_ascii_case("unicode-range") {
          let value = decl.value();
          if value.is_empty() {
            return;
          }
          let newv = re
            .replace_all(&value, |caps: &regex::Captures| {
              let mut out = normalize_single_range(&caps[0]);
              if is_legacy {
                out = Regex::new(r"^u(?=\+)")
                  .unwrap()
                  .replace(&out, "U")
                  .to_string();
              }
              out
            })
            .to_string();
          if newv != value {
            decl.set_value(newv);
          }
        }
      };
      match css {
        pc::ast::nodes::RootLike::Root(root) => {
          root.walk_decls(|node, _| {
            if let Some(decl) = postcss::ast::nodes::as_declaration(&node) {
              process_decl(decl);
            }
            true
          });
        }
        pc::ast::nodes::RootLike::Document(doc) => {
          doc.walk_decls(|node, _| {
            if let Some(decl) = postcss::ast::nodes::as_declaration(&node) {
              process_decl(decl);
            }
            true
          });
        }
      }
      Ok(())
    })
    .build()
}
