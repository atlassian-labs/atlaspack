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
  constructor(opts: TransformerOpts<unknown>) {
    this[CONFIG] = opts;
  }
}

export class Resolver {
  constructor(opts: ResolverOpts<unknown>) {
    this[CONFIG] = opts;
  }
}

export class Bundler {
  constructor(opts: BundlerOpts<unknown>) {
    this[CONFIG] = opts;
  }
}

export class Namer {
  constructor(opts: NamerOpts<unknown>) {
    this[CONFIG] = opts;
  }
}

export class Runtime {
  constructor(opts: RuntimeOpts<unknown>) {
    this[CONFIG] = opts;
  }
}

export class Validator {
  constructor(opts: ValidatorOpts) {
    this[CONFIG] = opts;
  }
}

export class Packager {
  constructor(opts: PackagerOpts<unknown, unknown>) {
    this[CONFIG] = opts;
  }
}

export class Optimizer {
  constructor(opts: OptimizerOpts<unknown, unknown>) {
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
