import type {
  BundleGroup as IBundleGroup,
  Target as ITarget,
} from '@atlaspack/types';
import type {
  BundleGroup as InternalBundleGroup,
  AtlaspackOptions,
} from '../types';

import nullthrows from 'nullthrows';
import Target from './Target';

const internalBundleGroupToBundleGroup: WeakMap<
  InternalBundleGroup,
  BundleGroup
> = new WeakMap();
const _bundleGroupToInternalBundleGroup: WeakMap<
  IBundleGroup,
  InternalBundleGroup
> = new WeakMap();
export function bundleGroupToInternalBundleGroup(
  target: IBundleGroup,
): InternalBundleGroup {
  return nullthrows(_bundleGroupToInternalBundleGroup.get(target));
}

export default class BundleGroup implements IBundleGroup {
  #bundleGroup /*: InternalBundleGroup */;
  #options /*: AtlaspackOptions */;

  constructor(bundleGroup: InternalBundleGroup, options: AtlaspackOptions) {
    let existing = internalBundleGroupToBundleGroup.get(bundleGroup);
    if (existing != null) {
      return existing;
    }

    this.#bundleGroup = bundleGroup;
    this.#options = options;
    _bundleGroupToInternalBundleGroup.set(this, bundleGroup);
    internalBundleGroupToBundleGroup.set(bundleGroup, this);
    return this;
  }

  get target(): ITarget {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'. | TS2345 - Argument of type 'AtlaspackOptions | undefined' is not assignable to parameter of type 'AtlaspackOptions'.
    return new Target(this.#bundleGroup.target, this.#options);
  }

  get entryAssetId(): string {
    // @ts-expect-error - TS2532 - Object is possibly 'undefined'.
    return this.#bundleGroup.entryAssetId;
  }
}
