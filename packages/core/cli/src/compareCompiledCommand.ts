import {NodeFS} from '@atlaspack/fs';
import logger from '@atlaspack/logger';
import commander from 'commander';
import path from 'path';
import {normalizeOptions, Options} from './normalizeOptions';
import type {CommandExt} from './normalizeOptions';
import {applyOptions} from './applyOptions';
import {commonOptions} from './options';
import {handleUncaughtException} from './handleUncaughtException';

interface ComparisonOptions extends Options {
  fixturesPath?: string;
  configPath?: string;
  outputDir?: string;
  fixtureGlob?: string;
}

interface TransformerResult {
  code: string;
  map?: string;
  styleRules?: any;
  assets?: any[];
}

interface ComparisonResult {
  fixture: string;
  atlaspack: TransformerResult | null;
  parcel: TransformerResult | null;
  match: boolean;
  error?: string;
}

export function makeCompareCompiledCommand(): commander.Command {
  const compareCompiled = new commander.Command('compare-compiled-css [input...]')
    .description('Compare Atlaspack and Parcel Compiled CSS transformers')
    .option(
      '--fixtures-path <path>',
      'Path to fixtures directory',
      path.join(process.cwd(), 'crates/atlassian-swc-compiled-css/tests/fixtures')
    )
    .option(
      '--config-path <path>',
      'Path to .compiledcssrc config file',
      './.compiledcssrc'
    )
    .option(
      '--output-dir <path>',
      'Directory to write comparison results',
      './comparison-results'
    )
    .option(
      '--fixture-glob <pattern>',
      'Glob pattern for fixture files',
      '**/in.jsx'
    )
    .action(async (args: string[], opts: ComparisonOptions & Options, command: CommandExt) => {
      try {
        await runCompareCompiled(args, opts, command);
      } catch (err: any) {
        handleUncaughtException(err);
      }
    });

  applyOptions(compareCompiled, commonOptions);
  return compareCompiled;
}

async function runCompareCompiled(args: string[], opts: ComparisonOptions & Options, command: CommandExt) {
  const fs = new NodeFS();
  Object.assign(command, opts);
  const options = await normalizeOptions(command, fs);
  
  // Load configuration
  const configPath = path.resolve(opts.configPath || './.compiledcssrc');
  let config = {};
  
  try {
    const configContent = await fs.readFile(configPath);
    config = JSON.parse(configContent);
    logger.info({
      message: `Loaded config from ${configPath}`,
      origin: '@atlaspack/cli'
    });
  } catch (err) {
    logger.warn({
      message: `Could not load config from ${configPath}, using default config`,
      origin: '@atlaspack/cli'
    });
  }

  // Find fixture files
  const fixturesPath = path.resolve(opts.fixturesPath || 
    path.join(process.cwd(), 'crates/atlassian-swc-compiled-css/tests/fixtures'));
  const fixtureGlob = opts.fixtureGlob || '**/in.jsx';
  
  const fixtureFiles = await findFixtureFiles(fs, fixturesPath, fixtureGlob);

  logger.info({
    message: `Found ${fixtureFiles.length} fixture files in ${fixturesPath}`,
    origin: '@atlaspack/cli'
  });

  if (fixtureFiles.length === 0) {
    logger.warn({
      message: `No fixture files found matching pattern ${fixtureGlob} in ${fixturesPath}`,
      origin: '@atlaspack/cli'
    });
    return;
  }

  // Create output directory
  const outputDir = path.resolve(opts.outputDir || './comparison-results');
  try {
    await fs.mkdirp(outputDir);
  } catch (err) {
    // Directory might already exist
  }

  // Initialize Atlaspack
  const Atlaspack = require('@atlaspack/core').default;
  const atlaspack = new Atlaspack({
    entries: ['.'],
    defaultConfig: require.resolve('@atlaspack/config-default', {
      paths: [fs.cwd(), __dirname],
    }),
    shouldPatchConsole: false,
    shouldBuildLazily: false,
    ...options,
    mode: 'development',
    env: {
      NODE_ENV: 'development'
    }
  });

  // For now, we'll focus on comparing Atlaspack output with expected output from fixtures
  // TODO: Add @compiled/parcel-transformer comparison when dependencies are resolved

  const results: ComparisonResult[] = [];
  let successCount = 0;
  let errorCount = 0;

  // Process each fixture
  for (const fixtureFile of fixtureFiles) {
    const relativePath = path.relative(fixturesPath, fixtureFile);
    const fixtureName = path.dirname(relativePath);
    
    logger.info({
      message: `Processing fixture: ${fixtureName}`,
      origin: '@atlaspack/cli'
    });

    try {
      // Transform with Atlaspack
      const atlaspackResult = await transformWithAtlaspack(atlaspack, fixtureFile, config);
      
      // Load expected output from fixture directory
      const expectedResult = await loadExpectedResult(fs, fixtureFile);

      // Compare results
      const match = compareResults(atlaspackResult, expectedResult);
      
      const result: ComparisonResult = {
        fixture: fixtureName,
        atlaspack: atlaspackResult,
        parcel: expectedResult,
        match
      };

      results.push(result);
      
      if (match) {
        successCount++;
        logger.info({
          message: `✓ ${fixtureName}: Results match`,
          origin: '@atlaspack/cli'
        });
      } else {
        errorCount++;
        logger.warn({
          message: `✗ ${fixtureName}: Results differ`,
          origin: '@atlaspack/cli'
        });
      }

      // Write individual result files
      await writeResultFiles(fs, outputDir, fixtureName, result);

    } catch (err: any) {
      errorCount++;
      const result: ComparisonResult = {
        fixture: fixtureName,
        atlaspack: null,
        parcel: null,
        match: false,
        error: err.message
      };
      results.push(result);
      
      logger.error({
        message: `✗ ${fixtureName}: Error processing fixture: ${err.message}`,
        origin: '@atlaspack/cli'
      });
    }
  }

  // Write summary report
  await writeSummaryReport(fs, outputDir, results);

  // Log final summary
  logger.info({
    message: `\n=== Comparison Summary ===\n` +
             `Total fixtures: ${results.length}\n` +
             `Matches: ${successCount}\n` +
             `Mismatches: ${errorCount}\n` +
             `Results written to: ${outputDir}`,
    origin: '@atlaspack/cli'
  });

  process.exit(errorCount > 0 ? 1 : 0);
}

