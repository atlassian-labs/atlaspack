import load from '../src/helpers/browser/esm-js-loader-retry.js';
import bundleManifest from '../src/helpers/bundle-manifest.js';
import {mock} from 'node:test';
import type {Mock} from 'node:test';
import assert from 'node:assert';

declare var globalThis: Window & {[key: string]: any};

describe('esm-js-loader-retry', () => {
  let mockSetTimeout: Mock<Window['setTimeout']>;
  let mockParcelImport: Mock<() => Promise<void>>;
  let mockCreateElement: Mock<() => MockLink>;

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

    mockCreateElement = mock.fn(MockLink.willFail);

    globalThis.setTimeout = mockSetTimeout;

    // @ts-expect-error
    globalThis.document = {
      createElement: mockCreateElement,
      head: {
        appendChild: (el: MockLink) => {
          if (el.shouldPass) {
            el.onerror?.();
          } else {
            el.onload?.();
          }
        },
        removeChild: () => {},
      },
    };

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
    mockCreateElement.mock.mockImplementation(MockLink.willFail);
    mockParcelImport.mock.mockImplementation(importError);
    await assert.rejects(() => load('1'));
  });

  it('should resolve if the first request fails', async () => {
    mockCreateElement.mock.mockImplementationOnce(MockLink.willFail, 0);
    mockCreateElement.mock.mockImplementationOnce(MockLink.willPass, 1);
    await assert.doesNotReject(() => load('1'));
  });

  it('should resolve if the first few requests fails', async () => {
    mockCreateElement.mock.mockImplementationOnce(MockLink.willFail, 0);
    mockCreateElement.mock.mockImplementationOnce(MockLink.willFail, 1);
    mockCreateElement.mock.mockImplementationOnce(MockLink.willFail, 2);
    mockCreateElement.mock.mockImplementationOnce(MockLink.willPass, 3);
    await assert.doesNotReject(() => load('1'));
  });
});

class MockLink {
  shouldPass: boolean;
  onload?: () => {};
  onerror?: () => {};

  constructor(shouldPass: boolean = true) {
    this.shouldPass = shouldPass;
  }

  static willFail(): MockLink {
    return new MockLink(false);
  }

  static willPass(): MockLink {
    return new MockLink(true);
  }
}
