#!/usr/bin/env node
/* eslint-disable no-console */

/**
 * Usage:
 *
 * cd <afm>/jira
 * node ~/atlassian/atlaspack/scripts/compiled-css-diff-report.mjs [--filter=<substring>] [--limit=<number>] [--map=<path>] [--markdown]
 *
 * Reads `compiled-css-migration-map.json` and prints the Babel vs SWC differences
 * for every unsafe asset in a human-readable format (text or markdown table).
 */

import {readFile} from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';

const defaultMapPath = path.resolve('./compiled-css-migration-map.json');

function parseArgs(argv) {
  const options = {
    filter: undefined,
    limit: undefined,
    mapPath: defaultMapPath,
    format: 'text',
  };

  for (const arg of argv) {
    if (arg.startsWith('--filter=')) {
      options.filter = arg.slice('--filter='.length);
    } else if (arg.startsWith('--limit=')) {
      const value = Number.parseInt(arg.slice('--limit='.length), 10);
      if (!Number.isNaN(value) && value > 0) {
        options.limit = value;
      }
    } else if (arg.startsWith('--map=')) {
      options.mapPath = path.resolve(arg.slice('--map='.length));
    } else if (arg === '--markdown') {
      options.format = 'markdown';
    } else if (arg.startsWith('--format=')) {
      const format = arg.slice('--format='.length);
      if (format === 'markdown' || format === 'text') {
        options.format = format;
      }
    }
  }

  return options;
}

function extractProperty(rule) {
  const openBrace = rule.indexOf('{');
  const colon = rule.indexOf(':', openBrace + 1);
  if (openBrace === -1 || colon === -1) {
    return 'unknown';
  }

  const property = rule.slice(openBrace + 1, colon).trim();
  return property || 'unknown';
}

function categorizeRules(rules = []) {
  const counts = new Map();
  for (const rule of rules) {
    const property = extractProperty(rule);
    counts.set(property, (counts.get(property) ?? 0) + 1);
  }

  return Array.from(counts.entries())
    .map(([property, count]) => ({property, count}))
    .sort((a, b) => b.count - a.count || a.property.localeCompare(b.property));
}

function formatCategorySummary(categories) {
  if (!categories.length) {
    return '-';
  }

  const top = categories.slice(0, 5);
  return top.map(({property, count}) => `${property}:${count}`).join(', ');
}

function computeDifferences(babelRules = [], swcRules = []) {
  const swcSet = new Set(swcRules);
  const babelSet = new Set(babelRules);

  const babelOnly = babelRules.filter((rule) => !swcSet.has(rule));
  const swcOnly = swcRules.filter((rule) => !babelSet.has(rule));

  return {
    babelOnly: babelRules.filter((rule) => !swcSet.has(rule)),
    swcOnly: swcRules.filter((rule) => !babelSet.has(rule)),
    babelCategories: categorizeRules(babelOnly),
    swcCategories: categorizeRules(swcOnly),
  };
}

function logDiff({assetId, assetPath, babelOnly, swcOnly, diagnostics}) {
  console.log(`\n${assetPath} (id ${assetId})`);
  console.log(`  Babel-only (${babelOnly.length})`);
  for (const rule of babelOnly) {
    console.log(`    - ${rule}`);
  }

  console.log(`  SWC-only (${swcOnly.length})`);
  for (const rule of swcOnly) {
    console.log(`    - ${rule}`);
  }

  if (diagnostics.length) {
    console.log(`  Diagnostics (${diagnostics.length})`);
    for (const diagnostic of diagnostics) {
      console.log(`    - ${diagnostic}`);
    }
  }
}

function renderMarkdownReport({
  mapPath,
  safeCount,
  totalCount,
  entries,
  totalBabelOnly,
  totalSwcOnly,
}) {
  const lines = [];
  lines.push('# Compiled CSS migration map diff report');
  lines.push('');
  lines.push(`- Map: \`${mapPath}\``);
  lines.push(`- Safe assets: ${safeCount}/${totalCount}`);
  lines.push(`- Unsafe assets reported: ${entries.length}`);
  lines.push(`- Babel-only rules: ${totalBabelOnly}`);
  lines.push(`- SWC-only rules: ${totalSwcOnly}`);
  lines.push('');
  lines.push(
    '| Asset | Babel-only | SWC-only | Diagnostics | Babel categories | SWC categories |',
  );
  lines.push('| --- | --- | --- | --- | --- | --- |');

  for (const entry of entries) {
    const babelCategoriesSummary = formatCategorySummary(entry.babelCategories);
    const swcCategoriesSummary = formatCategorySummary(entry.swcCategories);
    const assetLabel = `${entry.assetPath} (id ${entry.assetId})`;

    lines.push(
      [
        assetLabel,
        entry.babelOnly.length,
        entry.swcOnly.length,
        entry.diagnostics.length,
        babelCategoriesSummary,
        swcCategoriesSummary,
      ]
        .map((cell) => String(cell).replace(/\n/g, '<br>'))
        .map((cell) => ` ${cell} `)
        .join('|'),
    );
  }

  return lines.join('\n');
}

