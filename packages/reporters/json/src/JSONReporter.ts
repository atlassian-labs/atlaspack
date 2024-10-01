import type {BuildProgressEvent, LogEvent} from '@atlaspack/types';
import type {BuildMetrics} from '@atlaspack/utils';

import {Reporter} from '@atlaspack/plugin';
import {generateBuildMetrics} from '@atlaspack/utils';

/* eslint-disable no-console */
const writeToStdout = makeWriter(console.log);
const writeToStderr = makeWriter(console.error);
/* eslint-enable no-console */

const LOG_LEVELS = {
  none: 0,
  error: 1,
  warn: 2,
  info: 3,
  progress: 3,
  success: 3,
  verbose: 4,
} as const;

export default new Reporter({
  async report({event, options}) {
    let logLevelFilter = options.logLevel || 'info';

    switch (event.type) {
      case 'buildStart':
        if (LOG_LEVELS[logLevelFilter] >= LOG_LEVELS.info) {
          writeToStdout({type: 'buildStart'}, logLevelFilter);
        }
        break;
      case 'buildFailure':
        if (LOG_LEVELS[logLevelFilter] >= LOG_LEVELS.error) {
          writeToStderr(
            {type: 'buildFailure', message: event.diagnostics[0].message},
            logLevelFilter,
          );
        }
        break;
      case 'buildProgress':
        if (LOG_LEVELS[logLevelFilter] >= LOG_LEVELS.progress) {
          let jsonEvent = progressEventToJSONEvent(event);
          if (jsonEvent != null) {
            writeToStdout(jsonEvent, logLevelFilter);
          }
        }
        break;
      case 'buildSuccess':
        if (LOG_LEVELS[logLevelFilter] >= LOG_LEVELS.success) {
          let {bundles} = await generateBuildMetrics(
            event.bundleGraph.getBundles(),
            options.outputFS,
            options.projectRoot,
          );

          writeToStdout(
            {
              type: 'buildSuccess',
              buildTime: event.buildTime,
              bundles: bundles,
            },
            logLevelFilter,
          );
        }
        break;
      case 'log':
        writeLogEvent(event, logLevelFilter);
    }
  },
}) as Reporter;

function makeWriter(
  write: (arg1: string) => unknown,
): (arg1: JSONReportEvent, arg2: keyof typeof LOG_LEVELS) => void {
  return (
    event: JSONReportEvent,
    logLevelFilter: keyof typeof LOG_LEVELS,
  ): void => {
    let stringified;
    try {
      stringified = JSON.stringify(event);
    } catch (err: any) {
      // This should never happen so long as JSONReportEvent is easily serializable
      if (LOG_LEVELS[logLevelFilter] >= LOG_LEVELS.error) {
        writeToStderr(
          {
            type: 'log',
            level: 'error',
            diagnostics: [
              {
                origin: '@atlaspack/reporter-json',
                message: err.message,
                stack: err.stack,
              },
            ],
          },
          logLevelFilter,
        );
      }
      return;
    }

    write(stringified);
  };
}

function writeLogEvent(
  event: LogEvent,
  logLevelFilter: keyof typeof LOG_LEVELS,
): void {
  if (LOG_LEVELS[logLevelFilter] < LOG_LEVELS[event.level]) {
    return;
  }
  switch (event.level) {
    case 'info':
    case 'progress':
    case 'success':
    case 'verbose':
      writeToStdout(event, logLevelFilter);
      break;
    case 'warn':
    case 'error':
      writeToStderr(event, logLevelFilter);
      break;
  }
}

function progressEventToJSONEvent(
  progressEvent: BuildProgressEvent,
): JSONProgressEvent | null | undefined {
  switch (progressEvent.phase) {
    case 'transforming':
      return {
        type: 'buildProgress',
        phase: 'transforming',
        filePath: progressEvent.filePath,
      };
    case 'bundling':
      return {
        type: 'buildProgress',
        phase: 'bundling',
      };
    case 'optimizing':
    case 'packaging':
      return {
        type: 'buildProgress',
        phase: progressEvent.phase,
        bundleName: progressEvent.bundle.displayName,
      };
  }
}

type JSONReportEvent =
  | LogEvent
  | {
      readonly type: 'buildStart';
    }
  | {
      readonly type: 'buildFailure';
      message: string;
    }
  | {
      readonly type: 'buildSuccess';
      buildTime: number;
      bundles?: BuildMetrics['bundles'];
    }
  | JSONProgressEvent;

type JSONProgressEvent =
  | {
      readonly type: 'buildProgress';
      phase: 'transforming';
      filePath: string;
    }
  | {
      readonly type: 'buildProgress';
      phase: 'bundling';
    }
  | {
      readonly type: 'buildProgress';
      readonly phase: 'packaging' | 'optimizing';
      bundleName?: string;
    };
