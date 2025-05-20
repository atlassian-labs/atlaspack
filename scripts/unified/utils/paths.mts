import * as path from 'node:path';
import * as url from 'node:url';

const __dirname = path.dirname(url.fileURLToPath(import.meta.url));
const __root = path.dirname(path.dirname(path.dirname(__dirname)));
const __src = path.join(__root, 'packages', 'unified', 'src');
const __dist = path.join(__root, 'packages', 'unified', 'lib');

export const Paths = {
  dirname(importMetaUrl) {
    return path.dirname(url.fileURLToPath(importMetaUrl));
  },
  root: __root,
  node_modules: path.join(__root, 'node_modules'),
  unifiedSrc: path.join(__root, 'packages', 'unified', 'src'),
  unifiedDist: path.join(__root, 'packages', 'unified', 'lib'),
  vendorSrc: path.join(__src, 'vendor'),
  vendorDist: path.join(__dist, 'vendor'),
};
