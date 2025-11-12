#!/usr/bin/env node
/* eslint-disable no-console */

/**
 * Standalone script to compare Compiled CSS transformers
 * Usage: node scripts/compare-compiled-css.js [options]
 */

const path = require('path');
const {NodeFS} = require('@atlaspack/fs');
const {$} = require('zx');

// Configuration
const DEFAULT_CONFIG = {
  importSources: ['@compiled/react'],
  extractToFile: false,
  transformCssMap: true,
  optimizeCss: true,
  cache: true,
  sourceMaps: true,
};

const DEFAULT_OPTIONS = {
  fixturesPath: path.join(
    __dirname,
    '../crates/atlassian-swc-compiled-css/tests/fixtures',
  ),
  configPath: path.join(__dirname, '../.compiledcssrc'),
  outputDir: path.join(__dirname, '../comparison-results'),
  fixtureGlob: '**/in.jsx',
};

async function findFixtureFiles(fs, fixturesPath) {
  const files = [];

  async function walkDir(dir) {
    const entries = await fs.readdir(dir);

    for (const entry of entries) {
      const fullPath = path.join(dir, entry);
      const stat = await fs.stat(fullPath);

      if (stat.isDirectory()) {
        await walkDir(fullPath);
      } else if (['in.jsx', 'in.js', 'in.ts', 'in.tsx'].includes(entry)) {
        files.push(fullPath);
      }
    }
  }

  await walkDir(fixturesPath);
  return files;
}

async function createAtlaspackWithTransformer(transformerPath, config) {
  // Create a temporary Atlaspack config that uses the specified transformer
  const tempConfigPath = path.join(
    __dirname,
    `temp-atlaspack-config-${Date.now()}-${Math.random().toString(36).substr(2, 9)}.json`,
  );

  const nodeFs = require('fs');

  // Create JSON config using proper extends format
  const jsPattern = '*.{js,mjs,jsm,jsx,es6,cjs,ts,tsx}';

  console.log(`    Creating config for transformer: ${transformerPath}`);

  // Create JSON config object using the extends pattern like the E2E tests
  const configObject = {
    extends: '@atlaspack/config-default',
    transformers: {
      [jsPattern]: [transformerPath, '...'],
    },
  };

  // Write as JSON
  const configContent = JSON.stringify(configObject, null, 2);
  nodeFs.writeFileSync(tempConfigPath, configContent);

  console.log(
    `    Generated JSON config for ${transformerPath} at ${tempConfigPath}`,
  );

  const Atlaspack = require('@atlaspack/core').default;

  // Set appropriate feature flags based on transformer
  const featureFlags = {
    compiledCssInJsTransformer: true, // Always enable for both transformers
  };

  const atlaspack = new Atlaspack({
    entries: ['.'],
    config: tempConfigPath,
    shouldPatchConsole: false,
    shouldBuildLazily: false,
    mode: 'development',
    env: {
      NODE_ENV: 'development',
    },
    featureFlags,
  });

  return {
    atlaspack,
    cleanup: () => {
      try {
        console.log(`    🗑️  Cleaning up config: ${tempConfigPath}`);
        if (nodeFs.existsSync(tempConfigPath)) {
          nodeFs.unlinkSync(tempConfigPath);
        }
      } catch (err) {
        // Ignore cleanup errors
      }
    },
  };
}

