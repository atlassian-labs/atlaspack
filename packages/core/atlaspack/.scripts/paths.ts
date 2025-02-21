import * as path from 'node:path';
import * as url from 'node:url';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
export const Paths = {
  '~/': (...args: string[]) =>
    path.normalize(path.join(__dirname, '..', ...args)),
  'root/': (...args: string[]) =>
    path.join(
      path.normalize(path.join(__dirname, '..', '..', '..', '..')),
      ...args,
    ),
};
