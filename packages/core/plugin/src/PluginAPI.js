// @flow strict-local

import type {
  Transformer as TransformerOpts,
  Resolver as ResolverOpts,
  Bundler as BundlerOpts,
  Namer as NamerOpts,
  Runtime as RuntimeOpts,
  Packager as PackagerOpts,
  Optimizer as OptimizerOpts,
  Compressor as CompressorOpts,
  Reporter as ReporterOpts,
  Validator as ValidatorOpts,
} from '@atlaspack/types';

// This uses the `parcel-plugin-config` symbol so it's backwards compatible with
// parcel plugins.
const CONFIG = Symbol.for('parcel-plugin-config');

export class Transformer<T> {
  [CONFIG]: TransformerOpts<T>;

  constructor(opts: TransformerOpts<T>) {
    this[CONFIG] = opts;
  }
}

export class Resolver<T> {
  [CONFIG]: ResolverOpts<T>;

  constructor(opts: ResolverOpts<T>) {
    this[CONFIG] = opts;
  }
}

export class Bundler<T> {
  [CONFIG]: BundlerOpts<T>;

  constructor(opts: BundlerOpts<T>) {
    this[CONFIG] = opts;
  }
}

export class Namer<T> {
  [CONFIG]: NamerOpts<T>;

  constructor(opts: NamerOpts<T>) {
    this[CONFIG] = opts;
  }
}

export class Runtime<T> {
  [CONFIG]: RuntimeOpts<T>;

  constructor(opts: RuntimeOpts<T>) {
    this[CONFIG] = opts;
  }
}

export class Validator {
  [CONFIG]: ValidatorOpts;

  constructor(opts: ValidatorOpts) {
    this[CONFIG] = opts;
  }
}

export class Packager<T, U> {
  [CONFIG]: PackagerOpts<T, U>;

  constructor(opts: PackagerOpts<T, U>) {
    this[CONFIG] = opts;
  }
}

export class Optimizer<T, U> {
  [CONFIG]: OptimizerOpts<T, U>;

  constructor(opts: OptimizerOpts<T, U>) {
    this[CONFIG] = opts;
  }
}

export class Compressor {
  [CONFIG]: CompressorOpts;

  constructor(opts: CompressorOpts) {
    this[CONFIG] = opts;
  }
}

export class Reporter {
  [CONFIG]: ReporterOpts;

  constructor(opts: ReporterOpts) {
    this[CONFIG] = opts;
  }
}
