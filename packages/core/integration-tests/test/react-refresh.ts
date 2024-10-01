import assert from 'assert';
import invariant from 'assert';
import path from 'path';
import {
  bundle,
  bundler,
  describe,
  getNextBuild,
  it,
  overlayFS as fs,
  sleep,
  run,
  getNextBuildSuccess,
} from '@atlaspack/test-utils';
import getPort from 'get-port';
import type {BuildEvent, Asset} from '@atlaspack/types';
// flowlint-next-line untyped-import:off
// @ts-expect-error - TS7016 - Could not find a declaration file for module 'jsdom'. '/home/ubuntu/parcel/node_modules/jsdom/lib/api.js' implicitly has an 'any' type.
import JSDOM from 'jsdom';
import nullthrows from 'nullthrows';

// @ts-expect-error - TS7034 - Variable 'MessageChannel' implicitly has type 'any' in some locations where its type cannot be determined.
let MessageChannel;
try {
  ({MessageChannel} = require('worker_threads'));
} catch (_: any) {
  // eslint-disable-next-line no-console
  console.log(
    'Skipping React Fast Refresh tests because they require worker_threads',
  );
}

if (MessageChannel) {
  describe.v2('react-refresh', function () {
    describe('synchronous (automatic runtime)', () => {
      const testDir = path.join(
        __dirname,
        '/integration/react-refresh-automatic',
      );

      // @ts-expect-error - TS7034 - Variable 'b' implicitly has type 'any' in some locations where its type cannot be determined.
      let b,
        // @ts-expect-error - TS7034 - Variable 'root' implicitly has type 'any' in some locations where its type cannot be determined.
        root,
        // @ts-expect-error - TS7034 - Variable 'randoms' implicitly has type 'any' in some locations where its type cannot be determined.
        randoms,
        // @ts-expect-error - TS7034 - Variable 'subscription' implicitly has type 'any' in some locations where its type cannot be determined.
        subscription,
        window: Record<string, any> = {};

      beforeEach(async () => {
        ({b, root, randoms, subscription, window} = await setup(
          path.join(testDir, 'index.html'),
        ));
      });

      it('retains state in functional components', async function () {
        await fs.mkdirp(testDir);
        await fs.copyFile(
          path.join(testDir, 'Foo.1.js'),
          path.join(testDir, 'Foo.js'),
        );
        // @ts-expect-error - TS7005 - Variable 'b' implicitly has an 'any' type.
        assert.equal((await getNextBuild(b)).type, 'buildSuccess');

        // Wait for the hmr-runtime to process the event
        await sleep(100);

        // @ts-expect-error - TS7005 - Variable 'root' implicitly has an 'any' type.
        let [, indexNum, appNum, fooText, fooNum] = root.textContent.match(
          /^([\d.]+) ([\d.]+) ([\w]+):([\d.]+)$/,
        );
        // @ts-expect-error - TS7005 - Variable 'randoms' implicitly has an 'any' type.
        assert.equal(randoms?.indexNum, indexNum);
        // @ts-expect-error - TS7005 - Variable 'randoms' implicitly has an 'any' type.
        assert.equal(randoms?.appNum, appNum);
        // @ts-expect-error - TS7005 - Variable 'randoms' implicitly has an 'any' type.
        assert.equal(randoms?.fooNum, fooNum);
        assert.equal(fooText, 'OtherFunctional');
      });

      afterEach(async () => {
        // @ts-expect-error - TS7005 - Variable 'subscription' implicitly has an 'any' type.
        await cleanup({subscription, window});
      });
    });

    describe('synchronous', () => {
      const testDir = path.join(__dirname, '/integration/react-refresh');

      // @ts-expect-error - TS7034 - Variable 'b' implicitly has type 'any' in some locations where its type cannot be determined.
      let b,
        // @ts-expect-error - TS7034 - Variable 'root' implicitly has type 'any' in some locations where its type cannot be determined.
        root,
        // @ts-expect-error - TS7034 - Variable 'window' implicitly has type 'any' in some locations where its type cannot be determined.
        window,
        // @ts-expect-error - TS7034 - Variable 'subscription' implicitly has type 'any' in some locations where its type cannot be determined.
        subscription,
        randoms: Record<string, any> = {};

      beforeEach(async () => {
        // @ts-expect-error - TS2322 - Type '{ indexNum: any; appNum: any; fooText: any; fooNum: any; } | undefined' is not assignable to type 'Record<string, any>'.
        ({b, root, window, subscription, randoms} = await setup(
          path.join(testDir, 'index.html'),
        ));
      });

      it('retains state in functional components', async function () {
        await fs.mkdirp(testDir);
        await fs.copyFile(
          path.join(testDir, 'Foo.1.js'),
          path.join(testDir, 'Foo.js'),
        );
        // @ts-expect-error - TS7005 - Variable 'b' implicitly has an 'any' type.
        assert.equal((await getNextBuild(b)).type, 'buildSuccess');

        // Wait for the hmr-runtime to process the event
        await sleep(100);

        // @ts-expect-error - TS7005 - Variable 'root' implicitly has an 'any' type.
        let [, indexNum, appNum, fooText, fooNum] = root.textContent.match(
          /^([\d.]+) ([\d.]+) ([\w]+):([\d.]+)$/,
        );
        assert.equal(randoms?.indexNum, indexNum);
        assert.equal(randoms?.appNum, appNum);
        assert.equal(randoms?.fooNum, fooNum);
        assert.equal(fooText, 'OtherFunctional');
      });

      it('supports changing hooks in functional components', async function () {
        await fs.mkdirp(testDir);
        await fs.copyFile(
          path.join(testDir, 'Foo.2-hooks.js'),
          path.join(testDir, 'Foo.js'),
        );
        // @ts-expect-error - TS7005 - Variable 'b' implicitly has an 'any' type.
        assert.equal((await getNextBuild(b)).type, 'buildSuccess');

        // Wait for the hmr-runtime to process the event
        await sleep(100);

        let [, indexNum, appNum, fooText, fooNum, fooNum2] =
          // @ts-expect-error - TS7005 - Variable 'root' implicitly has an 'any' type.
          root.textContent.match(
            /^([\d.]+) ([\d.]+) ([\w]+):([\d.]+):([\d.]+)$/,
          );
        assert.equal(randoms?.indexNum, indexNum);
        assert.equal(randoms?.appNum, appNum);
        assert.notEqual(randoms?.fooNum, fooNum);
        assert(fooNum2);
        assert.equal(fooText, 'Hooks');
      });

      it('retains state in parent components when swapping function and class component', async function () {
        await fs.mkdirp(testDir);
        await fs.copyFile(
          path.join(testDir, 'Foo.3-class.js'),
          path.join(testDir, 'Foo.js'),
        );
        // @ts-expect-error - TS7005 - Variable 'b' implicitly has an 'any' type.
        assert.equal((await getNextBuild(b)).type, 'buildSuccess');

        // Wait for the hmr-runtime to process the event
        await sleep(100);

        // @ts-expect-error - TS7005 - Variable 'root' implicitly has an 'any' type.
        let [, indexNum, appNum, fooText, fooNum] = root.textContent.match(
          /^([\d.]+) ([\d.]+) ([\w]+):([\d.]+)$/,
        );
        assert.equal(randoms?.indexNum, indexNum);
        assert.equal(randoms?.appNum, appNum);
        assert.notEqual(randoms?.fooNum, fooNum);
        assert.equal(fooText, 'Class');
      });

      afterEach(async () => {
        // @ts-expect-error - TS7005 - Variable 'subscription' implicitly has an 'any' type. | TS7005 - Variable 'window' implicitly has an 'any' type.
        await cleanup({subscription, window});
      });
    });

    describe('lazy child component', () => {
      const testDir = path.join(
        __dirname,
        '/integration/react-refresh-lazy-child',
      );

      // @ts-expect-error - TS7034 - Variable 'b' implicitly has type 'any' in some locations where its type cannot be determined.
      let b,
        // @ts-expect-error - TS7034 - Variable 'root' implicitly has type 'any' in some locations where its type cannot be determined.
        root,
        // @ts-expect-error - TS7034 - Variable 'window' implicitly has type 'any' in some locations where its type cannot be determined.
        window,
        // @ts-expect-error - TS7034 - Variable 'subscription' implicitly has type 'any' in some locations where its type cannot be determined.
        subscription,
        randoms: Record<string, any> = {};

      beforeEach(async () => {
        // @ts-expect-error - TS2322 - Type '{ indexNum: any; appNum: any; fooText: any; fooNum: any; } | undefined' is not assignable to type 'Record<string, any>'.
        ({b, root, window, subscription, randoms} = await setup(
          path.join(testDir, 'index.html'),
        ));
      });

      it('retains state in async components on change', async function () {
        assert.equal(randoms?.fooText, 'Async');

        await fs.mkdirp(testDir);
        await fs.copyFile(
          path.join(testDir, 'Async.1.js'),
          path.join(testDir, 'Async.js'),
        );
        // @ts-expect-error - TS7005 - Variable 'b' implicitly has an 'any' type.
        assert.equal((await getNextBuild(b)).type, 'buildSuccess');

        // Wait for the hmr-runtime to process the event
        await sleep(100);

        // @ts-expect-error - TS7005 - Variable 'root' implicitly has an 'any' type.
        let [, indexNum, appNum, fooText, fooNum] = root.textContent.match(
          /^([\d.]+) ([\d.]+) ([\w]+):([\d.]+)$/,
        );
        assert.equal(randoms?.indexNum, indexNum);
        assert.equal(randoms?.appNum, appNum);
        assert.equal(randoms?.fooNum, fooNum);
        assert.equal(fooText, 'OtherAsync');
      });

      afterEach(async () => {
        // @ts-expect-error - TS7005 - Variable 'subscription' implicitly has an 'any' type. | TS7005 - Variable 'window' implicitly has an 'any' type.
        await cleanup({subscription, window});
      });
    });

    describe('circular context dependency', () => {
      const testDir = path.join(
        __dirname,
        '/integration/react-refresh-circular',
      );

      // @ts-expect-error - TS7034 - Variable 'b' implicitly has type 'any' in some locations where its type cannot be determined.
      let b,
        // @ts-expect-error - TS7034 - Variable 'root' implicitly has type 'any' in some locations where its type cannot be determined.
        root,
        // @ts-expect-error - TS7034 - Variable 'subscription' implicitly has type 'any' in some locations where its type cannot be determined.
        subscription,
        window: Record<string, any> = {};

      beforeEach(async () => {
        ({b, root, subscription, window} = await setup(
          path.join(testDir, 'index.html'),
        ));
      });

      it('does not become null when modifying provider', async function () {
        await fs.mkdirp(testDir);
        let f = path.join(testDir, 'Provider.js');
        await fs.writeFile(f, (await fs.readFile(f, 'utf8')).replace('2', '3'));
        // @ts-expect-error - TS7005 - Variable 'b' implicitly has an 'any' type.
        assert.equal((await getNextBuild(b)).type, 'buildSuccess');

        // Wait for the hmr-runtime to process the event
        await sleep(100);

        // @ts-expect-error - TS7005 - Variable 'root' implicitly has an 'any' type.
        assert.equal(root.textContent, '3');
      });

      it('does not become null when modifying consumer', async function () {
        await fs.mkdirp(testDir);
        let f = path.join(testDir, 'Consumer.js');
        await fs.writeFile(
          f,
          (await fs.readFile(f, 'utf8')).replace('tmp', 'foo'),
        );
        // @ts-expect-error - TS7005 - Variable 'b' implicitly has an 'any' type.
        assert.equal((await getNextBuild(b)).type, 'buildSuccess');

        // Wait for the hmr-runtime to process the event
        await sleep(100);

        // @ts-expect-error - TS7005 - Variable 'root' implicitly has an 'any' type.
        assert.equal(root.textContent, '2');
      });

      afterEach(async () => {
        // @ts-expect-error - TS7005 - Variable 'subscription' implicitly has an 'any' type.
        await cleanup({subscription, window});
      });
    });

    it('does not error on inline scripts', async () => {
      let port = await getPort();
      let b = await bundle(
        path.join(
          __dirname,
          'integration/react-refresh-inline-script/index.html',
        ),
        {
          hmrOptions: {
            port,
          },
        },
      );
      await run(b, {}, {require: false});
    });

    it('does not apply to library targets', async () => {
      let port = await getPort();
      let parcel = await bundler(
        path.join(
          __dirname,
          '/integration/react-refresh-library-target/index.js',
        ),
        {
          hmrOptions: {
            port,
          },
        },
      );
      let result = await getNextBuildSuccess(parcel);
      let bundle = nullthrows(
        result.bundleGraph.getBundles().find((b) => b.type === 'js'),
      );

      // Make sure react-refresh transforms were not applied.
      let assets: Asset[] = [];
      bundle.traverse((node) => {
        if (node.type === 'asset') {
          assets.push(node.value);
        } else if (node.type === 'dependency') {
          assert(
            !node.value.specifier.startsWith('react-refresh/runtime') &&
              !node.value.specifier.startsWith(
                '@atlaspack/transformer-react-refresh-wrap',
              ),
          );
        }
      });
      for (let asset of assets) {
        let code = await asset.getCode();
        assert(
          !code.includes('$RefreshReg$') && !code.includes('$RefreshSig$'),
        );
      }
    });
  });
}

