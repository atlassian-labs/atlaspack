/* eslint-disable no-console */
import path from 'path';
import commander from 'commander';
import {
  runPackagingTest,
  isComparisonResult,
  formatComparisonResults,
  formatSize,
} from './index';

const program = new commander.Command();

program
  .name('packaging-test-harness')
  .description('Test harness for Atlaspack packaging')
  .option(
    '-o, --output <dir>',
    'Output directory for packaged bundle',
    './packaging-output',
  )
  .option('--bundle-type <type>', 'Filter bundles by type (e.g., "js", "css")')
  .option('--bundle-name <pattern>', 'Filter bundles by name pattern (regex)')
  .option('-v, --verbose', 'Enable verbose output')
  .option(
    '-c, --compare',
    'Compare native packager with JS packager (DevPackager/ScopeHoistingPackager)',
  )
  .arguments('<cache-dir>')
  .action(
    async (
      cacheDir: string,
      options: {
        output: string;
        bundleType?: string;
        bundleName?: string;
        verbose?: boolean;
        compare?: boolean;
      },
    ) => {
      const resolvedCacheDir = path.resolve(process.cwd(), cacheDir);
      const outputDir = path.resolve(process.cwd(), options.output);

      // Build bundle filter if specified
      let bundleFilter: ((bundle: any) => boolean) | undefined;
      if (options.bundleType || options.bundleName) {
        bundleFilter = (bundle: any) => {
          if (options.bundleType && bundle.type !== options.bundleType) {
            return false;
          }
          if (options.bundleName) {
            const regex = new RegExp(options.bundleName);
            if (!bundle.name || !regex.test(bundle.name)) {
              return false;
            }
          }
          return true;
        };
      }

      try {
        const result = await runPackagingTest({
          cacheDir: resolvedCacheDir,
          outputDir,
          bundleFilter,
          verbose: options.verbose,
          compare: options.compare,
        });

        if (isComparisonResult(result)) {
          // Print comparison results
          console.log(formatComparisonResults(result));

          console.log('\nOutput files:');
          console.log(`  Native: ${result.native.outputPath}`);
          console.log(`  JS:     ${result.js.outputPath}`);
        } else {
          console.log('\nPackaging complete!');
          console.log(`  Bundle ID: ${result.bundleId}`);
          console.log(`  Type: ${result.bundleType}`);
          console.log(`  Size: ${formatSize(result.size)}`);
          console.log(`  Time: ${result.timeMs.toFixed(2)}ms`);
          console.log(`  Hash: ${result.hash}`);
          console.log(`  Output: ${result.outputPath}`);
        }
      } catch (error) {
        console.error('Error:', error);
        process.exitCode = 1;
      }
    },
  );

program.addHelpText(
  'after',
  `
Environment Variables:
  ATLASPACK_SOURCES=true           Run from source TypeScript files
  ATLASPACK_REGISTER_USE_SRC=true  Use source imports for internal modules
  ATLASPACK_TRACING_MODE=stdout    Enable Rust tracing output
  RUST_LOG=trace                   Set Rust log level

Examples:
  $ packaging-test-harness /path/to/.parcel-cache
  $ packaging-test-harness -v -o ./output /path/to/.parcel-cache
  $ packaging-test-harness --bundle-type js /path/to/.parcel-cache
  $ packaging-test-harness --compare /path/to/.parcel-cache
`,
);

export function run(args: string[]): void {
  program.parse(['node', 'packaging-test-harness', ...args]);
}
