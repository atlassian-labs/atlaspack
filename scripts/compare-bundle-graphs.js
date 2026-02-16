#!/usr/bin/env node
/* eslint-disable no-console */
'use strict';

const fs = require('fs');
const path = require('path');

function usageAndExit(msg) {
  if (msg) console.log(String(msg));
  console.log(
    [
      'Usage:',
      '  node scripts/compare-bundle-graphs.js <snapshotDir>',
      '',
      'Where <snapshotDir> contains:',
      '  - bundle-graph-js.json',
      '  - bundle-graph-rust.json',
    ].join('\n'),
  );
  process.exit(1);
}

function readJson(filePath) {
  let raw;
  try {
    raw = fs.readFileSync(filePath, 'utf8');
  } catch (e) {
    usageAndExit(`Failed to read ${filePath}: ${e.message}`);
  }

  try {
    return JSON.parse(raw);
  } catch (e) {
    usageAndExit(`Failed to parse JSON ${filePath}: ${e.message}`);
  }
}

function assertSnapshot(snapshot, expectedVariant, filePath) {
  if (!snapshot || typeof snapshot !== 'object') {
    usageAndExit(`Invalid snapshot object in ${filePath}`);
  }
  if (snapshot.version !== 1) {
    usageAndExit(
      `Unsupported snapshot version in ${filePath}: ${snapshot.version} (expected 1)`,
    );
  }
  if (snapshot.variant !== expectedVariant) {
    usageAndExit(
      `Unexpected snapshot variant in ${filePath}: ${snapshot.variant} (expected ${expectedVariant})`,
    );
  }
  if (!snapshot.stats || !snapshot.bundles || !snapshot.bundleGroups) {
    usageAndExit(`Missing required keys in snapshot ${filePath}`);
  }
}

function padRight(s, width) {
  s = String(s);
  if (s.length >= width) return s;
  return s + ' '.repeat(width - s.length);
}

function padLeft(s, width) {
  s = String(s);
  if (s.length >= width) return s;
  return ' '.repeat(width - s.length) + s;
}

function makeTable(headers, rows, options = {}) {
  const align = options.align || headers.map(() => 'left');
  const widths = headers.map((h, i) => {
    let w = String(h).length;
    for (let r of rows) {
      w = Math.max(w, String(r[i] ?? '').length);
    }
    return w;
  });

  const line = '+' + widths.map((w) => '-'.repeat(w + 2)).join('+') + '+';
  const fmtRow = (cols) =>
    '|' +
    cols
      .map((c, i) => {
        let v = String(c ?? '');
        let cell =
          align[i] === 'right' ? padLeft(v, widths[i]) : padRight(v, widths[i]);
        return ' ' + cell + ' ';
      })
      .join('|') +
    '|';

  let out = [];
  out.push(line);
  out.push(fmtRow(headers));
  out.push(line);
  for (let r of rows) out.push(fmtRow(r));
  out.push(line);
  return out.join('\n');
}

function shortenToLastSegments(p, segments = 2) {
  if (!p) return p;
  let parts = String(p)
    .split(/[\\/]+/)
    .filter(Boolean);
  if (parts.length <= segments) return parts.join('/');
  return parts.slice(parts.length - segments).join('/');
}

function basename(p) {
  if (!p) return p;
  return path.posix.basename(String(p).replace(/\\/g, '/'));
}

function commonPathPrefix(paths) {
  const clean = paths
    .filter(Boolean)
    .map((p) => String(p).replace(/\\/g, '/'))
    .filter((p) => p.length > 0);
  if (clean.length === 0) return '';

  let prefix = clean[0];
  for (let i = 1; i < clean.length; i++) {
    const s = clean[i];
    let j = 0;
    const len = Math.min(prefix.length, s.length);
    while (j < len && prefix[j] === s[j]) j++;
    prefix = prefix.slice(0, j);
    if (!prefix) break;
  }

  // Trim to directory boundary.
  const lastSlash = prefix.lastIndexOf('/');
  if (lastSlash >= 0) prefix = prefix.slice(0, lastSlash + 1);
  return prefix;
}

function stripPrefix(p, prefix) {
  if (!p) return p;
  let s = String(p).replace(/\\/g, '/');
  if (prefix && s.startsWith(prefix)) return s.slice(prefix.length);
  return s;
}

