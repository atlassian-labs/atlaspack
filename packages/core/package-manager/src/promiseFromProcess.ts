import type {ChildProcess} from 'child_process';

export default function promiseFromProcess(childProcess: ChildProcess): Promise<void> {
  return new Promise((resolve: (result: Promise<undefined> | undefined) => void, reject: (error?: any) => void) => {
    childProcess.on('error', reject);
    childProcess.on('close', (code) => {
      if (code !== 0) {
        reject(new Error('Child process failed'));
        return;
      }

      resolve();
    });
  });
}
