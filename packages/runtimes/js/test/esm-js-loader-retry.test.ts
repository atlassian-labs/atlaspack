import load from '../src/helpers/browser/esm-js-loader-retry.js';
import bundleManifest from '../src/helpers/bundle-manifest.js';
import {mock} from 'node:test';
import type {Mock} from 'node:test';
import assert from 'node:assert';

declare var globalThis: Window & {[key: string]: any};

describe('esm-js-loader-retry', () => {
  let mockSetTimeout: Mock<Window['setTimeout']>;
  let mockParcelImport: Mock<() => Promise<void>>;

  // eslint-disable-next-line require-await
  const importError = async () => {
    throw new Error('TypeError: Failed to fetch dynamically imported module');
  };

  before(() => {
    bundleManifest.register('http://localhost', ['1', 'foo.js']);
  });

  beforeEach(() => {
    mockSetTimeout = mock.fn((callback: any, duration: any, ...args: any[]) =>
      callback(),
    );
    globalThis.setTimeout = mockSetTimeout;

    mockParcelImport = mock.fn(() => Promise.resolve());
    globalThis.__parcel__import__ = mockParcelImport;

    globalThis.parcelRequire = mock.fn();
    // @ts-expect-error
    globalThis.navigator = {onLine: true};
    globalThis.CustomEvent = globalThis.CustomEvent || class {};
    globalThis.dispatchEvent = mock.fn();
  });

  it('should not throw', async () => {
    await assert.doesNotReject(() => load('1'));
  });

  it('should throw if all requests fail', async () => {
    mockParcelImport.mock.mockImplementationOnce(importError, 0);
    mockParcelImport.mock.mockImplementationOnce(importError, 1);
    mockParcelImport.mock.mockImplementationOnce(importError, 2);
    mockParcelImport.mock.mockImplementationOnce(importError, 3);
    mockParcelImport.mock.mockImplementationOnce(importError, 4);
    mockParcelImport.mock.mockImplementationOnce(importError, 5);
    mockParcelImport.mock.mockImplementationOnce(importError, 6);
    await assert.rejects(() => load('1'));
  });

  it('should resolve if the first request fails', async () => {
    mockParcelImport.mock.mockImplementationOnce(importError, 0);
    await assert.doesNotReject(() => load('1'));
  });

  it('should resolve if the first few requests fails', async () => {
    mockParcelImport.mock.mockImplementationOnce(importError, 0);
    mockParcelImport.mock.mockImplementationOnce(importError, 1);
    mockParcelImport.mock.mockImplementationOnce(importError, 2);
    mockParcelImport.mock.mockImplementationOnce(importError, 3);
    await assert.doesNotReject(() => load('1'));
  });
});
