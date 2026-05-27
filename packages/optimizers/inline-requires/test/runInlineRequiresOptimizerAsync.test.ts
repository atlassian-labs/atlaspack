import assert from 'assert';
// @ts-expect-error - the Rust napi binding has no types for this entry
import {runInlineRequiresOptimizerAsync} from '@atlaspack/rust';
import SourceMap from '@atlaspack/source-map';

const PROJECT_ROOT = '/project';

/**
 * Build a minimal source map for `code` that says "this whole bundle came
 * from src/foo.js, line by line, column 0". This is the kind of map the
 * packager hands the inline-requires optimizer in practice.
 */
function makeInputMap(code: string, sourcePath: string): SourceMap {
  const map = new SourceMap(PROJECT_ROOT);
  const lines = code.split('\n');
  for (let i = 0; i < lines.length; i++) {
    map.addIndexedMapping({
      generated: {line: i + 1, column: 0},
      original: {line: i + 1, column: 0},
      source: sourcePath,
    });
  }
  // Embed the source content so consumers can do lookups without disk access.
  map.setSourceContent(sourcePath, code);
  return map;
}

describe('runInlineRequiresOptimizerAsync (Rust binding)', function () {
  // The Rust binding gained a new optional `inputSourceMap` parameter. When
  // it's supplied, swc composes its (transformed -> input) map with the
  // (input -> original) map natively, producing a result that references
  // the original sources directly. Without it, the result references the
  // synthetic `<anon>` source swc invents for un-named inputs, and the
  // caller must compose post-hoc with `SourceMap.extends()` — which uses a
  // lossy nearest-mapping snap and produces systematic mis-attribution.

  it('returns a code result and a string source map when sourceMaps is enabled', async function () {
    const code = `
parcelRequire.register("a", function (module, exports) {
  exports.foo = 1;
});
parcelRequire.register("b", function (module, exports) {
  var a = parcelRequire("a");
  exports.bar = a.foo + 1;
});
`.trim();

    const result = await runInlineRequiresOptimizerAsync({
      code,
      sourceMaps: true,
      ignoreModuleIds: [],
    });

    assert.strictEqual(typeof result.code, 'string', 'code is a string');
    assert.ok(result.code.length > 0, 'code is non-empty');
    assert.strictEqual(
      typeof result.sourceMap,
      'string',
      'sourceMap is a string when sourceMaps:true',
    );
  });

  it('without inputSourceMap, the result map contains the synthetic <anon> source', async function () {
    const code = `parcelRequire.register("a", function (m, e) { e.x = 1; });
parcelRequire.register("b", function (m, e) { var a = parcelRequire("a"); e.y = a.x; });`;

    const result = await runInlineRequiresOptimizerAsync({
      code,
      sourceMaps: true,
      ignoreModuleIds: [],
    });

    const map = JSON.parse(result.sourceMap!);
    // swc invents "<anon>" when no source file is associated with the input.
    assert.deepStrictEqual(
      map.sources,
      ['<anon>'],
      'sanity: without inputSourceMap, result map references the synthetic <anon> source',
    );
  });

  it('with inputSourceMap, the result map references the original sources (NOT <anon>)', async function () {
    const code = `parcelRequire.register("a", function (m, e) { e.x = 1; });
parcelRequire.register("b", function (m, e) { var a = parcelRequire("a"); e.y = a.x; });`;

    const inputMap = makeInputMap(code, 'src/index.js');
    // swc requires `version: 3` to parse the input map; atlaspack's
    // `toVLQ()` omits it (it's added by the JS wrapper in production).
    const inputSourceMap = JSON.stringify({version: 3, ...inputMap.toVLQ()});

    const result = await runInlineRequiresOptimizerAsync({
      code,
      sourceMaps: true,
      ignoreModuleIds: [],
      inputSourceMap,
    });

    const map = JSON.parse(result.sourceMap!);
    // The result map must now reference the *original* source path that
    // the input map described. It must NOT contain the placeholder
    // `<anon>` source — that would mean swc's native composition didn't
    // run.
    assert.ok(
      map.sources.includes('src/index.js') ||
        // Some swc versions strip the leading directory; accept either.
        map.sources.some((s: string) => s.endsWith('index.js')),
      `result map must include 'src/index.js' (got: ${JSON.stringify(map.sources)})`,
    );
    assert.ok(
      !map.sources.includes('<anon>'),
      `result map must NOT include the synthetic <anon> source (got: ${JSON.stringify(map.sources)})`,
    );
  });

  it('without sourceMaps, the result has no source map regardless of inputSourceMap', async function () {
    const code = `parcelRequire.register("a", function (m, e) { e.x = 1; });`;
    const inputMap = makeInputMap(code, 'src/index.js');
    const inputSourceMap = JSON.stringify({version: 3, ...inputMap.toVLQ()});

    const result = await runInlineRequiresOptimizerAsync({
      code,
      sourceMaps: false,
      ignoreModuleIds: [],
      inputSourceMap,
    });

    assert.ok(
      result.sourceMap == null,
      'sourceMap is null/undefined when sourceMaps:false',
    );
  });
});
