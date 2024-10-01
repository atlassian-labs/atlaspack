import type {AST, MutableAsset, FilePath} from '@atlaspack/types';
// @ts-expect-error - TS2305 - Module '"posthtml"' has no exported member 'PostHTMLNode'.
import type {PostHTMLNode} from 'posthtml';
import PostHTML from 'posthtml';
import {parse, stringify} from 'srcset';
// A list of all attributes that may produce a dependency
// Based on https://developer.mozilla.org/en-US/docs/Web/HTML/Attributes
const ATTRS = {
  src: [
    'script',
    'img',
    'audio',
    'video',
    'source',
    'track',
    'iframe',
    'embed',
    'amp-img',
  ],
  // Using href with <script> is described here: https://developer.mozilla.org/en-US/docs/Web/SVG/Element/script
  href: ['link', 'a', 'use', 'script', 'image'],
  srcset: ['img', 'source'],
  imagesrcset: ['link'],
  poster: ['video'],
  'xlink:href': ['use', 'image', 'script'],
  content: ['meta'],
  data: ['object'],
} as const;

// A list of metadata that should produce a dependency
// Based on:
// - http://schema.org/
// - http://ogp.me
// - https://developer.twitter.com/en/docs/tweets/optimize-with-cards/overview/markup
// - https://msdn.microsoft.com/en-us/library/dn255024.aspx
// - https://vk.com/dev/publications
const META = {
  property: [
    'og:image',
    'og:image:url',
    'og:image:secure_url',
    'og:audio',
    'og:audio:secure_url',
    'og:video',
    'og:video:secure_url',
    'vk:image',
  ],
  name: [
    'twitter:image',
    'msapplication-square150x150logo',
    'msapplication-square310x310logo',
    'msapplication-square70x70logo',
    'msapplication-wide310x150logo',
    'msapplication-TileImage',
    'msapplication-config',
  ],
  itemprop: [
    'image',
    'logo',
    'screenshot',
    'thumbnailUrl',
    'contentUrl',
    'downloadUrl',
  ],
} as const;

const FEED_TYPES = new Set(['application/rss+xml', 'application/atom+xml']);

// Options to be passed to `addDependency` for certain tags + attributes
const OPTIONS = {
  a: {
    href: {needsStableName: true},
  },
  iframe: {
    src: {needsStableName: true},
  },
  link(attrs: any) {
    if (attrs.rel === 'stylesheet') {
      return {
        // Keep in the same bundle group as the HTML.
        priority: 'parallel',
      };
    }
  },
} as const;

function collectSrcSetDependencies(
  asset: MutableAsset,
  srcset: string,
  opts: any,
) {
  let parsed = parse(srcset).map(({url, ...v}) => ({
    url: asset.addURLDependency(url, opts),
    ...v,
  }));
  return stringify(parsed);
}

function getAttrDepHandler(attr: string) {
  if (attr === 'srcset' || attr === 'imagesrcset') {
    return collectSrcSetDependencies;
  }

  return (asset: MutableAsset, src: string, opts: any) =>
    asset.addURLDependency(src, opts);
}

