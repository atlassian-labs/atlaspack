import assert from 'assert';
import SourceMap from '@atlaspack/source-map';

import {minifyWithSourceMap} from '../src/minifyWithSourceMap';

const PROJECT_ROOT = '/project';

/**
 * Walk the bundle for a set of literal strings, and for each one, resolve its
 * position via the source map and check that the resolved original source line
 * actually contains that string. This is the same "string-literal probe"
 * technique that exposed the real-world misalignment.
 *
 * Returns { matched, total, mismatches } so tests can assert specific bounds.
 */
function probeStringLiterals(opts: {
  bundle: string;
  map: SourceMap;
  /**
   * Map of source name → expected (line, col) of each probe within that source.
   * The probe is considered "correctly aligned" when the source map's reverse
   * lookup returns the exact (source, line, col) where the probe actually
   * lives.
   */
  expected: Record<
    string,
    {
      source: string;
      line: number;
      column: number;
    }
  >;
  probes: string[];
}): Promise<{
  matched: number;
  total: number;
  mismatches: Array<{
    text: string;
    bundleLine: number;
    bundleCol: number;
    resolved: {source?: string; line?: number; column?: number} | null;
    expected: {source: string; line: number; column: number};
  }>;
}> {
  const bundleLines = opts.bundle.split('\n');
  let matched = 0;
  const mismatches: Array<any> = [];

  for (const probe of opts.probes) {
    const expected = opts.expected[probe];
    let found = false;
    for (let l = 0; l < bundleLines.length; l++) {
      const re = new RegExp(`['"\`]${probe}['"\`]`);
      const m = bundleLines[l].match(re);
      if (!m) continue;
      const idx = (m.index ?? 0) + 1; // step past the quote so we land on the literal text
      const resolved = opts.map.findClosestMapping(l + 1, idx);
      const resolvedSource = resolved?.source;
      const resolvedLine = resolved?.original?.line;
      const resolvedCol = resolved?.original?.column;
      // We require the resolved (source, line, column) to match the
      // expected position exactly. The buggy `extends()` approach loses
      // column precision (it snaps to the nearest input mapping), so this
      // check is what surfaces the bug.
      const ok =
        resolvedSource === expected.source &&
        resolvedLine === expected.line &&
        resolvedCol === expected.column;
      if (ok) {
        matched++;
      } else {
        mismatches.push({
          text: probe,
          bundleLine: l + 1,
          bundleCol: idx,
          resolved: {
            source: resolvedSource,
            line: resolvedLine,
            column: resolvedCol,
          },
          expected,
        });
      }
      found = true;
      break;
    }
    if (!found) {
      mismatches.push({
        text: probe,
        bundleLine: -1,
        bundleCol: -1,
        resolved: null,
        expected,
      });
    }
  }

  return {matched, total: opts.probes.length, mismatches};
}

