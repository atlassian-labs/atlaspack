import os from 'os';
import {execSync} from 'child_process';

declare const navigator: {hardwareConcurrency: number} | undefined;

const exec = (command: string): string => {
  try {
    let stdout = execSync(command, {
      encoding: 'utf8',
      // This prevents the command from outputting to the console
      stdio: [null, null, null],
    });
    return stdout.trim();
  } catch (e: any) {
    return '';
  }
};

export function detectRealCores(): number {
  let platform = os.platform();
  let amount = 0;

  if (platform === 'linux') {
    amount = parseInt(
      exec('lscpu -p | egrep -v "^#" | sort -u -t, -k 2,4 | wc -l'),
      10,
    );
  } else if (platform === 'darwin') {
    amount = parseInt(exec('sysctl -n hw.physicalcpu_max'), 10);
  } else if (platform === 'win32') {
    const str = exec('wmic cpu get NumberOfCores').match(/\d+/g);
    if (str !== null) {
      amount = parseInt(str.filter((n) => n !== '')[0], 10);
    }
  }

  if (!amount || amount <= 0) {
    throw new Error('Could not detect cpu count!');
  }

  return amount;
}

// @ts-expect-error TS7034
let cores;
export default function getCores(bypassCache: boolean = false): number {
  // Do not re-run commands if we already have the count...
  // @ts-expect-error TS7005
  if (cores && !bypassCache) {
    return cores;
  }

  // @ts-expect-error TS2339
  if (process.browser) {
    cores = (navigator as any).hardwareConcurrency / 2;
  }

  // @ts-expect-error TS7005
  if (!cores) {
    try {
      cores = detectRealCores();
    } catch (e: any) {
      // Guess the amount of real cores
      cores = os
        .cpus()
        .filter(
          (cpu, index) => !cpu.model.includes('Intel') || index % 2 === 1,
        ).length;
    }
  }

  // Another fallback
  if (!cores) {
    cores = 1;
  }

  return cores;
}
