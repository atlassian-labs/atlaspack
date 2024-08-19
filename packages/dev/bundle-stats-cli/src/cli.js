/* eslint-disable no-console, monorepo/no-internal-import */
// @flow strict-local
import type {PackagedBundle} from '@atlaspack/types';
import type {AtlaspackOptions} from '@atlaspack/core/src/types';
import type {commander$Command} from 'commander';

// $FlowFixMe[untyped-import]
import {version} from '../package.json';

import commander from 'commander';
import fs from 'fs';
import path from 'path';
import {DefaultMap} from '@atlaspack/utils';

const {
  loadGraphs,
  getBundleStats,
  PackagedBundleClass,
} = require('./deep-imports.js');

async function run({cacheDir, outDir}) {
  // 1. load bundle graph and info via atlaspack~query
  let {bundleGraph, bundleInfo} = await loadGraphs(cacheDir);

  if (bundleGraph == null) {
    console.error('Bundle Graph could not be found');
    process.exit(1);
    throw new Error();
  }

  if (bundleInfo == null) {
    console.error('Bundle Info could not be found');
    process.exit(1);
    throw new Error();
  }

  // 2. generate stats files for each target
  fs.mkdirSync(outDir, {recursive: true});

  let projectRoot = process.cwd();

  // $FlowFixMe[unclear-type]
  let atlaspackOptions: AtlaspackOptions = ({projectRoot}: any);

  let bundlesByTarget: DefaultMap<
    string /* target name */,
    Array<PackagedBundle>,
  > = new DefaultMap(() => []);
  for (let bundle of bundleGraph.getBundles()) {
    bundlesByTarget
      .get(bundle.target.name)
      .push(
        PackagedBundleClass.getWithInfo(
          bundle,
          bundleGraph,
          atlaspackOptions,
          bundleInfo.get(bundle.id),
        ),
      );
  }

  for (let [targetName, bundles] of bundlesByTarget) {
    fs.writeFileSync(
      path.join(outDir, `${targetName}-stats.json`),
      JSON.stringify(getBundleStats(bundles, atlaspackOptions), null, 2),
    );
  }
}

export const command: commander$Command = new commander.Command()
  .version(version, '-V, --version')
  .description('Generate a stats report for a Atlaspack build')
  .option('-v, --verbose', 'Print verbose output')
  .option(
    '-c, --cache-dir <path>',
    'Directory to the atlaspack cache',
    '.atlaspack-cache',
  )
  .option(
    '-o, --out-dir <path>',
    'Directory to write the stats to',
    'atlaspack-bundle-reports',
  )
  .action(run);
