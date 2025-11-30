use crate::postcss::value_parser as vp;
use percent_encoding::percent_decode_str;
use postcss as pc;
use regex::Regex;
use url::Url;

// ===== Strict port of postcss-normalize-url + normalize-url (6.1.0) =====

// Matches JS ABSOLUTE_URL_REGEX and WINDOWS_PATH_REGEX
fn is_absolute(url: &str) -> bool {
  // Absolute URL scheme
  let abs = Regex::new(r"^[a-zA-Z][a-zA-Z\d+\-.]*?:").unwrap();
  // Windows paths like `c:\` or `c:/`
  let windows = Regex::new(
    r"^[a-zA-Z]:\\|
                               ^[a-zA-Z]:/",
  )
  .unwrap();
  if windows.is_match(url) {
    return false;
  }
  abs.is_match(url)
}

#[derive(Clone)]
struct NormalizeOptions {
  default_protocol: String,
  normalize_protocol: bool,
  force_http: bool,
  force_https: bool,
  strip_authentication: bool,
  strip_hash: bool,
  strip_text_fragment: bool,
  strip_www: bool,
  remove_query_parameters_default: bool, // whether default UTM regex set is active
  remove_query_parameters_all: bool,     // when true, remove all
  remove_trailing_slash: bool,
  remove_single_slash: bool,
  remove_directory_index: bool,
  sort_query_parameters: bool,
  strip_protocol: bool,
}

impl Default for NormalizeOptions {
  fn default() -> Self {
    Self {
      default_protocol: "http:".to_string(),
      normalize_protocol: true,
      force_http: false,
      force_https: false,
      strip_authentication: true,
      strip_hash: false,
      strip_text_fragment: true,
      strip_www: true,
      remove_query_parameters_default: true, // [/^utm_\w+/i]
      remove_query_parameters_all: false,
      remove_trailing_slash: true,
      remove_single_slash: true,
      remove_directory_index: false,
      sort_query_parameters: true,
      strip_protocol: false,
    }
  }
}

// Default overrides set by postcss-normalize-url
fn plugin_default_options() -> NormalizeOptions {
  let mut o = NormalizeOptions::default();
  o.normalize_protocol = false;
  o.sort_query_parameters = false;
  o.strip_hash = false;
  o.strip_www = false;
  o.strip_text_fragment = false;
  o
}

fn test_parameter(name: &str) -> bool {
  // default filters: [/^utm_\w+/i]
  let re = Regex::new(r"(?i)^utm_\w+").unwrap();
  re.is_match(name)
}

fn normalize_data_url(url: &str, strip_hash: bool) -> Result<String, ()> {
  let re = Regex::new(r"(?i)^data:(?P<type>[^,]*?),(?P<data>[^#]*?)(?:#(?P<hash>.*))?$").unwrap();
  let caps = re.captures(url).ok_or(())?;
  let typ = caps
    .name("type")
    .map(|m| m.as_str())
    .unwrap_or("")
    .to_string();
  let data = caps
    .name("data")
    .map(|m| m.as_str())
    .unwrap_or("")
    .to_string();
  let mut hash = caps
    .name("hash")
    .map(|m| m.as_str())
    .unwrap_or("")
    .to_string();
  if strip_hash {
    hash.clear();
  }

  let mut media: Vec<String> = if typ.is_empty() {
    vec![]
  } else {
    typ.split(';').map(|s| s.to_string()).collect()
  };
  let mut is_base64 = false;
  if media.last().map(|s| s.as_str()) == Some("base64") {
    media.pop();
    is_base64 = true;
  }
  let mime_type = media
    .get(0)
    .map(|s| s.to_lowercase())
    .unwrap_or_else(|| "".to_string());
  let mut attrs: Vec<String> = media
    .into_iter()
    .skip(1)
    .map(|attribute| {
      let mut parts = attribute.splitn(2, '=').map(|s| s.trim().to_string());
      let key = parts.next().unwrap_or_default();
      let mut value = parts.next().unwrap_or_default();
      if key == "charset" {
        value = value.to_lowercase();
        if value == "us-ascii" {
          return String::new();
        }
      }
      if value.is_empty() {
        key
      } else {
        format!("{}={}", key, value)
      }
    })
    .filter(|s| !s.is_empty())
    .collect();
  if is_base64 {
    attrs.push("base64".to_string());
  }
  if !attrs.is_empty() || (!mime_type.is_empty() && mime_type != "text/plain") {
    attrs.insert(0, mime_type);
  }
  let hash_part = if hash.is_empty() {
    String::new()
  } else {
    format!("#{}", hash)
  };
  Ok(format!(
    "data:{},{}{}",
    attrs.join(";"),
    if is_base64 {
      data.trim().to_string()
    } else {
      data
    },
    hash_part
  ))
}

