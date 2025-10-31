#!/usr/bin/env node

const fs = require('fs/promises');
const path = require('path');

const STYLE_RULES_ROOT = path.resolve(__dirname, 'tmp/style-rules');
const TOOLING_DIRECTORIES = {
  babel: 'babel',
  swc: 'swc',
};
const INCLUDED_PATH_PREFIXES = ['src/packages/'];

function shouldInclude(relativePath) {
  if (!INCLUDED_PATH_PREFIXES.length) {
    return true;
  }

  return INCLUDED_PATH_PREFIXES.some((prefix) => relativePath.startsWith(prefix));
}

async function assertDirectoryExists(name, absolutePath) {
  try {
    const stats = await fs.stat(absolutePath);
    if (!stats.isDirectory()) {
      throw new Error(`Expected '${absolutePath}' to be a directory for '${name}'`);
    }
  } catch (error) {
    if (error && error.code === 'ENOENT') {
      throw new Error(`Missing '${name}' directory at '${absolutePath}'`);
    }
    throw error;
  }
}

async function walkDirectory(rootDirectory) {
  const result = [];
  const queue = [rootDirectory];

  while (queue.length) {
    const current = queue.pop();
    const entries = await fs.readdir(current, { withFileTypes: true });

    for (const entry of entries) {
      const absolutePath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        queue.push(absolutePath);
        continue;
      }

      if (entry.isFile() && entry.name.endsWith('.json')) {
        result.push(absolutePath);
      }
    }
  }

  return result;
}

function canonicalizeHash(rule) {
  return rule
    .replace(/\._[a-z0-9_]+/gi, '._HASH')
    .replace(/var\(([^)]+)\)/gi, (match) => match.replace(/\s+/g, ''))
    .replace(/calc\(([^)]+)\)/gi, (match) => match.replace(/\s+/g, ''))
    .replace(/:\s*/g, ':')
    .replace(/,\s*/g, ',')
    .replace(/#([0-9a-f]{6})/gi, (match, hex) => `#${hex.toLowerCase()}`)
    .replace(/0(px|em|rem|%|vw|vh|vmin|vmax)/gi, '0');
}

function prepareStyleRules(rawRules, label, relativePath) {
  if (!Array.isArray(rawRules)) {
    throw new Error(`Found non-array styleRules for '${relativePath}' in '${label}'`);
  }

  const uniqueRaw = Array.from(new Set(rawRules));
  const entries = uniqueRaw.map((raw) => ({
    raw,
    canonical: canonicalizeHash(raw),
  }));

  entries.sort((a, b) => {
    if (a.canonical === b.canonical) {
      return a.raw.localeCompare(b.raw);
    }
    return a.canonical.localeCompare(b.canonical);
  });

  return {
    raw: entries.map((entry) => entry.raw),
    canonical: entries.map((entry) => entry.canonical),
  };
}

async function loadStyleRuleMap(directoryPath, label) {
  const files = await walkDirectory(directoryPath);
  const entries = new Map();

  for (const filePath of files) {
    const relativePath = path.relative(directoryPath, filePath);
    const normalizedPath = relativePath.split(path.sep).join('/');

    if (!shouldInclude(normalizedPath)) {
      continue;
    }

    let parsed;
    try {
      const contents = await fs.readFile(filePath, 'utf8');
      parsed = JSON.parse(contents);
    } catch (error) {
      throw new Error(`Unable to read style rules for '${relativePath}' in '${label}': ${error.message}`);
    }

    const prepared = prepareStyleRules(parsed.styleRules, label, relativePath);
    entries.set(normalizedPath, {
      canonical: prepared.canonical,
      isEmpty: prepared.canonical.length === 0,
    });
  }

  return entries;
}

function summarizeProgress(babelMap, swcMap) {
  const keys = new Set([...babelMap.keys(), ...swcMap.keys()]);
  let matching = 0;
  let emptyMatches = 0;
  let mismatching = 0;
  let missingInSwc = 0;
  let missingInBabel = 0;

  for (const key of keys) {
    const babelEntry = babelMap.get(key);
    const swcEntry = swcMap.get(key);

    if (!babelEntry) {
      missingInBabel += 1;
      continue;
    }

    if (!swcEntry) {
      missingInSwc += 1;
      continue;
    }

    const { canonical: babelRules } = babelEntry;
    const { canonical: swcRules } = swcEntry;

    if (babelRules.length === 0 && swcRules.length === 0) {
      matching += 1;
      emptyMatches += 1;
      continue;
    }

    if (babelRules.length !== swcRules.length) {
      mismatching += 1;
      continue;
    }

    let mismatchFound = false;
    for (let index = 0; index < babelRules.length; index += 1) {
      if (babelRules[index] !== swcRules[index]) {
        mismatchFound = true;
        break;
      }
    }

    if (mismatchFound) {
      mismatching += 1;
    } else {
      matching += 1;
    }
  }

  const totalBabelFiles = babelMap.size;
  const totalSwcFiles = swcMap.size;
  const comparedFiles = keys.size - missingInBabel - missingInSwc;
  const matchRate = totalBabelFiles
    ? ((matching / totalBabelFiles) * 100).toFixed(2)
    : '0.00';

  return {
    totalBabelFiles,
    totalSwcFiles,
    comparedFiles,
    matching,
    emptyMatches,
    mismatching,
    missingInSwc,
    missingInBabel,
    matchRate,
  };
}

async function main() {
  const babelDir = path.join(STYLE_RULES_ROOT, TOOLING_DIRECTORIES.babel);
  const swcDir = path.join(STYLE_RULES_ROOT, TOOLING_DIRECTORIES.swc);

  await Promise.all([
    assertDirectoryExists('babel', babelDir),
    assertDirectoryExists('swc', swcDir),
  ]);

  const [babelMap, swcMap] = await Promise.all([
    loadStyleRuleMap(babelDir, 'babel'),
    loadStyleRuleMap(swcDir, 'swc'),
  ]);

  const summary = summarizeProgress(babelMap, swcMap);

  console.log(`Babel files analyzed: ${summary.totalBabelFiles}`);
  console.log(`SWC files analyzed: ${summary.totalSwcFiles}`);
  console.log(`Files compared: ${summary.comparedFiles}`);
  console.log(`Matching files: ${summary.matching}`);
  console.log(`Matching empty files: ${summary.emptyMatches}`);
  console.log(`Mismatching files: ${summary.mismatching}`);
  console.log(`Missing in SWC: ${summary.missingInSwc}`);
  console.log(`Missing in Babel: ${summary.missingInBabel}`);
  console.log(`Match rate vs Babel: ${summary.matchRate}%`);
}

main().catch((error) => {
  console.error(error.message || error);
  process.exitCode = 1;
});
