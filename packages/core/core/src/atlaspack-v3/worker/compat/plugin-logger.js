// @flow

import type {PluginLogger as IPlguinLogger} from '@atlaspack/types';
import type {
  Diagnostifiable,
  DiagnosticWithoutOrigin,
} from '@atlaspack/diagnostic';

export class PluginLogger implements IPlguinLogger {
  verbose(
    diagnostic: DiagnosticWithoutOrigin | Array<DiagnosticWithoutOrigin>,
  ): void {
    // eslint-disable-next-line no-console
    console.log(diagnostic);
  }

  info(
    diagnostic: DiagnosticWithoutOrigin | Array<DiagnosticWithoutOrigin>,
  ): void {
    // eslint-disable-next-line no-console
    console.info(diagnostic);
  }

  log(
    diagnostic: DiagnosticWithoutOrigin | Array<DiagnosticWithoutOrigin>,
  ): void {
    // eslint-disable-next-line no-console
    console.log(diagnostic);
  }

  warn(
    diagnostic: DiagnosticWithoutOrigin | Array<DiagnosticWithoutOrigin>,
  ): void {
    // eslint-disable-next-line no-console
    console.warn(diagnostic);
  }

  error(
    input:
      | Diagnostifiable
      | DiagnosticWithoutOrigin
      | Array<DiagnosticWithoutOrigin>,
  ): void {
    // eslint-disable-next-line no-console
    console.error(input);
  }
}
