import {Validator} from '@atlaspack/plugin';
import {DiagnosticCodeFrame, escapeMarkdown} from '@atlaspack/diagnostic';
// @ts-expect-error TS7016
import eslint from 'eslint';
import invariant from 'assert';

// For eslint <8.0.0
// @ts-expect-error TS7034
let cliEngine = null;
// For eslint >=8.0.0
// @ts-expect-error TS7034
let eslintEngine = null;

export default new Validator({
  async validate({asset}) {
    // @ts-expect-error TS7005
    if (!cliEngine && !eslintEngine) {
      if (eslint.ESLint) {
        eslintEngine = new eslint.ESLint({});
      } else {
        cliEngine = new eslint.CLIEngine({});
      }
    }
    let code = await asset.getCode();

    let results;
    // @ts-expect-error TS7005
    if (cliEngine != null) {
      // @ts-expect-error TS7005
      results = cliEngine.executeOnText(code, asset.filePath).results;
      // @ts-expect-error TS7005
    } else if (eslintEngine != null) {
      // @ts-expect-error TS7005
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
        // @ts-expect-error TS7006
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
        // @ts-expect-error TS2345
        validatorResult.errors.push(diagnostic);
      } else {
        // @ts-expect-error TS2345
        validatorResult.warnings.push(diagnostic);
      }
    }

    return validatorResult;
  },
}) as Validator;
