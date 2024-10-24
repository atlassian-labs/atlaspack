// @flow
import type {ReporterEvent, PluginOptions} from '@atlaspack/types';
import type {Diagnostic} from '@atlaspack/diagnostic';
import type {Color} from 'chalk';

import {Reporter} from '@atlaspack/plugin';
import {
  getProgressMessage,
  prettifyTime,
  prettyDiagnostic,
  throttle,
} from '@atlaspack/utils';
import chalk from 'chalk';

import {getTerminalWidth} from './utils';
import logLevels from './logLevels';
import bundleReport from './bundleReport';
import phaseReport from './phaseReport';
import {
  writeOut,
  persistSpinner,
  isTTY,
  resetWindow,
  persistMessage,
} from './render';
import * as emoji from './emoji';
import wrapAnsi from 'wrap-ansi';

const updateSpinner = (msg: string) => {
  console.log(new Date().toISOString(), msg);
};

const THROTTLE_DELAY = 100;
const seenWarnings = new Set();
const seenPhases = new Set();
const seenPhasesGen = new Set();

let phaseStartTimes = {};
let pendingIncrementalBuild = false;

let statusThrottle = throttle((message: string) => {
  updateSpinner(message);
}, THROTTLE_DELAY);

class RateOfWorkTracker {
  currentWindow = [];

  addSample(sample: {|date: number, total: number, running: number|}) {
    this.currentWindow.push(sample);
    if (this.currentWindow.length > 10) {
      this.currentWindow.shift();
    }
  }

  getRateOfWork() {
    // the slope of the "total" and "running" values over time is highly
    // variable, so we want to use the rate of change of the rate of change
    // to smooth out the data
    let totalRate = 0;
    let runningRate = 0;
    for (let i = 1; i < this.currentWindow.length; i++) {
      totalRate +=
        this.currentWindow[i].total - this.currentWindow[i - 1].total;
      runningRate +=
        this.currentWindow[i].running - this.currentWindow[i - 1].running;
    }

    return {
      totalRate,
      runningRate,
    };
  }
}

class CLIReporterImpl {
  pending = false;
  phaseCounts = {};
  lastQueueStatistics = {
    total: 0,
    running: 0,
    date: Date.now(),
  };
  requestStats = {
    requests: 0,
    cacheHits: 0,
  };
  cacheWriteStart = 0;

  constructor() {}

  start() {
    setInterval(() => {
      if (this.pending) {
        this.writeToConsole();
      }
    }, 1000);

    let lastQueueStatistics = [
      {
        ...this.lastQueueStatistics,
        discoveryRate: 0,
        completedRate: 0,
        discoveryRateSlope: 0,
      },
    ];
    setInterval(() => {
      const {total, running, date} = this.lastQueueStatistics;
      const elapsed =
        date - lastQueueStatistics[lastQueueStatistics.length - 1].date;

      // total is the total number of jobs queued and completed, so calculate the
      // rate in which jobs are added to the pendign queue (by looking at the running size)
      // and the rate at which they are completed
      const discoveryRate =
        (total - lastQueueStatistics[lastQueueStatistics.length - 1].total) /
        (elapsed / 1000);
      const completedJobs = total - running;
      const lastCompletedJobs =
        lastQueueStatistics[lastQueueStatistics.length - 1].total -
        lastQueueStatistics[lastQueueStatistics.length - 1].running;
      const completedRate =
        (completedJobs - lastCompletedJobs) / (elapsed / 1000);
      const lastRunning = lastQueueStatistics[0].running;
      const runningRate =
        (running - lastRunning) / ((date - lastQueueStatistics[0].date) / 1000);

      const discoveryRateSlope =
        discoveryRate -
        lastQueueStatistics[lastQueueStatistics.length - 1].discoveryRate;

      const timeToCompleteBasedOnRunningShift = running / runningRate;
      const timeToComplete = -timeToCompleteBasedOnRunningShift;

      console.log(
        new Date().toISOString(),
        'Time to complete',
        timeToComplete > 0 ? timeToComplete : 'N/A',
        'Discovered jobs growing at',
        discoveryRate,
        'jobs/sec',
        'completing at',
        completedRate,
        'jobs/sec',
        'total jobs running',
        running,
        'total jobs completed',
        completedJobs,
        'total jobs',
        total,
      );

      lastQueueStatistics.push({
        total,
        running,
        date,
        completedRate,
        discoveryRate,
        discoveryRateSlope,
      });
      if (lastQueueStatistics.length > 100) {
        lastQueueStatistics.shift();
      }
    }, 1000);
  }

