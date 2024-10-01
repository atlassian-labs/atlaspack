import type {PackageInstaller, InstallerOptions} from '@atlaspack/types';

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

const YARN_CMD = 'yarn';

type YarnStdOutMessage =
  | {
      readonly type: 'step';
      data: {
        message: string;
        current: number;
        total: number;
      };
    }
  | {
      readonly type: 'success';
      data: string;
    }
  | {
      readonly type: 'info';
      data: string;
    }
  | {
      readonly type: 'tree' | 'progressStart' | 'progressTick';
    };

type YarnStdErrMessage = {
  readonly type: 'error' | 'warning';
  data: string;
};

let hasYarn: boolean | null | undefined;
let yarnVersion: number | null | undefined;

export class Yarn implements PackageInstaller {
  static async exists(): Promise<boolean> {
    if (hasYarn != null) {
      return hasYarn;
    }

    try {
      hasYarn = Boolean(await commandExists('yarn'));
    } catch (err: any) {
      hasYarn = false;
    }

    return hasYarn;
  }

  async install({
    modules,
    cwd,
    saveDev = true,
  }: InstallerOptions): Promise<void> {
    if (yarnVersion == null) {
      let version = await exec('yarn --version');
      // @ts-expect-error - TS2345 - Argument of type 'string | Buffer' is not assignable to parameter of type 'string'.
      yarnVersion = parseInt(version.stdout, 10);
    }

    let args = ['add', '--json'].concat(
      modules.map(npmSpecifierFromModuleRequest),
    );

    if (saveDev) {
      args.push('-D');
      if (yarnVersion < 2) {
        args.push('-W');
      }
    }

    // When Parcel is run by Yarn (e.g. via package.json scripts), several environment variables are
    // added. When parcel in turn calls Yarn again, these can cause Yarn to behave stragely, so we
    // filter them out when installing packages.
    let env: Record<string, any> = {};
    for (let key in process.env) {
      if (
        !key.startsWith('npm_') &&
        key !== 'YARN_WRAP_OUTPUT' &&
        key !== 'INIT_CWD' &&
        key !== 'NODE_ENV'
      ) {
        env[key] = process.env[key];
      }
    }

    let installProcess = spawn(YARN_CMD, args, {cwd, env});
    installProcess.stdout
      // Invoking yarn with --json provides streaming, newline-delimited JSON output.
      .pipe(split())
      // @ts-expect-error - TS2554 - Expected 1 arguments, but got 0.
      .pipe(new JSONParseStream())
      // @ts-expect-error - TS7006 - Parameter 'e' implicitly has an 'any' type.
      .on('error', (e) => {
        logger.error(e, '@atlaspack/package-manager');
      })
      .on('data', (message: YarnStdOutMessage) => {
        switch (message.type) {
          case 'step':
            logger.progress(
              prefix(
                `[${message.data.current}/${message.data.total}] ${message.data.message}`,
              ),
            );
            return;
          case 'success':
          case 'info':
            logger.info({
              origin: '@atlaspack/package-manager',
              message: prefix(message.data),
            });
            return;
          default:
          // ignore
        }
      });

    installProcess.stderr
      .pipe(split())
      // @ts-expect-error - TS2554 - Expected 1 arguments, but got 0.
      .pipe(new JSONParseStream())
      // @ts-expect-error - TS7006 - Parameter 'e' implicitly has an 'any' type.
      .on('error', (e) => {
        logger.error(e, '@atlaspack/package-manager');
      })
      .on('data', (message: YarnStdErrMessage) => {
        switch (message.type) {
          case 'warning':
            logger.warn({
              origin: '@atlaspack/package-manager',
              message: prefix(message.data),
            });
            return;
          case 'error':
            logger.error({
              origin: '@atlaspack/package-manager',
              message: prefix(message.data),
            });
            return;
          default:
          // ignore
        }
      });

    try {
      return await promiseFromProcess(installProcess);
    } catch (e: any) {
      throw new Error('Yarn failed to install modules:' + e.message);
    }
  }
}

function prefix(message: string): string {
  return 'yarn: ' + message;
}

registerSerializableClass(`${pkg.version}:Yarn`, Yarn);
