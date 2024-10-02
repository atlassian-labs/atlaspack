// @flow

import type {PackageManager} from '@atlaspack/rust';
import type {
  PackageManager as ClassicPackageManager,
  PackageManagerResolveResult,
  DependencySpecifier,
  FilePath,
  PackageManagerPackageOptions,
} from '@atlaspack/types';
import type {JsCallable} from './jsCallable';
import {jsCallable} from './jsCallable';

export class NativePackageManager implements PackageManager {
  #packageManager: ClassicPackageManager;

  constructor(packageManager: ClassicPackageManager) {
    this.#packageManager = packageManager;
  }

  resolve: JsCallable<
    [DependencySpecifier, FilePath, ?PackageManagerPackageOptions],
    Promise<PackageManagerResolveResult>,
  > = jsCallable((...args) => this.#packageManager.resolve(...args));
}
