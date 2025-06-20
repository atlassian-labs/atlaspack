import assert from 'assert';
import {readFileSync} from 'fs';
import {join as joinPath} from 'path';

import codeframe from '../src/index.mts';

const LINE_END = '\n';

describe('codeframe', () => {
  it('should create a codeframe', () => {
    const codeframeString = codeframe(
      'hello world',
      [
        {
          start: {
            column: 1,
            line: 1,
          },
          end: {
            column: 1,
            line: 1,
          },
        },
        {
          start: {
            column: 3,
            line: 1,
          },
          end: {
            column: 5,
            line: 1,
          },
        },
      ],
      {useColor: false},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines[0], '> 1 | hello world');
    assert.equal(lines[1], '>   | ^ ^^^');
  });

  it('should create a codeframe with multiple lines', () => {
    const codeframeString = codeframe(
      'hello world\nEnjoy this nice codeframe',
      [
        {
          start: {
            column: 1,
            line: 1,
          },
          end: {
            column: 1,
            line: 1,
          },
        },
        {
          start: {
            column: 7,
            line: 1,
          },
          end: {
            column: 10,
            line: 2,
          },
        },
      ],
      {useColor: false},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines[0], '> 1 | hello world');
    assert.equal(lines[1], '>   | ^     ^^^^^');
    assert.equal(lines[2], '> 2 | Enjoy this nice codeframe');
    assert.equal(lines[3], '>   | ^^^^^^^^^^');
  });

  it('should handle unordered overlapping highlights properly', () => {
    const codeframeString = codeframe(
      'hello world\nEnjoy this nice codeframe',
      [
        {
          start: {
            column: 1,
            line: 1,
          },
          end: {
            column: 1,
            line: 1,
          },
        },
        {
          start: {
            column: 7,
            line: 1,
          },
          end: {
            column: 10,
            line: 2,
          },
        },
        {
          start: {
            column: 4,
            line: 2,
          },
          end: {
            column: 7,
            line: 2,
          },
        },
      ],
      {useColor: false},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines[0], '> 1 | hello world');
    assert.equal(lines[1], '>   | ^     ^^^^^');
    assert.equal(lines[2], '> 2 | Enjoy this nice codeframe');
    assert.equal(lines[3], '>   | ^^^^^^^^^^');
  });

  it('should handle partial overlapping highlights properly', () => {
    const codeframeString = codeframe(
      'hello world\nEnjoy this nice codeframe',
      [
        {
          start: {
            column: 1,
            line: 1,
          },
          end: {
            column: 1,
            line: 1,
          },
        },
        {
          start: {
            column: 7,
            line: 1,
          },
          end: {
            column: 10,
            line: 2,
          },
        },
        {
          start: {
            column: 4,
            line: 2,
          },
          end: {
            column: 12,
            line: 2,
          },
        },
      ],
      {useColor: false},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines[0], '> 1 | hello world');
    assert.equal(lines[1], '>   | ^     ^^^^^');
    assert.equal(lines[2], '> 2 | Enjoy this nice codeframe');
    assert.equal(lines[3], '>   | ^^^^^^^^^^^^');
  });

  it('should be able to render inline messages', () => {
    const codeframeString = codeframe(
      'hello world\nEnjoy this nice codeframe',
      [
        {
          start: {
            column: 1,
            line: 1,
          },
          end: {
            column: 6,
            line: 1,
          },
          message: 'test',
        },
      ],
      {useColor: false},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines[0], '> 1 | hello world');
    assert.equal(lines[1], '>   | ^^^^^^ test');
    assert.equal(lines[2], '  2 | Enjoy this nice codeframe');
  });

  it('should only render last inline message of a column', () => {
    const codeframeString = codeframe(
      'hello world\nEnjoy this nice codeframe',
      [
        {
          start: {
            column: 1,
            line: 1,
          },
          end: {
            column: 3,
            line: 1,
          },
          message: 'test',
        },
        {
          start: {
            column: 1,
            line: 1,
          },
          end: {
            column: 6,
            line: 1,
          },
          message: 'this should be printed',
        },
      ],
      {useColor: false},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines[0], '> 1 | hello world');
    assert.equal(lines[1], '>   | ^^^^^^ this should be printed');
    assert.equal(lines[2], '  2 | Enjoy this nice codeframe');
  });

  it('should only render last inline message of a column with space', () => {
    const codeframeString = codeframe(
      'hello world\nEnjoy this nice codeframe',
      [
        {
          start: {
            column: 1,
            line: 1,
          },
          end: {
            column: 1,
            line: 1,
          },
          message: 'test',
        },
        {
          start: {
            column: 3,
            line: 1,
          },
          end: {
            column: 7,
            line: 1,
          },
          message: 'this should be printed',
        },
      ],
      {useColor: false},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines[0], '> 1 | hello world');
    assert.equal(lines[1], '>   | ^ ^^^^^ this should be printed');
    assert.equal(lines[2], '  2 | Enjoy this nice codeframe');
  });

  it('should only render last inline message of a column with multiple lines and space', () => {
    const codeframeString = codeframe(
      'hello world\nEnjoy this nice codeframe\nThis is another line',
      [
        {
          start: {
            column: 1,
            line: 1,
          },
          end: {
            column: 1,
            line: 1,
          },
          message: 'test',
        },
        {
          start: {
            column: 3,
            line: 1,
          },
          end: {
            column: 7,
            line: 1,
          },
          message: 'this should be printed',
        },
        {
          start: {
            column: 3,
            line: 2,
          },
          end: {
            column: 7,
            line: 3,
          },
          message: 'message line 2',
        },
      ],
      {useColor: false},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines[0], '> 1 | hello world');
    assert.equal(lines[1], '>   | ^ ^^^^^ this should be printed');
    assert.equal(lines[2], '> 2 | Enjoy this nice codeframe');
    assert.equal(lines[3], '>   |   ^^^^^^^^^^^^^^^^^^^^^^^');
    assert.equal(lines[4], '> 3 | This is another line');
    assert.equal(lines[5], '>   | ^^^^^^^ message line 2');
  });

  it('should only render last inline message of a column with multiple lines and space', () => {
    const codeframeString = codeframe(
      'hello world\nEnjoy this nice codeframe\nThis is another line',
      [
        {
          start: {
            column: 1,
            line: 1,
          },
          end: {
            column: 1,
            line: 1,
          },
          message: 'test',
        },
        {
          start: {
            column: 3,
            line: 1,
          },
          end: {
            column: 7,
            line: 1,
          },
          message: 'this should be printed',
        },
        {
          start: {
            column: 3,
            line: 2,
          },
          end: {
            column: 7,
            line: 3,
          },
          message: 'message line 2',
        },
      ],
      {useColor: false},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines[0], '> 1 | hello world');
    assert.equal(lines[1], '>   | ^ ^^^^^ this should be printed');
    assert.equal(lines[2], '> 2 | Enjoy this nice codeframe');
    assert.equal(lines[3], '>   |   ^^^^^^^^^^^^^^^^^^^^^^^');
    assert.equal(lines[4], '> 3 | This is another line');
    assert.equal(lines[5], '>   | ^^^^^^^ message line 2');
  });

  it('should properly use padding', () => {
    const codeframeString = codeframe(
      'test\n'.repeat(100),
      [
        {
          start: {
            column: 2,
            line: 5,
          },
          end: {
            column: 2,
            line: 5,
          },
          message: 'test',
        },
      ],
      {
        useColor: false,
        padding: {
          before: 2,
          after: 4,
        },
      },
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines.length, 8);
    assert.equal(lines[0], '  3 | test');
    assert.equal(lines[2], '> 5 | test');
    assert.equal(lines[3], '>   |  ^ test');
    assert.equal(lines[7], '  9 | test');
  });

  it('should properly pad numbers for large files', () => {
    const codeframeString = codeframe('test\n'.repeat(1000), [
      {
        start: {
          column: 2,
          line: 99,
        },
        end: {
          column: 2,
          line: 99,
        },
        message: 'test',
      },
      {
        start: {
          column: 2,
          line: 100,
        },
        end: {
          column: 2,
          line: 100,
        },
        message: 'test 2',
      },
    ]);

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines.length, 7);
    assert.equal(lines[0], '   98 | test');
    assert.equal(lines[1], '>  99 | test');
    assert.equal(lines[2], '>     |  ^ test');
    assert.equal(lines[3], '> 100 | test');
    assert.equal(lines[4], '>     |  ^ test 2');
    assert.equal(lines[5], '  101 | test');
    assert.equal(lines[6], '  102 | test');
  });

  it('should properly pad numbers for short files', () => {
    const codeframeString = codeframe('test\n'.repeat(1000), [
      {
        start: {
          column: 2,
          line: 7,
        },
        end: {
          column: 2,
          line: 7,
        },
        message: 'test',
      },
      {
        start: {
          column: 2,
          line: 12,
        },
        end: {
          column: 2,
          line: 12,
        },
        message: 'test',
      },
    ]);

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines.length, 11);
    assert.equal(lines[0], '   6 | test');
    assert.equal(lines[4], '   9 | test');
    assert.equal(lines[5], '  10 | test');
    assert.equal(lines[6], '  11 | test');
    assert.equal(lines[10], '  14 | test');
  });

  it('should properly use maxLines', () => {
    const line = 'test '.repeat(100);
    const codeframeString = codeframe(
      `${line}\n`.repeat(100),
      [
        {
          start: {
            column: 2,
            line: 5,
          },
          end: {
            column: 2,
            line: 5,
          },
          message: 'test',
        },
        {
          start: {
            column: 2,
            line: 12,
          },
          end: {
            column: 2,
            line: 20,
          },
          message: 'test',
        },
      ],
      {
        useColor: false,
        maxLines: 10,
        terminalWidth: 5,
      },
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines.length, 13);
    assert.equal(lines[0], '   4 | test test ');
    assert.equal(lines[7], '  10 | test test ');
    assert.equal(lines[11], '> 13 | test test ');
    assert.equal(lines[12], '>    | ^^^^^^^^^^');
  });

  it('should be able to handle tabs', () => {
    const codeframeString = codeframe(
      'hel\tlo wor\tld\nEnjoy thi\ts nice cod\teframe',
      [
        {
          start: {
            column: 5,
            line: 1,
          },
          end: {
            column: 8,
            line: 1,
          },
          message: 'test',
        },
      ],
      {useColor: false},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines[0], '> 1 | hel  lo wor  ld');
    assert.equal(lines[1], '>   |      ^^^^ test');
    assert.equal(lines[2], '  2 | Enjoy thi  s nice cod  eframe');
  });

  it('should be able to handle tabs with multiple highlights', () => {
    const codeframeString = codeframe(
      'hel\tlo wor\tld\nEnjoy thi\ts nice cod\teframe',
      [
        {
          start: {
            column: 3,
            line: 1,
          },
          end: {
            column: 5,
            line: 1,
          },
          message: 'test',
        },
        {
          start: {
            column: 7,
            line: 1,
          },
          end: {
            column: 8,
            line: 1,
          },
          message: 'test',
        },
      ],
      {useColor: false},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines[0], '> 1 | hel  lo wor  ld');
    assert.equal(lines[1], '>   |   ^^^^ ^^ test');
    assert.equal(lines[2], '  2 | Enjoy thi  s nice cod  eframe');
  });

  it('multiline highlights with tabs', () => {
    const codeframeString = codeframe(
      'hel\tlo wor\tld\nEnjoy thi\ts nice cod\teframe\ntest',
      [
        {
          start: {
            column: 3,
            line: 1,
          },
          end: {
            column: 2,
            line: 3,
          },
          message: 'test',
        },
      ],
      {useColor: false},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines[0], '> 1 | hel  lo wor  ld');
    assert.equal(lines[1], '>   |   ^^^^^^^^^^^^^');
    assert.equal(lines[2], '> 2 | Enjoy thi  s nice cod  eframe');
    assert.equal(lines[3], '>   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^');
    assert.equal(lines[4], '> 3 | test');
    assert.equal(lines[5], '>   | ^^ test');
  });

  it('Should truncate long lines and print message', () => {
    const originalLine = 'hello world '.repeat(1000);
    const codeframeString = codeframe(
      originalLine,
      [
        {
          start: {
            column: 1000,
            line: 1,
          },
          end: {
            column: 1200,
            line: 1,
          },
          message: 'This is a message',
        },
      ],
      {useColor: false, terminalWidth: 25},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines.length, 2);
    assert.equal(lines[0], '> 1 | d hello world hello');
    assert.equal(lines[1], '>   |      ^^^^^^^^^^^^^^ This is a message');
  });

  it('Truncation across multiple lines', () => {
    const originalLine =
      'hello world '.repeat(100) + '\n' + 'new line '.repeat(100);
    const codeframeString = codeframe(
      originalLine,
      [
        {
          start: {
            column: 15,
            line: 1,
          },
          end: {
            column: 400,
            line: 1,
          },
          message: 'This is the first line',
        },
        {
          start: {
            column: 2,
            line: 2,
          },
          end: {
            column: 100,
            line: 2,
          },
          message: 'This is the second line',
        },
      ],
      {useColor: false, terminalWidth: 25},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines.length, 4);
    assert.equal(lines[0], '> 1 | ld hello world hell');
    assert.equal(lines[1], '>   |      ^^^^^^^^^^^^^^ This is the first line');
    assert.equal(lines[2], '> 2 | new line new line n');
    assert.equal(lines[3], '>   |  ^^^^^^^^^^^^^^^^^^ This is the second line');
  });

  it('Truncation across various types and positions of highlights', () => {
    const originalLine =
      'hello world '.repeat(100) + '\n' + 'new line '.repeat(100);
    const codeframeString = codeframe(
      originalLine,
      [
        {
          start: {
            column: 2,
            line: 1,
          },
          end: {
            column: 5,
            line: 1,
          },
        },
        {
          start: {
            column: 6,
            line: 1,
          },
          end: {
            column: 10,
            line: 1,
          },
          message: 'I have a message',
        },
        {
          start: {
            column: 15,
            line: 1,
          },
          end: {
            column: 25,
            line: 1,
          },
          message: 'I also have a message',
        },
        {
          start: {
            column: 2,
            line: 2,
          },
          end: {
            column: 5,
            line: 2,
          },
          message: 'This is the second line',
        },
      ],
      {useColor: false, terminalWidth: 25},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines.length, 4);
    assert.equal(lines[0], '> 1 | hello world hello w');
    assert.equal(lines[1], '>   |  ^^^^^^^^^    ^^^^^ I also have a message');
    assert.equal(lines[2], '> 2 | new line new line n');
    assert.equal(lines[3], '>   |  ^^^^ This is the second line');
  });

  it('Multi-line highlight w/ truncation', () => {
    const originalLine =
      'hello world '.repeat(100) + '\n' + 'new line '.repeat(100);
    const codeframeString = codeframe(
      originalLine,
      [
        {
          start: {
            column: 2,
            line: 1,
          },
          end: {
            column: 151,
            line: 2,
          },
          message: 'I have a message',
        },
      ],
      {useColor: false, terminalWidth: 25},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines.length, 4);
    assert.equal(lines[0], '> 1 | hello world hello w');
    assert.equal(lines[1], '>   |  ^^^^^^^^^^^^^^^^^^');
    assert.equal(lines[2], '> 2 | ew line new line ne');
    assert.equal(lines[3], '>   | ^^^^^^ I have a message');
  });

  it('Should pad properly, T-650', () => {
    const fileContent = readFileSync(
      joinPath(__dirname, './fixtures/a.js'),
      'utf8',
    );
    const codeframeString = codeframe(
      fileContent,
      [
        {
          start: {
            line: 8,
            column: 10,
          },
          end: {
            line: 8,
            column: 48,
          },
        },
      ],
      {
        useColor: false,
        syntaxHighlighting: false,
        language: 'js',
        terminalWidth: 100,
      },
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines.length, 5);
    assert.equal(lines[0], `   7 | import Tooltip from '../tooltip';`);
    assert.equal(
      lines[1],
      `>  8 | import VisuallyHidden from '../visually-hidden';`,
    );
    assert.equal(
      lines[2],
      '>    |          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^',
    );
    assert.equal(lines[3], '   9 | ');
    assert.equal(lines[4], '  10 | /**');
  });

  it('should still generate a codeframe when end is before start', () => {
    const codeframeString = codeframe(
      'hello world',
      [
        {
          start: {
            column: 5,
            line: 1,
          },
          end: {
            column: 1,
            line: 1,
          },
        },
      ],
      {useColor: false},
    );

    const lines = codeframeString.split(LINE_END);
    assert.equal(lines[0], '> 1 | hello world');
    assert.equal(lines[1], '>   |     ^');
  });
});
