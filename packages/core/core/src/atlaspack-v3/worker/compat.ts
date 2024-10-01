import type {PluginOptions, BundleBehavior, AST} from '@atlaspack/types';

import Environment from '../../public/Environment';
import {
  BundleBehaviorNames,
  BundleBehavior as BundleBehaviorMap,
} from '../../types';

export type InnerAsset = {
  id: string;
  filePath: string;
  code: Array<number>;
  type: string;
  bundleBehavior: number | null;
  env: any;
};

export class AssetCompat {
  _inner: InnerAsset;
  _ast: AST | null | undefined;
  // TODO: Type properly
  _dependencies: Array<any>;

  constructor(_inner: InnerAsset, options: PluginOptions) {
    this._inner = _inner;
    this.env = new Environment(_inner.env, options);
    this._dependencies = [];
  }

  get id(): string {
    return this._inner.id;
  }

  get filePath(): string {
    return this._inner.filePath;
  }

  get type(): string {
    return this._inner.type;
  }
  set type(value: string) {
    this._inner.type = value;
  }

  get bundleBehavior(): BundleBehavior | null | undefined {
    let bundleBehavior = this._inner.bundleBehavior;
    return bundleBehavior == null ? null : BundleBehaviorNames[bundleBehavior];
  }
  set bundleBehavior(bundleBehavior?: BundleBehavior | null): void {
    this._inner.bundleBehavior = bundleBehavior
      ? BundleBehaviorMap[bundleBehavior]
      : null;
  }

  getCode(): string {
    return Buffer.from(this._inner.code).toString();
  }
  setCode(code: string) {
    this._inner.code = Array.from(new TextEncoder().encode(code));
  }

  getAST(): AST | null | undefined {
    return this._ast;
  }
  setAST(ast: AST) {
    this._ast = ast;
  }

  addDependency(): string {
    throw new Error(
      '[V3] Unimplemented: Asset.addDependency not yet implemented',
    );
  }

  addURLDependency(): string {
    throw new Error(
      '[V3] Unimplemented: Asset.addURLDependency not yet implemented',
    );
  }
}
