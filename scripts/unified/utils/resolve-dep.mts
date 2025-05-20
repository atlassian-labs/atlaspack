import {spawn} from './spawn.mts';

export async function resolveDependencySlow(
  specifier: string,
  cwd: string,
): Promise<string> {
  return await spawn(
    'node',
    ['-e', `process.stdout.write(require.resolve("${specifier}"))`],
    {
      cwd,
    },
  );
}
