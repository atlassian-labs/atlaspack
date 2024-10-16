// @flow strict-local

import type {CmdOptions} from './utils';
import type {FileSystem} from '@atlaspack/fs';

import {AtlaspackLinkConfig} from './AtlaspackLinkConfig';
import {
  findParcelPackages,
  mapNamespacePackageAliases,
  cleanupBin,
  cleanupNodeModules,
  fsWrite,
  fsSymlink,
} from './utils';

import nullthrows from 'nullthrows';
import path from 'path';
import {NodeFS} from '@atlaspack/fs';
import commander from 'commander';

export type LinkOptions = {|
  dryRun?: boolean,
  log?: (...data: mixed[]) => void,
|};

export type LinkCommandOptions = {|
  +link?: typeof link,
  +fs?: FileSystem,
  +log?: (...data: mixed[]) => void,
|};

const NOOP: (...data: mixed[]) => void = () => {};

export async function link(
  config: AtlaspackLinkConfig,
  {dryRun = false, log = NOOP}: LinkOptions,
): Promise<void> {
  config.validate();

  let {appRoot, packageRoot, namespace} = config;

  let nodeModulesPaths = config.getNodeModulesPaths();

  let opts: CmdOptions = {appRoot, packageRoot, dryRun, log, fs: config.fs};

  // Step 1: Determine all Parcel packages to link
  // --------------------------------------------------------------------------------

  let parcelPackages = await findParcelPackages(config.fs, packageRoot);

  // Step 2: Delete all official packages (`@atlaspack/*`) from node_modules
  // --------------------------------------------------------------------------------

  for (let nodeModules of nodeModulesPaths) {
    await cleanupBin(nodeModules, opts);
    await cleanupNodeModules(
      nodeModules,
      (packageName) => parcelPackages.has(packageName),
      opts,
    );
  }

  // Step 3: Link the Parcel packages into node_modules
  // --------------------------------------------------------------------------------

  for (let [packageName, p] of parcelPackages) {
    await fsSymlink(p, path.join(appRoot, 'node_modules', packageName), opts);
  }

  // Step 4: Point `parcel` bin symlink to linked `packages/core/cli/src/bin.js`
  // --------------------------------------------------------------------------------

  await fsSymlink(
    path.join(packageRoot, 'core/cli/src/bin.js'),
    path.join(appRoot, 'node_modules/.bin/atlaspack'),
    opts,
  );

  // Step 5 (optional): If a namespace is not "@atlaspack", map namespaced package aliases.
  // --------------------------------------------------------------------------------

  if (namespace != null && namespace !== '@atlaspack') {
    let namespacePackages = mapNamespacePackageAliases(
      namespace,
      parcelPackages,
    );

    // Step 5.1: In .parcelrc, rewrite all references to official plugins to `@atlaspack/*`
    // --------------------------------------------------------------------------------

    let atlaspackConfigPath = path.join(appRoot, '.parcelrc');
    if (config.fs.existsSync(atlaspackConfigPath)) {
      let atlaspackConfig = config.fs.readFileSync(atlaspackConfigPath, 'utf8');
      await fsWrite(
        atlaspackConfigPath,
        atlaspackConfig.replace(
          new RegExp(`"(${namespace}/atlaspack-[^"]*)"`, 'g'),
          (_, match) => `"${namespacePackages.get(match) ?? match}"`,
        ),
        opts,
      );
    }

    // Step 5.2: In the root package.json, rewrite all references to official plugins to @atlaspack/...
    // For configs like "@namespace/parcel-bundler-default":{"maxParallelRequests": 10}
    // --------------------------------------------------------------------------------

    let rootPkgPath = path.join(appRoot, 'package.json');
    if (config.fs.existsSync(rootPkgPath)) {
      let rootPkg = config.fs.readFileSync(rootPkgPath, 'utf8');
      await fsWrite(
        rootPkgPath,
        rootPkg.replace(
          new RegExp(`"(${namespace}/atlaspack-[^"]*)"(\\s*:\\s*{)`, 'g'),
          (_, match, suffix) =>
            `"${namespacePackages.get(match) ?? match}"${suffix}`,
        ),
        opts,
      );
    }

    // Step 5.3: Delete namespaced packages (`@namespace/parcel-*`) from node_modules
    // --------------------------------------------------------------------------------

    for (let nodeModules of nodeModulesPaths) {
      await cleanupNodeModules(
        nodeModules,
        (packageName) => namespacePackages.has(packageName),
        opts,
      );
    }

    // Step 5.4: Link the Parcel packages into node_modules as `@namespace/parcel-*`
    // --------------------------------------------------------------------------------

    for (let [alias, parcelName] of namespacePackages) {
      let p = nullthrows(parcelPackages.get(parcelName));
      await fsSymlink(p, path.join(appRoot, 'node_modules', alias), opts);
    }
  }
}

export function createLinkCommand(
  opts?: LinkCommandOptions,
  // $FlowFixMe[invalid-exported-annotation]
): commander.Command {
  let action = opts?.link ?? link;
  let log = opts?.log ?? NOOP;
  let fs = opts?.fs ?? new NodeFS();

  return new commander.Command('link')
    .arguments('[packageRoot]')
    .description('Link a dev copy of Parcel into an app', {
      packageRoot:
        'Path to the Parcel package root\nDefaults to the package root containing this package',
    })
    .option('-d, --dry-run', 'Do not write any changes')
    .option(
      '-n, --namespace <namespace>',
      'Namespace for packages',
      '@atlaspack',
    )
    .option(
      '-g, --node-modules-glob <glob>',
      'Location where node_modules should be linked in the app.\nCan be repeated with multiple globs.',
      (glob, globs) => globs.concat([glob.replace(/["']/g, '')]),
      ['node_modules'],
    )
    .action(async (packageRoot, options) => {
      if (options.dryRun) log('Dry run...');
      let appRoot = process.cwd();

      let parcelLinkConfig;

      try {
        parcelLinkConfig = await AtlaspackLinkConfig.load(appRoot, {fs});
      } catch (e) {
        // boop!
      }

      if (parcelLinkConfig) {
        throw new Error(
          'A Parcel link already exists! Try `atlaspack-link unlink` to re-link.',
        );
      }

      parcelLinkConfig = new AtlaspackLinkConfig({
        fs,
        appRoot,
        packageRoot: packageRoot ?? path.join(__dirname, '../../../'),
        namespace: options.namespace,
        nodeModulesGlobs: options.nodeModulesGlob,
      });

      await action(parcelLinkConfig, {dryRun: options.dryRun, log});

      if (!options.dryRun) await parcelLinkConfig.save();

      log('🎉 Linking successful');
    });
}
