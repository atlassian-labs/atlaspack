// @flow
import type {ReporterEvent, PluginOptions} from '@atlaspack/types';
import type {Diagnostic} from '@atlaspack/diagnostic';
import type {Color} from 'chalk';

import {Reporter} from '@atlaspack/plugin';
import {
  getProgressMessage,
  getPackageProgressMessage,
  prettifyTime,
  prettyDiagnostic,
  throttle,
  debugTools,
} from '@atlaspack/utils';
import chalk from 'chalk';

import {getTerminalWidth} from './utils';
import logLevels from './logLevels';
import bundleReport from './bundleReport';
import phaseReport from './phaseReport';
import {
  writeOut,
  updateSpinner,
  persistSpinner,
  isTTY,
  resetWindow,
  persistMessage,
} from './render';
import * as emoji from './emoji';
import wrapAnsi from 'wrap-ansi';

const THROTTLE_DELAY = 100;
const seenWarnings = new Set();
const seenPhases = new Set();
const seenPhasesGen = new Set();

let phaseStartTimes = {};
let pendingIncrementalBuild = false;
let packagingProgress = 0;

let updatePackageProgress = (completeBundles: number, totalBundles: number) => {
  let updateThreshold = 0;
  if (totalBundles > 5000) {
    // If more than 5000 bundles, update every 5%
    updateThreshold = 5;
  } else if (totalBundles > 1000) {
    // If more than 1000 bundles, update every 10%
    updateThreshold = 10;
  } else {
    // othewise update every 25%
    updateThreshold = 25;
  }

  let percent = Math.floor((completeBundles / totalBundles) * 100);
  if (percent - packagingProgress >= updateThreshold) {
    packagingProgress = percent;
    updateSpinner(getPackageProgressMessage(completeBundles, totalBundles));
  }
};

let statusThrottle = throttle((message: string) => {
  updateSpinner(message);
}, THROTTLE_DELAY);

const cacheWriteState: {|
  startTime: number | null,
|} = {
  startTime: null,
};

// Exported only for test
export async function _report(
  event: ReporterEvent,
  options: PluginOptions,
): Promise<void> {
  let logLevelFilter = logLevels[options.logLevel || 'info'];

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
        } else if (event.phase === 'packagingAndOptimizing') {
          updatePackageProgress(event.completeBundles, event.totalBundles);
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
        if (debugTools['simple-cli-reporter']) {
          writeOut(
            `🛠️ Built ${event.bundleGraph.getBundles().length} bundles.`,
          );
        } else {
          await bundleReport(
            event.bundleGraph,
            options.outputFS,
            options.projectRoot,
            options.detailedReport?.assetsPerBundle,
          );
        }
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
            cacheWriteState.startTime = Date.now();
            break;
          case 'end':
            persistSpinner(
              'cache',
              'success',
              chalk.grey.bold(
                `Cache written to disk in ${prettifyTime(
                  Date.now() - (cacheWriteState.startTime ?? 0),
                )}`,
              ),
            );

            cacheWriteState.startTime = null;
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
