import * as child_process from 'node:child_process';

export function spawn(
  cmd: string,
  args?: Array<string>,
  options?: child_process.SpawnOptionsWithoutStdio,
): Promise<string> {
  return new Promise<string>((resolve, reject) => {
    const cp = child_process.spawn(cmd, args, options);
    const error: string[] = [];
    const stdout: string[] = [];

    cp.stdout.on('data', (data: any) => {
      stdout.push(`${data}`);
    });

    cp.on('error', (e) => {
      error.push(e.toString());
    });

    cp.on('close', () => {
      if (error.length) reject(error.join(''));
      else resolve(stdout.join(''));
    });
  });
}
