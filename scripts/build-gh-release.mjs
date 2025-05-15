/* eslint-disable no-console */
import * as path from 'node:path';
import * as fs from 'node:fs';
import fsExtra from 'fs-extra';
import * as process from 'node:process';
import * as module from 'node:module';
import * as child_process from 'node:child_process';
import * as url from 'node:url';
import glob from 'glob';
import tmpDir from 'temp-dir';
import semver from 'semver';

const __tmp = path.join(
  tmpDir,
  `atlaspack-${(Math.random() * 100000000000).toFixed()}`,
);
const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(__dirname);
const require = module.createRequire(path.join(__root, 'index.js'));

const platform = {
  linux: 'linux',
  darwin: 'macos',
  win32: 'windows',
}[process.platform];

const arch = {
  x64: 'amd64',
  arm64: 'arm64',
}[process.arch];

const release = `atlaspack-${platform}-${arch}`;

const version = process.env.ATLASPACK_VERSION || '0.0.13-local';
if (!semver.valid(version)) {
  console.error('Invalid semver specified');
  process.exit(1);
}

if (!platform || !arch || !version) {
  console.error('Invalid os/arch/version');
  console.table({
    platform,
    arch,
    version,
  });
  process.exit(1);
}

const tarInclude = [];

const packageJson = {
  name: release,
  version: version,
  bin: {
    atlaspack: './lib/packages/core/cli/bin/atlaspack.js',
  },
  exports: {
    '.': './lib/packages/core/core/lib/index.js',
    './*': './lib/packages/core/*/lib/index.js',
    './package.json': './package.json',
  },
  dependencies: {},
  devDependencies: {},
};

for (const pkgPath of glob.sync('./packages/**/package.json', {
  cwd: __root,
  ignore: [
    '**/node_modules/**',
    '**/integration-tests/**',
    '**/test/**',
    '**/examples/**',
    '**/apvm/**/*',
  ],
})) {
  if (pkgPath.includes('apvm')) continue;
  if (pkgPath.includes('node_modules')) continue;
  if (pkgPath.includes('test')) continue;
  try {
    const pkg = JSON.parse(fs.readFileSync(path.join(__root, pkgPath), 'utf8'));
    if (!pkg.publishConfig || pkg.publishConfig.access !== 'public') {
      continue;
    }

    tarInclude.push(path.dirname(pkgPath));
    const entry = require.resolve(pkg.name).replace(__root + '/', '');
    const specifier = path.dirname(pkgPath).replace('./packages/', '');

    packageJson.exports[`./${specifier}`] = `./lib/${entry}`;

    packageJson.dependencies[pkg.name] = `file:./lib/${path
      .dirname(pkgPath)
      .replace('./', '')}`;

    for (const [key, version] of Object.entries(pkg.dependencies)) {
      if (key.startsWith('@atlaspack/')) continue;
      // Resolve dependencies to their exact versions
      // const pkgPath = module.findPackageJSON(key, new URL('../', import.meta.url));
      // const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'))
      if (
        !packageJson.dependencies[key] ||
        semver.gt(version, packageJson.dependencies[key])
      ) {
        packageJson.dependencies[key] = version;
      }
    }

    if (pkg.name === '@atlaspack/node-resolver-core') {
      packageJson.devDependencies = {
        ...packageJson.devDependencies,
        ...pkg.devDependencies,
      };
    }
  } catch (error) {
    continue;
  }
}

packageJson.exports['./*'] = './lib/packages/core/*/lib/index.js';
packageJson.dependencies = sortObject(packageJson.dependencies);
packageJson.devDependencies = sortObject(packageJson.devDependencies);

if (fs.existsSync(path.join(__root, 'release', release))) {
  fs.rmSync(path.join(__root, 'release', release), {
    recursive: true,
    force: true,
  });
}
fs.mkdirSync(path.join(__root, 'release', release, 'lib'), {recursive: true});

