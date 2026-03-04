//! Query functions for debugging bundling decisions.
//!
//! These functions produce human-readable explanations of bundling decisions
//! from a [`BundlingReport`].

use std::collections::HashMap;
use std::fmt::Write;

use super::types::{AssetInterner, BundleReason, BundleReport, BundlingReport};

fn format_bundle_reason(reason: &BundleReason) -> &'static str {
  match reason {
    BundleReason::EntryPoint => "Entry point",
    BundleReason::LazyImport => "Lazy import",
    BundleReason::Isolated => "Isolated",
    BundleReason::TypeChange => "Type change",
    BundleReason::SharedAssets => "Shared assets",
    BundleReason::Parallel => "Parallel",
  }
}

fn format_number_commas(n: usize) -> String {
  let s = n.to_string();
  let mut out = String::with_capacity(s.len() + (s.len() / 3));

  let mut first_group_len = s.len() % 3;
  if first_group_len == 0 {
    first_group_len = 3;
  }

  out.push_str(&s[..first_group_len]);

  let mut idx = first_group_len;
  while idx < s.len() {
    out.push(',');
    out.push_str(&s[idx..idx + 3]);
    idx += 3;
  }

  out
}

fn compute_common_prefix(paths: &[&str]) -> usize {
  // Filter out synthetic paths (@@shared:, @@typechange:) that would break the prefix.
  let real_paths: Vec<&str> = paths
    .iter()
    .copied()
    .filter(|p| !p.starts_with("@@"))
    .collect();
  if real_paths.is_empty() {
    return 0;
  }

  let mut prefix_len = real_paths[0].len();
  for p in real_paths.iter().skip(1) {
    let max = prefix_len.min(p.len());
    let mut i = 0;
    while i < max && real_paths[0].as_bytes()[i] == p.as_bytes()[i] {
      i += 1;
    }
    prefix_len = i;
    if prefix_len == 0 {
      break;
    }
  }

  // Only strip at a `/` boundary so we don't leave partial segments.
  match real_paths[0][..prefix_len].rfind('/') {
    Some(idx) => idx + 1,
    None => 0,
  }
}

const MAX_DISPLAY_PATH_LEN: usize = 60;

fn shorten_path(path: &str, common_prefix_len: usize) -> String {
  let mut s = if common_prefix_len > 0 && common_prefix_len <= path.len() {
    path[common_prefix_len..].to_string()
  } else {
    path.to_string()
  };

  if s.starts_with('/') {
    s.remove(0);
  }

  for ext in [".ts", ".tsx", ".js", ".jsx", ".css", ".less", ".scss"] {
    if s.ends_with(ext) {
      let new_len = s.len() - ext.len();
      s.truncate(new_len);
      break;
    }
  }

  if s.len() <= MAX_DISPLAY_PATH_LEN {
    return s;
  }

  // Truncate at the nearest `/` boundary to stay under the max length.
  // Show the tail of the path with `...` prefix.
  let target_len = MAX_DISPLAY_PATH_LEN - 4; // room for ".../"
  let tail = &s[s.len() - target_len..];
  match tail.find('/') {
    Some(idx) => format!(".../{}", &tail[idx + 1..]),
    None => format!("...{}", tail),
  }
}

fn bundle_display_name(
  report: &BundleReport,
  _interner: &AssetInterner,
  common_prefix_len: usize,
) -> String {
  if matches!(report.reason, BundleReason::SharedAssets) {
    if report.source_bundles.is_empty() {
      // Fallback to synthetic root path.
      let root = report.root_asset_file_path.as_deref().unwrap_or("?");
      let name = root.strip_prefix("@@shared:").unwrap_or(root);
      return format!("Shared({})", shorten_path(name, common_prefix_len));
    }

    // Show "Shared(from: source1, source2, +N)" using shortened source bundle paths.
    let shortened: Vec<String> = report
      .source_bundles
      .iter()
      .map(|p| {
        let s = shorten_path(p, common_prefix_len);
        // Take last 2 path segments for readability.
        let parts: Vec<&str> = s.rsplitn(3, '/').collect();
        if parts.len() >= 2 {
          format!("{}/{}", parts[1], parts[0])
        } else {
          s
        }
      })
      .collect();

    let max_show = 3;
    if shortened.len() <= max_show {
      return format!("Shared(from: {})", shortened.join(", "));
    }
    let shown: Vec<&str> = shortened
      .iter()
      .take(max_show)
      .map(|s| s.as_str())
      .collect();
    return format!(
      "Shared(from: {}, +{})",
      shown.join(", "),
      shortened.len() - max_show
    );
  }

  if let Some(root) = &report.root_asset_file_path {
    let name = root.strip_prefix("@@typechange:").unwrap_or(root);
    return format!("Bundle({})", shorten_path(name, common_prefix_len));
  }

  "Bundle(?)".to_string()
}