fn remove_duplicate_slashes_not_after_protocol(pathname: &str) -> String {
  // Collapse multiple slashes that are not after a schema like xx:/
  let mut out = String::with_capacity(pathname.len());
  let mut prev_was_slash = false;
  let mut i = 0usize;
  while i < pathname.len() {
    let ch = pathname.as_bytes()[i] as char;
    if ch == '/' {
      if prev_was_slash {
        i += 1;
        continue;
      }
      prev_was_slash = true;
    } else {
      prev_was_slash = false;
    }
    out.push(ch);
    i += 1;
  }
  out
}

fn collapse_dots(parts: &[&str], keep_root: bool) -> Vec<String> {
  let mut stack: Vec<String> = Vec::new();
  for part in parts {
    if part.is_empty() || *part == "." {
      continue;
    }
    if *part == ".." {
      if !stack.is_empty() && stack.last().map(|s| s.as_str()) != Some("..") {
        stack.pop();
      } else if !keep_root {
        stack.push("..".to_string());
      }
    } else {
      stack.push((*part).to_string());
    }
  }
  stack
}

fn path_normalize_like_node(input: &str) -> String {
  // Emulate Node's path.normalize for relative/absolute paths in a platform-agnostic way
  // 1) Unify separators to '/'
  let mut s = input.replace('\\', "/");
  // 2) Preserve drive prefix like 'C:' if present
  let mut prefix = String::new();
  if let Some(capt) = Regex::new(r"^[A-Za-z]:").unwrap().find(&s) {
    prefix = s[capt.start()..capt.end()].to_string();
    s = s[capt.end()..].to_string();
  }
  let is_abs = s.starts_with('/');
  let parts: Vec<&str> = s.split('/').collect();
  let collapsed = collapse_dots(&parts, is_abs);
  let mut out = collapsed.join("/");
  if is_abs {
    out = format!("/{}", out);
  }
  if !prefix.is_empty() {
    out = format!("{}{}", prefix, out);
  }
  if out.is_empty() {
    out = if is_abs {
      "/".to_string()
    } else {
      ".".to_string()
    };
  }
  out
}

fn strip_trailing_slash_once(s: &str) -> String {
  s.trim_end_matches('/').to_string()
}

fn strip_www(hostname: &str) -> String {
  // Match a single leading "www." that is not followed by another "www."
  // (mirrors the JS lookahead `^(?i)www\.(?!www\.)`).
  let lower = hostname.to_ascii_lowercase();
  if lower.starts_with("www.") && !lower[4..].starts_with("www.") {
    hostname[4..].to_string()
  } else {
    hostname.to_string()
  }
}

fn can_strip_www(hostname: &str) -> bool {
  let lower = hostname.to_ascii_lowercase();
  if !lower.starts_with("www.") {
    return false;
  }
  let rest = &lower[4..];
  if rest.starts_with("www.") || rest.is_empty() {
    return false;
  }
  if !rest.contains('.') {
    return false;
  }
  // Validate label lengths/characters similarly to the original regex.
  for label in rest.split('.') {
    if label.is_empty() || label.len() > 63 {
      return false;
    }
    if !label
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    {
      return false;
    }
  }
  true
}

