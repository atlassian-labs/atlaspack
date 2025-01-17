// @ts-check
const load = require('../src/helpers/browser/esm-js-loader-retry.js')
const bundleManifest  = require( '../src/helpers/bundle-manifest.js')
const { mock }  = require( 'node:test')
const assert  = require( 'node:assert')

// eslint-disable-next-line require-await
const importError = async () => {
  throw new Error('TypeError: Failed to fetch dynamically imported module')
}

describe('esm-js-loader-retry', () => {
  /** @type {import('node:test').Mock} */
  let mockSetTimeout

  /** @type {import('node:test').Mock} */
  let mockParcelImport

  before(() => {
    bundleManifest.register('http://localhost', ['1', 'foo.js'])
  })

  beforeEach(() => {
    mockSetTimeout = mock.fn((callback) => callback())
    globalThis.setTimeout = mockSetTimeout

    mockParcelImport = mock.fn()
    globalThis.__parcel__import__ = mockParcelImport

    globalThis.parcelRequire = mock.fn()
    globalThis.navigator = { onLine: true }
    globalThis.dispatchEvent = mock.fn()
  })

  it('should not throw', async () => {
    await assert.doesNotReject(() => load('1'))
  })

  it('should throw if all requests fail', async () => {
    mockParcelImport.mock.mockImplementationOnce(importError, 0)
    mockParcelImport.mock.mockImplementationOnce(importError, 1)
    mockParcelImport.mock.mockImplementationOnce(importError, 2)
    mockParcelImport.mock.mockImplementationOnce(importError, 3)
    mockParcelImport.mock.mockImplementationOnce(importError, 4)
    mockParcelImport.mock.mockImplementationOnce(importError, 5)
    mockParcelImport.mock.mockImplementationOnce(importError, 6)
    await assert.rejects(() => load('1'))
  })

  it('should resolve if the first request fails', async () => {
    mockParcelImport.mock.mockImplementationOnce(importError, 0)
    await assert.doesNotReject(() => load('1'))
  })

  it('should resolve if the first few requests fails', async () => {
    mockParcelImport.mock.mockImplementationOnce(importError, 0)
    mockParcelImport.mock.mockImplementationOnce(importError, 1)
    mockParcelImport.mock.mockImplementationOnce(importError, 2)
    mockParcelImport.mock.mockImplementationOnce(importError, 3)
    await assert.doesNotReject(() => load('1'))
  })
})
