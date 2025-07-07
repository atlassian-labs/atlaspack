import * as module from 'node:module'
import * as url from 'node:url'

/**
 *
 * @param {string | URL} filePath
 * @returns {NodeRequire}
 */
export function createRequire(filePath) {
  if (filePath instanceof URL) {
    filePath = url.fileURLToPath(filePath)
  }
  const customRequire = module.createRequire(filePath)
  customRequire(url.fileURLToPath(import.meta.resolve('./index.js')))
  return (path) => customRequire(path)
}

export const USE_LIB = process.env.ATLASPACK_USE_LIB === 'true'
