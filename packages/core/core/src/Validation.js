// @flow strict-local

import type {WorkerApi} from '@atlaspack/workers';
import type {AssetGroup, AtlaspackOptions, ReportFn} from './types';
import type {Validator, ValidateResult} from '@atlaspack/types';
import type {Diagnostic} from '@atlaspack/diagnostic';

import path from 'path';
import {resolveConfig} from '@atlaspack/utils';
import logger, {PluginLogger} from '@atlaspack/logger';
import ThrowableDiagnostic, {errorToDiagnostic} from '@atlaspack/diagnostic';
import {AtlaspackConfig} from './AtlaspackConfig';
import UncommittedAsset from './UncommittedAsset';
import {createAsset} from './assetUtils';
import {Asset} from './public/Asset';
import PluginOptions from './public/PluginOptions';
import summarizeRequest from './summarizeRequest';
import {fromProjectPath, fromProjectPathRelative} from './projectPath';
import {PluginTracer} from '@atlaspack/profiler';

export type ValidationOpts = {|
  config: AtlaspackConfig,
  /**
   * If true, this Validation instance will run all validators that implement the single-threaded "validateAll" method.
   * If falsy, it will run validators that implement the one-asset-at-a-time "validate" method.
   */
  dedicatedThread?: boolean,
  options: AtlaspackOptions,
  requests: AssetGroup[],
  report: ReportFn,
  workerApi?: WorkerApi,
|};

export default class Validation {
  allAssets: {[validatorName: string]: UncommittedAsset[], ...} = {};
  allValidators: {[validatorName: string]: Validator, ...} = {};
  dedicatedThread: boolean;
  impactfulOptions: $Shape<AtlaspackOptions>;
  options: AtlaspackOptions;
  atlaspackConfig: AtlaspackConfig;
  report: ReportFn;
  requests: AssetGroup[];
  workerApi: ?WorkerApi;

  constructor({
    config,
    dedicatedThread,
    options,
    requests,
    report,
    workerApi,
  }: ValidationOpts) {
    this.dedicatedThread = dedicatedThread ?? false;
    this.options = options;
    this.atlaspackConfig = config;
    this.report = report;
    this.requests = requests;
    this.workerApi = workerApi;
  }

  async run(): Promise<void> {
    let pluginOptions = new PluginOptions(this.options);
    await this.buildAssetsAndValidators();
    await Promise.all(
      Object.keys(this.allValidators).map(async (validatorName) => {
        let assets = this.allAssets[validatorName];
        if (assets) {
          let plugin = this.allValidators[validatorName];
          let validatorLogger = new PluginLogger({origin: validatorName});
          let validatorTracer = new PluginTracer({
            origin: validatorName,
            category: 'validator',
          });
          let validatorResults: Array<?ValidateResult> = [];
          try {
            // If the plugin supports the single-threading validateAll method, pass all assets to it.
            if (plugin.validateAll && this.dedicatedThread) {
              validatorResults = await plugin.validateAll({
                assets: assets.map((asset) => new Asset(asset)),
                options: pluginOptions,
                logger: validatorLogger,
                tracer: validatorTracer,
                resolveConfigWithPath: (
                  configNames: Array<string>,
                  assetFilePath: string,
                ) =>
                  resolveConfig(
                    this.options.inputFS,
                    assetFilePath,
                    configNames,
                    this.options.projectRoot,
                  ),
              });
            }

            // Otherwise, pass the assets one-at-a-time
            else if (plugin.validate && !this.dedicatedThread) {
              await Promise.all(
                assets.map(async (input) => {
                  let config = null;
                  let publicAsset = new Asset(input);
                  if (plugin.getConfig) {
                    config = await plugin.getConfig({
                      asset: publicAsset,
                      options: pluginOptions,
                      logger: validatorLogger,
                      tracer: validatorTracer,
                      resolveConfig: (configNames: Array<string>) =>
                        resolveConfig(
                          this.options.inputFS,
                          publicAsset.filePath,
                          configNames,
                          this.options.projectRoot,
                        ),
                    });
                  }

                  let validatorResult = await plugin.validate({
                    asset: publicAsset,
                    options: pluginOptions,
                    config,
                    logger: validatorLogger,
                    tracer: validatorTracer,
                  });
                  validatorResults.push(validatorResult);
                }),
              );
            }
            this.handleResults(validatorResults);
          } catch (e) {
            throw new ThrowableDiagnostic({
              diagnostic: errorToDiagnostic(e, {
                origin: validatorName,
              }),
            });
          }
        }
      }),
    );
  }

  async buildAssetsAndValidators() {
    // Figure out what validators need to be run, and group the assets by the relevant validators.
    await Promise.all(
      this.requests.map(async (request) => {
        this.report({
          type: 'validation',
          filePath: fromProjectPath(this.options.projectRoot, request.filePath),
        });

        let asset = await this.loadAsset(request);

        let validators = await this.atlaspackConfig.getValidators(
          request.filePath,
        );

        for (let validator of validators) {
          this.allValidators[validator.name] = validator.plugin;
          if (this.allAssets[validator.name]) {
            this.allAssets[validator.name].push(asset);
          } else {
            this.allAssets[validator.name] = [asset];
          }
        }
      }),
    );
  }

  handleResults(validatorResults: Array<?ValidateResult>) {
    let warnings: Array<Diagnostic> = [];
    let errors: Array<Diagnostic> = [];
    validatorResults.forEach((result) => {
      if (result) {
        warnings.push(...result.warnings);
        errors.push(...result.errors);
      }
    });

    if (errors.length > 0) {
      throw new ThrowableDiagnostic({
        diagnostic: errors,
      });
    }

    if (warnings.length > 0) {
      logger.warn(warnings);
    }
  }

  async loadAsset(request: AssetGroup): Promise<UncommittedAsset> {
    let {filePath, env, code, sideEffects, query} = request;
    let {content, size, isSource} = await summarizeRequest(
      this.options.inputFS,
      {
        filePath: fromProjectPath(this.options.projectRoot, request.filePath),
      },
    );

    return new UncommittedAsset({
      value: createAsset(this.options.projectRoot, {
        code,
        filePath: filePath,
        isSource,
        type: path.extname(fromProjectPathRelative(filePath)).slice(1),
        query,
        env: env,
        stats: {
          time: 0,
          size,
        },
        sideEffects: sideEffects,
      }),
      options: this.options,
      content,
    });
  }
}
