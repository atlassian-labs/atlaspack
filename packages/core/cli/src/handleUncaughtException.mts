// @ts-ignore TS:MIGRATE
import atlaspackDiagnostic from '@atlaspack/diagnostic';
// @ts-ignore TS:MIGRATE
import atlaspackLogger from '@atlaspack/logger';
// @ts-ignore TS:MIGRATE
import atlaspackUtils from '@atlaspack/utils';
import chalk from 'chalk';

// @ts-ignore TS:MIGRATE
const ThrowableDiagnostic = atlaspackDiagnostic.default;
// @ts-ignore TS:MIGRATE
const {INTERNAL_ORIGINAL_CONSOLE} = atlaspackLogger;
// @ts-ignore TS:MIGRATE
const {prettyDiagnostic} = atlaspackUtils;

export async function logUncaughtError(
  e: typeof ThrowableDiagnostic | unknown,
) {
  if (e instanceof ThrowableDiagnostic) {
    for (let diagnostic of e.diagnostics) {
      let {message, codeframe, stack, hints, documentation} =
        await prettyDiagnostic(diagnostic);
      INTERNAL_ORIGINAL_CONSOLE.error(chalk.red(message));
      if (codeframe || stack) {
        INTERNAL_ORIGINAL_CONSOLE.error('');
      }
      INTERNAL_ORIGINAL_CONSOLE.error(codeframe);
      INTERNAL_ORIGINAL_CONSOLE.error(stack);
      if ((stack || codeframe) && hints.length > 0) {
        INTERNAL_ORIGINAL_CONSOLE.error('');
      }
      for (let h of hints) {
        INTERNAL_ORIGINAL_CONSOLE.error(chalk.blue(h));
      }
      if (documentation) {
        INTERNAL_ORIGINAL_CONSOLE.error(chalk.magenta.bold(documentation));
      }
    }
  } else {
    INTERNAL_ORIGINAL_CONSOLE.error(e);
  }

  // A hack to definitely ensure we logged the uncaught exception
  await new Promise((resolve) => setTimeout(resolve, 100));
}

export async function handleUncaughtException(exception: unknown) {
  try {
    await logUncaughtError(exception);
  } catch (err) {
    console.error(exception);
    console.error(err);
  }

  process.exit(1);
}
