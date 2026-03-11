import logger from '@atlaspack/logger';
import type {PluginLogger as IPluginLogger} from '@atlaspack/types';
import type {
  Diagnostic,
  Diagnostifiable,
  DiagnosticWithoutOrigin,
} from '@atlaspack/diagnostic';

export type PluginLoggerOpts = {
  origin: string;
};

export class PluginLogger implements IPluginLogger {
  origin: string;

  constructor({origin}: PluginLoggerOpts) {
    this.origin = origin;
  }

  updateOrigin(
    diagnostic: DiagnosticWithoutOrigin | Array<DiagnosticWithoutOrigin>,
  ): Diagnostic | Array<Diagnostic> {
    return Array.isArray(diagnostic)
      ? diagnostic.map((d) => ({...d, origin: this.origin}))
      : ({...diagnostic, origin: this.origin} as Diagnostic);
  }

  verbose(
    diagnostic: DiagnosticWithoutOrigin | Array<DiagnosticWithoutOrigin>,
  ): void {
    logger.verbose(this.updateOrigin(diagnostic));
  }

  info(
    diagnostic: DiagnosticWithoutOrigin | Array<DiagnosticWithoutOrigin>,
  ): void {
    logger.info(this.updateOrigin(diagnostic));
  }

  log(
    diagnostic: DiagnosticWithoutOrigin | Array<DiagnosticWithoutOrigin>,
  ): void {
    logger.log(this.updateOrigin(diagnostic));
  }

  warn(
    diagnostic: DiagnosticWithoutOrigin | Array<DiagnosticWithoutOrigin>,
  ): void {
    logger.warn(this.updateOrigin(diagnostic));
  }

  error(
    input:
      | Diagnostifiable
      | DiagnosticWithoutOrigin
      | Array<DiagnosticWithoutOrigin>,
  ): void {
    logger.error(input, this.origin);
  }
}
