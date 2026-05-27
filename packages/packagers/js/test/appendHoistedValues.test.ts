import assert from 'assert';

import {appendHoistedValues} from '../src/ScopeHoistingPackager';

describe('appendHoistedValues', function () {
  // This helper exists because the previous implementation in
  // `getHoistedParcelRequires` would emit `res += '\n' + ''` and then
  // `lineCount += hoisted.size` whenever the filtered hoisted set was empty
  // (or smaller than the unfiltered set). Both bugs desynchronised the
  // source-map line-offset bookkeeping in `ScopeHoistingPackager`. These
  // tests pin the corrected behaviour.

  it('returns an empty result and zero lineCount when given no values', function () {
    const out = appendHoistedValues([]);
    assert.strictEqual(
      out.text,
      '',
      'no text should be appended when there are zero hoisted values (avoids stray leading \\n)',
    );
    assert.strictEqual(
      out.lineCount,
      0,
      'lineCount must be 0 when no values were appended (was previously off-by-`hoisted.size`)',
    );
  });

  it('emits exactly one leading \\n plus the joined values, with lineCount equal to value count', function () {
    const out = appendHoistedValues([
      'parcelRequire("aaaaaaa");',
      'parcelRequire("bbbbbbb");',
      'parcelRequire("ccccccc");',
    ]);
    assert.strictEqual(
      out.text,
      '\nparcelRequire("aaaaaaa");\nparcelRequire("bbbbbbb");\nparcelRequire("ccccccc");',
      'joined values must be prefixed with one \\n and separated by \\n',
    );
    assert.strictEqual(
      out.lineCount,
      3,
      'lineCount must equal the number of newlines actually written (3 for 3 values: 1 leading + 2 separators = 3 \\n)',
    );
    // Cross-check: count actual newlines in the returned text.
    assert.strictEqual(
      out.text.split('\n').length - 1,
      out.lineCount,
      'lineCount must equal the actual number of \\n characters in the appended text',
    );
  });

  it('handles a single value correctly', function () {
    const out = appendHoistedValues(['parcelRequire("only");']);
    assert.strictEqual(out.text, '\nparcelRequire("only");');
    assert.strictEqual(out.lineCount, 1);
    assert.strictEqual(
      out.text.split('\n').length - 1,
      out.lineCount,
      'lineCount must equal the actual number of \\n characters in the appended text',
    );
  });

  it('lineCount always equals the newline count of the emitted text', function () {
    // Property: for *any* input, the lineCount must match the actual number
    // of newline characters in `text`. This is the invariant the
    // ScopeHoistingPackager relies on to track lineOffset for the merged
    // source map. The previous bug violated this invariant when filtering
    // dropped some entries (lineCount was `hoisted.size`, but text only
    // contained `filteredValues.length` newlines).
    for (const values of [
      [],
      ['x'],
      ['x', 'y'],
      ['x', 'y', 'z'],
      ['p\nq', 'r\ns'], // values containing embedded newlines — lineCount
      // intentionally tracks only the separator newlines we emit (1 leading
      // + N-1 separators = N), NOT the embedded ones; the caller is
      // responsible for not embedding extra newlines in hoisted require
      // strings. Document the assumption with this guard:
    ]) {
      const out = appendHoistedValues(values);
      if (values.length === 0) {
        assert.strictEqual(out.text, '');
        assert.strictEqual(out.lineCount, 0);
      } else if (values.every((v) => !v.includes('\n'))) {
        const actualNewlines = out.text.split('\n').length - 1;
        assert.strictEqual(
          out.lineCount,
          actualNewlines,
          `lineCount mismatch for values=${JSON.stringify(values)}: claimed ${out.lineCount}, actual ${actualNewlines}`,
        );
      }
    }
  });
});
