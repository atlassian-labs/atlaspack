/**
 * Atlaspack transformer for handling external Compiled CSS imports.
 *
 * This is a direct port of @compiled/parcel-transformer-external to Atlaspack,
 * allowing users to transition without any change in functionality.
 *
 * This transformer handles `.compiled.css` imports from pre-compiled packages,
 * strips the import statements, and adds the CSS content to asset.meta.styleRules
 * for later extraction by the optimizer.
 */
import {join, dirname, isAbsolute} from 'path';

import {Transformer} from '@atlaspack/plugin';
import SourceMap from '@atlaspack/source-map';

interface SourcePosition {
  source: string;
  groups: {[key: string]: string} | undefined;
  line: number;
  column: number;
}

function findTargetSourcePositions(
  source: string,
  regex: RegExp,
): SourcePosition[] {
  const lines = source.split('\n');

  const results: SourcePosition[] = [];

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const matches = line.matchAll(regex);

    for (const match of matches) {
      if (match && match.index != null) {
        results.push({
          source: match[0],
          groups: match.groups,
          line: i,
          column: match.index,
        });
      }
    }
  }

  return results;
}

export default new Transformer({
  async transform({asset, options}) {
    let code = await asset.getCode();

    if (code.indexOf('.compiled.css') < 0) {
      // Early exit if no relevant files
      return [asset];
    }

    let map = await asset.getMap();
    for (const match of findTargetSourcePositions(
      code,
      /(import ['"](?<importSpec>.+\.compiled\.css)['"];)|(require\(['"](?<requireSpec>.+\.compiled\.css)['"]\);)/g,
    )) {
      const specifierPath =
        match.groups?.importSpec || match.groups?.requireSpec;
      if (!specifierPath) continue;

      if (asset.env.sourceMap) {
        if (!map) map = new SourceMap(options.projectRoot);

        map.offsetColumns(
          match.line + 1,
          match.column + match.source.length,
          -match.source.length,
        );
      }

      code = code.replace(match.source, '');

      const cssFilePath = isAbsolute(specifierPath)
        ? specifierPath
        : join(dirname(asset.filePath), specifierPath);

      const cssContent = (await asset.fs.readFile(cssFilePath))
        .toString()
        .split('\n');
      if (!asset.meta.styleRules) {
        asset.meta.styleRules = [];
      }
      (asset.meta.styleRules as string[]).push(...cssContent);
    }

    asset.setCode(code);

    if (map) {
      asset.setMap(map);
    }

    return [asset];
  },
});
