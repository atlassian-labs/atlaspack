import assert from 'assert';
import sinon from 'sinon';
import path from 'path';

import {StaticServerDataProvider} from '../src/StaticServerDataProvider';

type MockBundle = {
  name: string;
  filePath: string;
};

describe('StaticServerDataProvider', () => {
  describe('getHTMLBundleFilePaths', () => {
    it('returns an empty array when no bundleGraph is set', () => {
      const provider = new StaticServerDataProvider('/dist');
      assert.deepStrictEqual(provider.getHTMLBundleFilePaths(), []);
    });

    it('filters only .html bundles and returns relative paths', () => {
      const distDir = path.resolve('/project/dist');
      const provider = new StaticServerDataProvider(distDir);

      const bundles: MockBundle[] = [
        {
          name: 'index.html',
          filePath: path.join(distDir, 'index.html'),
        },
        {
          name: 'app.js',
          filePath: path.join(distDir, 'app.js'),
        },
        {
          name: 'about/index.html',
          filePath: path.join(distDir, 'about', 'index.html'),
        },
      ];

      const bundleGraphMock = {
        getBundles: () => bundles,
      } as any;

      // We don't care about requestBundleFn for this test, provide a noop.
      provider.update(bundleGraphMock, () => Promise.resolve({} as any));

      const htmlPaths = provider.getHTMLBundleFilePaths().sort();
      assert.deepStrictEqual(htmlPaths, ['about/index.html', 'index.html']);
    });
  });

  describe('requestBundle', () => {
    it('returns "not-found" when no bundleGraph is set', async () => {
      const provider = new StaticServerDataProvider('/dist');
      const result = await provider.requestBundle('index.html');
      assert.strictEqual(result, 'not-found');
    });

    it('returns "not-found" when the bundle is not present in the graph', async () => {
      const distDir = '/dist';
      const bundles: MockBundle[] = [
        {name: 'index.html', filePath: path.join(distDir, 'index.html')},
      ];
      const bundleGraphMock = {
        getBundles: () => bundles,
      } as any;

      const provider = new StaticServerDataProvider(distDir);
      provider.update(bundleGraphMock, () => Promise.resolve({} as any));

      const result = await provider.requestBundle('missing.html');
      assert.strictEqual(result, 'not-found');
    });

    it('returns "not-found" when requestBundleFn is not set', async () => {
      const distDir = '/dist';
      const bundles: MockBundle[] = [
        {name: 'index.html', filePath: path.join(distDir, 'index.html')},
      ];
      const provider = new StaticServerDataProvider(distDir);
      // Directly assign bundleGraph without a requestBundleFn
      (provider as any).bundleGraph = {getBundles: () => bundles} as any;

      const result = await provider.requestBundle('index.html');
      assert.strictEqual(result, 'not-found');
    });

    it('calls requestBundleFn and returns "requested" for a matching bundle', async () => {
      const distDir = '/dist';
      const bundles: MockBundle[] = [
        {name: 'index.html', filePath: path.join(distDir, 'index.html')},
      ];
      const bundleGraphMock = {
        getBundles: () => bundles,
      } as any;

      const requestBundleFn = sinon.stub().resolves();

      const provider = new StaticServerDataProvider(distDir);
      provider.update(bundleGraphMock, requestBundleFn);

      const result = await provider.requestBundle('index.html');

      assert.strictEqual(result, 'requested');
      sinon.assert.calledOnce(requestBundleFn);
      sinon.assert.calledWithExactly(requestBundleFn, bundles[0]);
    });

    it('handles nested bundle paths correctly', async () => {
      const distDir = '/dist';
      const bundles: MockBundle[] = [
        {
          name: 'nested/index.html',
          filePath: path.join(distDir, 'nested', 'index.html'),
        },
      ];
      const bundleGraphMock = {
        getBundles: () => bundles,
      } as any;

      const requestBundleFn = sinon.stub().resolves();

      const provider = new StaticServerDataProvider(distDir);
      provider.update(bundleGraphMock, requestBundleFn);

      const result = await provider.requestBundle('nested/index.html');

      assert.strictEqual(result, 'requested');
      sinon.assert.calledOnce(requestBundleFn);
      sinon.assert.calledWithExactly(requestBundleFn, bundles[0]);
    });
  });
});
