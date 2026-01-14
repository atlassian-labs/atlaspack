#!/usr/bin/env node
/* eslint-disable import/no-extraneous-dependencies */
/* eslint-disable no-console */
'use strict';

/**
 * Script to transform one or more files using Atlaspack
 *
 * Usage:
 *   node scripts/transform-file.js <file> [--cwd <directory>] [--profile-native [instruments|samply]]
 *
 * Options:
 *   <file>                  Path to the file to transform (required)
 *   --cwd <directory>       Working directory to load settings from (default: current directory)
 *   --profile-native        Enable native profiling (instruments on macOS, samply otherwise)
 *   --profile-native=<type> Specify profiler type: instruments or samply
 *   --production            Use production mode (default: development)
 *   --no-cache              Disable caching
 *   --config <path>         Path to atlaspack config file
 *   --feature-flag <name=value>  Set a feature flag value
 *   --output                Output the transformed code to stdout
 *
 * Examples:
 *   node scripts/transform-file.js src/index.js
 *   node scripts/transform-file.js src/index.js --cwd /path/to/project --profile-native
 *   node scripts/transform-file.js src/index.js --profile-native=samply --production
 */

const path = require('path');
const fs = require('fs').promises;
const os = require('os');
const glob = require('glob');
const {DEFAULT_FEATURE_FLAGS} = require('@atlaspack/feature-flags');

const Atlaspack = require('@atlaspack/core').default;

async function main() {
  const args = process.argv.slice(2);

  if (args.length === 0 || args.includes('--help') || args.includes('-h')) {
    printUsage();
    process.exit(args.includes('--help') || args.includes('-h') ? 0 : 1);
  }

  const options = parseArgs(args);

  if (!options.file) {
    console.error('Error: No file specified');
    printUsage();
    process.exit(1);
  }

  // Change to specified cwd if provided
  const cwd = options.cwd ? path.resolve(options.cwd) : process.cwd();
  process.chdir(cwd);

  // Resolve file pattern relative to cwd and expand globs
  const filePattern = path.resolve(cwd, options.file);
  const filePaths = Array.from(
    new Set(
      glob.sync(filePattern, {
        cwd,
        nodir: true,
        absolute: true,
      }),
    ),
  );

  if (filePaths.length === 0) {
    console.error(`Error: No files matched pattern: ${options.file}`);
    process.exit(1);
  }

  console.log(
    `Transforming ${filePaths.length} file(s) matching: ${options.file}`,
  );
  console.log(`Working directory: ${cwd}`);
  for (const filePath of filePaths) {
    console.log(`  - ${filePath}`);
  }

  // Determine native profiler type
  let nativeProfiler;
  if (options.profileNative) {
    if (
      options.profileNative === 'instruments' ||
      options.profileNative === 'samply'
    ) {
      nativeProfiler = options.profileNative;
    } else if (options.profileNative === true) {
      nativeProfiler = os.platform() === 'darwin' ? 'instruments' : 'samply';
    }
    console.log(`Native profiling enabled: ${nativeProfiler}`);
  }

  const mode = options.production ? 'production' : 'development';
  console.log(`Mode: ${mode}`);

  try {
    // Find default config
    let defaultConfig;
    try {
      defaultConfig = require.resolve('@atlaspack/config-default', {
        paths: [cwd, __dirname],
      });
    } catch (e) {
      console.error(
        'Error: Could not find @atlaspack/config-default. Make sure atlaspack is properly installed.',
      );
      process.exit(1);
    }

    const atlaspackOptions = {
      entries: filePaths,
      defaultConfig,
      shouldPatchConsole: false,
      shouldDisableCache: options.noCache ?? false,
      mode,
      nativeProfiler,
      shouldProfile: options.profile ?? false,
      logLevel: options.logLevel ?? 'info',
      projectRoot: cwd,
      config: options.config,
      featureFlags: options.featureFlags,
      additionalReporters: [
        {
          packageName: '@atlaspack/reporter-cli',
          resolveFrom: __filename,
        },
      ],
      defaultTargetOptions: {
        shouldOptimize: mode === 'production',
        sourceMaps: true,
      },
    };

    const atlaspack = new Atlaspack(atlaspackOptions);

    console.log('\nStarting transformation...\n');

    // Warm up the code base using the first matched file
    if (options.profile || options.profileNative) {
      await atlaspack.unstable_transform({
        filePath: filePaths[0],
      });
      await new Promise((resolve) => setTimeout(resolve, 5000));
    }

    for (const filePath of filePaths) {
      console.log(`\nStarting transformation for ${filePath}...\n`);

      try {
        const startTime = Date.now();

        const assets = await atlaspack.unstable_transform({
          filePath,
          nativeProfiler,
        });
        const duration = Date.now() - startTime;

        console.log(
          `\nTransformation completed for ${filePath} in ${duration}ms`,
        );
        console.log(`Generated ${assets.length} asset(s):\n`);

        for (const asset of assets) {
          console.log(`  - ${asset.filePath}`);
          console.log(`    Type: ${asset.type}`);
          console.log(
            `    Bundle behavior: ${asset.bundleBehavior || 'default'}`,
          );

          if (options.output) {
            const code = await asset.getCode();
            console.log('\n--- Code ---');
            console.log(code);
            console.log('--- End Code ---\n');
          }

          if (options.outputFile) {
            const code = await asset.getCode();
            await fs.writeFile(asset.filePath + '.' + options.outputFile, code);
            console.log(`Output file: ${asset.filePath}.${options.outputFile}`);
          }
        }
      } catch (error) {
        console.error('\nTransformation failed:');
        console.error(error.message);
        if (error.diagnostics) {
          for (const diagnostic of error.diagnostics) {
            console.error(`\n${diagnostic.message}`);
            if (diagnostic.codeFrames) {
              for (const frame of diagnostic.codeFrames) {
                console.error(frame.code);
              }
            }
          }
        }
      }
    }

    // Stop profiling if it was started
    if (atlaspack.isProfiling) {
      console.log('\nStopping profiler...');
      await atlaspack.stopProfiling();
    }

    process.exit(0);
  } catch (error) {
    console.error(`\nUnexpected error: ${error.message}`);
    process.exit(1);
  }
}

