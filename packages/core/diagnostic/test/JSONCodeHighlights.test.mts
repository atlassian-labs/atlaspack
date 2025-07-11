import assert from 'assert';

import {generateJSONCodeHighlights} from '../src/index.mts';

describe('generateJSONCodeHighlights', () => {
  it('returns an escaped string 01', () => {
    const result = generateJSONCodeHighlights(
      `{
  "a": 1
}`,
      [
        {key: '/a', type: 'key', message: 'foo1'},
        {key: '/a', type: 'value', message: 'foo2'},
        {key: '/a', message: 'foo3'},
      ],
    );
    assert.deepEqual(result, [
      {
        start: {line: 2, column: 3},
        end: {line: 2, column: 5},
        message: 'foo1',
      },
      {
        start: {line: 2, column: 8},
        end: {line: 2, column: 8},
        message: 'foo2',
      },
      {
        start: {line: 2, column: 3},
        end: {line: 2, column: 8},
        message: 'foo3',
      },
    ]);
  });
});
