import {encodeJSONKeyComponent} from '@atlaspack/diagnostic';
import type {
  Config,
  PluginOptions,
  BuildMode,
  PluginLogger,
} from '@atlaspack/types-internal';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {SchemaEntity, validateSchema} from '@atlaspack/utils';
import invariant from 'assert';

type Glob = string;

type ManualSharedBundles = Array<{
  name: string;
  assets: Array<Glob>;
  types?: Array<string>;
  root?: string;
  split?: number;
}>;

export type SharedBundleMergeCandidates = Array<{
  overlapThreshold?: number;
  maxBundleSize?: number;
  sourceBundles?: Array<string>;
  minBundlesInGroup?: number;
}>;

export interface AsyncBundleMerge {
  /** Consider all async bundles smaller than this for merging */
  bundleSize: number;
  /** The max bytes allowed to be potentially overfetched due to a merge  */
  maxOverfetchSize: number;
  /** Bundles to ignore from merging */
  ignore?: Array<Glob>;
}

type BaseBundlerConfig = {
  http?: number;
  minBundles?: number;
  minBundleSize?: number;
  maxParallelRequests?: number;
  disableSharedBundles?: boolean;
  manualSharedBundles?: ManualSharedBundles;
  loadConditionalBundlesInParallel?: boolean;
  sharedBundleMerge?: SharedBundleMergeCandidates;
  asyncBundleMerge?: AsyncBundleMerge;
};

type BundlerConfig = Partial<Record<BuildMode, BaseBundlerConfig>> &
  BaseBundlerConfig;

export type ResolvedBundlerConfig = {
  minBundles: number;
  minBundleSize: number;
  maxParallelRequests: number;
  projectRoot: string;
  disableSharedBundles: boolean;
  manualSharedBundles: ManualSharedBundles;
  loadConditionalBundlesInParallel?: boolean;
  sharedBundleMerge?: SharedBundleMergeCandidates;
  asyncBundleMerge?: AsyncBundleMerge;
};

function resolveModeConfig(
  config: BundlerConfig,
  mode: BuildMode,
): BaseBundlerConfig {
  let generalConfig: Record<string, any> = {};
  let modeConfig: Record<string, any> = {};

  for (const key of Object.keys(config)) {
    if (key === 'development' || key === 'production') {
      if (key === mode) {
        // @ts-expect-error TS2322
        modeConfig = config[key];
      }
    } else {
      generalConfig[key] = config[key];
    }
  }

  return {
    ...generalConfig,
    ...modeConfig,
  };
}

// Default options by http version.
const HTTP_OPTIONS = {
  '1': {
    minBundles: 1,
    manualSharedBundles: [],
    minBundleSize: 30000,
    maxParallelRequests: 6,
    disableSharedBundles: false,
    sharedBundleMerge: [],
  },
  '2': {
    minBundles: 1,
    manualSharedBundles: [],
    minBundleSize: 20000,
    maxParallelRequests: 25,
    disableSharedBundles: false,
    sharedBundleMerge: [],
  },
} as const;

const CONFIG_SCHEMA: SchemaEntity = {
  type: 'object',
  properties: {
    http: {
      type: 'number',
      enum: Object.keys(HTTP_OPTIONS).map((k) => Number(k)),
    },
    manualSharedBundles: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          name: {
            type: 'string',
          },
          assets: {
            type: 'array',
            items: {
              type: 'string',
            },
          },
          types: {
            type: 'array',
            items: {
              type: 'string',
            },
          },
          root: {
            type: 'string',
          },
          split: {
            type: 'number',
          },
        },
        required: ['name', 'assets'],
        additionalProperties: false,
      },
    },
    sharedBundleMerge: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          overlapThreshold: {
            type: 'number',
          },
          maxBundleSize: {
            type: 'number',
          },
          sourceBundles: {
            type: 'array',
            items: {
              type: 'string',
            },
          },
          minBundlesInGroup: {
            type: 'number',
          },
        },
        additionalProperties: false,
      },
    },
    asyncBundleMerge: {
      type: 'object',
      properties: {
        bundleSize: {
          type: 'number',
          required: true,
        },
        maxOverfetchSize: {
          type: 'number',
          required: true,
        },
        ignore: {
          type: 'array',
          items: {
            type: 'string',
          },
        },
      },
      additionalProperties: false,
    },
    minBundles: {
      type: 'number',
    },
    minBundleSize: {
      type: 'number',
    },
    maxParallelRequests: {
      type: 'number',
    },
    disableSharedBundles: {
      type: 'boolean',
    },
    loadConditionalBundlesInParallel: {
      type: 'boolean',
    },
    sharedBundleMergeThreshold: {
      type: 'number',
    },
  },
  additionalProperties: false,
};