/// Compute the value at a given percentile from a **sorted** slice.
/// `pct` is 0–100. Returns 0 for empty slices.
fn percentile(sorted: &[usize], pct: usize) -> usize {
  if sorted.is_empty() {
    return 0;
  }
  if pct >= 100 {
    return sorted[sorted.len() - 1];
  }
  let idx = (pct * (sorted.len() - 1)) / 100;
  sorted[idx]
}

fn format_percentage(count: usize, total: usize) -> String {
  if total == 0 {
    return "(0%)".to_string();
  }

  let pct = (count * 100) / total;
  if pct == 0 && count > 0 {
    "(<1%)".to_string()
  } else {
    format!("({}%)", pct)
  }
}

fn format_stat_line(label: &str, value: usize, total: Option<usize>) -> String {
  let mut out = String::new();
  let _ = write!(
    &mut out,
    "  {label:<24}{value:>10}",
    label = label,
    value = format_number_commas(value)
  );

  if let Some(total) = total {
    let pct = format_percentage(value, total);
    let _ = write!(&mut out, "  {}", pct);
  }

  out
}

fn reason_tag(reason: &BundleReason) -> &'static str {
  match reason {
    BundleReason::EntryPoint => "entry",
    BundleReason::LazyImport => "lazy",
    BundleReason::SharedAssets => "shared",
    BundleReason::TypeChange => "type-change",
    BundleReason::Isolated => "isolated",
    BundleReason::Parallel => "parallel",
  }
}