for (const include of tarInclude) {
  await fsExtra.copy(
    path.join(__root, include),
    path.join(__root, 'release', release, 'lib', include),
    {
      filter: (path) => {
        if (path.includes('fixture')) return false;
        if (path.endsWith('.gitignore')) return false;
        if (path.endsWith('.map')) return false;
        if (path.endsWith('.test.')) return false;
        if (fs.lstatSync(path).isFile()) return true;
        return !(path.indexOf('node_modules') > -1);
      },
    },
  );
}

fs.writeFileSync(
  path.join(__root, 'release', release, 'lib', 'package.json'),
  JSON.stringify(
    {
      name: '@atlaspack/monorepo',
      private: true,
      workspaces: ['packages/*/*'],
    },
    null,
    2,
  ),
  'utf8',
);

fs.writeFileSync(
  path.join(__root, 'release', release, 'package.json'),
  JSON.stringify(packageJson, null, 2),
  'utf8',
);
fs.writeFileSync(
  path.join(__root, 'release', release, '.npmignore'),
  '!*',
  'utf8',
);

for (const pkgPath of glob.sync(
  `./release/${release}/lib/packages/**/package.json`,
  {cwd: __root},
)) {
  try {
    const pkg = JSON.parse(fs.readFileSync(path.join(__root, pkgPath), 'utf8'));
    if (!pkg.publishConfig || pkg.publishConfig.access !== 'public') {
      continue;
    }

    const original = pkg.dependencies;
    pkg.dependencies = {};

    for (const [key, version] of Object.entries(original)) {
      if (key.startsWith('@atlaspack/')) continue;
      pkg.dependencies[key] = version;
    }

    pkg.version = version;
    pkg.dependencies = sortObject(pkg.dependencies);
    pkg.peerDependencies = undefined;

    if (pkg.name !== '@atlaspack/node-resolver-core') {
      pkg.devDependencies = undefined;
    }
    pkg.scripts = undefined;
    pkg.exports = undefined;
    pkg.engines = undefined;
    pkg.source = undefined;

    fs.writeFileSync(
      path.join(__root, pkgPath),
      JSON.stringify(pkg, null, 2),
      'utf8',
    );
  } catch (error) {
    continue;
  }
}

// Generate lock files
try {
  if (fs.existsSync(__tmp)) {
    fs.rmSync(__tmp, {
      recursive: true,
      force: true,
    });
  }
  fs.cpSync(path.join(__root, 'release', release), __tmp, {recursive: true});

  child_process.execFileSync('npm', ['install', '--legacy-peer-deps'], {
    stdio: 'inherit',
    shell: true,
    cwd: __tmp,
  });

  fs.cpSync(
    path.join(__tmp, 'package-lock.json'),
    path.join(__root, 'release', release, '.package-lock.json'),
  );
} finally {
  fs.rmSync(__tmp, {
    recursive: true,
    force: true,
  });
}

try {
  if (fs.existsSync(__tmp)) {
    fs.rmSync(__tmp, {
      recursive: true,
      force: true,
    });
  }
  fs.cpSync(path.join(__root, 'release', release), __tmp, {recursive: true});

  child_process.execFileSync('yarn', {
    stdio: 'inherit',
    shell: true,
    cwd: __tmp,
  });

  fs.cpSync(
    path.join(__tmp, 'yarn.lock'),
    path.join(__root, 'release', release, '.yarn.lock'),
  );
} finally {
  fs.rmSync(__tmp, {
    recursive: true,
    force: true,
  });
}

child_process.execFileSync('npm', ['pack'], {
  stdio: 'inherit',
  shell: true,
  cwd: path.join(__root, 'release', release),
});

fs.renameSync(
  path.join(__root, 'release', release, `${release}-${version}.tgz`),
  path.join(__root, 'release', `${release}-${version}.tar.gz`),
);

function sortObject(input) {
  return Object.keys(input)
    .sort()
    .reduce((obj, key) => {
      obj[key] = input[key];
      return obj;
    }, {});
}
