// eslint-disable-next-line flowtype/no-types-missing-file-annotation
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

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Transformer as TransformerOpts} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Resolver as ResolverOpts} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Reporter as ReporterOpts} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Validator as ValidatorOpts} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Optimizer as OptimizerOpts} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Packager as PackagerOpts} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Namer as NamerOpts} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Runtime as RuntimeOpts} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Transformer} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Resolver} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Reporter} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Validator} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Optimizer} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Packager} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Namer} from '@atlaspack/types';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export type {Runtime} from '@atlaspack/types';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type SomeType = any; // replace with actual type

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type SomeOtherType = any; // replace with actual type

export declare class Transformer<T> {
  constructor(opts: TransformerOpts<T>);
}

export declare class Resolver<T> {
  constructor(opts: ResolverOpts<T>);
}

export declare class Bundler<T> {
  constructor(opts: BundlerOpts<T>);
}

export declare class Namer<T> {
  constructor(opts: NamerOpts<T>);
}

export declare class Runtime<T> {
  constructor(opts: RuntimeOpts<T>);
}

export declare class Validator {
  constructor(opts: ValidatorOpts);
}

export declare class Packager<C, B> {
  constructor(opts: PackagerOpts<C, B>);
}

export declare class Optimizer<C, B> {
  constructor(opts: OptimizerOpts<C, B>);
}

export declare class Compressor {
  constructor(opts: CompressorOpts);
}

export declare class Reporter {
  constructor(opts: ReporterOpts);
}