export default function collectDependencies(
  asset: MutableAsset,
  ast: AST,
): boolean {
  let isDirty = false;
  let hasModuleScripts = false;
  let seen = new Set();
  let errors: Array<{
    message: string;
    filePath: FilePath;
    loc: unknown;
  }> = [];
  // @ts-expect-error - TS2339 - Property 'walk' does not exist on type 'PostHTML<unknown, unknown>'. | TS7006 - Parameter 'node' implicitly has an 'any' type.
  PostHTML().walk.call(ast.program, (node) => {
    let {tag, attrs} = node;
    if (!attrs || seen.has(node)) {
      return node;
    }

    seen.add(node);

    if (tag === 'meta') {
      const isMetaDependency = Object.keys(attrs).some((attr) => {
        // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'string' can't be used to index type '{ readonly property: readonly ["og:image", "og:image:url", "og:image:secure_url", "og:audio", "og:audio:secure_url", "og:video", "og:video:secure_url", "vk:image"]; readonly name: readonly ["twitter:image", ... 5 more ..., "msapplication-config"]; readonly itemprop: readonly [...]; }'.
        let values = META[attr];
        return (
          values &&
          values.includes(attrs[attr]) &&
          attrs.content !== '' &&
          !(attrs.name === 'msapplication-config' && attrs.content === 'none')
        );
      });
      if (isMetaDependency) {
        const metaAssetUrl = attrs.content;
        if (metaAssetUrl) {
          attrs.content = asset.addURLDependency(attrs.content, {
            needsStableName: !(
              attrs.name && attrs.name.includes('msapplication')
            ),
          });
          isDirty = true;
          asset.setAST(ast);
        }
      }
      return node;
    }

    if (
      tag === 'link' &&
      (attrs.rel === 'canonical' ||
        attrs.rel === 'manifest' ||
        (attrs.rel === 'alternate' && FEED_TYPES.has(attrs.type))) &&
      attrs.href
    ) {
      let href = attrs.href;
      if (attrs.rel === 'manifest') {
        // A hack to allow manifest.json rather than manifest.webmanifest.
        // If a custom pipeline is used, it is responsible for running @atlaspack/transformer-webmanifest.
        if (!href.includes(':')) {
          href = 'webmanifest:' + href;
        }
      }

      attrs.href = asset.addURLDependency(href, {
        needsStableName: true,
      });
      isDirty = true;
      asset.setAST(ast);
      return node;
    }

    if (tag === 'script' && attrs.src) {
      let sourceType = attrs.type === 'module' ? 'module' : 'script';
      let loc = node.location
        ? {
            filePath: asset.filePath,
            start: node.location.start,
            end: {
              line: node.location.end.line,
              // PostHTML's location is inclusive
              column: node.location.end.column + 1,
            },
          }
        : undefined;

      let outputFormat = 'global';
      if (attrs.type === 'module' && asset.env.shouldScopeHoist) {
        outputFormat = 'esmodule';
      } else {
        if (attrs.type === 'module') {
          attrs.defer = '';
        }

        delete attrs.type;
      }

      // If this is a <script type="module">, and not all of the browser targets support ESM natively,
      // add a copy of the script tag with a nomodule attribute.
      let copy: PostHTMLNode | null | undefined;
      if (
        outputFormat === 'esmodule' &&
        !asset.env.supports('esmodules', true)
      ) {
        let attrs = Object.assign({}, node.attrs);
        copy = {...node, attrs};
        delete attrs.type;
        attrs.nomodule = '';
        attrs.defer = '';
        attrs.src = asset.addURLDependency(attrs.src, {
          // Keep in the same bundle group as the HTML.
          priority: 'parallel',
          bundleBehavior:
            sourceType === 'script' || attrs.async != null
              ? 'isolated'
              : undefined,
          env: {
            // @ts-expect-error - TS2322 - Type 'string' is not assignable to type 'SourceType | undefined'.
            sourceType,
            outputFormat: 'global',
            loc,
          },
        });

        seen.add(copy);
      }

      attrs.src = asset.addURLDependency(attrs.src, {
        // Keep in the same bundle group as the HTML.
        priority: 'parallel',
        // If the script is async it can be executed in any order, so it cannot depend
        // on any sibling scripts for dependencies. Keep all dependencies together.
        // Also, don't share dependencies between classic scripts and nomodule scripts
        // because nomodule scripts won't run when modules are supported.
        bundleBehavior:
          sourceType === 'script' || attrs.async != null
            ? 'isolated'
            : undefined,
        env: {
          // @ts-expect-error - TS2322 - Type 'string' is not assignable to type 'SourceType | undefined'.
          sourceType,
          // @ts-expect-error - TS2322 - Type 'string' is not assignable to type 'OutputFormat | undefined'.
          outputFormat,
          loc,
        },
      });

      asset.setAST(ast);
      if (sourceType === 'module') hasModuleScripts = true;
      return copy ? [node, copy] : node;
    }

    for (let attr in attrs) {
      // Check for virtual paths
      if (tag === 'a' && attrs[attr].split('#')[0].lastIndexOf('.') < 1) {
        continue;
      }

      // Check for id references
      if (attrs[attr][0] === '#') {
        continue;
      }

      // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'string' can't be used to index type '{ readonly src: readonly ["script", "img", "audio", "video", "source", "track", "iframe", "embed", "amp-img"]; readonly href: readonly ["link", "a", "use", "script", "image"]; readonly srcset: readonly ["img", "source"]; ... 4 more ...; readonly data: readonly [...]; }'.
      let elements = ATTRS[attr];
      if (elements && elements.includes(node.tag)) {
        // Check for empty string
        if (attrs[attr].length === 0) {
          errors.push({
            message: `'${attr}' should not be empty string`,
            filePath: asset.filePath,
            loc: node.location,
          });
        }

        let depHandler = getAttrDepHandler(attr);
        // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'any' can't be used to index type '{ readonly a: { readonly href: { readonly needsStableName: true; }; }; readonly iframe: { readonly src: { readonly needsStableName: true; }; }; readonly link: (attrs: any) => { priority: string; } | undefined; }'.
        let depOptionsHandler = OPTIONS[node.tag];
        let depOptions =
          typeof depOptionsHandler === 'function'
            ? depOptionsHandler(attrs, asset.env)
            : depOptionsHandler && depOptionsHandler[attr];
        attrs[attr] = depHandler(asset, attrs[attr], depOptions);
        isDirty = true;
      }
    }

    if (isDirty) {
      asset.setAST(ast);
    }

    return node;
  });

  if (errors.length > 0) {
    throw errors;
  }

  return hasModuleScripts;
}
