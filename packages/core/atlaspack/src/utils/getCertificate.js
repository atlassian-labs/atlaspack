// @flow
import type {HTTPSOptions} from '../types/index.js';
import type {FileSystem} from '../fs/index.js';

export default async function getCertificate(
  fs: FileSystem,
  options: HTTPSOptions,
): Promise<{|cert: Buffer, key: Buffer|}> {
  try {
    let cert = await fs.readFile(options.cert);
    let key = await fs.readFile(options.key);

    return {key, cert};
  } catch (err) {
    throw new Error('Certificate and/or key not found');
  }
}
