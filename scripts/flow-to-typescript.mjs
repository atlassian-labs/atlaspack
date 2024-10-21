// Run the following command before running this script:
// yarn typescriptify convert -p ../atlaspack --write --delete

import {execSync} from 'child_process';
import {readFile, rm, writeFile} from 'fs/promises';
import {join, relative} from 'path';

// TODO packages/utils/atlaspack-watcher-watchman-js/src/index.js flow local
// TODO require to import packages/utils/atlaspack-lsp/src/LspServer.ts
// TODO remove atlaspack-lsp tsconfig
const repositoryRoot = join(import.meta.dirname, '..');
const packagesRoot = join(repositoryRoot, 'packages');

async function replaceFile(path, replacer) {
  const file = await readFile(path, 'utf8');

  await writeFile(path, replacer(file));
}

function getWorkspaces() {
  const workspaces = JSON.parse(
    execSync('yarn --silent workspaces info --json', {
      encoding: 'utf8',
    }),
  );

  for (const [name, workspace] of Object.entries(workspaces)) {
    workspaces[name] = workspace.location;
  }

  return workspaces;
}

async function fixSyntax() {
  await replaceFile(
    join(packagesRoot, 'core', 'plugin', 'src', 'PluginAPI.ts'),
    (plugins) =>
      plugins
        .replace(/constructor<T>/g, 'constructor')
        .replace(/constructor<T, U>/g, 'constructor')
        .replace(/Opts<T>/g, 'Opts<mixed>')
        .replace(/Opts<T, U>/g, 'Opts<mixed, mixed>'),
  );

  await replaceFile(
    join(packagesRoot, 'transformers', 'less', 'src', 'LessTransformer.ts'),
    (transformer) =>
      transformer.replace('import {typeof default', 'import type {default'),
  );

  const corePackageRoot = join(packagesRoot, 'core', 'core');

  const filenames = [
    join(
      corePackageRoot,
      'src',
      'atlaspack-v3',
      'worker',
      'compat',
      'plugin-options.ts',
    ),
    join(corePackageRoot, 'src', 'public', 'BundleGraph.ts'),
    join(corePackageRoot, 'src', 'BundleGraph.ts'),
    join(corePackageRoot, 'src', 'SymbolPropagation.ts'),
    join(corePackageRoot, 'test', 'SymbolPropagation.test.ts'),
    join(packagesRoot, 'core', 'utils', 'src', 'collection.ts'),
    join(packagesRoot, 'core', 'workers', 'src', 'Worker.ts'),
    join(packagesRoot, 'packagers', 'js', 'src', 'ScopeHoistingPackager.ts'),
    join(packagesRoot, 'core', 'types-internal', 'src', 'index.ts'),
  ];

  await Promise.all(
    filenames.map((filename) =>
      replaceFile(filename, (file) =>
        file
          .replace(/\$Partial/g, 'Partial')
          .replace(/\$ReadOnlyMap/g, 'ReadonlyMap')
          .replace(/\$ReadOnlySet/g, 'ReadonlySet'),
      ),
    ),
  );
}