async function transformWithAtlaspack(atlaspack: any, filePath: string, config: any): Promise<TransformerResult> {
  try {
    const assets = await atlaspack.unstable_transform({
      filePath,
      env: {
        NODE_ENV: 'development',
        context: 'browser'
      },
      config
    });

    const mainAsset = assets[0];
    if (!mainAsset) {
      throw new Error('No assets returned from transformation');
    }

    return {
      code: await mainAsset.getCode(),
      map: await mainAsset.getMapBuffer()?.toString(),
      styleRules: mainAsset.meta.styleRules,
      assets: assets.map((asset: any) => ({
        type: asset.type,
        code: asset.getCode(),
        meta: asset.meta
      }))
    };
  } catch (err: any) {
    throw new Error(`Atlaspack transform failed: ${err.message}`);
  }
}

async function findFixtureFiles(fs: NodeFS, fixturesPath: string, pattern: string): Promise<string[]> {
  try {
    const files: string[] = [];
    
    async function walkDir(dir: string) {
      const entries = await fs.readdir(dir);
      
      for (const entry of entries) {
        const fullPath = path.join(dir, entry);
        const stat = await fs.stat(fullPath);
        
        if (stat.isDirectory()) {
          await walkDir(fullPath);
        } else if (entry === 'in.jsx' || entry === 'in.js' || entry === 'in.ts' || entry === 'in.tsx') {
          files.push(fullPath);
        }
      }
    }
    
    await walkDir(fixturesPath);
    return files;
  } catch (err: any) {
    throw new Error(`Failed to find fixture files: ${err.message}`);
  }
}

async function loadExpectedResult(fs: NodeFS, fixtureFile: string): Promise<TransformerResult | null> {
  const fixtureDir = path.dirname(fixtureFile);
  
  // Try to load expected output files (out.js, actual.js, etc.)
  const expectedFiles = ['out.js', 'actual.js', 'expected.js'];
  
  for (const expectedFile of expectedFiles) {
    const expectedPath = path.join(fixtureDir, expectedFile);
    
    try {
      const content = await fs.readFile(expectedPath, 'utf8');
      
      // Also try to load style rules if available
      let styleRules = null;
      try {
        const styleRulesPath = path.join(fixtureDir, 'style-rules.json');
        const styleRulesContent = await fs.readFile(styleRulesPath, 'utf8');
        styleRules = JSON.parse(styleRulesContent);
      } catch {
        // Style rules file might not exist
      }
      
      return {
        code: content,
        styleRules,
        assets: []
      };
    } catch {
      // Try next expected file
      continue;
    }
  }
  
  return null;
}

