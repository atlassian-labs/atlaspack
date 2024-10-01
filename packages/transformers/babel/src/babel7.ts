import type {
  MutableAsset,
  AST,
  PluginOptions,
  PluginTracer,
  PluginLogger,
} from '@atlaspack/types';
// @ts-expect-error - TS7016 - Could not find a declaration file for module '@babel/core'. '/home/ubuntu/parcel/node_modules/@babel/core/lib/index.js' implicitly has an 'any' type.
import * as BabelCore from '@babel/core';

import invariant from 'assert';
import path from 'path';
import {md} from '@atlaspack/diagnostic';
import {relativeUrl} from '@atlaspack/utils';
import {remapAstLocations} from './remapAstLocations';

// @ts-expect-error - TS2732 - Cannot find module '../package.json'. Consider using '--resolveJsonModule' to import module with '.json' extension.
import packageJson from '../package.json';

const transformerVersion: unknown = packageJson.version;
invariant(typeof transformerVersion === 'string');

type Babel7TransformOptions = {
  asset: MutableAsset;
  options: PluginOptions;
  logger: PluginLogger;
  babelOptions: any;
  additionalPlugins?: Array<any>;
  tracer: PluginTracer;
};

export default async function babel7(
  opts: Babel7TransformOptions,
  // @ts-expect-error - TS2355 - A function whose declared type is neither 'void' nor 'any' must return a value.
): Promise<AST | null | undefined> {
  let {asset, options, babelOptions, additionalPlugins = [], tracer} = opts;
  const babelCore: BabelCore = await options.packageManager.require(
    '@babel/core',
    asset.filePath,
    {
      range: '^7.12.0',
      saveDev: true,
      shouldAutoInstall: options.shouldAutoInstall,
    },
  );

  let config = {
    ...babelOptions.config,
    plugins: additionalPlugins.concat(babelOptions.config.plugins),
    code: false,
    ast: true,
    filename: asset.filePath,
    babelrc: false,
    configFile: false,
    parserOpts: {
      ...babelOptions.config.parserOpts,
      sourceFilename: relativeUrl(options.projectRoot, asset.filePath),
      allowReturnOutsideFunction: true,
      strictMode: false,
      sourceType: 'module',
      plugins: [
        ...(babelOptions.config.parserOpts?.plugins ?? []),
        ...(babelOptions.syntaxPlugins ?? []),
        // Applied by preset-env
        'classProperties',
        'classPrivateProperties',
        'classPrivateMethods',
        'exportDefaultFrom',
        // 'topLevelAwait'
      ],
    },
    caller: {
      name: 'parcel',
      version: transformerVersion,
      targets: JSON.stringify(babelOptions.targets),
      outputFormat: asset.env.outputFormat,
    },
  };

  if (tracer.enabled) {
    config.wrapPluginVisitorMethod = (
      key: string,
      nodeType: string,
      fn: any,
    ) => {
      return function () {
        let pluginKey = key;
        if (pluginKey.startsWith(options.projectRoot)) {
          pluginKey = path.relative(options.projectRoot, pluginKey);
        }
        const measurement = tracer.createMeasurement(
          pluginKey,
          nodeType,
          path.relative(options.projectRoot, asset.filePath),
        );
        // @ts-expect-error - TS2683 - 'this' implicitly has type 'any' because it does not have a type annotation.
        fn.apply(this, arguments);
        measurement && measurement.end();
      };
    };
  }

  let ast = await asset.getAST();
  let res;
  if (ast) {
    res = await babelCore.transformFromAstAsync(
      ast.program,
      asset.isASTDirty() ? undefined : await asset.getCode(),
      config,
    );
  } else {
    res = await babelCore.transformAsync(await asset.getCode(), config);
    if (res.ast) {
      let map = await asset.getMap();
      if (map) {
        remapAstLocations(babelCore.types, res.ast, map);
      }
    }
    if (res.externalDependencies) {
      for (let f of res.externalDependencies) {
        if (!path.isAbsolute(f)) {
          opts.logger.warn({
            // @ts-expect-error - TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
            message: md`Ignoring non-absolute Babel external dependency: ${f}`,
            hints: [
              'Please report this to the corresponding Babel plugin and/or to Parcel.',
            ],
          });
        } else {
          if (await options.inputFS.exists(f)) {
            asset.invalidateOnFileChange(f);
          } else {
            asset.invalidateOnFileCreate({filePath: f});
          }
        }
      }
    }
  }

  if (res.ast) {
    asset.setAST({
      type: 'babel',
      version: '7.0.0',
      program: res.ast,
    });
  }
}