fn normalize_url_impl(url_string_in: &str, opts: &NormalizeOptions) -> Result<String, ()> {
  // Data URL
  if Regex::new(r"(?i)^data:").unwrap().is_match(url_string_in) {
    return normalize_data_url(url_string_in, opts.strip_hash);
  }
  if Regex::new(r"(?i)^view-source:")
    .unwrap()
    .is_match(url_string_in)
  {
    return Err(());
  }

  let mut url_string = url_string_in.trim().to_string();
  let has_relative_protocol = url_string.starts_with("//");
  let is_relative_url =
    !has_relative_protocol && Regex::new(r"^\.*?/").unwrap().is_match(&url_string);
  if !is_relative_url {
    // Prepend protocol when missing. JS uses a negative lookahead /
    // protocol-relative check; reproduce the behavior without lookarounds.
    let has_protocol = {
      // Scheme per RFC 3986: ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) ":"
      let mut chars = url_string.chars();
      if let Some(first) = chars.next() {
        if first.is_ascii_alphabetic() {
          let mut seen_colon = false;
          for ch in chars {
            if ch == ':' {
              seen_colon = true;
              break;
            }
            if !(ch.is_ascii_alphanumeric() || ch == '+' || ch == '-' || ch == '.') {
              break;
            }
          }
          seen_colon
        } else {
          false
        }
      } else {
        false
      }
    };

    if has_relative_protocol {
      url_string = format!("{}{}", opts.default_protocol, url_string);
    } else if !has_protocol {
      url_string = format!("{}{}", opts.default_protocol, url_string);
    }
  }

  let mut url_obj = Url::parse(&url_string).map_err(|_| ())?;

  if opts.force_http && opts.force_https {
    return Err(());
  }
  if opts.force_http && url_obj.scheme() == "https" {
    url_obj.set_scheme("http").ok();
  }
  if opts.force_https && url_obj.scheme() == "http" {
    url_obj.set_scheme("https").ok();
  }

  if opts.strip_authentication {
    url_obj.set_username("").ok();
    url_obj.set_password(None).ok();
  }

  if opts.strip_hash {
    url_obj.set_fragment(None);
  } else if opts.strip_text_fragment {
    if let Some(f) = url_obj.fragment() {
      let target = format!("#{}", f);
      let replaced = Regex::new(r"(?i)#?:~:text.*?$")
        .unwrap()
        .replace(&target, "");
      let new = replaced.to_string();
      let frag = if new.is_empty() {
        None
      } else {
        Some(&new[1..])
      };
      url_obj.set_fragment(frag);
    }
  }

  // Remove duplicate slashes not preceded by a protocol (applies only to pathname)
  if !url_obj.path().is_empty() {
    let mut pathname = url_obj.path().to_string();
    pathname = remove_duplicate_slashes_not_after_protocol(&pathname);
    // Decode URI octets on pathname
    if !pathname.is_empty() {
      if let Ok(decoded) = std::str::from_utf8(&percent_decode_str(&pathname).collect::<Vec<_>>()) {
        pathname = decoded.to_string();
      }
    }
    url_obj.set_path(&pathname);
  }

  // Remove directory index if requested
  if opts.remove_directory_index {
    let mut path_components: Vec<&str> = url_obj.path().split('/').collect();
    if let Some(last) = path_components.last().cloned() {
      if !last.is_empty() && Regex::new(r"(?i)^index\.[a-z]+$").unwrap().is_match(last) {
        path_components.pop();
        let mut joined = path_components.join("/");
        if !joined.starts_with('/') {
          joined = format!("/{}", joined);
        }
        if !joined.ends_with('/') {
          joined.push('/');
        }
        url_obj.set_path(&joined);
      }
    }
  }

  if let Some(host) = url_obj.host_str() {
    let mut hostname = host.trim_end_matches('.').to_string();
    if opts.strip_www {
      if can_strip_www(&hostname) {
        hostname = strip_www(&hostname);
      }
    }
    // update hostname (Url API has no direct setter for host without port; rebuild authority)
    let port = url_obj.port();
    url_obj.set_host(Some(&hostname)).ok();
    if let Some(p) = port {
      url_obj.set_port(Some(p)).ok();
    }
  }

  // Remove unwanted query parameters
  {
    // Collect into vec
    let mut pairs: Vec<(String, String)> = url_obj
      .query_pairs()
      .map(|(k, v)| (k.to_string(), v.to_string()))
      .collect();
    if opts.remove_query_parameters_all {
      pairs.clear();
    } else if opts.remove_query_parameters_default {
      pairs.retain(|(k, _)| !test_parameter(k));
    }
    if opts.sort_query_parameters {
      pairs.sort_by(|a, b| a.0.cmp(&b.0));
    }
    if pairs.is_empty() {
      url_obj.set_query(None);
    } else {
      let mut qp = url::form_urlencoded::Serializer::new(String::new());
      for (k, v) in pairs {
        qp.append_pair(&k, &v);
      }
      let built = qp.finish();
      url_obj.set_query(Some(&built));
    }
  }

  if opts.remove_trailing_slash {
    let newp = strip_trailing_slash_once(url_obj.path());
    url_obj.set_path(&newp);
  }

  let old_url_string = url_string.clone();
  let mut out = url_obj.to_string();

  if !opts.remove_single_slash
    && url_obj.path() == "/"
    && !old_url_string.ends_with('/')
    && url_obj.fragment().is_none()
  {
    out = out.trim_end_matches('/').to_string();
  }
  if (opts.remove_trailing_slash || url_obj.path() == "/")
    && url_obj.fragment().is_none()
    && opts.remove_single_slash
  {
    out = out.trim_end_matches('/').to_string();
  }

  if has_relative_protocol && !opts.normalize_protocol {
    out = Regex::new(r"^http://")
      .unwrap()
      .replace(&out, "//")
      .to_string();
  }
  if opts.strip_protocol {
    out = Regex::new(r"^(?:https?:)?//")
      .unwrap()
      .replace(&out, "")
      .to_string();
  }

  Ok(out)
}