  onEvent(event: ReporterEvent) {
    this.pending = true;

    if (event.type === 'buildProgress') {
      this.phaseCounts[event.phase] ??= 0;
      this.phaseCounts[event.phase] += 1;
      if (event.phase !== 'resolving' && event.phase !== 'transforming') {
        console.log(new Date().toISOString(), event.phase, event);
      }
    } else if (event.type === 'log') {
      event.diagnostics?.forEach((diagnostic) => {
        console.log(
          new Date().toISOString(),
          event.level,
          diagnostic.origin,
          diagnostic.message,
          diagnostic.meta,
        );
      });
    } else if (event.type === 'assetGraphQueueEvent') {
      this.lastQueueStatistics = {
        total: event.total,
        running: event.running,
        date: Date.now(),
      };
    } else if (event.type === 'requestTrackerEvent') {
      this.requestStats.requests += 1;
      this.requestStats.cacheHits += event.cacheHit ? 1 : 0;
    } else if (event.type === 'cache') {
      if (event.phase === 'start') {
        this.cacheWriteStart = Date.now();
        console.log(new Date().toISOString(), 'Cache write start');
      } else if (event.phase === 'end') {
        console.log(
          new Date().toISOString(),
          'Cache write end',
          Date.now() - this.cacheWriteStart,
        );
      }
    } else {
      console.log(event);
    }
  }

  writeToConsole() {
    console.log(new Date().toISOString(), 'Building...', {
      ...this.phaseCounts,
      ...this.requestStats,
    });
  }
}

const cliReporter = new CLIReporterImpl();
// cliReporter.start();

// Exported only for test
export async function _report(
  event: ReporterEvent,
  options: PluginOptions,
): Promise<void> {
  let logLevelFilter = logLevels[options.logLevel || 'info'];

  cliReporter.onEvent(event);

  switch (event.type) {
    case 'buildStart': {
      seenWarnings.clear();
      seenPhases.clear();
      if (logLevelFilter < logLevels.info) {
        break;
      }

      // Clear any previous output
      resetWindow();

      if (options.serveOptions) {
        persistMessage(
          chalk.blue.bold(
            `Server running at ${
              options.serveOptions.https ? 'https' : 'http'
            }://${options.serveOptions.host ?? 'localhost'}:${
              options.serveOptions.port
            }`,
          ),
        );
      }

      break;
    }
    case 'buildProgress': {
      if (logLevelFilter < logLevels.info) {
        break;
      }

      if (pendingIncrementalBuild) {
        pendingIncrementalBuild = false;
        phaseStartTimes = {};
        seenPhasesGen.clear();
        seenPhases.clear();
      }

      if (!seenPhasesGen.has(event.phase)) {
        phaseStartTimes[event.phase] = Date.now();
        seenPhasesGen.add(event.phase);
      }

      if (!isTTY && logLevelFilter != logLevels.verbose) {
        if (event.phase == 'transforming' && !seenPhases.has('transforming')) {
          updateSpinner('Building...');
        } else if (event.phase == 'bundling' && !seenPhases.has('bundling')) {
          updateSpinner('Bundling...');
        } else if (
          (event.phase == 'packaging' || event.phase == 'optimizing') &&
          !seenPhases.has('packaging') &&
          !seenPhases.has('optimizing')
        ) {
          updateSpinner('Packaging & Optimizing...');
        }
        seenPhases.add(event.phase);
        break;
      }

      let message = getProgressMessage(event);
      if (message != null) {
        if (isTTY) {
          statusThrottle(chalk.gray.bold(message));
        } else {
          updateSpinner(message);
        }
      }
      break;
    }
    case 'buildSuccess':
      if (logLevelFilter < logLevels.info) {
        break;
      }

      phaseStartTimes['buildSuccess'] = Date.now();

      persistSpinner(
        'buildProgress',
        'success',
        chalk.green.bold(`Built in ${prettifyTime(event.buildTime)}`),
      );

      if (options.mode === 'production') {
        await bundleReport(
          event.bundleGraph,
          options.outputFS,
          options.projectRoot,
          options.detailedReport?.assetsPerBundle,
        );
      } else {
        pendingIncrementalBuild = true;
      }

      if (process.env.ATLASPACK_SHOW_PHASE_TIMES) {
        phaseReport(phaseStartTimes);
      }
      break;
    case 'buildFailure':
      if (logLevelFilter < logLevels.error) {
        break;
      }

      resetWindow();

      persistSpinner('buildProgress', 'error', chalk.red.bold('Build failed.'));

      await writeDiagnostic(options, event.diagnostics, 'red', true);
      break;
    case 'cache':
      if (event.size > 500000) {
        switch (event.phase) {
          case 'start':
            updateSpinner('Writing cache to disk');
            break;
          case 'end':
            persistSpinner(
              'cache',
              'success',
              chalk.grey.bold(`Cache written to disk`),
            );
            break;
        }
      }
      break;
    case 'log': {
      if (logLevelFilter < logLevels[event.level]) {
        break;
      }

      switch (event.level) {
        case 'success':
          writeOut(chalk.green(event.message));
          break;
        case 'progress':
          writeOut(event.message);
          break;
        case 'verbose':
        case 'info':
          await writeDiagnostic(options, event.diagnostics, 'blue');
          break;
        case 'warn':
          if (
            event.diagnostics.some(
              (diagnostic) => !seenWarnings.has(diagnostic.message),
            )
          ) {
            await writeDiagnostic(options, event.diagnostics, 'yellow', true);
            for (let diagnostic of event.diagnostics) {
              seenWarnings.add(diagnostic.message);
            }
          }
          break;
        case 'error':
          await writeDiagnostic(options, event.diagnostics, 'red', true);
          break;
        default:
          throw new Error('Unknown log level ' + event.level);
      }
    }
  }
}

