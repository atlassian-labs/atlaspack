// @flow strict-local
import {join} from 'path';
import type {PluginOptions} from '@atlaspack/types-internal';

type Config = {|
  filename: string,
|};

export async function getConfig({
  env,
  inputFS,
  projectRoot,
}: PluginOptions): Promise<Config> {
  const packageJson = JSON.parse(
    await inputFS.readFile(join(projectRoot, 'package.json'), 'utf8'),
  );

  const config = packageJson['@atlaspack/reporter-conditional-manifest'] ?? {};
  for (const [key, value] of Object.entries(config)) {
    // Replace values in the format of ${VARIABLE} with their corresponding env
    if (typeof value === 'string') {
      config[key] = value.replace(/\${([^}]+)}/g, (_, v) => env[v] ?? '');
    }
  }

  const {filename} = config;

  return {
    filename: filename ?? 'conditional-manifest.json',
  };
}
