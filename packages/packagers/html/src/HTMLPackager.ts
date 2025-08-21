import type {
  Asset,
  Async,
  Blob,
  Bundle,
  BundleGraph,
  NamedBundle,
} from '@atlaspack/types';

import assert from 'assert';
import {Readable} from 'stream';
import {Packager} from '@atlaspack/plugin';
import {setSymmetricDifference, setDifference} from '@atlaspack/utils';
import posthtml from 'posthtml';
import {
  bufferStream,
  replaceInlineReferences,
  replaceURLReferences,
  urlJoin,
} from '@atlaspack/utils';
import nullthrows from 'nullthrows';

// https://www.w3.org/TR/html5/dom.html#metadata-content-2
const metadataContent = new Set([
  'base',
  'link',
  'meta',
  'noscript',
  // 'script', // retain script order (somewhat)
  'style',
  'template',
  'title',
]);

type HTMLPackagerConfig = {
  evaluateRootConditionalBundles: boolean;
};

export default new Packager({
  async loadConfig({config, options}) {
    let posthtmlConfig = await config.getConfig<{
      render: any;
    }>(
      [
        '.posthtmlrc',
        '.posthtmlrc.js',
        '.posthtmlrc.cjs',
        '.posthtmlrc.mjs',
        'posthtml.config.js',
        'posthtml.config.cjs',
        'posthtml.config.mjs',
      ],
      {
        packageKey: 'posthtml',
      },
    );

    let conf = await config.getConfigFrom<HTMLPackagerConfig>(
      options.projectRoot + '/index',
      [],
      {
        packageKey: '@atlaspack/packager-html',
      },
    );

    return {
      render: posthtmlConfig?.contents?.render,
      evaluateRootConditionalBundles: Boolean(
        conf?.contents?.evaluateRootConditionalBundles,
      ),
    };
  },
  async package({bundle, bundleGraph, getInlineBundleContents, config}) {
    let assets: Array<Asset> = [];
    bundle.traverseAssets((asset) => {
      assets.push(asset);
    });

    assert.equal(assets.length, 1, 'HTML bundles must only contain one asset');

    let asset = assets[0];
    let code = await asset.getCode();

    // Add bundles in the same bundle group that are not inline. For example, if two inline
    // bundles refer to the same library that is extracted into a shared bundle.
    let referencedBundlesRecursive = bundleGraph.getReferencedBundles(bundle);
    let referencedBundles = [
      ...setSymmetricDifference(
        new Set(referencedBundlesRecursive),
        new Set(bundleGraph.getReferencedBundles(bundle, {recursive: false})),
      ),
    ];

    let conditionalBundles = config.evaluateRootConditionalBundles
      ? setDifference(
          getReferencedConditionalScripts(
            bundleGraph,
            referencedBundlesRecursive,
          ),
          new Set(referencedBundles),
        )
      : new Set<NamedBundle>();
    let renderConfig = config?.render;

    let {html} = await posthtml([
      (tree: any) =>
        insertBundleReferences(
          [...conditionalBundles, ...referencedBundles],
          tree,
          conditionalBundles,
        ),
      (tree: any) =>
        replaceInlineAssetContent(bundleGraph, getInlineBundleContents, tree),
    ]).process(code, {
      ...renderConfig,
      xmlMode: bundle.type === 'xhtml',
      closingSingleTag: bundle.type === 'xhtml' ? 'slash' : undefined,
    });

    let {contents, map} = replaceURLReferences({
      bundle,
      bundleGraph,
      contents: html,
      relative: false,
      getReplacement: (contents) => contents.replace(/"/g, '&quot;'),
    });

    return replaceInlineReferences({
      bundle,
      bundleGraph,
      contents,
      getInlineBundleContents,
      getInlineReplacement: (dep, inlineType, contents) => ({
        from: dep.id,
        to: contents.replace(/"/g, '&quot;').trim(),
      }),
      map,
    });
  },
}) as Packager<unknown, unknown>;

async function getAssetContent(
  bundleGraph: BundleGraph<NamedBundle>,
  getInlineBundleContents: (
    arg1: Bundle,
    arg2: BundleGraph<NamedBundle>,
  ) => Async<{
    contents: Blob;
  }>,
  assetId: any,
) {
  let inlineBundle: Bundle | null | undefined;
  bundleGraph.traverseBundles((bundle, context, {stop}) => {
    let entryAssets = bundle.getEntryAssets();
    if (entryAssets.some((a) => a.uniqueKey === assetId)) {
      inlineBundle = bundle;
      stop();
    }
  });

  if (inlineBundle) {
    const bundleResult = await getInlineBundleContents(
      inlineBundle,
      bundleGraph,
    );

    return {bundle: inlineBundle, contents: bundleResult.contents};
  }

  return null;
}

async function replaceInlineAssetContent(
  bundleGraph: BundleGraph<NamedBundle>,
  getInlineBundleContents: (
    arg1: Bundle,
    arg2: BundleGraph<NamedBundle>,
  ) => Async<{
    contents: Blob;
  }>,
  tree: any,
) {
  const inlineNodes: Array<any> = [];
  tree.walk((node: any) => {
    if (node.attrs && node.attrs['data-parcel-key']) {
      inlineNodes.push(node);
    }
    return node;
  });

  for (let node of inlineNodes) {
    let newContent = await getAssetContent(
      bundleGraph,
      getInlineBundleContents,
      node.attrs['data-parcel-key'],
    );

    if (newContent != null) {
      let {contents, bundle} = newContent;
      node.content = (
        contents instanceof Readable ? await bufferStream(contents) : contents
      ).toString();

      if (
        node.tag === 'script' &&
        nullthrows(bundle).env.outputFormat === 'esmodule'
      ) {
        node.attrs.type = 'module';
      }

      // Escape closing script tags and HTML comments in JS content.
      // https://www.w3.org/TR/html52/semantics-scripting.html#restrictions-for-contents-of-script-elements
      // Avoid replacing </script with <\/script as it would break the following valid JS: 0</script/ (i.e. regexp literal).
      // Instead, escape the s character.
      if (node.tag === 'script') {
        node.content = node.content
          .replace(/<!--/g, '<\\!--')
          .replace(/<\/(script)/gi, '</\\$1');
      }

      // Escape closing style tags in CSS content.
      if (node.tag === 'style') {
        node.content = node.content.replace(/<\/(style)/gi, '<\\/$1');
      }

      // remove attr from output
      delete node.attrs['data-parcel-key'];
    }
  }

  return tree;
}

function insertBundleReferences(
  siblingBundles: Array<NamedBundle>,
  tree: any,
  conditionalBundles: Set<NamedBundle>,
) {
  const bundles = [];

  for (let bundle of siblingBundles) {
    if (bundle.type === 'css') {
      bundles.push({
        tag: 'link',
        attrs: {
          rel: 'stylesheet',
          href: urlJoin(bundle.target.publicUrl, bundle.name),
        },
      });
    } else if (bundle.type === 'js') {
      let nomodule =
        bundle.env.outputFormat !== 'esmodule' &&
        bundle.env.sourceType === 'module' &&
        bundle.env.shouldScopeHoist;
      bundles.push({
        tag: 'script',
        attrs: {
          type: bundle.env.outputFormat === 'esmodule' ? 'module' : undefined,
          nomodule: nomodule ? '' : undefined,
          defer: nomodule ? '' : undefined,
          'data-conditional': conditionalBundles.has(bundle) ? true : undefined,
          src: urlJoin(bundle.target.publicUrl, bundle.name),
        },
      });
    }
  }

  addBundlesToTree(bundles, tree);
}

function addBundlesToTree(bundles: any[], tree: any) {
  const main = find(tree, 'head') || find(tree, 'html');
  // @ts-expect-error TS2339
  const content = main ? main.content || (main.content = []) : tree;
  const index = findBundleInsertIndex(content);

  content.splice(index, 0, ...bundles);
}

function find(tree: any, tag: string) {
  let res;
  // @ts-expect-error TS7006
  tree.match({tag}, (node) => {
    res = node;
    return node;
  });

  return res;
}

function findBundleInsertIndex(content: any) {
  // HTML document order (https://html.spec.whatwg.org/multipage/syntax.html#writing)
  //   - Any number of comments and ASCII whitespace.
  //   - A DOCTYPE.
  //   - Any number of comments and ASCII whitespace.
  //   - The document element, in the form of an html element.
  //   - Any number of comments and ASCII whitespace.
  //
  // -> Insert before first non-metadata (or script) element; if none was found, after the doctype

  let doctypeIndex;
  for (let index = 0; index < content.length; index++) {
    const node = content[index];
    if (node && node.tag && !metadataContent.has(node.tag)) {
      return index;
    }
    if (
      typeof node === 'string' &&
      node.toLowerCase().startsWith('<!doctype')
    ) {
      doctypeIndex = index;
    }
  }

  return doctypeIndex ? doctypeIndex + 1 : 0;
}

function getReferencedConditionalScripts(
  bundleGraph: BundleGraph<NamedBundle>,
  referencedBundles: NamedBundle[],
): Set<NamedBundle> {
  const conditionalBundleMapping = bundleGraph.getConditionalBundleMapping();

  const bundles: Array<NamedBundle> = [];
  for (const bundle of referencedBundles) {
    const conditions = conditionalBundleMapping.get(bundle.id);
    if (conditions) {
      for (const [, cond] of conditions) {
        // Reverse so dependent bundles are loaded first
        const dependentBundles = [
          ...cond.ifTrueBundles.reverse(),
          ...cond.ifFalseBundles.reverse(),
        ];
        bundles.push(...dependentBundles);
        referencedBundles.push(...dependentBundles);
      }
    }
  }

  return new Set(bundles);
}
