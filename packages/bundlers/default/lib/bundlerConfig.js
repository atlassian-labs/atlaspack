"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.loadBundlerConfig = loadBundlerConfig;
function _diagnostic() {
  const data = require("@atlaspack/diagnostic");
  _diagnostic = function () {
    return data;
  };
  return data;
}
function _featureFlags() {
  const data = require("@atlaspack/feature-flags");
  _featureFlags = function () {
    return data;
  };
  return data;
}
function _utils() {
  const data = require("@atlaspack/utils");
  _utils = function () {
    return data;
  };
  return data;
}
function _assert() {
  const data = _interopRequireDefault(require("assert"));
  _assert = function () {
    return data;
  };
  return data;
}
function _interopRequireDefault(e) { return e && e.__esModule ? e : { default: e }; }
function resolveModeConfig(config, mode) {
  let generalConfig = {};
  let modeConfig = {};
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
    ...modeConfig
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
    sharedBundleMerge: []
  },
  '2': {
    minBundles: 1,
    manualSharedBundles: [],
    minBundleSize: 20000,
    maxParallelRequests: 25,
    disableSharedBundles: false,
    sharedBundleMerge: []
  }
};
const CONFIG_SCHEMA = {
  type: 'object',
  properties: {
    http: {
      type: 'number',
      enum: Object.keys(HTTP_OPTIONS).map(k => Number(k))
    },
    manualSharedBundles: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          name: {
            type: 'string'
          },
          assets: {
            type: 'array',
            items: {
              type: 'string'
            }
          },
          types: {
            type: 'array',
            items: {
              type: 'string'
            }
          },
          root: {
            type: 'string'
          },
          split: {
            type: 'number'
          }
        },
        required: ['name', 'assets'],
        additionalProperties: false
      }
    },
    sharedBundleMerge: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          overlapThreshold: {
            type: 'number'
          },
          maxBundleSize: {
            type: 'number'
          },
          sourceBundles: {
            type: 'array',
            items: {
              type: 'string'
            }
          },
          minBundlesInGroup: {
            type: 'number'
          }
        },
        additionalProperties: false
      }
    },
    asyncBundleMerge: {
      type: 'object',
      properties: {
        bundleSize: {
          type: 'number',
          required: true
        },
        maxOverfetchSize: {
          type: 'number',
          required: true
        },
        ignore: {
          type: 'array',
          items: {
            type: 'string'
          }
        }
      },
      additionalProperties: false
    },
    minBundles: {
      type: 'number'
    },
    minBundleSize: {
      type: 'number'
    },
    maxParallelRequests: {
      type: 'number'
    },
    disableSharedBundles: {
      type: 'boolean'
    },
    loadConditionalBundlesInParallel: {
      type: 'boolean'
    },
    sharedBundleMergeThreshold: {
      type: 'number'
    }
  },
  additionalProperties: false
};
async function loadBundlerConfig(config, options, logger) {
  var _conf;
  let conf;
  if ((0, _featureFlags().getFeatureFlag)('resolveBundlerConfigFromCwd')) {
    conf = await config.getConfigFrom(`${process.cwd()}/index`, [], {
      packageKey: '@atlaspack/bundler-default'
    });
  } else {
    conf = await config.getConfig([], {
      packageKey: '@atlaspack/bundler-default'
    });
  }
  if (!conf) {
    const modDefault = {
      ...HTTP_OPTIONS['2'],
      projectRoot: options.projectRoot
    };
    // @ts-expect-error TS2322
    return modDefault;
  }
  (0, _assert().default)(((_conf = conf) === null || _conf === void 0 ? void 0 : _conf.contents) != null);
  let modeConfig = resolveModeConfig(conf.contents, options.mode);

  // minBundles will be ignored if shared bundles are disabled
  if (modeConfig.minBundles != null && modeConfig.disableSharedBundles === true) {
    logger.warn({
      origin: '@atlaspack/bundler-default',
      message: `The value of "${modeConfig.minBundles}" set for minBundles will not be used as shared bundles have been disabled`
    });
  }

  // minBundleSize will be ignored if shared bundles are disabled
  if (modeConfig.minBundleSize != null && modeConfig.disableSharedBundles === true) {
    logger.warn({
      origin: '@atlaspack/bundler-default',
      message: `The value of "${modeConfig.minBundleSize}" set for minBundleSize will not be used as shared bundles have been disabled`
    });
  }

  // maxParallelRequests will be ignored if shared bundles are disabled
  if (modeConfig.maxParallelRequests != null && modeConfig.disableSharedBundles === true) {
    logger.warn({
      origin: '@atlaspack/bundler-default',
      message: `The value of "${modeConfig.maxParallelRequests}" set for maxParallelRequests will not be used as shared bundles have been disabled`
    });
  }
  if (modeConfig.manualSharedBundles) {
    let nameArray = modeConfig.manualSharedBundles.map(a => a.name);
    let nameSet = new Set(nameArray);
    (0, _assert().default)(nameSet.size == nameArray.length, 'The name field must be unique for property manualSharedBundles');
  }
  _utils().validateSchema.diagnostic(CONFIG_SCHEMA, {
    data: modeConfig,
    source: () => options.inputFS.readFileSync(conf.filePath, 'utf8'),
    filePath: conf.filePath,
    prependKey: `/${(0, _diagnostic().encodeJSONKeyComponent)('@atlaspack/bundler-default')}`
  }, '@atlaspack/bundler-default', 'Invalid config for @atlaspack/bundler-default');
  let http = modeConfig.http ?? 2;
  // @ts-expect-error TS7053
  let defaults = HTTP_OPTIONS[http];
  return {
    minBundles: modeConfig.minBundles ?? defaults.minBundles,
    minBundleSize: modeConfig.minBundleSize ?? defaults.minBundleSize,
    sharedBundleMerge: modeConfig.sharedBundleMerge ?? defaults.sharedBundleMerge,
    asyncBundleMerge: modeConfig.asyncBundleMerge,
    maxParallelRequests: modeConfig.maxParallelRequests ?? defaults.maxParallelRequests,
    projectRoot: options.projectRoot,
    disableSharedBundles: modeConfig.disableSharedBundles ?? defaults.disableSharedBundles,
    manualSharedBundles: modeConfig.manualSharedBundles ?? defaults.manualSharedBundles,
    loadConditionalBundlesInParallel: modeConfig.loadConditionalBundlesInParallel
  };
}