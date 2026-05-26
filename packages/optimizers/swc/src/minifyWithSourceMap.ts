import nullthrows from 'nullthrows';
import {transform} from '@swc/core';
import SourceMap from '@atlaspack/source-map';

export type MinifyOptions = {
  /** ES target for SWC. */
  target?: string;
  /** Whether to enable name mangling. */
  mangle?: boolean;
  /** Whether to enable compression. */
  compress?: boolean;
  /** Mark output as a top-level module (esmodule/commonjs). */
  toplevel?: boolean;
  /** Treat the input as an ES module. */
  module?: boolean;
  /** Extra terser-style overrides forwarded into `jsc.minify`. */
  userConfig?: Record<string, unknown>;
  /** Project root used when constructing the SourceMap instance. */
  projectRoot?: string;
};

export type MinifyResult = {
  code: string;
  map: SourceMap | null;
};

/**
 * Minify `code` with SWC and produce a SourceMap whose `original*` positions
 * point back to the sources described by `originalMap`.
 *
 * When `originalMap` is provided, we hand its VLQ-encoded form to SWC via the
 * `inputSourceMap` option. SWC then composes its own (minified -> code) map
 * with the input (code -> original sources) map per-token during
 * minification, producing a final map that points straight back to the
 * original sources.
 *
 * This is much more accurate than composing the maps *after* the fact with
 * `SourceMap.extends(originalMap)`. The post-hoc `extends` approach calls
 * `findClosestMapping`, which snaps each minified token to the previous
 * mapping in the input map. When the input map has many mappings on a single
 * generated line (which is the norm – every asset's tokens get packed onto
 * the same generated line by the packager), "the previous mapping" is often
 * the wrong source/line entirely, producing systematically misaligned maps.
 */
export async function minifyWithSourceMap(
  code: string,
  originalMap: SourceMap | null,
  options: MinifyOptions = {},
): Promise<MinifyResult> {
  const {
    target = 'es2022',
    mangle = true,
    compress = true,
    toplevel = false,
    module: isModule = false,
    userConfig = {},
    projectRoot = '/',
  } = options;

  let inputSourceMap: string | undefined;
  if (originalMap) {
    // Hand the input map to swc as `inputSourceMap` so swc composes the
    // (minified -> code) and (code -> original sources) maps together
    // per-token during minification. This is more accurate than calling
    // `SourceMap.extends(originalMap)` after the fact, which uses
    // `findClosestMapping` and snaps each minified token to the *nearest*
    // mapping in the input map – losing column precision and sometimes
    // mis-attributing tokens across asset boundaries when the input map
    // has gaps. It also leaks swc's placeholder `<anon>` source into the
    // resulting map's `sources` array.
    // swc requires `version: 3` to parse the input map. The atlaspack
    // SourceMap.toVLQ() helper doesn't include it, so we add it here.
    inputSourceMap = JSON.stringify({version: 3, ...originalMap.toVLQ()});
  }

  const result = await transform(code, {
    jsc: {
      target: target as any,
      minify: {
        mangle,
        compress,
        ...(userConfig as object),
        toplevel,
        module: isModule,
      } as any,
    },
    minify: true,
    sourceMaps: true,
    inputSourceMap,
    configFile: false,
    swcrc: false,
  });

  const minifiedCode: string = nullthrows(result.code);
  if (!result.map) {
    return {code: minifiedCode, map: null};
  }

  // With `inputSourceMap` provided, swc has already composed the maps;
  // the result already references the original sources directly.
  const sourceMap = new SourceMap(projectRoot);
  sourceMap.addVLQMap(JSON.parse(result.map));
  return {code: minifiedCode, map: sourceMap};
}
