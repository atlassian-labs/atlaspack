import type {PackageInstaller, InstallerOptions} from '@atlaspack/types';

import path from 'path';
import fs from 'fs';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'command-exists'. '/home/ubuntu/parcel/node_modules/command-exists/index.js' implicitly has an 'any' type.
import commandExists from 'command-exists';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'cross-spawn'. '/home/ubuntu/parcel/packages/core/package-manager/node_modules/cross-spawn/index.js' implicitly has an 'any' type.
import spawn from 'cross-spawn';
import logger from '@atlaspack/logger';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'split2'. '/home/ubuntu/parcel/node_modules/split2/index.js' implicitly has an 'any' type.
import split from 'split2';
import JSONParseStream from './JSONParseStream';
import promiseFromProcess from './promiseFromProcess';
import {registerSerializableClass} from '@atlaspack/core';
import {exec, npmSpecifierFromModuleRequest} from './utils';

// @ts-expect-error - TS2732 - Cannot find module '../package.json'. Consider using '--resolveJsonModule' to import module with '.json' extension.
import pkg from '../package.json';

const PNPM_CMD = 'pnpm';

type LogLevel = 'error' | 'warn' | 'info' | 'debug';

type ErrorLog = {
  err: {
    message: string;
    code: string;
    stack: string;
  };
};

type PNPMLog =
  | {
      readonly name: 'pnpm:progress';
      packageId: string;
      status: 'fetched' | 'found_in_store' | 'resolved';
    }
  | {
      readonly name: 'pnpm:root';
      added?: {
        id?: string;
        name: string;
        realName: string;
        version?: string;
        dependencyType?: 'prod' | 'dev' | 'optional';
        latest?: string;
        linkedFrom?: string;
      };
      removed?: {
        name: string;
        version?: string;
        dependencyType?: 'prod' | 'dev' | 'optional';
      };
    }
  | {
      readonly name: 'pnpm:importing';
      from: string;
      method: string;
      to: string;
    }
  | {
      readonly name: 'pnpm:link';
      target: string;
      link: string;
    }
  | {
      readonly name: 'pnpm:stats';
      prefix: string;
      removed?: number;
      added?: number;
    };

type PNPMResults = {
  level: LogLevel;
  prefix?: string;
  message?: string;
} & ErrorLog &
  PNPMLog;

let hasPnpm: boolean | null | undefined;
let pnpmVersion: number | null | undefined;

export class Pnpm implements PackageInstaller {
  static async exists(): Promise<boolean> {
    if (hasPnpm != null) {
      return hasPnpm;
    }

    try {
      hasPnpm = Boolean(await commandExists('pnpm'));
    } catch (err: any) {
      hasPnpm = false;
    }

    return hasPnpm;
  }

  async install({
    modules,
    cwd,
    saveDev = true,
  }: InstallerOptions): Promise<void> {
    if (pnpmVersion == null) {
      let version = await exec('pnpm --version');
      // @ts-expect-error - TS2345 - Argument of type 'string | Buffer' is not assignable to parameter of type 'string'.
      pnpmVersion = parseInt(version.stdout, 10);
    }

    let args = ['add', '--reporter', 'ndjson'];
    if (saveDev) {
      args.push('-D');
    }
    if (pnpmVersion >= 7) {
      if (fs.existsSync(path.join(cwd, 'pnpm-workspace.yaml'))) {
        // installs in workspace root (regardless of cwd)
        args.push('-w');
      }
    } else {
      // ignores workspace root check
      args.push('-W');
    }
    args = args.concat(modules.map(npmSpecifierFromModuleRequest));

    let env: Record<string, any> = {};
    for (let key in process.env) {
      if (!key.startsWith('npm_') && key !== 'INIT_CWD' && key !== 'NODE_ENV') {
        env[key] = process.env[key];
      }
    }

    let addedCount = 0,
      removedCount = 0;

    let installProcess = spawn(PNPM_CMD, args, {
      cwd,
      env,
    });
    installProcess.stdout
      .pipe(split())
      // @ts-expect-error - TS2554 - Expected 1 arguments, but got 0.
      .pipe(new JSONParseStream())
      // @ts-expect-error - TS7006 - Parameter 'e' implicitly has an 'any' type.
      .on('error', (e) => {
        logger.warn({
          origin: '@atlaspack/package-manager',
          message: e.chunk,
          stack: e.stack,
        });
      })
      .on('data', (json: PNPMResults) => {
        if (json.level === 'error') {
          logger.error({
            origin: '@atlaspack/package-manager',
            message: json.err.message,
            stack: json.err.stack,
          });
        } else if (json.level === 'info' && typeof json.message === 'string') {
          logger.info({
            origin: '@atlaspack/package-manager',
            message: prefix(json.message),
          });
        } else if (json.name === 'pnpm:stats') {
          addedCount += json.added ?? 0;
          removedCount += json.removed ?? 0;
        }
      });

    let stderr: Array<any> = [];
    installProcess.stderr
      // @ts-expect-error - TS7006 - Parameter 'str' implicitly has an 'any' type.
      .on('data', (str) => {
        stderr.push(str.toString());
      })
      // @ts-expect-error - TS7006 - Parameter 'e' implicitly has an 'any' type.
      .on('error', (e) => {
        logger.warn({
          origin: '@atlaspack/package-manager',
          message: e.message,
        });
      });

    try {
      await promiseFromProcess(installProcess);

      if (addedCount > 0 || removedCount > 0) {
        logger.log({
          origin: '@atlaspack/package-manager',
          message: `Added ${addedCount} ${
            removedCount > 0 ? `and removed ${removedCount} ` : ''
          }packages via pnpm`,
        });
      }

      // Since we succeeded, stderr might have useful information not included
      // in the json written to stdout. It's also not necessary to log these as
      // errors as they often aren't.
      for (let message of stderr) {
        logger.log({
          origin: '@atlaspack/package-manager',
          message,
        });
      }
    } catch (e: any) {
      throw new Error('pnpm failed to install modules');
    }
  }
}

function prefix(message: string): string {
  return 'pnpm: ' + message;
}

registerSerializableClass(`${pkg.version}:Pnpm`, Pnpm);
