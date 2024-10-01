import type {Bundle, BundleGraph, NamedBundle} from '@atlaspack/types';
import assert from 'assert';
import {Packager} from '@atlaspack/plugin';
import {
  blobToString,
  replaceInlineReferences,
  replaceURLReferences,
} from '@atlaspack/utils';
import {DOMParser, XMLSerializer} from '@xmldom/xmldom';

export default new Packager({
  async package({bundle, bundleGraph, getInlineBundleContents}) {
    const assets: Array<Asset> = [];
    bundle.traverseAssets((asset) => {
      assets.push(asset);
    });

    assert.strictEqual(
      assets.length,
      1,
      'XML bundles must only contain one asset',
    );

    let asset = assets[0];
    let code = await asset.getCode();
    let parser = new DOMParser();
    let dom = parser.parseFromString(code);

    let inlineElements = dom.getElementsByTagNameNS(
      'https://parceljs.org',
      'inline',
    );
    if (inlineElements.length > 0) {
      for (let element of Array.from(inlineElements)) {
        let key = element.getAttribute('key');
        let type = element.getAttribute('type');

        const newContent = await getAssetContent(
          bundleGraph,
          getInlineBundleContents,
          key,
        );

        if (newContent === null) {
          continue;
        }

        let contents = await blobToString(newContent.contents);
        if (type === 'xhtml' || type === 'xml') {
          let parsed = new DOMParser().parseFromString(
            contents,
            'application/xml',
          );
          if (parsed.documentElement != null) {
            let parent = element.parentNode;
            // @ts-expect-error - TS2531 - Object is possibly 'null'.
            parent.removeChild(element);
            // @ts-expect-error - TS2531 - Object is possibly 'null'.
            parent.appendChild(parsed.documentElement);
          }
        } else {
          // @ts-expect-error - TS2531 - Object is possibly 'null'.
          element.parentNode.textContent = contents;
        }
      }

      code = new XMLSerializer().serializeToString(dom);
    }

    const {contents, map} = replaceURLReferences({
      bundle,
      bundleGraph,
      contents: code,
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
}) as Packager;

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
    const entryAssets = bundle.getEntryAssets();
    if (entryAssets.some((a) => a.uniqueKey === assetId)) {
      inlineBundle = bundle;
      stop();
    }
  });

  if (!inlineBundle) {
    return null;
  }

  const bundleResult = await getInlineBundleContents(inlineBundle, bundleGraph);

  return {bundle: inlineBundle, contents: bundleResult.contents};
}