export async function loadBundlerConfig(
  config: Config,
  options: PluginOptions,
  logger: PluginLogger,
): Promise<ResolvedBundlerConfig> {
  let conf;

  if (getFeatureFlag('resolveBundlerConfigFromCwd')) {
    conf = await config.getConfigFrom(`${process.cwd()}/index`, [], {
      packageKey: '@atlaspack/bundler-default',
    });
  } else {
    conf = await config.getConfig<BundlerConfig>([], {
      packageKey: '@atlaspack/bundler-default',
    });
  }

  if (!conf) {
    const modDefault = {
      ...HTTP_OPTIONS['2'],
      projectRoot: options.projectRoot,
    } as const;
    // @ts-expect-error TS2322
    return modDefault;
  }

  invariant(conf?.contents != null);

  let modeConfig = resolveModeConfig(conf.contents, options.mode);

  // minBundles will be ignored if shared bundles are disabled
  if (
    modeConfig.minBundles != null &&
    modeConfig.disableSharedBundles === true
  ) {
    logger.warn({
      origin: '@atlaspack/bundler-default',
      message: `The value of "${modeConfig.minBundles}" set for minBundles will not be used as shared bundles have been disabled`,
    });
  }

  // minBundleSize will be ignored if shared bundles are disabled
  if (
    modeConfig.minBundleSize != null &&
    modeConfig.disableSharedBundles === true
  ) {
    logger.warn({
      origin: '@atlaspack/bundler-default',
      message: `The value of "${modeConfig.minBundleSize}" set for minBundleSize will not be used as shared bundles have been disabled`,
    });
  }

  // maxParallelRequests will be ignored if shared bundles are disabled
  if (
    modeConfig.maxParallelRequests != null &&
    modeConfig.disableSharedBundles === true
  ) {
    logger.warn({
      origin: '@atlaspack/bundler-default',
      message: `The value of "${modeConfig.maxParallelRequests}" set for maxParallelRequests will not be used as shared bundles have been disabled`,
    });
  }

  if (modeConfig.manualSharedBundles) {
    let nameArray = modeConfig.manualSharedBundles.map((a) => a.name);
    let nameSet = new Set(nameArray);
    invariant(
      nameSet.size == nameArray.length,
      'The name field must be unique for property manualSharedBundles',
    );
  }

  validateSchema.diagnostic(
    CONFIG_SCHEMA,
    {
      data: modeConfig,
      source: getFeatureFlag('schemaValidationDeferSourceLoading')
        ? () => options.inputFS.readFileSync(conf.filePath, 'utf8')
        : await options.inputFS.readFile(conf.filePath, 'utf8'),
      filePath: conf.filePath,
      prependKey: `/${encodeJSONKeyComponent('@atlaspack/bundler-default')}`,
    },
    '@atlaspack/bundler-default',
    'Invalid config for @atlaspack/bundler-default',
  );

  let http = modeConfig.http ?? 2;
  // @ts-expect-error TS7053
  let defaults = HTTP_OPTIONS[http];

  return {
    minBundles: modeConfig.minBundles ?? defaults.minBundles,
    minBundleSize: modeConfig.minBundleSize ?? defaults.minBundleSize,
    sharedBundleMerge:
      modeConfig.sharedBundleMerge ?? defaults.sharedBundleMerge,
    asyncBundleMerge: modeConfig.asyncBundleMerge,
    maxParallelRequests:
      modeConfig.maxParallelRequests ?? defaults.maxParallelRequests,
    projectRoot: options.projectRoot,
    disableSharedBundles:
      modeConfig.disableSharedBundles ?? defaults.disableSharedBundles,
    manualSharedBundles:
      modeConfig.manualSharedBundles ?? defaults.manualSharedBundles,
    loadConditionalBundlesInParallel:
      modeConfig.loadConditionalBundlesInParallel,
  };
}