async function main() {
  const {filter, limit, mapPath, format} = parseArgs(process.argv.slice(2));
  const raw = await readFile(mapPath, 'utf8');
  const map = JSON.parse(raw);

  const safeCount = map.stats?.safe ?? Object.keys(map.safeAssets ?? {}).length;
  const totalCount =
    map.stats?.total ?? safeCount + Object.keys(map.unsafeAssets ?? {}).length;

  const unsafeEntries = Object.entries(map.unsafeAssets ?? {}).map(
    ([assetId, value]) => ({
      assetId,
      assetPath: value.asset,
      babel: value.babel ?? [],
      swc: value.swc ?? [],
      diagnostics: value.diagnostics ?? [],
    }),
  );

  const filteredEntries = unsafeEntries
    .filter(
      (entry) =>
        !filter ||
        entry.assetPath.includes(filter) ||
        entry.assetId.includes(filter),
    )
    .sort((a, b) => a.assetPath.localeCompare(b.assetPath, 'en'));

  const entriesToPrint =
    typeof limit === 'number'
      ? filteredEntries.slice(0, limit)
      : filteredEntries;

  if (!entriesToPrint.length) {
    console.log(
      [
        `No unsafe assets matched the provided filter${filter ? ` "${filter}"` : ''}.`,
        `Map path: ${mapPath}`,
        `Safe assets: ${safeCount}/${totalCount}`,
      ].join('\n'),
    );
    return;
  }

  console.log('Compiled CSS migration map diff report');
  console.log(`Map: ${mapPath}`);
  console.log(`Safe assets: ${safeCount}/${totalCount}`);
  console.log(
    `Reporting ${entriesToPrint.length}/${filteredEntries.length} unsafe assets${
      filter ? ` matching "${filter}"` : ''
    }${limit ? ` (limit ${limit})` : ''}`,
  );

  let totalBabelOnly = 0;
  let totalSwcOnly = 0;

  const enrichedEntries = entriesToPrint.map((entry) => {
    const {babelOnly, swcOnly, babelCategories, swcCategories} =
      computeDifferences(entry.babel, entry.swc);
    totalBabelOnly += babelOnly.length;
    totalSwcOnly += swcOnly.length;

    return {
      ...entry,
      babelOnly,
      swcOnly,
      babelCategories,
      swcCategories,
    };
  });

  if (format === 'markdown') {
    const markdown = renderMarkdownReport({
      mapPath,
      safeCount,
      totalCount,
      entries: enrichedEntries,
      totalBabelOnly,
      totalSwcOnly,
    });
    console.log(markdown);
    return;
  }

  for (const entry of enrichedEntries) {
    logDiff({
      assetId: entry.assetId,
      assetPath: entry.assetPath,
      babelOnly: entry.babelOnly,
      swcOnly: entry.swcOnly,
      diagnostics: entry.diagnostics,
    });

    if (entry.babelCategories.length || entry.swcCategories.length) {
      console.log('  Categories');
      if (entry.babelCategories.length) {
        console.log(
          `    Babel: ${entry.babelCategories
            .map(({property, count}) => `${property}:${count}`)
            .join(', ')}`,
        );
      }
      if (entry.swcCategories.length) {
        console.log(
          `    SWC: ${entry.swcCategories
            .map(({property, count}) => `${property}:${count}`)
            .join(', ')}`,
        );
      }
    }
  }

  console.log('\nSummary');
  console.log(`  Unsafe assets reported: ${enrichedEntries.length}`);
  console.log(`  Babel-only rules: ${totalBabelOnly}`);
  console.log(`  SWC-only rules: ${totalSwcOnly}`);
}

main().catch((error) => {
  console.error('Failed to generate diff report');
  console.error(error);
  process.exitCode = 1;
});
