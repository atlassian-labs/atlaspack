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
import {fromEnvironmentId} from '../EnvironmentManager';

const inspect = Symbol.for('nodejs.util.inspect.custom');

const internalTargetToTarget: WeakMap<TargetValue, Target> = new WeakMap();
const _targetToInternalTarget: WeakMap<ITarget, TargetValue> = new WeakMap();
export function targetToInternalTarget(target: ITarget): TargetValue {
  return nullthrows(_targetToInternalTarget.get(target));
}

export default class Target implements ITarget {
  // @ts-expect-error TS2564
  #target: TargetValue;
  // @ts-expect-error TS2564
  #options: AtlaspackOptions;

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
    return this.#target.distEntry;
  }

  get distDir(): FilePath {
    return fromProjectPath(this.#options.projectRoot, this.#target.distDir);
  }

  get env(): IEnvironment {
    return new Environment(fromEnvironmentId(this.#target.env), this.#options);
  }

  get name(): string {
    return this.#target.name;
  }

  get publicUrl(): string {
    return this.#target.publicUrl;
  }

  get loc(): SourceLocation | null | undefined {
    return fromInternalSourceLocation(
      this.#options.projectRoot,
      this.#target.loc,
    );
  }

  [inspect](): string {
    // @ts-expect-error TS7053
    return `Target(${this.name} - ${this.env[inspect]()})`;
  }
}
