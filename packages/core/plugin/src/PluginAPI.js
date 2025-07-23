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

export class Transformer<T = mixed> {
  // $FlowFixMe: Computed property keys not supported. [unsupported-syntax]
  [CONFIG]: TransformerOpts<T>;

  constructor(opts: TransformerOpts<T>) {
    // $FlowFixMe: because an index signature declaring the expected key / value type is missing in
    this[CONFIG] = opts;
  }
}

export class Resolver<T = mixed> {
  // $FlowFixMe: Computed property keys not supported. [unsupported-syntax]
  [CONFIG]: ResolverOpts<T>;

  constructor(opts: ResolverOpts<T>) {
    // $FlowFixMe: because an index signature declaring the expected key / value type is missing in
    this[CONFIG] = opts;
  }
}

export class Bundler<T = mixed> {
  // $FlowFixMe: Computed property keys not supported. [unsupported-syntax]
  [CONFIG]: BundlerOpts<T>;

  constructor(opts: BundlerOpts<T>) {
    // $FlowFixMe: because an index signature declaring the expected key / value type is missing in
    this[CONFIG] = opts;
  }
}

export class Namer<T = mixed> {
  // $FlowFixMe: Computed property keys not supported. [unsupported-syntax]
  [CONFIG]: NamerOpts<T>;

  constructor(opts: NamerOpts<T>) {
    // $FlowFixMe: because an index signature declaring the expected key / value type is missing in
    this[CONFIG] = opts;
  }
}

export class Runtime<T = mixed> {
  // $FlowFixMe: Computed property keys not supported. [unsupported-syntax]
  [CONFIG]: RuntimeOpts<T>;

  constructor(opts: RuntimeOpts<T>) {
    // $FlowFixMe: because an index signature declaring the expected key / value type is missing in
    this[CONFIG] = opts;
  }
}

export class Validator {
  // $FlowFixMe: Computed property keys not supported. [unsupported-syntax]
  [CONFIG]: ValidatorOpts;

  constructor(opts: ValidatorOpts) {
    // $FlowFixMe: because an index signature declaring the expected key / value type is missing in
    this[CONFIG] = opts;
  }
}

export class Packager<T = mixed, U = mixed> {
  // $FlowFixMe: Computed property keys not supported. [unsupported-syntax]
  [CONFIG]: PackagerOpts<T, U>;

  constructor(opts: PackagerOpts<T, U>) {
    // $FlowFixMe: because an index signature declaring the expected key / value type is missing in
    this[CONFIG] = opts;
  }
}

export class Optimizer<T = mixed, U = mixed> {
  // $FlowFixMe: Computed property keys not supported. [unsupported-syntax]
  [CONFIG]: OptimizerOpts<T, U>;

  constructor(opts: OptimizerOpts<T, U>) {
    // $FlowFixMe: because an index signature declaring the expected key / value type is missing in
    this[CONFIG] = opts;
  }
}

export class Compressor {
  // $FlowFixMe: Computed property keys not supported. [unsupported-syntax]
  [CONFIG]: CompressorOpts;

  constructor(opts: CompressorOpts) {
    // $FlowFixMe: because an index signature declaring the expected key / value type is missing in
    this[CONFIG] = opts;
  }
}

export class Reporter {
  // $FlowFixMe: Computed property keys not supported. [unsupported-syntax]
  [CONFIG]: ReporterOpts;

  constructor(opts: ReporterOpts) {
    // $FlowFixMe: because an index signature declaring the expected key / value type is missing in
    this[CONFIG] = opts;
  }
}
