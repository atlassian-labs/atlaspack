import type {FileSystem} from '@atlaspack/fs';
import type {EnvMap, FilePath} from '@atlaspack/types';

import {resolveConfig} from '@atlaspack/utils';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'dotenv'. '/home/ubuntu/parcel/node_modules/dotenv/lib/main.js' implicitly has an 'any' type.
import dotenv from 'dotenv';
import variableExpansion from 'dotenv-expand';

export default async function loadEnv(
  env: EnvMap,
  fs: FileSystem,
  filePath: FilePath,
  projectRoot: FilePath,
): Promise<EnvMap> {
  const NODE_ENV = env.NODE_ENV ?? 'development';

  const dotenvFiles = [
    '.env',
    // Don't include `.env.local` for `test` environment
    // since normally you expect tests to produce the same
    // results for everyone
    NODE_ENV === 'test' ? null : '.env.local',
    `.env.${NODE_ENV}`,
    `.env.${NODE_ENV}.local`,
  ].filter(Boolean);

  let envs = await Promise.all(
    dotenvFiles.map(async (dotenvFile) => {
      const envPath = await resolveConfig(
        fs,
        filePath,
        // @ts-expect-error - TS2322 - Type 'string | null' is not assignable to type 'string'.
        [dotenvFile],
        projectRoot,
      );
      if (envPath == null) {
        return;
      }

      // `ignoreProcessEnv` prevents dotenv-expand from writing values into `process.env`:
      // https://github.com/motdotla/dotenv-expand/blob/ddb73d02322fe8522b4e05b73e1c1ad24ea7c14a/lib/main.js#L5
      let output = variableExpansion({
        parsed: dotenv.parse(await fs.readFile(envPath)),
        // @ts-expect-error - TS2345 - Argument of type '{ parsed: any; ignoreProcessEnv: boolean; }' is not assignable to parameter of type 'DotenvResult'.
        ignoreProcessEnv: true,
      });

      if (output.error != null) {
        throw output.error;
      }

      return output.parsed;
    }),
  );

  return Object.assign({}, ...envs);
}
