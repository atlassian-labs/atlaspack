import type {FilePath, ModuleRequest} from '@atlaspack/types';
import type {FileSystem} from '@atlaspack/fs';

import invariant from 'assert';
import ThrowableDiagnostic from '@atlaspack/diagnostic';
import {resolveConfig} from '@atlaspack/utils';
import {exec as _exec} from 'child_process';
import {promisify} from 'util';

export const exec: (
  command: string,
  options?: child_process.execOpts,
) => Promise<{
  stdout: string | Buffer;
  stderr: string | Buffer;
}> = _exec
  ? promisify(_exec)
  : // _exec is undefined in browser builds
    _exec;

export function npmSpecifierFromModuleRequest(
  moduleRequest: ModuleRequest,
): string {
  return moduleRequest.range != null
    ? [moduleRequest.name, moduleRequest.range].join('@')
    : moduleRequest.name;
}

export function moduleRequestsFromDependencyMap(dependencyMap: {
  [key: string]: string;
}): Array<ModuleRequest> {
  return Object.entries(dependencyMap).map(([name, range]: [any, any]) => {
    invariant(typeof range === 'string');
    return {
      name,
      range,
    };
  });
}

export async function getConflictingLocalDependencies(
  fs: FileSystem,
  name: string,
  local: FilePath,
  projectRoot: FilePath,
): Promise<
  | {
      json: string;
      filePath: FilePath;
      fields: Array<string>;
    }
  | null
  | undefined
> {
  let pkgPath = await resolveConfig(fs, local, ['package.json'], projectRoot);
  if (pkgPath == null) {
    return;
  }

  let pkgStr = await fs.readFile(pkgPath, 'utf8');
  let pkg;
  try {
    pkg = JSON.parse(pkgStr);
  } catch (e: any) {
    // TODO: codeframe
    throw new ThrowableDiagnostic({
      diagnostic: {
        message: 'Failed to parse package.json',
        origin: '@atlaspack/package-manager',
      },
    });
  }

  if (typeof pkg !== 'object' || pkg == null) {
    // TODO: codeframe
    throw new ThrowableDiagnostic({
      diagnostic: {
        message: 'Expected package.json contents to be an object.',
        origin: '@atlaspack/package-manager',
      },
    });
  }

  let fields: Array<string> = [];
  for (let field of ['dependencies', 'devDependencies', 'peerDependencies']) {
    if (
      typeof pkg[field] === 'object' &&
      pkg[field] != null &&
      pkg[field][name] != null
    ) {
      fields.push(field);
    }
  }

  if (fields.length > 0) {
    return {
      filePath: pkgPath,
      json: pkgStr,
      fields,
    };
  }
}