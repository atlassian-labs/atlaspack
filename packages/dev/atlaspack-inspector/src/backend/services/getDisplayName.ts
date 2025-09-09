/* eslint-disable monorepo/no-internal-import */
// @ts-expect-error TS2749
import type {Node} from '@atlaspack/core/lib/types.js';

export function getDisplayName(node: Node): string {
  if (node.type === 'asset') {
    return `asset: ${node.value.filePath}`;
  }
  if (node.type === 'dependency') {
    return `dependency: import '${node.value.specifier}'`;
  }
  if (node.type === 'asset_group') {
    return `asset group: ${node.value.filePath}`;
  }
  if (node.type === 'bundle') {
    return `bundle: ${node.value.displayName}`;
  }

  return node.id;
}
