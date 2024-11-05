// @flow

import {BuildError} from '@atlaspack/core';
import {NodeFS} from '@atlaspack/fs';
import {openInBrowser} from '@atlaspack/utils';
import {Disposable} from '@atlaspack/events';
import {INTERNAL_ORIGINAL_CONSOLE} from '@atlaspack/logger';
import chalk from 'chalk';
import commander from 'commander';
import path from 'path';
import {version} from '../package.json';
import {applyOptions} from './applyOptions';
import {makeDebugCommand} from './makeDebugCommand';
import {normalizeOptions} from './normalizeOptions';
import {
  handleUncaughtException,
  logUncaughtError,
} from './handleUncaughtException';
import {commonOptions, hmrOptions} from './options';

const program = new commander.Command();

// Exit codes in response to signals are traditionally
// 128 + signal value
// https://tldp.org/LDP/abs/html/exitcodes.html
const SIGINT_EXIT_CODE = 130;

process.on('unhandledRejection', handleUncaughtException);

program.storeOptionsAsProperties();
program.version(version);

let serve = program
  .command('serve [input...]')
  .description('starts a development server')
  .option('--public-url <url>', 'the path prefix for absolute urls')
  .option(
    '--open [browser]',
    'automatically open in specified browser, defaults to default browser',
  )
  .option('--watch-for-stdin', 'exit when stdin closes')
  .option(
    '--lazy [includes]',
    'Build async bundles on demand, when requested in the browser. Defaults to all async bundles, unless a comma separated list of source file globs is provided. Only async bundles whose entry points match these globs will be built lazily',
  )
  .option(
    '--lazy-exclude <excludes>',
    'Can only be used in combination with --lazy. Comma separated list of source file globs, async bundles whose entry points match these globs will not be built lazily',
  )
  .option('--production', 'Run with production mode defaults')
  .action(runCommand);

applyOptions(serve, hmrOptions);
applyOptions(serve, commonOptions);

let watch = program
  .command('watch [input...]')
  .description('starts the bundler in watch mode')
  .option('--public-url <url>', 'the path prefix for absolute urls')
  .option('--no-content-hash', 'disable content hashing')
  .option('--watch-for-stdin', 'exit when stdin closes')
  .option('--production', 'Run with production mode defaults')
  .action(runCommand);

applyOptions(watch, hmrOptions);
applyOptions(watch, commonOptions);

let build = program
  .command('build [input...]')
  .description('bundles for production')
  .option('--no-optimize', 'disable minification')
  .option('--no-scope-hoist', 'disable scope-hoisting')
  .option('--public-url <url>', 'the path prefix for absolute urls')
  .option('--no-content-hash', 'disable content hashing')
  .action(runCommand);

applyOptions(build, commonOptions);

program.addCommand(makeDebugCommand());

program
  .command('help [command]')
  .description('display help information for a command')
  .action(function (command) {
    let cmd = program.commands.find((c) => c.name() === command) || program;
    cmd.help();
  });

program.on('--help', function () {
  INTERNAL_ORIGINAL_CONSOLE.log('');
  INTERNAL_ORIGINAL_CONSOLE.log(
    '  Run `' +
      chalk.bold('atlaspack help <command>') +
      '` for more information on specific commands',
  );
  INTERNAL_ORIGINAL_CONSOLE.log('');
});

// Override to output option description if argument was missing
// $FlowFixMe[prop-missing]
commander.Command.prototype.optionMissingArgument = function (option) {
  INTERNAL_ORIGINAL_CONSOLE.error(
    "error: option `%s' argument missing",
    option.flags,
  );
  INTERNAL_ORIGINAL_CONSOLE.log(program.createHelp().optionDescription(option));
  process.exit(1);
};

// Make serve the default command except for --help
var args = process.argv;
if (args[2] === '--help' || args[2] === '-h') args[2] = 'help';

if (!args[2] || !program.commands.some((c) => c.name() === args[2])) {
  args.splice(2, 0, 'serve');
}

