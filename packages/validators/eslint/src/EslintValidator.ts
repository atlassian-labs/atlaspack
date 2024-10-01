import {Validator} from '@atlaspack/plugin';
import {DiagnosticCodeFrame, escapeMarkdown} from '@atlaspack/diagnostic';
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'eslint'. '/home/ubuntu/parcel/node_modules/eslint/lib/api.js' implicitly has an 'any' type.
import eslint from 'eslint';
import invariant from 'assert';

// For eslint <8.0.0
// @ts-expect-error - TS7034 - Variable 'cliEngine' implicitly has type 'any' in some locations where its type cannot be determined.
let cliEngine = null;
// For eslint >=8.0.0
// @ts-expect-error - TS7034 - Variable 'eslintEngine' implicitly has type 'any' in some locations where its type cannot be determined.
let eslintEngine = null;

export default new Validator({
  async validate({asset}) {
    // @ts-expect-error - TS7005 - Variable 'cliEngine' implicitly has an 'any' type. | TS7005 - Variable 'eslintEngine' implicitly has an 'any' type.
    if (!cliEngine && !eslintEngine) {
      if (eslint.ESLint) {
        eslintEngine = new eslint.ESLint({});
      } else {
        cliEngine = new eslint.CLIEngine({});
      }
    }
    let code = await asset.getCode();

    let results;
    // @ts-expect-error - TS7005 - Variable 'cliEngine' implicitly has an 'any' type.
    if (cliEngine != null) {
      // @ts-expect-error - TS7005 - Variable 'cliEngine' implicitly has an 'any' type.
      results = cliEngine.executeOnText(code, asset.filePath).results;
      // @ts-expect-error - TS7005 - Variable 'eslintEngine' implicitly has an 'any' type.
    } else if (eslintEngine != null) {
      // @ts-expect-error - TS7005 - Variable 'eslintEngine' implicitly has an 'any' type.
      results = await eslintEngine.lintText(code, {filePath: asset.filePath});
    } else {
      invariant(false);
    }

    let validatorResult = {
      warnings: [],
      errors: [],
    };

    for (let result of results) {
      if (!result.errorCount && !result.warningCount) continue;

      let codeframe: DiagnosticCodeFrame = {
        filePath: asset.filePath,
        code: result.source,
        // @ts-expect-error - TS7006 - Parameter 'message' implicitly has an 'any' type.
        codeHighlights: result.messages.map((message) => {
          let start = {
            line: message.line,
            column: message.column,
          };
          return {
            start,
            // Parse errors have no ending
            end:
              message.endLine != null
                ? {
                    line: message.endLine,
                    column: message.endColumn - 1,
                  }
                : start,
            message: escapeMarkdown(message.message),
          };
        }),
      };

      let diagnostic = {
        origin: '@atlaspack/validator-eslint',
        message: `ESLint found **${result.errorCount}** __errors__ and **${result.warningCount}** __warnings__.`,
        codeFrames: [codeframe],
      };

      if (result.errorCount > 0) {
        // @ts-expect-error - TS2345 - Argument of type '{ origin: string; message: string; codeFrames: DiagnosticCodeFrame[]; }' is not assignable to parameter of type 'never'.
        validatorResult.errors.push(diagnostic);
      } else {
        // @ts-expect-error - TS2345 - Argument of type '{ origin: string; message: string; codeFrames: DiagnosticCodeFrame[]; }' is not assignable to parameter of type 'never'.
        validatorResult.warnings.push(diagnostic);
      }
    }

    return validatorResult;
  },
}) as Validator;
