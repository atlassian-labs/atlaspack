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

export class Transformer {
  constructor<T>(opts: TransformerOpts<T>) {
    this[CONFIG] = opts;
  }
}

export class Resolver {
  constructor<T>(opts: ResolverOpts<T>) {
    this[CONFIG] = opts;
  }
}

export class Bundler {
  constructor<T>(opts: BundlerOpts<T>) {
    this[CONFIG] = opts;
  }
}

export class Namer {
  constructor<T>(opts: NamerOpts<T>) {
    this[CONFIG] = opts;
  }
}

export class Runtime {
  constructor<T>(opts: RuntimeOpts<T>) {
    this[CONFIG] = opts;
  }
}

export class Validator {
  constructor(opts: ValidatorOpts) {
    this[CONFIG] = opts;
  }
}

export class Packager {
  constructor<T, U>(opts: PackagerOpts<T, U>) {
    this[CONFIG] = opts;
  }
}

export class Optimizer {
  constructor<T, U>(opts: OptimizerOpts<T, U>) {
    this[CONFIG] = opts;
  }
}

export class Compressor {
  constructor(opts: CompressorOpts) {
    this[CONFIG] = opts;
  }
}

export class Reporter {
  constructor(opts: ReporterOpts) {
    this[CONFIG] = opts;
  }
}
