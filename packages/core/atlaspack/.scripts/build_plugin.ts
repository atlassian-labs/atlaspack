import * as path from 'node:path';
import * as fs from 'node:fs';
import glob from 'glob';
import {Paths} from './paths.ts';
import {writeFile, readJson, rm, cp} from './fs.ts';

export async function buildPlugins() {
  // Plugins
  const skip = {
    'bundler-experimental': true,
    'repl-runtimes': true,
    '@atlaspack/reporter-dev-server': true,
  };

  const override = {
    ['runtimes/hmr']: Paths['~/']('runtimes/browser-hmr'),
  };

  for (const folder of [
    'bundlers',
    'compressors',
    'namers',
    'optimizers',
    'packagers',
    'reporters',
    'resolvers',
    'runtimes',
    'transformers',
    'validators',
  ])
    for (const target of fs.readdirSync(Paths['root/']('packages', folder))) {
      if (skip[target]) continue;
      const from = Paths['root/']('packages', folder, target);
      const to = override[`${folder}/${target}`]
        ? override[`${folder}/${target}`]
        : Paths['~/'](folder, target);

      const {main, name} = await readJson<{main: string; name: string}>(
        path.join(from, 'package.json'),
      );
      if (skip[name]) continue;

      const relMain = main.replace('lib/', '');
      await cp(path.join(from, 'lib'), to);

      if (relMain.endsWith('index.js')) continue;
      await writeFile(
        path.join(to, 'index.js'),
        `module.exports = require('./${relMain}');\n`,
      );
    }

  await cp(
    Paths['root/']('packages/reporters/dev-server/lib'),
    Paths['~/']('reporters/dev-server/lib'),
  );

  await cp(
    Paths['root/']('packages/reporters/dev-server/src/templates'),
    Paths['~/']('reporters/dev-server/src/templates'),
  );

  await writeFile(
    Paths['~/']('reporters/dev-server/index.js'),
    `module.exports = require('./lib/ServerReporter.js');\n`,
  );

  // Configs
  await cp(Paths['root/']('packages/configs'), Paths['~/']('configs'));

  await rm(
    ...glob.sync(Paths['~/']('configs/**/package.json')),
    ...glob.sync(Paths['~/']('configs/**/test')),
    ...glob.sync(Paths['~/']('**/CHANGELOG.md')),
  );
}