fn convert(url: &str, opts: &NormalizeOptions) -> String {
  if is_absolute(url) || url.starts_with("//") {
    match normalize_url_impl(url, opts) {
      Ok(s) => s,
      Err(_) => url.to_string(),
    }
  } else {
    // Emulate path.normalize then replace path.sep with '/'
    path_normalize_like_node(url)
  }
}

pub fn plugin() -> pc::BuiltPlugin {
  pc::plugin("postcss-normalize-url")
    .once_exit(|css, _| {
      let opts = plugin_default_options();
      let multiline = Regex::new(r"\\[\r\n]").unwrap();
      let escape_chars = Regex::new(r#"([\s\(\)"'])"#).unwrap();

      match css {
        pc::ast::nodes::RootLike::Root(root) => {
          root.walk_decls(|node, _| {
            if let Some(decl) = postcss::ast::nodes::as_declaration(&node) {
              let mut parsed = vp::parse(&decl.value());
              let mut nodes = parsed.nodes.clone();
              vp::walk(
                &mut nodes[..],
                &mut |n| match n {
                  vp::Node::Function {
                    value,
                    nodes: inner,
                    before,
                    after,
                    ..
                  } => {
                    if value.to_lowercase() != "url" {
                      return true;
                    }
                    *before = String::new();
                    *after = String::new();
                    if inner.is_empty() {
                      return true;
                    }
                    let first = &mut inner[0];
                    match first {
                      vp::Node::String {
                        value: s, quote, ..
                      } => {
                        let mut v = s.trim().to_string();
                        v = multiline.replace_all(&v, "").to_string();
                        if v.is_empty() {
                          *quote = '\0';
                          return true;
                        }
                        if Regex::new(r"(?i)^data:(.*)?,").unwrap().is_match(&v) {
                          return true;
                        }
                        if !Regex::new(r"(?i)^.+-extension:/").unwrap().is_match(&v) {
                          v = convert(&v, &opts);
                        }
                        if escape_chars.is_match(&v) {
                          let escaped = escape_chars.replace_all(&v, r"\$1").to_string();
                          if escaped.len() < s.len() + 2 {
                            *n = vp::Node::Word { value: escaped };
                          } else {
                            *s = v;
                          }
                        } else {
                          *n = vp::Node::Word { value: v };
                        }
                      }
                      vp::Node::Word { value } => {
                        let mut v = value.trim().to_string();
                        v = multiline.replace_all(&v, "").to_string();
                        if v.is_empty() {
                          return true;
                        }
                        if Regex::new(r"(?i)^data:(.*)?,").unwrap().is_match(&v) {
                          return true;
                        }
                        if !Regex::new(r"(?i)^.+-extension:/").unwrap().is_match(&v) {
                          v = convert(&v, &opts);
                        }
                        *value = v;
                      }
                      _ => {}
                    }
                    true
                  }
                  _ => true,
                },
                false,
              );
              parsed.nodes = nodes;
              decl.set_value(vp::stringify(&parsed.nodes));
            }
            true
          });

          root.walk_at_rules(|node, _| {
            let (is_namespace, params_str) = {
              let borrowed = node.borrow();
              match &borrowed.data {
                postcss::ast::NodeData::AtRule(data) => (
                  data.name.eq_ignore_ascii_case("namespace"),
                  data.params.clone(),
                ),
                _ => (false, String::new()),
              }
            };
            if !is_namespace {
              return true;
            }

            let mut params = vp::parse(&params_str);
            let mut nodes = params.nodes.clone();
            vp::walk(
              &mut nodes[..],
              &mut |n| match n {
                vp::Node::Function {
                  value,
                  nodes: inner,
                  before,
                  after,
                  ..
                } => {
                  if value.to_lowercase() != "url" || inner.is_empty() {
                    return true;
                  }
                  *before = String::new();
                  let mut quote = '"';
                  if let vp::Node::String { quote: q, .. } = inner[0].clone() {
                    quote = q;
                  }
                  if let vp::Node::String {
                    value: s, quote: q, ..
                  } = &mut inner[0]
                  {
                    *q = quote;
                    *s = s.trim().to_string();
                  }
                  *after = String::new();
                  true
                }
                vp::Node::String { value: s, .. } => {
                  *s = s.trim().to_string();
                  true
                }
                _ => true,
              },
              false,
            );
            params.nodes = nodes;
            let new_params = vp::stringify(&params.nodes);
            {
              let mut borrowed = node.borrow_mut();
              if let postcss::ast::NodeData::AtRule(data) = &mut borrowed.data {
                data.params = new_params;
              }
            }
            true
          });
        }
        pc::ast::nodes::RootLike::Document(doc) => {
          doc.walk_decls(|node, _| {
            if let Some(decl) = postcss::ast::nodes::as_declaration(&node) {
              let mut parsed = vp::parse(&decl.value());
              let mut nodes = parsed.nodes.clone();
              vp::walk(
                &mut nodes[..],
                &mut |n| match n {
                  vp::Node::Function {
                    value,
                    nodes: inner,
                    before,
                    after,
                    ..
                  } => {
                    if value.to_lowercase() != "url" {
                      return true;
                    }
                    *before = String::new();
                    *after = String::new();
                    if inner.is_empty() {
                      return true;
                    }
                    let first = &mut inner[0];
                    match first {
                      vp::Node::String {
                        value: s, quote, ..
                      } => {
                        let mut v = s.trim().to_string();
                        v = multiline.replace_all(&v, "").to_string();
                        if v.is_empty() {
                          *quote = '\0';
                          return true;
                        }
                        if Regex::new(r"(?i)^data:(.*)?,").unwrap().is_match(&v) {
                          return true;
                        }
                        if !Regex::new(r"(?i)^.+-extension:/").unwrap().is_match(&v) {
                          v = convert(&v, &opts);
                        }
                        if escape_chars.is_match(&v) {
                          let escaped = escape_chars.replace_all(&v, r"\$1").to_string();
                          if escaped.len() < s.len() + 2 {
                            *n = vp::Node::Word { value: escaped };
                          } else {
                            *s = v;
                          }
                        } else {
                          *n = vp::Node::Word { value: v };
                        }
                      }
                      vp::Node::Word { value } => {
                        let mut v = value.trim().to_string();
                        v = multiline.replace_all(&v, "").to_string();
                        if v.is_empty() {
                          return true;
                        }
                        if Regex::new(r"(?i)^data:(.*)?,").unwrap().is_match(&v) {
                          return true;
                        }
                        if !Regex::new(r"(?i)^.+-extension:/").unwrap().is_match(&v) {
                          v = convert(&v, &opts);
                        }
                        *value = v;
                      }
                      _ => {}
                    }
                    true
                  }
                  _ => true,
                },
                false,
              );
              parsed.nodes = nodes;
              decl.set_value(vp::stringify(&parsed.nodes));
            }
            true
          });

          doc.walk_at_rules(|node, _| {
            let (is_namespace, params_str) = {
              let borrowed = node.borrow();
              match &borrowed.data {
                postcss::ast::NodeData::AtRule(data) => (
                  data.name.eq_ignore_ascii_case("namespace"),
                  data.params.clone(),
                ),
                _ => (false, String::new()),
              }
            };
            if !is_namespace {
              return true;
            }

            let mut params = vp::parse(&params_str);
            let mut nodes = params.nodes.clone();
            vp::walk(
              &mut nodes[..],
              &mut |n| match n {
                vp::Node::Function {
                  value,
                  nodes: inner,
                  before,
                  after,
                  ..
                } => {
                  if value.to_lowercase() != "url" || inner.is_empty() {
                    return true;
                  }
                  *before = String::new();
                  let mut quote = '"';
                  if let vp::Node::String { quote: q, .. } = inner[0].clone() {
                    quote = q;
                  }
                  if let vp::Node::String {
                    value: s, quote: q, ..
                  } = &mut inner[0]
                  {
                    *q = quote;
                    *s = s.trim().to_string();
                  }
                  *after = String::new();
                  true
                }
                vp::Node::String { value: s, .. } => {
                  *s = s.trim().to_string();
                  true
                }
                _ => true,
              },
              false,
            );
            params.nodes = nodes;
            let new_params = vp::stringify(&params.nodes);
            {
              let mut borrowed = node.borrow_mut();
              if let postcss::ast::NodeData::AtRule(data) = &mut borrowed.data {
                data.params = new_params;
              }
            }
            true
          });
        }
      }
      Ok(())
    })
    .build()
}
