// @flow strict-local

import {Bundler} from '@atlaspack/plugin';
import type {Asset, Dependency, MutableBundleGraph} from '@atlaspack/types';
import {DefaultMap} from '@atlaspack/utils';
import invariant from 'assert';

import {loadBundlerConfig} from './bundlerConfig';
import {decorateLegacyGraph} from './decorateLegacyGraph';
import {createIdealGraph} from './idealGraph';
import {addJSMonolithBundle} from './MonolithicBundler';

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
export default (new Bundler({
  loadConfig({config, options, logger}) {
    return loadBundlerConfig(config, options, logger);
  },

  bundle({bundleGraph, config, logger}) {
    let targetMap = getEntryByTarget(bundleGraph); // Organize entries by target output folder/ distDir
    let graphs = [];

    for (let entries of targetMap.values()) {
      let singleFileEntries = new Map();
      let idealGraphEntries = new Map();

      // Separate out the monolith bundles based on the option on target
      for (let [entryAsset, entryDep] of entries.entries()) {
        if (entryDep.target?.env.unstableSingleFileOutput === true) {
          singleFileEntries.set(entryAsset, entryDep);
        } else {
          idealGraphEntries.set(entryAsset, entryDep);
        }
      }

      // Create separate bundleGraphs per distDir
      graphs.push(
        createIdealGraph(bundleGraph, config, idealGraphEntries, logger),
      );

      // Do this after the ideal graph so that the mutation of the bundleGraph doesn't
      // interfere with the main bundling algorithm
      for (let [entryAsset, entryDep] of singleFileEntries.entries()) {
        addJSMonolithBundle(bundleGraph, entryAsset, entryDep);
      }
    }

    for (let g of graphs) {
      decorateLegacyGraph(g, bundleGraph); //mutate original graph
    }
  },
  optimize() {},
}): Bundler);

function getEntryByTarget(
  bundleGraph: MutableBundleGraph,
): DefaultMap<string, Map<Asset, Dependency>> {
  // Find entries from assetGraph per target
  let targets: DefaultMap<string, Map<Asset, Dependency>> = new DefaultMap(
    () => new Map(),
  );
  bundleGraph.traverse({
    enter(node, context, actions) {
      if (node.type !== 'asset') {
        return node;
      }
      invariant(
        context != null &&
          context.type === 'dependency' &&
          context.value.isEntry &&
          context.value.target != null,
      );
      targets.get(context.value.target.distDir).set(node.value, context.value);
      actions.skipChildren();
      return node;
    },
  });
  return targets;
}