async function setup(entry: string) {
  let port = await getPort(),
    b,
    // @ts-expect-error - TS7034 - Variable 'window' implicitly has type 'any' in some locations where its type cannot be determined.
    window,
    randoms,
    subscription,
    root;

  b = bundler(entry, {
    inputFS: fs,
    outputFS: fs,
    serveOptions: {
      https: false,
      port,
      host: '127.0.0.1',
    },
    hmrOptions: {
      port,
    },
    defaultConfig: path.join(
      __dirname,
      'integration/custom-configs/.parcelrc-dev-server',
    ),
  });

  subscription = await b.watch();
  let bundleEvent: BuildEvent = await getNextBuild(b);
  invariant(bundleEvent.type === 'buildSuccess');
  let bundleGraph = bundleEvent.bundleGraph;
  let dom = await JSDOM.JSDOM.fromURL(
    'http://127.0.0.1:' + port + '/index.html',
    {
      runScripts: 'dangerously',
      resources: 'usable',
      pretendToBeVisual: true,
    },
  );
  window = dom.window;
  await new Promise((res: (result: Promise<undefined> | undefined) => void) =>
    // @ts-expect-error - TS7005 - Variable 'window' implicitly has an 'any' type.
    window.document.addEventListener('load', () => {
      // @ts-expect-error - TS2794 - Expected 1 arguments, but got 0. Did you forget to include 'void' in your type argument to 'Promise'?
      res();
    }),
  );
  window.console.clear = () => {};
  // @ts-expect-error - TS7005 - Variable 'MessageChannel' implicitly has an 'any' type.
  window.MessageChannel = MessageChannel;
  root = window.document.getElementById('root');

  let bundle = nullthrows(
    bundleGraph.getBundles().find((b) => b.type === 'js'),
  );
  let parcelRequire = Object.keys(window).find((k) =>
    k.startsWith('parcelRequire'),
  );
  // ReactDOM.render
  await window[parcelRequire](
    // @ts-expect-error - TS2345 - Argument of type 'Asset | undefined' is not assignable to parameter of type 'Asset'.
    bundleGraph.getAssetPublicId(bundle.getEntryAssets().pop()),
  ).default();
  await sleep(100);

  let m = root.textContent.match(/^([\d.]+) ([\d.]+) ([\w]+):([\d.]+)$/);
  if (m) {
    let [, indexNum, appNum, fooText, fooNum] = m;
    assert(indexNum);
    assert(appNum);
    assert(fooNum);

    randoms = {indexNum, appNum, fooText, fooNum};
  }
  return {port, b, window, randoms, subscription, root};
}

// @ts-expect-error - TS7031 - Binding element 'window' implicitly has an 'any' type. | TS7031 - Binding element 'subscription' implicitly has an 'any' type.
async function cleanup({window, subscription}) {
  if (window) {
    window.close();
  }
  if (subscription) {
    await subscription.unsubscribe();
  }
}
