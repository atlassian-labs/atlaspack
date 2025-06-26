import ThrowableDiagnostic from '@atlaspack/diagnostic';
import {prettyDiagnostic} from '@atlaspack/utils';
import {INTERNAL_ORIGINAL_CONSOLE} from '@atlaspack/logger';
import chalk from 'chalk';

export async function logUncaughtError(e: unknown) {
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
  await new Promise((resolve: (result: Promise<undefined> | undefined) => void) => setTimeout(resolve, 100));
}

export async function handleUncaughtException(exception: unknown) {
  try {
    await logUncaughtError(exception);
  } catch (err: any) {
    console.error(exception);
    console.error(err);
  }

  process.exit(1);
}