async function transformWithAtlaspack(
  atlaspack,
  filePath,
  config,
  transformerName,
) {
  try {
    console.log(`    🔧 Transforming with ${transformerName}...`);
    console.log(`    📄 File: ${filePath}`);

    const assets = await atlaspack.unstable_transform({
      filePath,
      env: {
        NODE_ENV: 'development',
        context: 'browser',
      },
      config,
    });

    console.log(
      `    📦 Received ${assets.length} assets from ${transformerName}`,
    );

    const mainAsset = assets[0];
    if (!mainAsset) {
      throw new Error('No assets returned from transformation');
    }

    const code = await mainAsset.getCode();
    console.log(`    📝 Code length: ${code.length} characters`);
    console.log(
      `    🎯 Contains '@compiled/react':`,
      code.includes('@compiled/react'),
    );
    console.log(
      `    🎯 Contains 'css(':`,
      code.includes('css(') || code.includes('.css('),
    );
    console.log(
      `    🎯 Contains runtime imports:`,
      code.includes('@compiled/react/runtime'),
    );
    console.log(`    🎯 Contains CSS classes:`, /\._[a-z0-9]+\{/.test(code));
    console.log(`    🎯 Meta keys:`, Object.keys(mainAsset.meta || {}));

    const result = {
      code,
      map: await mainAsset.getMapBuffer()?.toString(),
      styleRules: mainAsset.meta.styleRules,
      assets: assets.map((asset) => ({
        type: asset.type,
        code: asset.getCode(),
        meta: asset.meta,
      })),
    };

    if (result.styleRules) {
      console.log(
        `    💎 Style rules found:`,
        Object.keys(result.styleRules).length,
      );
    }

    return result;
  } catch (err) {
    console.log(`    ❌ ${transformerName} transform error:`, err.message);
    throw new Error(`${transformerName} transform failed: ${err.message}`);
  }
}

function compareResults(baselineResult, experimentResult) {
  if (!baselineResult && !experimentResult) return true;
  if (!baselineResult || !experimentResult) return false;

  // Normalize and compare code output
  const normalizeCode = (code) => {
    return code
      .replace(/\s+/g, ' ') // Normalize whitespace
      .replace(/;+/g, ';') // Normalize semicolons
      .trim();
  };

  const baselineCode = normalizeCode(baselineResult.code);
  const experimentCode = normalizeCode(experimentResult.code);

  return baselineCode === experimentCode;
}

function getFixtureOutputDirName(fixturePath) {
  if (!fixturePath) {
    return 'root';
  }

  const normalizedPath = path.normalize(fixturePath);
  const ext = path.extname(normalizedPath);
  const withoutExt = ext
    ? normalizedPath.slice(0, -ext.length)
    : normalizedPath;
  const trimmed = withoutExt.replace(/^[./\\]+/, '');

  return trimmed || 'root';
}

async function writeResults(
  fs,
  outputDir,
  fixtureName,
  baselineResult,
  experimentResult,
  match,
  error,
) {
  const fixtureDirRelative = getFixtureOutputDirName(fixtureName);
  const fixtureDir = path.join(outputDir, fixtureDirRelative);
  await fs.mkdirp(fixtureDir);

  // Write baseline result (@compiled/parcel-transformer)
  if (baselineResult) {
    await fs.writeFile(
      path.join(fixtureDir, 'baseline-compiled-parcel.js'),
      baselineResult.code,
    );
    if (baselineResult.styleRules) {
      await fs.writeFile(
        path.join(fixtureDir, 'baseline-compiled-parcel.style-rules.json'),
        JSON.stringify(baselineResult.styleRules, null, 2),
      );
    }
  }

  // Write experiment result (@atlaspack/transformer-compiled-css-in-js)
  if (experimentResult) {
    await fs.writeFile(
      path.join(fixtureDir, 'experiment-atlaspack.js'),
      experimentResult.code,
    );
    if (experimentResult.styleRules) {
      await fs.writeFile(
        path.join(fixtureDir, 'experiment-atlaspack.style-rules.json'),
        JSON.stringify(experimentResult.styleRules, null, 2),
      );
    }
  }

  // Write comparison result
  const comparisonInfo = {
    fixture: fixtureName,
    outputDir: fixtureDirRelative,
    match: match,
    error: error,
    hasBaseline: !!baselineResult,
    hasExperiment: !!experimentResult,
    timestamp: new Date().toISOString(),
  };

  await fs.writeFile(
    path.join(fixtureDir, 'comparison.json'),
    JSON.stringify(comparisonInfo, null, 2),
  );

  // Write diff if results don't match
  if (baselineResult && experimentResult && !match) {
    const diff = createSimpleDiff(baselineResult.code, experimentResult.code);
    await fs.writeFile(path.join(fixtureDir, 'diff.txt'), diff);
  }
}

function createSimpleDiff(baseline, experiment) {
  const baselineLines = baseline.split('\n');
  const experimentLines = experiment.split('\n');

  let diff =
    '--- Baseline (@compiled/parcel-transformer)\n+++ Experiment (@atlaspack/transformer-compiled-css-in-js)\n\n';

  const maxLines = Math.max(baselineLines.length, experimentLines.length);

  for (let i = 0; i < maxLines; i++) {
    const baselineLine = baselineLines[i] || '';
    const experimentLine = experimentLines[i] || '';

    if (baselineLine !== experimentLine) {
      if (baselineLine) {
        diff += `- ${baselineLine}\n`;
      }
      if (experimentLine) {
        diff += `+ ${experimentLine}\n`;
      }
    } else if (baselineLine) {
      diff += `  ${baselineLine}\n`;
    }
  }

  return diff;
}

async function main() {
  console.log('🚀 Starting Compiled CSS Transformer Comparison');

  // Enable the feature flag for Compiled CSS transformations
  process.env.ATLASPACK_BUILD_ENV = 'test';
  console.log(
    '📋 Set ATLASPACK_BUILD_ENV=test to enable compiledCssInJsTransformer feature flag',
  );

  // Parse command line arguments
  const args = process.argv.slice(2);
  const options = {...DEFAULT_OPTIONS};

  for (let i = 0; i < args.length; i += 2) {
    const flag = args[i];
    const value = args[i + 1];

    switch (flag) {
      case '--fixtures-path':
        options.fixturesPath = path.resolve(value);
        break;
      case '--config-path':
        options.configPath = path.resolve(value);
        break;
      case '--output-dir':
        options.outputDir = path.resolve(value);
        break;
      case '--fixture-glob':
        options.fixtureGlob = value;
        break;
      case '--help':
        console.log(`
Usage: node scripts/compare-compiled-css.js [options]

Options:
  --fixtures-path <path>    Path to fixtures directory
  --config-path <path>      Path to .compiledcssrc config file  
  --output-dir <path>       Directory to write comparison results
  --fixture-glob <pattern>  Glob pattern for fixture files
  --help                    Show this help message
`);
        process.exit(0);
    }
  }

  const fs = new NodeFS();

  // Load configuration
  let config = DEFAULT_CONFIG;
  try {
    const configContent = await fs.readFile(options.configPath, 'utf8');
    config = {...DEFAULT_CONFIG, ...JSON.parse(configContent)};
    console.log(`✓ Loaded config from ${options.configPath}`);
  } catch (err) {
    console.log(
      `⚠️  Could not load config from ${options.configPath}, using default config`,
    );
  }

  console.log(`🔧 Rebuilding atlaspack core...`);
  await $`yarn build-native && yarn build`.nothrow();

  // Find fixture files
  console.log(`🔍 Searching for fixtures in ${options.fixturesPath}`);
  let fixtureFiles;
  try {
    fixtureFiles = await findFixtureFiles(fs, options.fixturesPath);
  } catch (err) {
    console.error(`❌ Error finding fixtures: ${err.message}`);
    process.exit(1);
  }

  console.log(`📁 Found ${fixtureFiles.length} fixture files`);

  if (fixtureFiles.length === 0) {
    console.log(`⚠️  No fixture files found`);
    process.exit(0);
  }

  // Create output directory
  try {
    await fs.mkdirp(options.outputDir);
    console.log(`📂 Output directory: ${options.outputDir}`);
  } catch (err) {
    console.error(`❌ Could not create output directory: ${err.message}`);
    process.exit(1);
  }

  // Initialize Atlaspack instances with different transformers
  console.log('🔧 Initializing transformers...');

  // Baseline: @compiled/parcel-transformer
  console.log(
    '  📦 Creating baseline transformer (@compiled/parcel-transformer)...',
  );
  const {atlaspack: baselineAtlaspack, cleanup: cleanupBaseline} =
    await createAtlaspackWithTransformer(
      '@compiled/parcel-transformer',
      config,
    );

  // Experiment: @atlaspack/transformer-compiled-css-in-js
  console.log(
    '  🧪 Creating experiment transformer (@atlaspack/transformer-compiled-css-in-js)...',
  );
  const {atlaspack: experimentAtlaspack, cleanup: cleanupExperiment} =
    await createAtlaspackWithTransformer(
      '@atlaspack/transformer-compiled-css-in-js',
      config,
    );

  const results = [];
  let successCount = 0;
  let errorCount = 0;

  try {
    // Process each fixture
    for (let i = 0; i < fixtureFiles.length; i++) {
      const fixtureFile = fixtureFiles[i];
      const fixtureRelativePath = path.relative(
        options.fixturesPath,
        fixtureFile,
      );
      const fixtureOutputDir = getFixtureOutputDirName(fixtureRelativePath);

      console.log(
        `\n[${i + 1}/${fixtureFiles.length}] Processing: ${fixtureRelativePath}`,
      );
      console.log(`  📂 Output directory: ${fixtureOutputDir}`);

      try {
        // Transform with baseline (@compiled/parcel-transformer)
        console.log('  📦 Running baseline transformer...');
        const baselineResult = await transformWithAtlaspack(
          baselineAtlaspack,
          fixtureFile,
          config,
          'Baseline',
        );

        // Transform with experiment (@atlaspack/transformer-compiled-css-in-js)
        console.log('  🧪 Running experiment transformer...');
        const experimentResult = await transformWithAtlaspack(
          experimentAtlaspack,
          fixtureFile,
          config,
          'Experiment',
        );

        // Compare results
        const match = compareResults(baselineResult, experimentResult);

        const result = {
          fixture: fixtureRelativePath,
          outputDir: fixtureOutputDir,
          baseline: baselineResult,
          experiment: experimentResult,
          match,
        };

        results.push(result);

        // Write individual result files
        await writeResults(
          fs,
          options.outputDir,
          fixtureRelativePath,
          baselineResult,
          experimentResult,
          match,
        );

        if (match) {
          successCount++;
          console.log('  ✅ Results match');
        } else {
          errorCount++;
          console.log('  ❌ Results differ');
        }
      } catch (err) {
        errorCount++;
        const result = {
          fixture: fixtureRelativePath,
          outputDir: fixtureOutputDir,
          baseline: null,
          experiment: null,
          match: false,
          error: err.message,
        };
        results.push(result);

        console.log(`  ❌ Error: ${err.message}`);

        // Write error result
        await writeResults(
          fs,
          options.outputDir,
          fixtureRelativePath,
          null,
          null,
          false,
          err.message,
        );
      }
    }
  } finally {
    // Clean up temporary config files
    cleanupBaseline();
    cleanupExperiment();
  }

  // Write summary report
  const summary = {
    timestamp: new Date().toISOString(),
    total: results.length,
    matches: successCount,
    mismatches: errorCount,
    results: results.map((r) => ({
      fixture: r.fixture,
      outputDir: r.outputDir,
      match: r.match,
      error: r.error,
      hasBaseline: !!r.baseline,
      hasExperiment: !!r.experiment,
    })),
  };

  await fs.writeFile(
    path.join(options.outputDir, 'summary.json'),
    JSON.stringify(summary, null, 2),
  );

  // Create markdown report
  let markdown = `# Compiled CSS Transformer Comparison Report\n\n`;
  markdown += `**Generated:** ${summary.timestamp}\n\n`;
  markdown += `**Baseline:** @compiled/parcel-transformer\n`;
  markdown += `**Experiment:** @atlaspack/transformer-compiled-css-in-js\n\n`;
  markdown += `## Summary\n\n`;
  markdown += `- **Total Fixtures:** ${summary.total}\n`;
  markdown += `- **Matches:** ${summary.matches}\n`;
  markdown += `- **Mismatches:** ${summary.mismatches}\n\n`;

  if (summary.mismatches > 0) {
    markdown += `## Issues\n\n`;

    for (const result of results) {
      if (!result.match || result.error) {
        markdown += `### ${result.fixture}\n\n`;
        if (result.error) {
          markdown += `**Error:** ${result.error}\n\n`;
        } else {
          markdown += `**Status:** Outputs differ\n`;
          markdown += `**Baseline:** ${result.baseline ? 'Success' : 'Failed'}\n`;
          markdown += `**Experiment:** ${result.experiment ? 'Success' : 'Failed'}\n\n`;
        }
      }
    }
  }

  await fs.writeFile(path.join(options.outputDir, 'report.md'), markdown);

  // Log final summary
  console.log(`\n${'='.repeat(50)}`);
  console.log('📊 COMPARISON SUMMARY');
  console.log(`${'='.repeat(50)}`);
  console.log(`Total fixtures: ${results.length}`);
  console.log(`✅ Matches: ${successCount}`);
  console.log(`❌ Mismatches: ${errorCount}`);
  console.log(`📁 Results written to: ${options.outputDir}`);
  console.log(`${'='.repeat(50)}`);

  process.exit(errorCount > 0 ? 1 : 0);
}

if (require.main === module) {
  main().catch((err) => {
    console.error('❌ Fatal error:', err.message);
    console.error(err.stack);
    process.exit(1);
  });
}

module.exports = {
  findFixtureFiles,
  createAtlaspackWithTransformer,
  transformWithAtlaspack,
  compareResults,
  writeResults,
  main,
};