/// Produce a comprehensive, formatted bundling report.
pub fn format_full_report(report: &BundlingReport, interner: &AssetInterner) -> String {
  let mut all_paths: Vec<&str> = Vec::new();
  for b in &report.bundles {
    if let Some(root) = &b.root_asset_file_path {
      all_paths.push(root);
    }
  }
  let common_prefix_len = compute_common_prefix(&all_paths);

  let total_bundles = report.total_bundles;

  let mut bundles_by_reason: HashMap<&'static str, usize> = HashMap::new();
  let mut bundles_by_type: HashMap<&str, usize> = HashMap::new();

  let mut shared_bundle_asset_counts: Vec<usize> = Vec::new();

  for b in &report.bundles {
    *bundles_by_reason
      .entry(format_bundle_reason(&b.reason))
      .or_insert(0) += 1;
    *bundles_by_type.entry(&b.bundle_type).or_insert(0) += 1;

    if matches!(b.reason, BundleReason::SharedAssets) {
      shared_bundle_asset_counts.push(b.asset_count);
    }
  }

  shared_bundle_asset_counts.sort_unstable();

  let mut out = String::new();
  out
    .push_str("================================================================================\n");
  out.push_str("                          ATLASPACK BUNDLING REPORT\n");
  out.push_str(
    "================================================================================\n\n",
  );

  out.push_str("Overview\n");
  out.push_str("────────────────────────────────────────\n");
  let _ = writeln!(
    &mut out,
    "{}",
    format_stat_line("Total assets:", report.total_assets, None)
  );
  let _ = writeln!(
    &mut out,
    "{}",
    format_stat_line("Total bundles:", report.total_bundles, None)
  );
  let _ = writeln!(
    &mut out,
    "{}",
    format_stat_line("Shared bundles:", report.total_shared_bundles, None)
  );
  let _ = writeln!(
    &mut out,
    "{}",
    format_stat_line(
      "Internalized bundles:",
      report.internalized_bundle_count,
      None
    )
  );
  out.push('\n');

  out.push_str("Bundles by Reason\n");
  out.push_str("────────────────────────────────────────\n");
  for label in [
    "Entry point",
    "Lazy import",
    "Isolated",
    "Type change",
    "Shared assets",
    "Parallel",
  ] {
    let count = bundles_by_reason.get(label).copied().unwrap_or(0);
    if count > 0 {
      let _ = writeln!(
        &mut out,
        "{}",
        format_stat_line(&format!("{}:", label), count, Some(total_bundles))
      );
    }
  }
  out.push('\n');

  out.push_str("Bundles by Type\n");
  out.push_str("────────────────────────────────────────\n");
  let mut types: Vec<(&str, usize)> = bundles_by_type.into_iter().collect();
  types.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
  for (ty, count) in types {
    let _ = writeln!(
      &mut out,
      "{}",
      format_stat_line(&format!("{}:", ty), count, Some(total_bundles))
    );
  }
  out.push('\n');

  out.push_str("Shared Bundle Analysis\n");
  out.push_str("────────────────────────────────────────\n");
  let _ = writeln!(
    &mut out,
    "{}",
    format_stat_line("Total shared bundles:", report.total_shared_bundles, None)
  );
  if !shared_bundle_asset_counts.is_empty() {
    out.push_str("\n  Assets per shared bundle (percentiles):\n");
    for (label, pct) in [
      ("p10", 10),
      ("p25", 25),
      ("p50 (median)", 50),
      ("p75", 75),
      ("p90", 90),
      ("p99", 99),
      ("max", 100),
    ] {
      let val = percentile(&shared_bundle_asset_counts, pct);
      let _ = writeln!(
        &mut out,
        "    {label:<14}{value:>10}",
        label = format!("{}:", label),
        value = format_number_commas(val),
      );
    }
  }
  out.push('\n');

  out.push_str("Largest Bundles (by asset count)\n");
  out.push_str("────────────────────────────────────────\n");

  let mut largest: Vec<&BundleReport> = report.bundles.iter().collect();
  largest.sort_by(|a, b| b.asset_count.cmp(&a.asset_count));
  for (idx, b) in largest.into_iter().take(10).enumerate() {
    let name = bundle_display_name(b, interner, common_prefix_len);
    let assets = format_number_commas(b.asset_count);
    let tag = reason_tag(&b.reason);
    let _ = writeln!(
      &mut out,
      "  {n}. {name:<45} {assets:>7} assets  ({tag})",
      n = idx + 1,
      name = name,
      assets = assets,
      tag = tag
    );
  }

  out.push('\n');
  out
    .push_str("================================================================================\n");

  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_format_full_report_realistic() {
    // Build a realistic-looking report to preview the formatted output.
    let interner = AssetInterner::default();

    let bundles = vec![
      BundleReport {
        bundle_id: super::super::types::IdealBundleId(0),
        reason: BundleReason::EntryPoint,
        bundle_type: "Js".into(),
        root_asset_file_path: Some("src/app/index.tsx".into()),
        asset_count: 4,
        source_bundles: vec![],
      },
      BundleReport {
        bundle_id: super::super::types::IdealBundleId(1),
        reason: BundleReason::LazyImport,
        bundle_type: "Js".into(),
        root_asset_file_path: Some("src/routes/editor/index.tsx".into()),
        asset_count: 3,
        source_bundles: vec![],
      },
      BundleReport {
        bundle_id: super::super::types::IdealBundleId(2),
        reason: BundleReason::LazyImport,
        bundle_type: "Js".into(),
        root_asset_file_path: Some("src/routes/dashboard/index.tsx".into()),
        asset_count: 2,
        source_bundles: vec![],
      },
      BundleReport {
        bundle_id: super::super::types::IdealBundleId(3),
        reason: BundleReason::LazyImport,
        bundle_type: "Js".into(),
        root_asset_file_path: Some("src/routes/settings/index.tsx".into()),
        asset_count: 2,
        source_bundles: vec![],
      },
      BundleReport {
        bundle_id: super::super::types::IdealBundleId(4),
        reason: BundleReason::SharedAssets,
        bundle_type: "Js".into(),
        root_asset_file_path: Some("@@shared:Button+Icon+Modal".into()),
        asset_count: 3,
        source_bundles: vec![
          "src/routes/editor/index.tsx".into(),
          "src/routes/dashboard/index.tsx".into(),
        ],
      },
      BundleReport {
        bundle_id: super::super::types::IdealBundleId(5),
        reason: BundleReason::TypeChange,
        bundle_type: "Css".into(),
        root_asset_file_path: Some("src/styles/main.css".into()),
        asset_count: 1,
        source_bundles: vec![],
      },
    ];

    let report = BundlingReport {
      bundles,
      total_assets: 15,
      total_bundles: 6,
      total_shared_bundles: 1,
      internalized_bundle_count: 0,
    };

    let out = format_full_report(&report, &interner);
    println!("\n{}", out);

    // Basic structural assertions
    assert!(out.contains("ATLASPACK BUNDLING REPORT"));
    assert!(out.contains("Bundle(app/index)"));
    assert!(out.contains("Bundle(routes/editor/index)"));
    assert!(out.contains("Shared(from: editor/index, dashboard/index)"));
    assert!(out.contains("Css"));
  }

  #[test]
  fn test_format_full_report_empty() {
    let report = BundlingReport {
      bundles: vec![],
      total_assets: 0,
      total_bundles: 0,
      total_shared_bundles: 0,
      internalized_bundle_count: 0,
    };

    let interner = AssetInterner::default();
    let out = format_full_report(&report, &interner);

    assert!(out.contains("ATLASPACK BUNDLING REPORT"));
    assert!(out.contains("Overview"));
    assert!(out.contains("Total assets:"));
    assert!(out.contains("Largest Bundles"));
  }
}