function setDiff(aArr, bArr) {
  const a = new Set(aArr);
  const b = new Set(bArr);
  const aOnly = [];
  const bOnly = [];
  for (let x of a) if (!b.has(x)) aOnly.push(x);
  for (let x of b) if (!a.has(x)) bOnly.push(x);
  aOnly.sort();
  bOnly.sort();
  return {aOnly, bOnly};
}

function assetsKey(assets) {
  // Assets are already sorted in the snapshot, but defensively sort.
  return JSON.stringify([...(assets || [])].slice().sort());
}

function countByType(bundles) {
  const out = new Map();
  for (let b of bundles) {
    const t = b.type || 'unknown';
    out.set(t, (out.get(t) || 0) + 1);
  }
  return out;
}

function formatCountByType(map) {
  const entries = [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  if (entries.length === 0) return '(none)';
  return entries.map(([t, c]) => `${t}:${c}`).join(', ');
}

function main() {
  const dir = process.argv[2];
  if (!dir) usageAndExit('Missing snapshot directory argument.');
  if (process.argv.length > 3) usageAndExit('Expected exactly one argument.');

  const jsPath = path.join(dir, 'bundle-graph-js.json');
  const rustPath = path.join(dir, 'bundle-graph-rust.json');

  const js = readJson(jsPath);
  const rust = readJson(rustPath);
  assertSnapshot(js, 'js', jsPath);
  assertSnapshot(rust, 'rust', rustPath);

  // Build common prefix across all file-like paths (assets + mainEntryPath + entryAssetPath + bundlePaths).
  const allPaths = [];
  for (let snap of [js, rust]) {
    for (let b of snap.bundles || []) {
      if (b.mainEntryPath) allPaths.push(b.mainEntryPath);
      for (let p of b.entryAssetPaths || []) allPaths.push(p);
      for (let p of b.assets || []) allPaths.push(p);
    }
    for (let g of snap.bundleGroups || []) {
      if (g.entryAssetPath) allPaths.push(g.entryAssetPath);
      for (let p of g.bundlePaths || []) {
        // bundlePaths may contain pseudo strings like [bundle:hash]. Only include filesystem paths.
        if (
          typeof p === 'string' &&
          (p.startsWith('/') || /^[A-Za-z]:[\\/]/.test(p))
        ) {
          allPaths.push(p);
        }
      }
    }
  }
  const prefix = commonPathPrefix(allPaths);

  function fmtPath(p) {
    return stripPrefix(p, prefix);
  }

  console.log('=== Atlaspack Bundle Graph Snapshot Comparison ===');
  console.log(`JS   : ${jsPath}`);
  console.log(`Rust : ${rustPath}`);
  console.log(
    prefix
      ? `Common path prefix stripped: ${prefix}`
      : 'Common path prefix stripped: (none)',
  );
  console.log('');

  // 1) Summary Stats
  console.log('1) Summary Stats');
  const summaryRows = [
    [
      'totalBundles',
      js.stats.totalBundles,
      rust.stats.totalBundles,
      js.stats.totalBundles - rust.stats.totalBundles,
    ],
    [
      'totalBundleGroups',
      js.stats.totalBundleGroups,
      rust.stats.totalBundleGroups,
      js.stats.totalBundleGroups - rust.stats.totalBundleGroups,
    ],
    [
      'totalAssets',
      js.stats.totalAssets,
      rust.stats.totalAssets,
      js.stats.totalAssets - rust.stats.totalAssets,
    ],
  ];
  console.log(
    makeTable(['metric', 'js', 'rust', 'js-rust'], summaryRows, {
      align: ['left', 'right', 'right', 'right'],
    }),
  );
  console.log('');

  // Split entry vs shared bundles
  const jsEntry = js.bundles.filter((b) => b.mainEntryPath != null);
  const rustEntry = rust.bundles.filter((b) => b.mainEntryPath != null);
  const jsShared = js.bundles.filter((b) => b.mainEntryPath == null);
  const rustShared = rust.bundles.filter((b) => b.mainEntryPath == null);

  // 2) Entry Bundle Matching
  console.log('2) Entry Bundle Matching');

  const jsEntryMap = new Map();
  for (let b of jsEntry) {
    const key = `${b.mainEntryPath}::${b.type}`;
    // If collisions exist (shouldn't), keep arrays.
    if (!jsEntryMap.has(key)) jsEntryMap.set(key, []);
    jsEntryMap.get(key).push(b);
  }
  const rustEntryMap = new Map();
  for (let b of rustEntry) {
    const key = `${b.mainEntryPath}::${b.type}`;
    if (!rustEntryMap.has(key)) rustEntryMap.set(key, []);
    rustEntryMap.get(key).push(b);
  }

  const allEntryKeys = new Set([...jsEntryMap.keys(), ...rustEntryMap.keys()]);
  let matchedEntry = 0;
  let matchedIdentical = 0;
  let matchedDifferent = 0;
  const diffs = [];
  const jsOnlyEntryByType = new Map();
  const rustOnlyEntryByType = new Map();

  for (let key of [...allEntryKeys].sort()) {
    const jsList = jsEntryMap.get(key) || [];
    const rustList = rustEntryMap.get(key) || [];

    if (jsList.length === 0) {
      const t = key.split('::').pop();
      rustOnlyEntryByType.set(
        t,
        (rustOnlyEntryByType.get(t) || 0) + rustList.length,
      );
      continue;
    }
    if (rustList.length === 0) {
      const t = key.split('::').pop();
      jsOnlyEntryByType.set(t, (jsOnlyEntryByType.get(t) || 0) + jsList.length);
      continue;
    }

    // Pair up by index when multiple exist (rare). This keeps totals consistent.
    const pairCount = Math.min(jsList.length, rustList.length);
    const t = key.split('::').pop();
    for (let i = 0; i < pairCount; i++) {
      matchedEntry++;
      const jb = jsList[i];
      const rb = rustList[i];
      const ja = jb.assets || [];
      const ra = rb.assets || [];
      const identical = assetsKey(ja) === assetsKey(ra);
      if (identical) {
        matchedIdentical++;
      } else {
        matchedDifferent++;
        const {aOnly, bOnly} = setDiff(ja, ra);
        diffs.push({
          key,
          type: t,
          entry: jb.mainEntryPath,
          jsCount: ja.length,
          rustCount: ra.length,
          jsOnly: aOnly,
          rustOnly: bOnly,
        });
      }
    }

    // Remainders are unpaired extras
    if (jsList.length > pairCount) {
      jsOnlyEntryByType.set(
        t,
        (jsOnlyEntryByType.get(t) || 0) + (jsList.length - pairCount),
      );
    }
    if (rustList.length > pairCount) {
      rustOnlyEntryByType.set(
        t,
        (rustOnlyEntryByType.get(t) || 0) + (rustList.length - pairCount),
      );
    }
  }

  // Shared bundle matching by asset list (exact)
  const jsSharedMap = new Map();
  for (let b of jsShared) {
    const key = `${b.type}::${assetsKey(b.assets)}`;
    jsSharedMap.set(key, (jsSharedMap.get(key) || 0) + 1);
  }
  const rustSharedMap = new Map();
  for (let b of rustShared) {
    const key = `${b.type}::${assetsKey(b.assets)}`;
    rustSharedMap.set(key, (rustSharedMap.get(key) || 0) + 1);
  }
  let sharedMatched = 0;
  let sharedUnmatchedJs = 0;
  let sharedUnmatchedRust = 0;
  const allSharedKeys = new Set([
    ...jsSharedMap.keys(),
    ...rustSharedMap.keys(),
  ]);
  for (let k of allSharedKeys) {
    const jc = jsSharedMap.get(k) || 0;
    const rc = rustSharedMap.get(k) || 0;
    sharedMatched += Math.min(jc, rc);
    if (jc > rc) sharedUnmatchedJs += jc - rc;
    if (rc > jc) sharedUnmatchedRust += rc - jc;
  }

  console.log(
    makeTable(
      ['metric', 'count'],
      [
        ['Entry bundles matched (mainEntryPath+type)', matchedEntry],
        ['  matched with identical assets', matchedIdentical],
        ['  matched with different assets', matchedDifferent],
        [
          'Entry bundles JS-only (unmatched or extra)',
          [...jsOnlyEntryByType.values()].reduce((a, b) => a + b, 0),
        ],
        [
          'Entry bundles Rust-only (unmatched or extra)',
          [...rustOnlyEntryByType.values()].reduce((a, b) => a + b, 0),
        ],
        ['Shared bundles matched by exact assets list', sharedMatched],
        ['Shared bundles JS-only (unmatched or extra)', sharedUnmatchedJs],
        ['Shared bundles Rust-only (unmatched or extra)', sharedUnmatchedRust],
      ],
      {align: ['left', 'right']},
    ),
  );

  console.log('');
  console.log(
    `JS-only entry bundles by type   : ${formatCountByType(jsOnlyEntryByType)}`,
  );
  console.log(
    `Rust-only entry bundles by type : ${formatCountByType(rustOnlyEntryByType)}`,
  );
  console.log('');

  // 3) Asset Placement Differences
  console.log('3) Asset Placement Differences');
  if (diffs.length === 0) {
    console.log(
      'No asset placement differences found for matched entry bundles.',
    );
    console.log('');
  } else {
    const top = diffs
      .map((d) => ({
        ...d,
        diffCount: (d.jsOnly?.length || 0) + (d.rustOnly?.length || 0),
      }))
      .sort((a, b) => b.diffCount - a.diffCount)
      .slice(0, 20);

    const rows = top.map((d) => {
      const entryShort = shortenToLastSegments(fmtPath(d.entry), 2);
      const samples = [];
      for (let p of (d.jsOnly || []).slice(0, 2))
        samples.push(`js:${basename(fmtPath(p))}`);
      for (let p of (d.rustOnly || []).slice(0, 2)) {
        if (samples.length >= 3) break;
        samples.push(`rust:${basename(fmtPath(p))}`);
      }
      return [
        `${entryShort} (${d.type})`,
        d.jsCount,
        d.rustCount,
        d.jsOnly.length,
        d.rustOnly.length,
        samples.join(', '),
      ];
    });

    console.log(
      makeTable(
        [
          'entry (type)',
          'jsAssets',
          'rustAssets',
          'jsOnly',
          'rustOnly',
          'sample diffs',
        ],
        rows,
        {align: ['left', 'right', 'right', 'right', 'right', 'left']},
      ),
    );
    console.log('');
  }

  // 4) Bundle Group Matching
  console.log('4) Bundle Group Matching');
  const jsBgMap = new Map();
  for (let g of js.bundleGroups) jsBgMap.set(g.entryAssetPath, g);
  const rustBgMap = new Map();
  for (let g of rust.bundleGroups) rustBgMap.set(g.entryAssetPath, g);

  const bgKeys = new Set([...jsBgMap.keys(), ...rustBgMap.keys()]);
  let bgMatched = 0;
  let bgJsOnly = 0;
  let bgRustOnly = 0;
  for (let k of bgKeys) {
    const j = jsBgMap.has(k);
    const r = rustBgMap.has(k);
    if (j && r) bgMatched++;
    else if (j) bgJsOnly++;
    else bgRustOnly++;
  }
  console.log(
    makeTable(
      ['metric', 'count'],
      [
        ['matched (by entryAssetPath)', bgMatched],
        ['js-only', bgJsOnly],
        ['rust-only', bgRustOnly],
      ],
      {align: ['left', 'right']},
    ),
  );
  console.log('');

  // 5) Shared Bundle Analysis
  console.log('5) Shared Bundle Analysis');
  const jsSharedByType = countByType(jsShared);
  const rustSharedByType = countByType(rustShared);

  console.log(
    `Shared bundles (no mainEntryPath) JS by type   : ${formatCountByType(jsSharedByType)}`,
  );
  console.log(
    `Shared bundles (no mainEntryPath) Rust by type : ${formatCountByType(rustSharedByType)}`,
  );

  // Exact match already computed (sharedMatched). Report match ratio.
  const jsSharedTotal = jsShared.length;
  const rustSharedTotal = rustShared.length;
  console.log(
    makeTable(
      ['metric', 'js', 'rust'],
      [
        ['shared bundle count', jsSharedTotal, rustSharedTotal],
        [
          'shared bundles matched by exact asset list',
          sharedMatched,
          sharedMatched,
        ],
      ],
      {align: ['left', 'right', 'right']},
    ),
  );
  console.log('');

  // Overall parity score
  const parity =
    matchedEntry === 0 ? 100 : (matchedIdentical / matchedEntry) * 100;
  console.log('Overall parity score');
  console.log(
    `${parity.toFixed(2)}% of matched entry bundles have identical assets (${matchedIdentical}/${matchedEntry}).`,
  );
}

main();