program.parse(args);

function runCommand(...args) {
  run(...args).catch(handleUncaughtException);
}

async function run(
  entries: Array<string>,
  _opts: any, // using pre v7 Commander options as properties
  command: any,
) {
  if (entries.length === 0) {
    entries = ['.'];
  }

  entries = entries.map((entry) => path.resolve(entry));

  let Atlaspack = require('@atlaspack/core').default;
  let fs = new NodeFS();
  let options = await normalizeOptions(command, fs);
  let atlaspack = new Atlaspack({
    entries,
    defaultConfig: require.resolve('@atlaspack/config-default', {
      paths: [fs.cwd(), __dirname],
    }),
    shouldPatchConsole: false,
    ...options,
  });

  let disposable = new Disposable();
  let unsubscribe: () => Promise<mixed>;
  let isExiting;
  async function exit(exitCode: number = 0) {
    if (isExiting) {
      return;
    }

    isExiting = true;
    if (unsubscribe != null) {
      await unsubscribe();
    } else if (atlaspack.isProfiling) {
      await atlaspack.stopProfiling();
    }

    if (process.stdin.isTTY && process.stdin.isRaw) {
      // $FlowFixMe
      process.stdin.setRawMode(false);
    }

    disposable.dispose();
    process.exit(exitCode);
  }

  const isWatching = command.name() === 'watch' || command.name() === 'serve';
  if (process.stdin.isTTY) {
    // $FlowFixMe
    process.stdin.setRawMode(true);
    require('readline').emitKeypressEvents(process.stdin);

    let stream = process.stdin.on('keypress', async (char, key) => {
      if (!key.ctrl) {
        return;
      }

      switch (key.name) {
        case 'c':
          // Detect the ctrl+c key, and gracefully exit after writing the asset graph to the cache.
          // This is mostly for tools that wrap Atlaspack as a child process like yarn and npm.
          //
          // Setting raw mode prevents SIGINT from being sent in response to ctrl-c:
          // https://nodejs.org/api/tty.html#tty_readstream_setrawmode_mode
          //
          // We don't use the SIGINT event for this because when run inside yarn, the parent
          // yarn process ends before Atlaspack and it appears that Atlaspack has ended while it may still
          // be cleaning up. Handling events from stdin prevents this impression.
          //
          // When watching, a 0 success code is acceptable when Atlaspack is interrupted with ctrl-c.
          // When building, fail with a code as if we received a SIGINT.
          await exit(isWatching ? 0 : SIGINT_EXIT_CODE);
          break;
        case 'e':
          await (atlaspack.isProfiling
            ? atlaspack.stopProfiling()
            : atlaspack.startProfiling());
          break;
        case 'y':
          await atlaspack.takeHeapSnapshot();
          break;
      }
    });

    disposable.add(() => {
      stream.destroy();
    });
  }

  if (isWatching) {
    ({unsubscribe} = await atlaspack.watch((err) => {
      if (err) {
        throw err;
      }
    }));

    if (command.open && options.serveOptions) {
      await openInBrowser(
        `${options.serveOptions.https ? 'https' : 'http'}://${
          options.serveOptions.host || 'localhost'
        }:${options.serveOptions.port}`,
        command.open,
      );
    }

    if (command.watchForStdin) {
      process.stdin.on('end', async () => {
        INTERNAL_ORIGINAL_CONSOLE.log('STDIN closed, ending');

        await exit();
      });
      process.stdin.resume();
    }

    // In non-tty cases, respond to SIGINT by cleaning up. Since we're watching,
    // a 0 success code is acceptable.
    process.on('SIGINT', () => exit());
    process.on('SIGTERM', () => exit());
  } else {
    try {
      await atlaspack.run();
    } catch (err) {
      // If an exception is thrown during Atlaspack.build, it is given to reporters in a
      // buildFailure event, and has been shown to the user.
      if (!(err instanceof BuildError)) {
        await logUncaughtError(err);
      }
      await exit(1);
    }

    await exit();
  }
}