describe('minifyWithSourceMap', () => {
  it('passes through when source map composition is not needed', async () => {
    const code = 'console.log("hello");';
    const out = await minifyWithSourceMap(code, null, {
      projectRoot: PROJECT_ROOT,
    });
    assert.ok(out.code.includes('"hello"'));
  });

  it('forwards the input map to swc as `inputSourceMap` (so map composition happens during minify, not as a post-hoc snap)', async () => {
    // This is a regression test for a real Confluence production bug:
    // `SwcOptimizer` used to compose its (minified -> code) map with the
    // packager's (code -> original sources) map by calling
    // `SourceMap.extends(originalMap)` *after* swc had finished minifying.
    //
    // That post-hoc composition uses `findClosestMapping`, which:
    //  - snaps to the previous mapping when the looked-up position has no
    //    exact mapping (losing column precision), and
    //  - can snap *across asset boundaries* when the input map has gaps
    //    (mis-attributing a token to the wrong source file).
    //
    // String-literal probes of real Confluence bundles showed 30–65 % of
    // mappings resolving to a source line that did not actually contain
    // the literal – evidence the maps were systematically wrong.
    //
    // The fix is to hand the input map to swc via the `inputSourceMap`
    // option, so swc composes the maps per-token natively during
    // minification.
    //
    // We can't easily reproduce the bug end-to-end here with synthetic data
    // (the test inputs are too regular for `findClosestMapping`'s snap-back
    // to produce visibly wrong output), so the test focuses on the
    // observable contract: when an input map is provided, the resulting
    // map's `sources` array must contain the original sources, and the
    // resulting map's mappings must successfully reverse-resolve a probe
    // back to one of those original sources.

    const source = [
      'console.log("alpha");',
      'console.log("beta");',
      'console.log("gamma");',
    ].join('\n');
    const sourceName = 'src/widget.js';

    const originalMap = new SourceMap(PROJECT_ROOT);
    originalMap.addIndexedMappings([
      {
        generated: {line: 1, column: 0},
        original: {line: 1, column: 0},
        source: sourceName,
      },
      {
        generated: {line: 2, column: 0},
        original: {line: 2, column: 0},
        source: sourceName,
      },
      {
        generated: {line: 3, column: 0},
        original: {line: 3, column: 0},
        source: sourceName,
      },
    ]);
    originalMap.setSourceContent(sourceName, source);

    const out = await minifyWithSourceMap(source, originalMap, {
      projectRoot: PROJECT_ROOT,
      mangle: false,
      compress: true,
    });

    assert.ok(out.map, 'expected a source map to be produced');

    const vlq = out.map!.toVLQ();
    // The output map must reference the ORIGINAL source file. The post-hoc
    // `extends()` path leaks `<anon>` (swc's placeholder source name for the
    // input it was given) into the sources array, alongside the real source
    // name from the input map. With `inputSourceMap` swc knows the real
    // source names up-front and emits only those.
    assert.ok(
      vlq.sources.includes(sourceName),
      `expected output map.sources to include ${sourceName}, got ${JSON.stringify(vlq.sources)}`,
    );
    assert.ok(
      !vlq.sources.some((s) => s === '<anon>' || s === '<source>'),
      `output map.sources must not contain swc placeholder source names, got ${JSON.stringify(vlq.sources)}`,
    );

    // Reverse-resolve each probe and check it lands on src/widget.js at
    // the right line.
    for (const probe of ['alpha', 'beta', 'gamma']) {
      const probeIdx = out.code.indexOf(`"${probe}"`);
      assert.ok(probeIdx >= 0, `probe ${probe} should be in minified code`);
      const resolved = out.map!.findClosestMapping(1, probeIdx + 1);
      assert.strictEqual(
        resolved?.source,
        sourceName,
        `probe ${probe} should resolve to ${sourceName}, got ${resolved?.source}`,
      );
      // Each probe lives on its own original line.
      const expectedLine = ['alpha', 'beta', 'gamma'].indexOf(probe) + 1;
      assert.strictEqual(
        resolved?.original?.line,
        expectedLine,
        `probe ${probe} should resolve to line ${expectedLine}, got ${resolved?.original?.line}`,
      );
    }
  });

  it('produces a source map that correctly resolves string literals back to their original lines', async () => {
    // Three source modules, each with several "log calls" on a single line.
    // This mirrors real-world bundles where the packager produces source maps
    // with multiple mappings per generated line: each asset's tokens get
    // packed onto one line of the bundle and several mappings share that
    // line.
    //
    // The optimizer must compose its (minified -> bundle) map with the input
    // (bundle -> original sources) map per-token. The buggy implementation
    // uses `SourceMap.extends(originalMap)`, which calls `findClosestMapping`
    // on the original map – and that snaps to the previous mapping in the
    // map. When the input map has gaps (e.g. an asset's prologue line has no
    // mapping, or the next asset's first line has no leading mapping), the
    // snap goes backwards across an asset boundary and the minified token
    // ends up attributed to the wrong source file entirely.
    type Mod = {sourceName: string; source: string; probes: string[]};

    const buildMod = (sourceName: string, prefix: string): Mod => {
      // 4 lines, each with 5 distinct console.log statements. Multiple
      // mappings per generated line at the column of each log call.
      const lines: string[] = [];
      const probes: string[] = [];
      for (let l = 1; l <= 4; l++) {
        const calls: string[] = [];
        for (let c = 1; c <= 5; c++) {
          const probe = `${prefix}_l${l}c${c}`;
          probes.push(probe);
          calls.push(`console.log("${probe}");`);
        }
        lines.push(calls.join(' '));
      }
      return {sourceName, source: lines.join('\n'), probes};
    };

    const mods: Mod[] = [
      buildMod('src/a.js', 'A'),
      buildMod('src/b.js', 'B'),
      buildMod('src/c.js', 'C'),
    ];

    // Build the bundle by concatenating each module's source *onto a single
    // line*, separated by a `;` – the way scope-hoisting bundlers often do
    // it. The mappings still reference the original module's per-line, per-
    // call positions, so the input map ends up with all mappings on
    // generated line 1, but the column ranges interleave across the three
    // sources.
    //
    // This is the structure that exposes the bug: when the minifier later
    // emits a mapping at some column, the buggy `extends()` snaps to the
    // closest mapping by *column*, which – with sparse mappings spanning
    // multiple source files on a single line – often lands on the
    // *previous source file entirely*.
    const indexedMappings: Array<{
      generated: {line: number; column: number};
      original: {line: number; column: number};
      source: string;
      name?: string;
    }> = [];

    // Concatenate each module onto separate bundle lines, separating
    // assets with a blank "boundary" line that has NO mapping in the input
    // map. Real packagers insert such gaps (e.g. blank lines between
    // assets, asset wrappers like `var $abc$exports = ...` that aren't
    // mapped back to a source).
    //
    // This is what reproduces the production bug: when SWC emits a mapping
    // at minified column X claiming it came from the gap line, the buggy
    // `extends()` approach snaps to the previous mapping on a different
    // bundle line entirely, mis-attributing the token.
    let lineOffset = 0;
    const bundleParts: string[] = [];
    for (const mod of mods) {
      bundleParts.push(mod.source);
      const lines = mod.source.split('\n');
      for (let l = 0; l < lines.length; l++) {
        const line = lines[l];
        const re = /console\.log\("[^"]+"\);?/g;
        let m;
        while ((m = re.exec(line))) {
          indexedMappings.push({
            generated: {line: l + 1 + lineOffset, column: m.index},
            original: {line: l + 1, column: m.index},
            source: mod.sourceName,
          });
        }
      }
      // +1 for the joining blank line below
      lineOffset += lines.length + 1;
    }
    // The blank lines between assets have NO mapping – the input map has a
    // gap that the buggy extends() will snap across.
    const bundleInput = bundleParts.join('\n\n');

    const originalMap = new SourceMap(PROJECT_ROOT);
    originalMap.addIndexedMappings(indexedMappings);
    for (const mod of mods) {
      originalMap.setSourceContent(mod.sourceName, mod.source);
    }

    const out = await minifyWithSourceMap(bundleInput, originalMap, {
      projectRoot: PROJECT_ROOT,
      mangle: false,
      compress: true,
    });

    assert.ok(out.map, 'expected a source map to be produced');

    // Each probe lives in a `console.log("<probe>");` statement at a known
    // (source, line, column) of its original module. Since the input map
    // has a mapping at the START of each `console.log` call, we expect the
    // post-minify map to reverse-resolve each probe to:
    //   - the right source file
    //   - the right line within that source
    //   - the column where the matching `console.log` call starts
    const allProbes = mods.flatMap((m) => m.probes);
    const expected: Record<
      string,
      {source: string; line: number; column: number}
    > = {};
    for (const mod of mods) {
      const lines = mod.source.split('\n');
      for (let l = 0; l < lines.length; l++) {
        const line = lines[l];
        for (const probe of mod.probes) {
          const idx = line.indexOf(`console.log("${probe}")`);
          if (idx >= 0) {
            expected[probe] = {
              source: mod.sourceName,
              line: l + 1,
              column: idx,
            };
          }
        }
      }
    }

    const result = probeStringLiterals({
      bundle: out.code,
      map: out.map!,
      expected,
      probes: allProbes,
    });

    if (result.matched !== result.total) {
      const summary = result.mismatches
        .slice(0, 8)
        .map(
          (m) =>
            `  - ${m.text} @ bundle ${m.bundleLine}:${m.bundleCol} -> ` +
            `resolved=${JSON.stringify(m.resolved)} expected=${JSON.stringify(m.expected)}`,
        )
        .join('\n');
      throw new Error(
        `Source map misaligned: only ${result.matched}/${result.total} string literals ` +
          `resolved to the exact original (source, line, column) where they live.\n` +
          `First mismatches:\n${summary}`,
      );
    }

    assert.strictEqual(result.matched, result.total);
  });
});
