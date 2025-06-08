/* eslint-disable no-console */

/*
  This script goes over the vendored dependencies in ./src/vendored,
  copies their resolved targets (recursively) to the ./dist/vendored
  directory and rewrites their source and type imports to be relative.

  This:
  /node_modules
    /@baz
      /buzz
    /bar
    /foo
      /node_modules
        /bar

  Becomes:
  ./dist/vendor
    /_packages
      /@baz__buzz@1.0.0
      /foo@1.0.0
      /bar@1.0.0
      /bar@1.0.1
    manifest.json
*/
import * as path from 'node:path';
import glob from 'glob';
import {renameImportsFile} from './utils/rename-imports.mts';
import {
  cpAll,
  createDirAll,
  isFile,
  readJson,
  recreateDirAll,
  rm,
  writeJson,
} from './utils/fs-extra.mts';
import {resolveDependencySlow} from './utils/resolve-dep.mts';
import {findPackageJson} from './utils/find-package-json.mts';
import {resolveTypesEntry} from './utils/resolve-types-entry.mts';
import {Paths} from './utils/paths.mts';

export type PackageJson = {
  name: string;
  version: string;
  main: string;
  types: string;
  dependencies: Record<string, string>;
};

export type VendorMeta = {
  name: string;
  ambient?: boolean;
};

export type VendoredPackage = {
  name: string;
  version: string;
  main: string;
  types: string | null;
  base: string;
  from: string;
  ambient?: boolean;
  dependencies: Set<string>;
};

export type VendorManifest = Record<string, VendoredPackage>;

export async function main() {
  await recreateDirAll(Paths.vendorDist);
  const vendored: VendorManifest = {};

  // Analyze vendored packages
  for (const foundRel of glob.sync('**/vendor.json', {
    cwd: Paths.vendorSrc,
  })) {
    const foundDir = path.dirname(path.join(foundRel));
    const meta = await readJson<VendorMeta>(
      path.join(Paths.vendorSrc, foundRel),
    );
    // Recurse through dependencies and include them in the manifest
    await includePackage(
      vendored,
      meta.name,
      path.join(Paths.vendorSrc, foundDir),
      meta.ambient,
    );
  }

  // Process vendored packages
  for (const [key, vendor] of Object.entries(vendored)) {
    const outputDir = path.join(Paths.vendorDist, '_packages', key);

    // Create inner directory
    await createDirAll(outputDir);

    // Copy to the target directory
    await cpAll(path.join(Paths.root, vendor.base), outputDir);

    // Clean up
    await rm(path.join(outputDir, 'node_modules'));

    // Rewrite dependency imports of vendored package to relative paths
    for (const key of Array.from(vendor.dependencies)) {
      const pathToDepAbs = path.join(Paths.vendorDist, '_packages', key);

      const dep = vendored[key];

      for (const foundRel of glob.sync('**/*', {
        cwd: outputDir,
        ignore: ['**/*.json'],
      })) {
        const found = path.join(outputDir, foundRel);
        if (!(await isFile(found))) {
          continue;
        }
        const pathToDep = path.relative(
          path.dirname(found),
          path.join(pathToDepAbs),
        );
        await renameImportsFile(found, {
          from: dep.name,
          to: pathToDep,
        });
      }
    }
  }

  // Copy packages from ./src/vendor folder into ./lib/vendor
  for (const [key, vendor] of Object.entries(vendored)) {
    if (!vendor.from.startsWith(path.join('packages', 'unified'))) {
      continue;
    }

    const vendorOutput = path.join(
      Paths.vendorDist,
      path.basename(vendor.from),
    );

    await cpAll(path.join(Paths.root, vendor.from), vendorOutput);

    for (const foundRel of glob.sync('**/*', {
      cwd: vendorOutput,
      ignore: ['**/*.json'],
    })) {
      const found = path.join(vendorOutput, foundRel);
      if (!(await isFile(found))) {
        continue;
      }
      await renameImportsFile(found, {
        from: vendor.name,
        to: `../_packages/${key}`,
      });
    }

    // Write manifest file
    await writeJson(path.join(Paths.vendorDist, 'manifest.json'), vendored);
  }

  // Rewrite import statements for ambient imports (added by the build process)
  for (const foundRel of glob.sync('**/*', {
    cwd: Paths.unifiedDist,
    ignore: ['**/*.json', 'vendor/**/*'],
  })) {
    const found = path.join(Paths.unifiedDist, foundRel);
    if (!(await isFile(found))) {
      continue;
    }
    for (const [, vendor] of Object.entries(vendored)) {
      if (!vendor.ambient) {
        continue;
      }

      const basename = path.basename(vendor.from);
      const vendorRelative = path.relative(
        path.dirname(found),
        Paths.vendorDist,
      );

      await renameImportsFile(found, {
        from: vendor.name,
        to: `${vendorRelative}/${basename}/index.js`,
      });
    }
  }
}

async function includePackage(
  vendored: VendorManifest,
  packageName: string,
  cwd: string,
  ambient?: boolean,
): Promise<string> {
  const pkgJsonPath: string = (await findPackageJson(packageName, cwd))!;
  const pkgJson = await readJson<PackageJson>(pkgJsonPath);
  const pkgJsonDir = path.dirname(pkgJsonPath);

  const sanitizedName = pkgJson.name.replace('/', '__');
  const key = `${sanitizedName}@${pkgJson.version}`;

  const mainAbs = await resolveDependencySlow(pkgJson.name, cwd);
  const main = mainAbs.replace(pkgJsonDir + path.sep, '');

  vendored[key] = {
    name: pkgJson.name,
    version: pkgJson.version,
    ambient,
    main: main,
    types: await resolveTypesEntry(pkgJsonDir),
    base: pkgJsonDir.replace(Paths.root + path.sep, ''),
    from: cwd.replace(Paths.root + path.sep, ''),
    dependencies: new Set(),
  };

  for (const dependency of Object.keys(pkgJson.dependencies || {})) {
    const result = await includePackage(vendored, dependency, pkgJsonDir);
    vendored[key].dependencies.add(result);
  }

  return key;
}

main().catch((error) => {
  console.log(error);
  process.exit(1);
});
