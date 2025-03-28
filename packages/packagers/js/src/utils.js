// @flow
import type {BundleGraph, Dependency, NamedBundle} from '@atlaspack/types';
import type SourceMap from '@parcel/source-map';
import nullthrows from 'nullthrows';

// This replaces __parcel__require__ references left by the transformer with
// parcelRequire calls of the resolved asset id. This lets runtimes work within
// script bundles, which must be outside the bundle wrapper so their variables are global.
export function replaceScriptDependencies(
  bundleGraph: BundleGraph<NamedBundle>,
  bundle: NamedBundle,
  code: string,
  map: ?SourceMap,
  parcelRequireName: string,
): string {
  let entry = nullthrows(bundle.getMainEntry());
  let dependencies = bundleGraph.getDependencies(entry);

  let lineCount = 0;
  let offset = 0;
  let columnStartIndex = 0;
  code = code.replace(/\n|__parcel__require__\(['"](.*?)['"]\)/g, (m, s, i) => {
    if (m === '\n') {
      columnStartIndex = i + offset + 1;
      lineCount++;
      return '\n';
    }

    let dep = nullthrows(dependencies.find((d) => getSpecifier(d) === s));
    let resolved = nullthrows(bundleGraph.getResolvedAsset(dep, bundle));
    let publicId = bundleGraph.getAssetPublicId(resolved);
    let replacement = `${parcelRequireName}("${publicId}")`;
    if (map) {
      let lengthDifference = replacement.length - m.length;
      if (lengthDifference !== 0) {
        map.offsetColumns(
          lineCount + 1,
          i + offset - columnStartIndex + m.length,
          lengthDifference,
        );
        offset += lengthDifference;
      }
    }

    return replacement;
  });

  return code;
}

export function getSpecifier(dep: Dependency): string {
  if (typeof dep.meta.placeholder === 'string') {
    return dep.meta.placeholder;
  }

  return dep.specifier;
}

// https://262.ecma-international.org/6.0/#sec-names-and-keywords
const IDENTIFIER_RE = /^[$_\p{ID_Start}][$_\u200C\u200D\p{ID_Continue}]*$/u;
const ID_START_RE = /^[$_\p{ID_Start}]/u;
const NON_ID_CONTINUE_RE = /[^$_\u200C\u200D\p{ID_Continue}]/gu;

export function isValidIdentifier(id: string): boolean {
  return IDENTIFIER_RE.test(id);
}

export function makeValidIdentifier(name: string): string {
  name = name.replace(NON_ID_CONTINUE_RE, '');
  if (!ID_START_RE.test(name)) {
    name = '_' + name;
  }
  return name;
}
