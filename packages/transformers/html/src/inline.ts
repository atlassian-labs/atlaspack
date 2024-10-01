import type {AST, MutableAsset, TransformerResult} from '@atlaspack/types';
import {hashString} from '@atlaspack/rust';
// @ts-expect-error - TS2305 - Module '"posthtml"' has no exported member 'PostHTMLNode'.
import type {PostHTMLNode} from 'posthtml';

import PostHTML from 'posthtml';

const SCRIPT_TYPES = {
  'application/javascript': 'js',
  'text/javascript': 'js',
  'application/json': false,
  'application/ld+json': 'jsonld',
  'text/html': false,
  module: 'js',
} as const;

interface ExtractInlineAssetsResult {
  hasModuleScripts: boolean;
  assets: Array<TransformerResult>;
}

export default function extractInlineAssets(
  asset: MutableAsset,
  ast: AST,
): ExtractInlineAssetsResult {
  let program: PostHTMLNode = ast.program;
  let key = 0;

  // Extract inline <script> and <style> tags for processing.
  let parts: Array<TransformerResult> = [];
  let hasModuleScripts = false;
  // @ts-expect-error - TS2339 - Property 'walk' does not exist on type 'PostHTML<unknown, unknown>'.
  PostHTML().walk.call(program, (node: PostHTMLNode) => {
    let parcelKey = hashString(`${asset.id}:${key++}`);
    if (node.tag === 'script' || node.tag === 'style') {
      let value = node.content && node.content.join('');
      if (value != null) {
        let type, env;

        if (node.tag === 'style') {
          if (node.attrs && node.attrs.type != null) {
            type = node.attrs.type.split('/')[1];
          } else {
            type = 'css';
          }
        } else if (node.attrs && node.attrs.type != null) {
          // Skip JSON
          // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'any' can't be used to index type '{ readonly 'application/javascript': "js"; readonly 'text/javascript': "js"; readonly 'application/json': false; readonly 'application/ld+json': "jsonld"; readonly 'text/html': false; readonly module: "js"; }'.
          if (SCRIPT_TYPES[node.attrs.type] === false) {
            return node;
          }

          // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'any' can't be used to index type '{ readonly 'application/javascript': "js"; readonly 'text/javascript': "js"; readonly 'application/json': false; readonly 'application/ld+json': "jsonld"; readonly 'text/html': false; readonly module: "js"; }'.
          if (SCRIPT_TYPES[node.attrs.type]) {
            // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'any' can't be used to index type '{ readonly 'application/javascript': "js"; readonly 'text/javascript': "js"; readonly 'application/json': false; readonly 'application/ld+json': "jsonld"; readonly 'text/html': false; readonly module: "js"; }'.
            type = SCRIPT_TYPES[node.attrs.type];
          } else {
            type = node.attrs.type.split('/')[1];
          }

          let outputFormat = 'global';
          let sourceType = 'script';
          let attrs = node.attrs;
          if (attrs && attrs.type === 'module') {
            if (
              asset.env.shouldScopeHoist &&
              asset.env.supports('esmodules', true)
            ) {
              outputFormat = 'esmodule';
            } else {
              delete attrs.type;
            }

            sourceType = 'module';
          }

          let loc = node.location
            ? {
                filePath: asset.filePath,
                start: node.location.start,
                end: node.location.end,
              }
            : undefined;

          env = {
            sourceType,
            outputFormat,
            loc,
          };
        } else {
          let loc = node.location
            ? {
                filePath: asset.filePath,
                start: node.location.start,
                end: node.location.end,
              }
            : undefined;
          type = 'js';
          env = {
            sourceType: 'script',
            loc,
          };
        }

        if (!type) {
          return node;
        }

        if (!node.attrs) {
          node.attrs = {};
        }

        // allow a script/style tag to declare its key
        if (node.attrs['data-parcel-key']) {
          parcelKey = node.attrs['data-parcel-key'];
        }

        // Inform packager to remove type, since CSS and JS are the defaults.
        if (node.attrs?.type && node.tag === 'style') {
          delete node.attrs.type;
        }

        // insert parcelId to allow us to retrieve node during packaging
        node.attrs['data-parcel-key'] = parcelKey;
        asset.setAST(ast); // mark dirty

        asset.addDependency({
          specifier: parcelKey,
          specifierType: 'esm',
        });

        parts.push({
          type,
          content: value,
          uniqueKey: parcelKey,
          bundleBehavior: 'inline',
          // @ts-expect-error - TS2322 - Type '{ sourceType: string; outputFormat: string; loc: { filePath: string; start: any; end: any; } | undefined; }' is not assignable to type 'Environment | EnvironmentOptions | undefined'.
          env,
          meta: {
            type: 'tag',
            // $FlowFixMe
            node,
            startLine: node.location?.start.line,
          },
        });

        if (env && env.sourceType === 'module') {
          hasModuleScripts = true;
        }
      }
    }

    // Process inline style attributes.
    let attrs = node.attrs;
    let style = attrs?.style;
    if (attrs != null && style != null) {
      attrs.style = asset.addDependency({
        specifier: parcelKey,
        specifierType: 'esm',
      });
      asset.setAST(ast); // mark dirty

      parts.push({
        type: 'css',
        content: style,
        uniqueKey: parcelKey,
        bundleBehavior: 'inline',
        meta: {
          type: 'attr',
          // $FlowFixMe
          node,
        },
      });
    }

    return node;
  });

  return {
    assets: parts,
    hasModuleScripts,
  };
}