async function writeDiagnostic(
  options: PluginOptions,
  diagnostics: Array<Diagnostic>,
  color: Color,
  isError: boolean = false,
) {
  let columns = getTerminalWidth().columns;
  let indent = 2;
  let spaceAfter = isError;
  for (let diagnostic of diagnostics) {
    let {message, stack, codeframe, hints, documentation} =
      await prettyDiagnostic(diagnostic, options, columns - indent);
    // $FlowFixMe[incompatible-use]
    message = chalk[color](message);

    if (spaceAfter) {
      writeOut('');
    }

    if (message) {
      writeOut(wrapWithIndent(message), isError);
    }

    if (stack || codeframe) {
      writeOut('');
    }

    if (stack) {
      writeOut(chalk.gray(wrapWithIndent(stack, indent)), isError);
    }

    if (codeframe) {
      writeOut(indentString(codeframe, indent), isError);
    }

    if ((stack || codeframe) && (hints.length > 0 || documentation)) {
      writeOut('');
    }

    // Write hints
    let hintIndent = stack || codeframe ? indent : 0;
    for (let hint of hints) {
      writeOut(
        wrapWithIndent(
          `${emoji.hint} ${chalk.blue.bold(hint)}`,
          hintIndent + 3,
          hintIndent,
        ),
      );
    }

    if (documentation) {
      writeOut(
        wrapWithIndent(
          `${emoji.docs} ${chalk.magenta.bold(documentation)}`,
          hintIndent + 3,
          hintIndent,
        ),
      );
    }

    spaceAfter = stack || codeframe || hints.length > 0 || documentation;
  }

  if (spaceAfter) {
    writeOut('');
  }
}

function wrapWithIndent(string, indent = 0, initialIndent = indent) {
  let width = getTerminalWidth().columns;
  return indentString(
    wrapAnsi(string.trimEnd(), width - indent, {trim: false}),
    indent,
    initialIndent,
  );
}

function indentString(string, indent = 0, initialIndent = indent) {
  return (
    ' '.repeat(initialIndent) + string.replace(/\n/g, '\n' + ' '.repeat(indent))
  );
}

export default (new Reporter({
  report({event, options}) {
    return _report(event, options);
  },
}): Reporter);
