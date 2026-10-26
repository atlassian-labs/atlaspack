"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = void 0;
function _plugin() {
  const data = require("@atlaspack/plugin");
  _plugin = function () {
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
var _bundlerConfig = require("./bundlerConfig");
var _decorateLegacyGraph = require("./decorateLegacyGraph");
var _idealGraph = require("./idealGraph");
var _MonolithicBundler = require("./MonolithicBundler");
function _interopRequireDefault(e) { return e && e.__esModule ? e : { default: e }; }
/**
 *
 * The Bundler works by creating an IdealGraph, which contains a BundleGraph that models bundles
 * connected to other bundles by what references them, and thus models BundleGroups.
 *
 * First, we enter `bundle({bundleGraph, config})`. Here, "bundleGraph" is actually just the
 * assetGraph turned into a type `MutableBundleGraph`, which will then be mutated in decorate,
 * and turned into what we expect the bundleGraph to be as per the old (default) bundler structure
 *  & what the rest of Atlaspack expects a BundleGraph to be.
 *
 * `bundle({bundleGraph, config})` First gets a Mapping of target to entries, In most cases there is
 *  only one target, and one or more entries. (Targets are pertinent in monorepos or projects where you
 *  will have two or more distDirs, or output folders.) Then calls create IdealGraph and Decorate per target.
 *
 */
var _default = exports.default = new (_plugin().Bundler)({
  loadConfig({
    config,
    options,
    logger
  }) {
    return (0, _bundlerConfig.loadBundlerConfig)(config, options, logger);
  },
  bundle({
    bundleGraph,
    config,
    logger
  }) {
    let targetMap = getEntryByTarget(bundleGraph); // Organize entries by target output folder/ distDir
    // @ts-expect-error TS2304
    let graphs = [];
    for (let entries of targetMap.values()) {
      let singleFileEntries = new Map();
      let idealGraphEntries = new Map();

      // Separate out the monolith bundles based on the option on target
      for (let [entryAsset, entryDep] of entries.entries()) {
        var _entryDep$target;
        if (((_entryDep$target = entryDep.target) === null || _entryDep$target === void 0 ? void 0 : _entryDep$target.env.unstableSingleFileOutput) === true) {
          singleFileEntries.set(entryAsset, entryDep);
        } else {
          idealGraphEntries.set(entryAsset, entryDep);
        }
      }

      // Create separate bundleGraphs per distDir
      graphs.push((0, _idealGraph.createIdealGraph)(bundleGraph, config, idealGraphEntries, logger));

      // Do this after the ideal graph so that the mutation of the bundleGraph doesn't
      // interfere with the main bundling algorithm
      for (let [entryAsset, entryDep] of singleFileEntries.entries()) {
        (0, _MonolithicBundler.addJSMonolithBundle)(bundleGraph, entryAsset, entryDep);
      }
    }
    for (let g of graphs) {
      (0, _decorateLegacyGraph.decorateLegacyGraph)(g, bundleGraph); //mutate original graph
    }
  },
  optimize() {}
});
function getEntryByTarget(bundleGraph) {
  // Find entries from assetGraph per target
  let targets = new (_utils().DefaultMap)(() => new Map());
  bundleGraph.traverse({
    enter(
    // @ts-expect-error TS2304
    node, context,
    // @ts-expect-error TS2304
    actions) {
      if (node.type !== 'asset') {
        return node;
      }
      (0, _assert().default)(context != null && context.type === 'dependency' && context.value.isEntry && context.value.target != null);
      targets.get(context.value.target.distDir).set(node.value, context.value);
      actions.skipChildren();
      return node;
    }
  });
  return targets;
}