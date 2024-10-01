import type {
  FilePath,
  Target as ITarget,
  Environment as IEnvironment,
  SourceLocation,
} from '@atlaspack/types';
import type {Target as TargetValue, AtlaspackOptions} from '../types';

import nullthrows from 'nullthrows';
import Environment from './Environment';
import {fromProjectPath} from '../projectPath';
import {fromInternalSourceLocation} from '../utils';

const inspect = Symbol.for('nodejs.util.inspect.custom');

const internalTargetToTarget: WeakMap<TargetValue, Target> = new WeakMap();
const _targetToInternalTarget: WeakMap<ITarget, TargetValue> = new WeakMap();
export function targetToInternalTarget(target: ITarget): TargetValue {
  return nullthrows(_targetToInternalTarget.get(target));
}

export default class Target implements ITarget {
  #target /*: TargetValue */;
  #options /*: AtlaspackOptions */;

  constructor(target: TargetValue, options: AtlaspackOptions) {
    let existing = internalTargetToTarget.get(target);
    if (existing != null) {
      return existing;
    }

    this.#target = target;
    this.#options = options;
    _targetToInternalTarget.set(this, target);
    internalTargetToTarget.set(target, this);
    return this;
  }

  get distEntry(): FilePath | null | undefined {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#target.distEntry;
  }

  get distDir(): FilePath {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'. | TS2532 - Object is possibly 'undefined'.
    return fromProjectPath(this.#options.projectRoot, this.#target.distDir);
  }

  get env(): IEnvironment {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'. | TS2345 - Argument of type 'AtlaspackOptions | undefined' is not assignable to parameter of type 'AtlaspackOptions'.
    return new Environment(this.#target.env, this.#options);
  }

  get name(): string {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#target.name;
  }

  get publicUrl(): string {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#target.publicUrl;
  }

  get loc(): SourceLocation | null | undefined {
    return fromInternalSourceLocation(
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      this.#options.projectRoot,
      // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
      this.#target.loc,
    );
  }

  // $FlowFixMe[unsupported-syntax]
  [inspect](): string {
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'unique symbol' can't be used to index type 'Environment'.
    return `Target(${this.name} - ${this.env[inspect]()})`;
  }
}
