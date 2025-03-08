import * as fs from 'node:fs';
import glob from 'glob';
import {Paths} from './paths.ts';
import {readJson, remapImports, rm, writeJson} from './fs.ts';
import {buildCore} from './build_core.ts';
import {buildPlugins} from './build_plugin.ts';
import {gatherDependencies, gatherDevDependencies} from './gather_deps.ts';

const whitelist = [
  '.scripts',
  '.gitignore',
  '.npmignore',
  'index.d.ts',
  'index.js',
  'package.json',
];

void (async function main() {
  // Clean dir
  for (const target of fs.readdirSync(Paths['~/']())) {
    if (whitelist.includes(target)) continue;
    await rm(Paths['~/'](target as string));
  }

  await buildCore();
  await buildPlugins();

  await remapImports(
    ...glob.sync(Paths['~/']('**/*.ts'), {ignore: '.scripts'}),
    ...glob.sync(Paths['~/']('**/*.js'), {ignore: '.scripts'}),
    ...glob.sync(Paths['~/']('**/*.json'), {ignore: '.scripts'}),
  );

  const pkg = await readJson(Paths['~/']('package.json'));
  pkg.dependencies = {
    ...(await gatherDependencies(
      Paths['root/']('packages/core/diagnostic/package.json'),
      Paths['root/']('packages/core/feature-flags/package.json'),
      Paths['root/']('packages/core/utils/package.json'),
      Paths['root/']('packages/core/logger/package.json'),
      Paths['root/']('packages/core/types-internal/package.json'),
      Paths['root/']('packages/core/markdown-ansi/package.json'),
      Paths['root/']('packages/core/profiler/package.json'),
      Paths['root/']('packages/core/build-cache/package.json'),
      Paths['root/']('packages/core/build-cache/package.json'),
      Paths['root/'](
        'packages/utils/atlaspack-watcher-watchman-js/package.json',
      ),
      Paths['root/']('packages/core/workers/package.json'),
      Paths['root/']('packages/core/types/package.json'),
      Paths['root/']('packages/core/fs/package.json'),
      Paths['root/']('packages/utils/domain-sharding/package.json'),
      Paths['root/']('packages/utils/ts-utils/package.json'),
      Paths['root/']('packages/core/plugin/package.json'),
      Paths['root/']('packages/utils/node-resolver-core/package.json'),
      Paths['root/']('packages/core/cache/package.json'),
      Paths['root/']('packages/core/graph/package.json'),
      Paths['root/']('packages/core/package-manager/package.json'),
      Paths['root/']('packages/core/core/package.json'),
      Paths['root/']('packages/core/cli/package.json'),
      ...glob.sync(Paths['root/']('packages/bundlers/**/package.json')),
      ...glob.sync(Paths['root/']('packages/compressors/**/package.json')),
      ...glob.sync(Paths['root/']('packages/namers/**/package.json')),
      ...glob.sync(Paths['root/']('packages/optimizers/**/package.json')),
      ...glob.sync(Paths['root/']('packages/packagers/**/package.json')),
      ...glob.sync(Paths['root/']('packages/reporters/**/package.json')),
      ...glob.sync(Paths['root/']('packages/resolvers/**/package.json')),
      ...glob.sync(Paths['root/']('packages/runtimes/**/package.json')),
      ...glob.sync(Paths['root/']('packages/transformers/**/package.json')),
      ...glob.sync(Paths['root/']('packages/validators/**/package.json')),
    )),
  };
  pkg.devDependencies = {
    ...(await gatherDevDependencies(
      Paths['root/']('packages/utils/node-resolver-core/package.json'),
    )),
  };

  await writeJson(Paths['~/']('package.json'), pkg);
})();
