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
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'unique symbol' can't be used to index type 'Transformer'.
    this[CONFIG] = opts;
  }
}

export class Resolver {
  constructor(opts: ResolverOpts<unknown>) {
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'unique symbol' can't be used to index type 'Resolver'.
    this[CONFIG] = opts;
  }
}

export class Bundler {
  constructor(opts: BundlerOpts<unknown>) {
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'unique symbol' can't be used to index type 'Bundler'.
    this[CONFIG] = opts;
  }
}

export class Namer {
  constructor(opts: NamerOpts<unknown>) {
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'unique symbol' can't be used to index type 'Namer'.
    this[CONFIG] = opts;
  }
}

export class Runtime {
  constructor(opts: RuntimeOpts<unknown>) {
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'unique symbol' can't be used to index type 'Runtime'.
    this[CONFIG] = opts;
  }
}

export class Validator {
  constructor(opts: ValidatorOpts) {
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'unique symbol' can't be used to index type 'Validator'.
    this[CONFIG] = opts;
  }
}

export class Packager {
  constructor(opts: PackagerOpts<unknown, unknown>) {
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'unique symbol' can't be used to index type 'Packager'.
    this[CONFIG] = opts;
  }
}

export class Optimizer {
  constructor(opts: OptimizerOpts<unknown, unknown>) {
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'unique symbol' can't be used to index type 'Optimizer'.
    this[CONFIG] = opts;
  }
}

export class Compressor {
  constructor(opts: CompressorOpts) {
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'unique symbol' can't be used to index type 'Compressor'.
    this[CONFIG] = opts;
  }
}

export class Reporter {
  constructor(opts: ReporterOpts) {
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'unique symbol' can't be used to index type 'Reporter'.
    this[CONFIG] = opts;
  }
}