function compareResults(atlaspack: TransformerResult | null, parcel: TransformerResult | null): boolean {
  if (!atlaspack && !parcel) return true;
  if (!atlaspack || !parcel) return false;

  // Normalize and compare code output
  const normalizeCode = (code: string) => {
    return code
      .replace(/\s+/g, ' ')  // Normalize whitespace
      .replace(/;+/g, ';')   // Normalize semicolons
      .trim();
  };

  const atlaspackCode = normalizeCode(atlaspack.code);
  const parcelCode = normalizeCode(parcel.code);

  return atlaspackCode === parcelCode;
}

async function writeResultFiles(fs: NodeFS, outputDir: string, fixtureName: string, result: ComparisonResult) {
  const fixtureDir = path.join(outputDir, fixtureName);
  await fs.mkdirp(fixtureDir);

  // Write Atlaspack result
  if (result.atlaspack) {
    await fs.writeFile(
      path.join(fixtureDir, 'atlaspack.js'),
      result.atlaspack.code
    );
    if (result.atlaspack.styleRules) {
      await fs.writeFile(
        path.join(fixtureDir, 'atlaspack.style-rules.json'),
        JSON.stringify(result.atlaspack.styleRules, null, 2)
      );
    }
  }

  // Write Expected result (from fixtures)
  if (result.parcel) {
    await fs.writeFile(
      path.join(fixtureDir, 'expected.js'),
      result.parcel.code
    );
  }

  // Write comparison result
  const comparisonInfo = {
    fixture: result.fixture,
    match: result.match,
    error: result.error,
    hasAtlaspack: !!result.atlaspack,
    hasExpected: !!result.parcel,
    timestamp: new Date().toISOString()
  };

  await fs.writeFile(
    path.join(fixtureDir, 'comparison.json'),
    JSON.stringify(comparisonInfo, null, 2)
  );

  // Write diff if results don't match
  if (result.atlaspack && result.parcel && !result.match) {
    const diff = createSimpleDiff(result.atlaspack.code, result.parcel.code);
    await fs.writeFile(
      path.join(fixtureDir, 'diff.txt'),
      diff
    );
  }
}

function createSimpleDiff(atlaspack: string, parcel: string): string {
  const atlaspackLines = atlaspack.split('\n');
  const parcelLines = parcel.split('\n');
  
  let diff = '--- Atlaspack Output\n+++ Parcel Output\n\n';
  
  const maxLines = Math.max(atlaspackLines.length, parcelLines.length);
  
  for (let i = 0; i < maxLines; i++) {
    const atlaspackLine = atlaspackLines[i] || '';
    const parcelLine = parcelLines[i] || '';
    
    if (atlaspackLine !== parcelLine) {
      if (atlaspackLine) {
        diff += `- ${atlaspackLine}\n`;
      }
      if (parcelLine) {
        diff += `+ ${parcelLine}\n`;
      }
    } else if (atlaspackLine) {
      diff += `  ${atlaspackLine}\n`;
    }
  }
  
  return diff;
}

async function writeSummaryReport(fs: NodeFS, outputDir: string, results: ComparisonResult[]) {
  const summary = {
    timestamp: new Date().toISOString(),
    total: results.length,
    matches: results.filter(r => r.match).length,
    mismatches: results.filter(r => !r.match).length,
    errors: results.filter(r => r.error).length,
    results: results.map(r => ({
      fixture: r.fixture,
      match: r.match,
      error: r.error,
      hasAtlaspack: !!r.atlaspack,
      hasExpected: !!r.parcel
    }))
  };

  await fs.writeFile(
    path.join(outputDir, 'summary.json'),
    JSON.stringify(summary, null, 2)
  );

  // Create markdown report
  let markdown = `# Compiled CSS Transformer Comparison Report\n\n`;
  markdown += `**Generated:** ${summary.timestamp}\n\n`;
  markdown += `## Summary\n\n`;
  markdown += `- **Total Fixtures:** ${summary.total}\n`;
  markdown += `- **Matches:** ${summary.matches}\n`;
  markdown += `- **Mismatches:** ${summary.mismatches}\n`;
  markdown += `- **Errors:** ${summary.errors}\n\n`;

  if (summary.mismatches > 0 || summary.errors > 0) {
    markdown += `## Issues\n\n`;
    
    for (const result of results) {
      if (!result.match || result.error) {
        markdown += `### ${result.fixture}\n\n`;
        if (result.error) {
          markdown += `**Error:** ${result.error}\n\n`;
        } else {
          markdown += `**Status:** Outputs differ\n`;
          markdown += `**Atlaspack:** ${result.atlaspack ? 'Success' : 'Failed'}\n`;
          markdown += `**Expected:** ${result.parcel ? 'Found' : 'Missing'}\n\n`;
        }
      }
    }
  }

  await fs.writeFile(
    path.join(outputDir, 'report.md'),
    markdown
  );
}