async function updateJsReferences() {
  await Promise.all(
    ['packages/runtimes/hmr/src/HMRRuntime.ts'].map((path) =>
      replaceFile(path, (file) => file.replace(/\.js'/g, ".ts'")),
    ),
  );
}

async function updatePackageFields(workspaces) {
  await Promise.all([
    ...Object.values(workspaces).map(async (workspace) => {
      const path = join(repositoryRoot, workspace, 'package.json');
      const packageJson = JSON.parse(await readFile(path, 'utf8'));

      if (
        ['@atlaspack/babel-preset-env', 'lmdb-js-lite'].includes(
          packageJson.name,
        )
      ) {
        return;
      }

      if (packageJson.main && packageJson.main.includes('src')) {
        packageJson.main = packageJson.main.replace(/\.js$/, '.ts');
      }

      if (packageJson.main && packageJson.main.endsWith('.js')) {
        packageJson.types = packageJson.main.replace(/\.js$/, '.d.ts');
      }

      if (packageJson.source && packageJson.source.endsWith('.js')) {
        packageJson.source = packageJson.source.replace(/\.js$/, '.ts');
      }

      if (packageJson.scripts) {
        delete packageJson.scripts['build-ts'];
        delete packageJson.scripts['check-ts'];

        if (Object.keys(packageJson.scripts).length === 0) {
          delete packageJson.scripts;
        }
      }

      await writeFile(path, JSON.stringify(packageJson, null, 2) + '\n');
    }),
    rm(join(packagesRoot, 'core', 'types-internal', 'scripts', 'build-ts.js')),
    rm(join(packagesRoot, 'core', 'types-internal', 'scripts', 'build-ts.sh')),
  ]);
}

async function removeFlowFiles() {
  await Promise.all([
    rm(join(repositoryRoot, 'flow-libs'), {recursive: true}),
    rm(join(repositoryRoot, 'flow-typed'), {recursive: true}),
    rm(join(repositoryRoot, '.flowconfig')),
  ]);
}

async function removeFlowReferences() {
  await Promise.all([
    replaceFile(join(repositoryRoot, '.github', 'workflows', 'ci.yml'), (ci) =>
      ci
        .replace('flow:', 'typecheck:')
        .replace('name: Flow', 'name: Typecheck')
        .replace('- run: yarn flow check', '- run: tsc --noEmit'),
    ),
    replaceFile(join(repositoryRoot, '.eslintignore'), (eslintIgnore) =>
      eslintIgnore.replace(/flow-typed\n\n/, ''),
    ),
    replaceFile(join(repositoryRoot, '.prettierignore'), (prettierIgnore) =>
      prettierIgnore.replace(/flow-libs\n/, '').replace(/flow-typed\n/, ''),
    ),
    replaceFile(
      join(repositoryRoot, '.vscode', 'extensions.json'),
      (vscodeExtensions) =>
        vscodeExtensions.replace(/\s+"flowtype.flow-for-vscode",/, ''),
    ),
    replaceFile(
      join(packagesRoot, 'dev', 'babel-preset', 'index.js'),
      (preset) =>
        preset.replace(/@babel\/preset-flow/g, '@babel/preset-typescript'),
    ),
  ]);
}

async function updateDependencies() {
  execSync('yarn remove -W flow-bin @khanacademy/flow-to-ts', {
    stdio: 'inherit',
  });

  execSync('yarn add -W --dev @types/sinon', {stdio: 'inherit'});
  execSync('yarn workspace @atlaspack/babel-preset remove @babel/preset-flow', {
    stdio: 'inherit',
  });

  // Manually update package.json due to https://github.com/yarnpkg/yarn/issues/7807
  const babelPackagePath = join(
    packagesRoot,
    'dev',
    'babel-preset',
    'package.json',
  );

  const babelPackageJson = JSON.parse(await readFile(babelPackagePath, 'utf8'));

  babelPackageJson.dependencies['@babel/preset-typescript'] = '^7.22.5';
  babelPackageJson.dependencies = Object.keys(babelPackageJson.dependencies)
    .sort()
    .reduce((obj, key) => {
      obj[key] = babelPackageJson.dependencies[key];
      return obj;
    }, {});

  await writeFile(
    babelPackagePath,
    JSON.stringify(babelPackageJson, null, 2) + '\n',
  );

  execSync('yarn', {stdio: 'inherit'});
}

async function addTsConfigs(workspaces) {
  await writeFile(
    join(repositoryRoot, 'tsconfig.node.json'),
    JSON.stringify(
      {
        compilerOptions: {
          composite: true,
          esModuleInterop: true,
          jsx: 'react',
          module: 'commonjs',
          noImplicitAny: false, // Enable this later
          skipLibCheck: true,
          strict: true,
          target: 'es2016',
        },
      },
      null,
      2,
    ) + '\n',
  );

  const references = [];

  await Promise.all(
    Object.values(workspaces).map(async (workspace) => {
      const workspaceReferences = new Set();
      const packageJson = JSON.parse(
        await readFile(join(workspace, 'package.json'), 'utf8'),
      );

      for (const key of [
        'dependencies',
        'devDependencies',
        'peerDependencies',
      ]) {
        for (const dependency of Object.keys(packageJson[key] ?? {})) {
          if (workspaces[dependency]) {
            workspaceReferences.add(
              relative(workspace, workspaces[dependency]),
            );
          }
        }
      }

      await writeFile(
        join(repositoryRoot, workspace, 'tsconfig.json'),
        JSON.stringify(
          {
            extends: `${relative(
              workspace,
              repositoryRoot,
            )}/tsconfig.node.json`,
            references: Array.from(workspaceReferences)
              .sort()
              .map((path) => ({path})),
          },
          null,
          2,
        ) + '\n',
      );

      references.push({path: relative(repositoryRoot, workspace)});
    }),
  );

  await writeFile(
    join(repositoryRoot, 'tsconfig.json'),
    JSON.stringify({references}, null, 2) + '\n',
  );
}

const workspaces = getWorkspaces();

// await fixSyntax();
// await updateJsReferences();
await updatePackageFields(workspaces);
// await removeFlowFiles();
// await removeFlowReferences();
// await updateDependencies();
// await addTsConfigs(workspaces);
