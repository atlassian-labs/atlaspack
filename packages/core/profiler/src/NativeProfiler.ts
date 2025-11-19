import {getTimeId} from '@atlaspack/utils';
import logger from '@atlaspack/logger';
import readline from 'readline';
import chalk from 'chalk';
import {exec} from 'child_process';
import {promisify} from 'util';

const execAsync = promisify(exec);

export type NativeProfilerType = 'instruments' | 'samply';

export default class NativeProfiler {
  startProfiling(profilerType: NativeProfilerType): Promise<void> {
    const pid = process.pid;
    const timeId = getTimeId();

    let filename: string;
    let command: string;

    logger.info({
      origin: '@atlaspack/profiler',
      message: 'Starting native profiling...',
    });

    if (profilerType === 'instruments') {
      filename = `native-profile-${timeId}.trace`;
      command = `xcrun xctrace record --template "CPU Profiler" --output ${filename} --attach ${pid}`;
    } else {
      filename = `native-profile-${timeId}.json`;
      command = `samply record --save-only --output ${filename} --pid ${pid}`;
    }

    // Display banner with PID and command
    // Strip ANSI codes for length calculation
    // eslint-disable-next-line no-control-regex
    const stripAnsi = (str: string) => str.replace(/\u001b\[[0-9;]*m/g, '');
    const boxWidth = Math.max(60, stripAnsi(command).length + 6);
    const title = 'Native Profiling';
    const titlePadding = Math.floor((boxWidth - title.length - 2) / 2);

    const padLine = (content: string) => {
      const contentLength = stripAnsi(content).length;
      const padding = Math.max(0, boxWidth - contentLength - 2);
      return (
        chalk.blue('│') +
        ' ' +
        content +
        ' '.repeat(padding) +
        ' ' +
        chalk.blue('│')
      );
    };

    // Make the command visually distinct and easy to copy
    // Note: Hyperlinks can cause issues with commands (words become separate links)
    // So we just make it visually prominent with colors
    const makeCommandDisplay = (cmd: string) => {
      return chalk.cyan.bold(cmd);
    };

    const banner = [
      '',
      chalk.blue('┌' + '─'.repeat(boxWidth) + '┐'),
      chalk.blue('│') +
        ' '.repeat(titlePadding) +
        chalk.blue.bold(title) +
        ' '.repeat(boxWidth - title.length - titlePadding) +
        chalk.blue('│'),
      chalk.blue('├' + '─'.repeat(boxWidth) + '┤'),
      padLine(`${chalk.gray('PID:')} ${chalk.white.bold(String(pid))}`),
      padLine(''),
      padLine(chalk.gray('Command:')),
      padLine(makeCommandDisplay(command)),
      padLine(''),
      padLine(chalk.gray('Run the command above to start profiling.')),
      padLine(chalk.gray('Press Enter to continue...')),
      chalk.blue('└' + '─'.repeat(boxWidth) + '┘'),
      '',
    ].join('\n');

    // eslint-disable-next-line no-console
    console.log(banner);

    // In both interactive and non-interactive mode, detect when profiler is running
    // In interactive mode, also allow user to press Enter to continue
    if (!process.stdin.isTTY) {
      return this.waitForProfiler(profilerType, pid);
    }

    // Interactive mode: wait for either user to press Enter OR profiler to be detected
    return new Promise<void>((resolve) => {
      let resolved = false;
      const doResolve = () => {
        if (resolved) return;
        resolved = true;
        logger.info({
          origin: '@atlaspack/profiler',
          message: 'Native profiling setup complete',
        });
        resolve();
      };

      const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout,
      });

      // User presses Enter
      rl.on('line', () => {
        rl.close();
        doResolve();
      });

      // Also poll for profiler in the background
      this.pollForProfiler(profilerType, pid, doResolve);
    });
  }

  private waitForProfiler(
    profilerType: NativeProfilerType,
    pid: number,
  ): Promise<void> {
    logger.info({
      origin: '@atlaspack/profiler',
      message: 'Non-interactive mode: waiting for profiler to attach...',
    });

    return new Promise<void>((resolve) => {
      this.pollForProfiler(profilerType, pid, () => {
        logger.info({
          origin: '@atlaspack/profiler',
          message: 'Native profiling setup complete',
        });
        resolve();
      });
    });
  }

  private async pollForProfiler(
    profilerType: NativeProfilerType,
    pid: number,
    onDetected: () => void,
  ): Promise<void> {
    const maxAttempts = 60; // 60 attempts * 500ms = 30 seconds max
    const pollInterval = 500; // 500ms between checks

    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      const isRunning = await this.checkProfilerRunning(profilerType, pid);

      if (isRunning) {
        logger.info({
          origin: '@atlaspack/profiler',
          message: 'Profiler detected, waiting 500ms before continuing...',
        });
        await new Promise((resolve) => setTimeout(resolve, 500));
        onDetected();
        return;
      }

      await new Promise((resolve) => setTimeout(resolve, pollInterval));
    }

    // If we couldn't detect the profiler after 30 seconds, log a warning and continue anyway
    logger.warn({
      origin: '@atlaspack/profiler',
      message:
        'Could not detect profiler after 30 seconds, continuing anyway...',
    });
    onDetected();
  }

  private async checkProfilerRunning(
    profilerType: NativeProfilerType,
    pid: number,
  ): Promise<boolean> {
    try {
      // Get all processes and filter in JavaScript
      const {stdout} = await execAsync('ps aux');
      const lines = stdout.split('\n').filter((line) => line.trim().length > 0);

      // Use word boundaries to match the PID as a complete number
      const pidRegex = new RegExp(`\\b${pid}\\b`);

      // Determine the profiler process name to look for
      const profilerName =
        profilerType === 'instruments' ? 'xctrace' : 'samply';

      for (const line of lines) {
        const lowerLine = line.toLowerCase();

        // Check if this line contains both the profiler name and our PID
        if (lowerLine.includes(profilerName) && pidRegex.test(line)) {
          return true;
        }
      }

      return false;
    } catch (error: any) {
      // If the command fails, log and return false
      logger.warn({
        origin: '@atlaspack/profiler',
        message: `Error checking profiler status: ${error.message}`,
      });
      return false;
    }
  }
}
