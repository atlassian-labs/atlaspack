import assert from 'assert';
import invariant from 'assert';
import path from 'path';
import {
  bundle,
  bundler,
  describe,
  it,
  run,
  overlayFS,
  fsFixture,
  getNextBuild,
} from '@atlaspack/test-utils';

describe.v2('macros', function () {
  let count = 0;
  // @ts-expect-error - TS7034 - Variable 'dir' implicitly has type 'any' in some locations where its type cannot be determined.
  let dir;
  beforeEach(async () => {
    dir = path.join(__dirname, 'macros', '' + ++count);
    await overlayFS.mkdirp(dir);
  });

  after(async () => {
    await overlayFS.rimraf(path.join(__dirname, 'macros'));
  });

  it('should support named imports', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { hash } from "./macro" with { type: "macro" };
        output = hash('hi');

      macro.js:
        import {hashString} from '@atlaspack/rust';
        export function hash(s) {
          return hashString(s);
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('output="2a2300bbd7ea6e9a"'));
  });

  it('should support renamed imports', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { hashString as foo } from "@atlaspack/rust" with { type: "macro" };
        output = foo('hi');
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('output="2a2300bbd7ea6e9a"'));
  });

  it('should support default imports', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import test from "./macro.js" with { type: "macro" };
        output = test('hi');

      macro.js:
        import {hashString} from '@atlaspack/rust';
        export default function test(s) {
          return hashString(s);
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('output="2a2300bbd7ea6e9a"'));
  });

  it('should support default interop with CommonJS modules', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import test from "./macro.js" with { type: "macro" };
        output = test('hi');

      macro.js:
        import {hashString} from '@atlaspack/rust';
        module.exports = function(s) {
          return hashString(s);
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('output="2a2300bbd7ea6e9a"'));
  });

  it('should support namespace imports', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import * as ns from "@atlaspack/rust" with { type: "macro" };
        output = ns.hashString('hi');
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('output="2a2300bbd7ea6e9a"'));
  });

  it('should support various JS value types', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.js" with { type: "macro" };
        output = test(undefined, null, true, false, 1, 0, -2, 'hi', /yo/i, [1, {test: 8}]);

      macro.js:
        import {inspect} from 'util';
        export function test(...args) {
          return inspect(args);
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    // @ts-expect-error - TS2554 - Expected 2-4 arguments, but got 1.
    let res = await run(b);
    assert.equal(
      res,
      require('util').inspect([
        undefined,
        null,
        true,
        false,
        1,
        0,
        -2,
        'hi',
        /yo/i,
        [1, {test: 8}],
      ]),
    );
  });

  it('should support returning various JS value types', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.js" with { type: "macro" };
        output = test();

      macro.js:
        export function test() {
          return [undefined, null, true, false, 1, 0, -2, 'hi', /yo/i, [1, {test: 8}]];
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    // @ts-expect-error - TS2554 - Expected 2-4 arguments, but got 1.
    let res = await run(b);
    assert.deepEqual(res, [
      undefined,
      null,
      true,
      false,
      1,
      0,
      -2,
      'hi',
      /yo/i,
      [1, {test: 8}],
    ]);
  });

  it('should support evaluating expressions', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.js" with { type: "macro" };
        output = test(1 + 2, 'foo ' + 'bar', 3 + 'em', 'test'.length, 'test'['length'], 'test'[1], !true, [1, ...[2, 3]], {x: 2, ...{y: 3}}, true ? 1 : 0, typeof false, null ?? 2);

      macro.js:
        export function test(...args) {
          return args;
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    // @ts-expect-error - TS2554 - Expected 2-4 arguments, but got 1.
    let res = await run(b);
    assert.deepEqual(res, [
      3,
      'foo bar',
      '3em',
      4,
      4,
      'e',
      false,
      [1, 2, 3],
      {x: 2, y: 3},
      1,
      'boolean',
      2,
    ]);
  });

  it('should dead code eliminate falsy branches', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.js" with { type: "macro" };

        if (test()) {
          console.log('bad');
        } else {
          console.log('good');
        }

      macro.js:
        export function test() {
          return false;
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('console.log("good")'));
    assert(!res.includes('console.log("bad")'));
  });

  it('should support async macros', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.js" with { type: "macro" };
        output = test();

      macro.js:
        export function test() {
          return Promise.resolve(2);
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('output=2'));
  });

  it('should ignore macros in node_modules', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import test from "foo";
        output = test;

      node_modules/foo/index.js:
        import { test } from "./macro.js" with { type: "macro" };
        export default test();

      node_modules/foo/macro.js:
        export function test() {
          return 2;
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('function test'));
  });

  it('should throw a diagnostic when an argument cannot be converted', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.js" with { type: "macro" };
        output = test(1, foo);

      macro.js:
        export function test() {
          return 2;
        }
    `;

    try {
      // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
      await bundle(path.join(dir, '/index.js'), {
        inputFS: overlayFS,
        mode: 'production',
      });
    } catch (err: any) {
      assert.deepEqual(err.diagnostics, [
        {
          message: 'Could not statically evaluate macro argument',
          origin: '@atlaspack/transformer-js',
          codeFrames: [
            {
              // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
              filePath: path.join(dir, 'index.js'),
              codeHighlights: [
                {
                  message: undefined,
                  start: {
                    line: 2,
                    column: 18,
                  },
                  end: {
                    line: 2,
                    column: 20,
                  },
                },
              ],
            },
          ],
          hints: null,
        },
      ]);
    }
  });

  it('should throw a diagnostic when a macro errors', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.js" with { type: "macro" };
        output = test(1);

      macro.js:
        exports.test = function test() {
          throw new Error('test');
        }
    `;

    try {
      // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
      await bundle(path.join(dir, '/index.js'), {
        inputFS: overlayFS,
        mode: 'production',
      });
    } catch (err: any) {
      assert(
        err.diagnostics[0].message.startsWith('Error evaluating macro: test'),
      );
      assert.deepEqual(err.diagnostics[0].codeFrames, [
        {
          // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
          filePath: path.join(dir, 'index.js'),
          codeHighlights: [
            {
              message: undefined,
              start: {
                line: 2,
                column: 10,
              },
              end: {
                line: 2,
                column: 16,
              },
            },
          ],
        },
      ]);
    }
  });

  it('should throw a diagnostic when a macro cannot be resolved', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.js" with { type: "macro" };
        output = test(1, 2);
    `;

    try {
      // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
      await bundle(path.join(dir, '/index.js'), {
        inputFS: overlayFS,
        mode: 'production',
      });
    } catch (err: any) {
      assert.deepEqual(
        // \ gets escaped by Node -> Rust -> Node in Windows, so we normalize it for the test
        // @ts-expect-error - TS7006 - Parameter 'd' implicitly has an 'any' type.
        err.diagnostics.map((d) => ({
          ...d,
          message: d.message.replace(/\\\\/g, '\\'),
        })),
        [
          {
            message: `Error loading macro: Could not resolve module "./macro.js" from "${path.join(
              // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
              dir,
              'index.js',
            )}"`,
            origin: '@atlaspack/transformer-js',
            codeFrames: [
              {
                // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
                filePath: path.join(dir, 'index.js'),
                codeHighlights: [
                  {
                    message: undefined,
                    start: {
                      line: 1,
                      column: 1,
                    },
                    end: {
                      line: 1,
                      column: 57,
                    },
                  },
                ],
              },
            ],
            hints: null,
          },
        ],
      );
    }
  });

  it('should support returning functions', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.js" with { type: "macro" };
        output = test(1, 2)(3);

      macro.js:
        export function test(a, b) {
          return new Function('c', \`return \${a} + \${b} + c\`);
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('output=6'));
  });

  it('should support macros written in typescript', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.ts" with { type: "macro" };
        output = test(1, 2);

      macro.ts:
        export function test(a: number, b: number) {
          return a + b;
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('output=3'));
  });

  it('should support macros written in typescript without extension', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro" with { type: "macro" };
        output = test(1, 2);

      macro.ts:
        export function test(a: number, b: number) {
          return a + b;
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('output=3'));
  });

  it('should allow emitting additional assets', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { css } from "./macro.ts" with { type: "macro" };
        output = css('background: red') + css('color: pink');

      macro.ts:
        export function css(v) {
          this.addAsset({
            type: 'css',
            content: '.foo {\\n' + v + '\\n}'
          });
          return 'foo';
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('output="foofoo"'));

    res = await overlayFS.readFile(b.getBundles()[1].filePath, 'utf8');
    assert(res.includes('.foo{color:pink;background:red}'));
  });

  it('should invalidate the cache when changing a macro', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.js" with { type: "macro" };
        output = test();

      macro.js:
        export function test() {
          return 2;
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
      shouldDisableCache: false,
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('output=2'));

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      macro.js:
        export function test() {
          return 3;
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
      shouldDisableCache: false,
    });

    res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    assert(res.includes('output=3'));
  });

  it('should invalidate the cache on build', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.js" with { type: "macro" };
        output = test('test.txt');

      macro.js:
        export function test() {
          this.invalidateOnBuild();
          return Date.now();
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
      shouldDisableCache: false,
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    let match = res.match(/output=(\d+)/);
    assert(match);

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
      shouldDisableCache: false,
    });

    res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    let match2 = res.match(/output=(\d+)/);
    assert(match2);
    assert.notEqual(match[1], match2[1]);
  });

  it('should only error once if a macro errors during loading', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.js" with { type: "macro" };
        output = test(1, 2);
        output2 = test(1, 3);

      macro.js:
        export function test() {
          return Date.now(
        }
    `;

    try {
      // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
      await bundle(path.join(dir, '/index.js'), {
        inputFS: overlayFS,
        mode: 'production',
      });
    } catch (err: any) {
      assert.equal(err.diagnostics.length, 1);
    }
  });

  it('should rebuild in watch mode after fixing an error', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { test } from "./macro.ts" with { type: "macro" };
        output = test('test.txt');

      macro.ts:
        export function test() {
          return Date.now(
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundler(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
      shouldDisableCache: false,
    });

    let subscription;
    try {
      subscription = await b.watch();
      let buildEvent = await getNextBuild(b);
      assert.equal(buildEvent.type, 'buildFailure');

      // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
      await fsFixture(overlayFS, dir)`
        macro.ts:
          export function test() {
            return Date.now();
          }
      `;

      buildEvent = await getNextBuild(b);
      assert.equal(buildEvent.type, 'buildSuccess');
      invariant(buildEvent.type === 'buildSuccess'); // flow

      let res = await overlayFS.readFile(
        buildEvent.bundleGraph.getBundles()[0].filePath,
        'utf8',
      );
      let match = res.match(/output=(\d+)/);
      assert(match);
    } finally {
      await subscription?.unsubscribe();
    }
  });

  it('should support evaluating constants', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { hashString } from "@atlaspack/rust" with { type: "macro" };
        import { test, test2 } from './macro' with { type: "macro" };
        const hi = "hi";
        const ref = hi;
        const arr = [hi];
        const obj = {a: {b: hi}};
        const [a, [b], ...c] = [hi, [hi], 2, 3, hi];
        const [x, y = hi] = [1];
        const {hi: d, e, ...f} = {hi, e: hi, x: 2, y: hi};
        const res = test();
        output1 = hashString(hi);
        output2 = hashString(ref);
        output3 = hashString(arr[0]);
        output4 = hashString(obj.a.b);
        output5 = hashString(a);
        output6 = hashString(b);
        output7 = hashString(c[2]);
        output8 = hashString(y);
        output9 = hashString(d);
        output10 = hashString(e);
        output11 = hashString(f.y);
        output12 = hashString(f?.y);
        output13 = hashString(res);
        output14 = test2(obj)();

      macro.js:
        import { hashString } from "@atlaspack/rust";
        export function test() {
          return "hi";
        }

        export function test2(obj) {
          return new Function('return "' + hashString(obj.a.b) + '"');
        }
    `;

    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
    let b = await bundle(path.join(dir, '/index.js'), {
      inputFS: overlayFS,
      mode: 'production',
    });

    let res = await overlayFS.readFile(b.getBundles()[0].filePath, 'utf8');
    for (let i = 1; i <= 14; i++) {
      assert(res.includes(`output${i}="2a2300bbd7ea6e9a"`));
    }
  });

  it('should throw a diagnostic when a constant is mutated', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { hashString } from "@atlaspack/rust" with { type: "macro" };
        const object = {foo: 'bar'};
        object.foo = 'test';
        output = hashString(object.foo);

        const arr = ['foo'];
        arr[0] = 'bar';
        output = hashString(arr[0]);
    `;

    try {
      // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
      await bundle(path.join(dir, '/index.js'), {
        inputFS: overlayFS,
        mode: 'production',
      });
    } catch (err: any) {
      assert.deepEqual(err.diagnostics, [
        {
          message: 'Could not statically evaluate macro argument',
          origin: '@atlaspack/transformer-js',
          codeFrames: [
            {
              // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
              filePath: path.join(dir, 'index.js'),
              codeHighlights: [
                {
                  message: undefined,
                  start: {
                    line: 3,
                    column: 1,
                  },
                  end: {
                    line: 3,
                    column: 19,
                  },
                },
              ],
            },
          ],
          hints: null,
        },
        {
          message: 'Could not statically evaluate macro argument',
          origin: '@atlaspack/transformer-js',
          codeFrames: [
            {
              // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
              filePath: path.join(dir, 'index.js'),
              codeHighlights: [
                {
                  message: undefined,
                  start: {
                    line: 7,
                    column: 1,
                  },
                  end: {
                    line: 7,
                    column: 14,
                  },
                },
              ],
            },
          ],
          hints: null,
        },
      ]);
    }
  });

  it('should throw a diagnostic when a constant object is passed to a function', async function () {
    // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type. | TS2345 - Argument of type 'TemplateStringsArray' is not assignable to parameter of type 'string[]'.
    await fsFixture(overlayFS, dir)`
      index.js:
        import { hashString } from "@atlaspack/rust" with { type: "macro" };
        const bar = 'bar';
        const object = {foo: bar};
        doSomething(bar); // ok (string)
        doSomething(object.foo); // ok (evaluates to a string)
        doSomething(object); // error (object could be mutated)
        output = hashString(object.foo);

        const object2 = {foo: bar, obj: {}};
        doSomething(object2.obj); // error (object could be mutated)
        output2 = hashString(object2);

        const arr = ['foo'];
        doSomething(arr);
        output3 = hashString(arr[0]);

        const object3 = {foo: bar};
        doSomething(object3[unknown]);
        output4 = hashString(object3);
    `;

    try {
      // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
      await bundle(path.join(dir, '/index.js'), {
        inputFS: overlayFS,
        mode: 'production',
      });
    } catch (err: any) {
      assert.deepEqual(err.diagnostics, [
        {
          message: 'Could not statically evaluate macro argument',
          origin: '@atlaspack/transformer-js',
          codeFrames: [
            {
              // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
              filePath: path.join(dir, 'index.js'),
              codeHighlights: [
                {
                  message: undefined,
                  start: {
                    line: 6,
                    column: 13,
                  },
                  end: {
                    line: 6,
                    column: 18,
                  },
                },
              ],
            },
          ],
          hints: null,
        },
        {
          message: 'Could not statically evaluate macro argument',
          origin: '@atlaspack/transformer-js',
          codeFrames: [
            {
              // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
              filePath: path.join(dir, 'index.js'),
              codeHighlights: [
                {
                  message: undefined,
                  start: {
                    line: 10,
                    column: 13,
                  },
                  end: {
                    line: 10,
                    column: 19,
                  },
                },
              ],
            },
          ],
          hints: null,
        },
        {
          message: 'Could not statically evaluate macro argument',
          origin: '@atlaspack/transformer-js',
          codeFrames: [
            {
              // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
              filePath: path.join(dir, 'index.js'),
              codeHighlights: [
                {
                  message: undefined,
                  start: {
                    line: 14,
                    column: 13,
                  },
                  end: {
                    line: 14,
                    column: 15,
                  },
                },
              ],
            },
          ],
          hints: null,
        },
        {
          message: 'Could not statically evaluate macro argument',
          origin: '@atlaspack/transformer-js',
          codeFrames: [
            {
              // @ts-expect-error - TS7005 - Variable 'dir' implicitly has an 'any' type.
              filePath: path.join(dir, 'index.js'),
              codeHighlights: [
                {
                  message: undefined,
                  start: {
                    line: 18,
                    column: 13,
                  },
                  end: {
                    line: 18,
                    column: 19,
                  },
                },
              ],
            },
          ],
          hints: null,
        },
      ]);
    }
  });
});