function parseArgs(args) {
  const options = {
    file: null,
    cwd: null,
    profileNative: false,
    production: false,
    noCache: false,
    config: null,
    output: false,
    profile: false,
    logLevel: 'info',
    featureFlags: {},
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];

    if (arg === '--cwd') {
      options.cwd = args[++i];
    } else if (arg.startsWith('--cwd=')) {
      options.cwd = arg.slice(6);
    } else if (arg === '--profile-native') {
      options.profileNative = true;
    } else if (arg.startsWith('--profile-native=')) {
      options.profileNative = arg.slice(17);
    } else if (arg === '--production') {
      options.production = true;
    } else if (arg === '--no-cache') {
      options.noCache = true;
    } else if (arg === '--config') {
      options.config = args[++i];
    } else if (arg.startsWith('--config=')) {
      options.config = arg.slice(9);
    } else if (arg === '--output') {
      options.output = true;
    } else if (arg === '--profile') {
      options.profile = true;
    } else if (arg === '--log-level') {
      options.logLevel = args[++i];
    } else if (arg.startsWith('--log-level=')) {
      options.logLevel = arg.slice(12);
    } else if (arg === '--feature-flag') {
      parseFeatureFlag(args[++i], options.featureFlags);
    } else if (arg.startsWith('--feature-flag=')) {
      parseFeatureFlag(arg.slice(15), options.featureFlags);
    } else if (!arg.startsWith('-')) {
      options.file = arg;
    } else if (arg.startsWith('--output-file=')) {
      options.outputFile = arg.slice('--output-file='.length);
    }
  }

  return options;
}

function parseFeatureFlag(value, featureFlags) {
  const [name, val] = value.split('=');
  if (name in DEFAULT_FEATURE_FLAGS) {
    if (typeof DEFAULT_FEATURE_FLAGS[name] === 'boolean') {
      if (val !== 'true' && val !== 'false') {
        console.error(
          `Error: Feature flag ${name} must be set to true or false`,
        );
        process.exit(1);
      }
      featureFlags[name] = val === 'true';
    } else {
      featureFlags[name] = String(val);
    }
  } else {
    console.warn(`Warning: Unknown feature flag ${name}, it will be ignored`);
  }
}

function printUsage() {
  console.log(`
Usage: node scripts/transform-file.js <file> [options]

Transform a single file using Atlaspack.

Arguments:
  <file>                      Path or glob of the file(s) to transform (required)

Options:
  --cwd <directory>           Working directory to load settings from (default: current directory)
  --profile-native            Enable native profiling (instruments on macOS, samply otherwise)
  --profile-native=<type>     Specify profiler type: instruments or samply
  --profile                   Enable sampling build profiling
  --production                Use production mode (default: development)
  --no-cache                  Disable caching
  --config <path>             Path to atlaspack config file
  --feature-flag <name=value> Set a feature flag (can be used multiple times)
  --output                    Output the transformed code to stdout
  --log-level <level>         Set log level: none, error, warn, info, verbose
  --help, -h                  Show this help message

Examples:
  node scripts/transform-file.js src/index.js
  node scripts/transform-file.js 'src/**/*.js'
  node scripts/transform-file.js src/index.js --cwd /path/to/project
  node scripts/transform-file.js src/index.js --profile-native
  node scripts/transform-file.js src/index.js --profile-native=samply --production
  node scripts/transform-file.js src/index.js --output
  node scripts/transform-file.js src/index.js --feature-flag atlaspackV3=true
`);
}

main().catch((error) => {
  console.error('Unexpected error:', error);
  process.exit(1);
});
