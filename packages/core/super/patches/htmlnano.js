/* eslint-disable */
// node_modules/htmlnano/lib/htmlnano.js
// Modified to make requires static
'use strict';

Object.defineProperty(exports, '__esModule', {
  value: true,
});
exports.default = void 0;
exports.loadConfig = loadConfig;

var _posthtml = _interopRequireDefault(require('posthtml'));

var _cosmiconfig = require('cosmiconfig');

var _safe = _interopRequireDefault(require('./presets/safe'));

var _ampSafe = _interopRequireDefault(require('./presets/ampSafe'));

var _max = _interopRequireDefault(require('./presets/max'));

var _package = _interopRequireDefault(require('../package.json'));

function _interopRequireDefault(obj) {
  return obj && obj.__esModule ? obj : {default: obj};
}

const presets = {
  safe: _safe.default,
  ampSafe: _ampSafe.default,
  max: _max.default,
};

function loadConfig(options, preset, configPath) {
  var _options;

  if (
    !(
      (_options = options) !== null &&
      _options !== void 0 &&
      _options.skipConfigLoading
    )
  ) {
    const explorer = (0, _cosmiconfig.cosmiconfigSync)(_package.default.name);
    const rc = configPath ? explorer.load(configPath) : explorer.search();

    if (rc) {
      const {preset: presetName} = rc.config;

      if (presetName) {
        if (!preset && presets[presetName]) {
          preset = presets[presetName];
        }

        delete rc.config.preset;
      }

      if (!options) {
        options = rc.config;
      }
    }
  }

  return [options || {}, preset || _safe.default];
}

const optionalDependencies = {
  // We don't use minifyCss or minifyJs
  // minifyCss: ['cssnano', 'postcss'],
  // minifyJs: ['terser'],
  minifyUrl: [
    () => require('relateurl'),
    () => require('srcset'),
    () => require('terser'),
  ],
  minifySvg: [() => require('svgo')],
};

const modules = {
  collapseAttributeWhitespace: () =>
    require('./modules/collapseAttributeWhitespace'),
  collapseBooleanAttributes: () =>
    require('./modules/collapseBooleanAttributes'),
  collapseWhitespace: () => require('./modules/collapseWhitespace'),
  custom: () => require('./modules/custom'),
  deduplicateAttributeValues: () =>
    require('./modules/deduplicateAttributeValues'),
  mergeScripts: () => require('./modules/mergeScripts'),
  mergeStyles: () => require('./modules/mergeStyles'),
  minifyConditionalComments: () =>
    require('./modules/minifyConditionalComments'),
  minifyCss: () => require('./modules/minifyCss'),
  minifyJs: () => require('./modules/minifyJs'),
  minifyJson: () => require('./modules/minifyJson'),
  minifySvg: () => require('./modules/minifySvg'),
  minifyUrls: () => require('./modules/minifyUrls'),
  normalizeAttributeValues: () => require('./modules/normalizeAttributeValues'),
  removeAttributeQuotes: () => require('./modules/removeAttributeQuotes'),
  removeComments: () => require('./modules/removeComments'),
  removeEmptyAttributes: () => require('./modules/removeEmptyAttributes'),
  removeOptionalTags: () => require('./modules/removeOptionalTags'),
  removeRedundantAttributes: () =>
    require('./modules/removeRedundantAttributes'),
  removeUnusedCss: () => require('./modules/removeUnusedCss'),
  sortAttributes: () => require('./modules/sortAttributes'),
  sortAttributesWithLists: () => require('./modules/sortAttributesWithLists'),
};

function htmlnano(optionsRun, presetRun) {
  let [options, preset] = loadConfig(optionsRun, presetRun);
  return function minifier(tree) {
    options = {...preset, ...options};
    let promise = Promise.resolve(tree);

    for (const [moduleName, moduleOptions] of Object.entries(options)) {
      if (!moduleOptions) {
        // The module is disabled
        continue;
      }

      if (_safe.default[moduleName] === undefined) {
        throw new Error('Module "' + moduleName + '" is not defined');
      }

      (optionalDependencies[moduleName] || []).forEach((dependency) => {
        try {
          dependency();
        } catch (e) {
          if (e.code === 'MODULE_NOT_FOUND') {
            console.warn(
              `You have to install "${dependency}" in order to use htmlnano's "${moduleName}" module`,
            );
          } else {
            throw e;
          }
        }
      });

      let module = modules[moduleName]();

      promise = promise.then((tree) =>
        module.default(tree, options, moduleOptions),
      );
    }

    return promise;
  };
}

htmlnano.getRequiredOptionalDependencies = function (optionsRun, presetRun) {
  const [options] = loadConfig(optionsRun, presetRun);
  return [
    ...new Set(
      Object.keys(options)
        .filter((moduleName) => options[moduleName])
        .map((moduleName) => optionalDependencies[moduleName])
        .flat(),
    ),
  ];
};

htmlnano.process = function (html, options, preset, postHtmlOptions) {
  return (0, _posthtml.default)([htmlnano(options, preset)]).process(
    html,
    postHtmlOptions,
  );
};

htmlnano.presets = presets;
var _default = htmlnano;
exports.default = _default;
