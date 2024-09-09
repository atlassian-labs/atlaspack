// @flow
import type {
  PluginOptions,
  DependencyOptions,
  BundleBehavior,
  AST,
} from '@atlaspack/types';

import {createDependencyId} from '../../Dependency';
import {getEnvironmentHash} from '../../Environment';
import Environment from '../../public/Environment';
import {
  BundleBehaviorNames,
  BundleBehavior as BundleBehaviorMap,
  SpecifierType,
  Priority,
} from '../../types';

export type InnerAsset = {|
  id: string,
  filePath: string,
  code: Array<number>,
  type: string,
  bundleBehavior: number | null,
  env: any,
|};

export class AssetCompat {
  _inner: InnerAsset;
  _ast: ?AST;
  // TODO: Type properly
  _dependencies: Array<any>;

  constructor(_inner: InnerAsset, options: PluginOptions) {
    this._inner = _inner;
    // $FlowFixMe This isn't correct to flow but is enough to satisfy the runtime checks
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

  get bundleBehavior(): ?BundleBehavior {
    let bundleBehavior = this._inner.bundleBehavior;
    return bundleBehavior == null ? null : BundleBehaviorNames[bundleBehavior];
  }
  set bundleBehavior(bundleBehavior: ?BundleBehavior): void {
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

  getAST(): ?AST {
    return this._ast;
  }
  setAST(ast: AST) {
    this._ast = ast;
  }

  addDependency(opts: DependencyOptions): string {
    const sourceTypes = {
      module: 0,
      script: 1,
    };
    const env = {
      ...this._inner.env,
      ...opts.env,
      sourceType: opts.env?.sourceType
        ? sourceTypes[opts.env.sourceType]
        : null,
    };
    env.id = getEnvironmentHash(env);

    const dependency = {
      ...opts,
      env,
      specifierType: SpecifierType[opts.specifierType],
      priority: Priority[opts.priority ?? 'sync'],
      bundleBehavior: opts.bundleBehavior
        ? BundleBehaviorMap[opts.bundleBehavior]
        : null,
    };
    // $FlowFixMe
    dependency.id = createDependencyId(dependency);

    this._dependencies.push(dependency);

    // $FlowFixMe
    return dependency.id;
  }

  addURLDependency(url: string, opts: DependencyOptions): string {
    return this.addDependency({
      specifier: url,
      specifierType: 'url',
      priority: 'lazy',
      ...opts,
    });
  }
}
