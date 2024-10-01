import type {File as BabelNodeFile} from '@babel/types';
import type SourceMap from '@parcel/source-map';
import type {Node} from '@babel/types';
import * as BabelTypes from '@babel/types';

export function remapAstLocations(
  // @ts-expect-error - TS2709 - Cannot use namespace 'BabelTypes' as a type.
  t: BabelTypes,
  ast: BabelNodeFile,
  map: SourceMap,
) {
  // remap ast to original mappings
  // This improves sourcemap accuracy and fixes sourcemaps when scope-hoisting
  traverseAll(t, ast.program, (node) => {
    if (node.loc) {
      if (node.loc?.start) {
        let mapping = map.findClosestMapping(
          node.loc.start.line,
          node.loc.start.column,
        );

        if (mapping?.original) {
          node.loc.start.line = mapping.original.line;
          node.loc.start.column = mapping.original.column;

          let length = node.loc.end.column - node.loc.start.column;

          node.loc.end.line = mapping.original.line;
          node.loc.end.column = mapping.original.column + length;

          // @ts-expect-error - TS2322 - Type 'string | undefined' is not assignable to type 'string'.
          node.loc.filename = mapping.source;
        } else {
          // Maintain null mappings?
          node.loc = null;
        }
      }
    }
  });
}

function traverseAll(
  // @ts-expect-error - TS2709 - Cannot use namespace 'BabelTypes' as a type.
  t: BabelTypes,
  node: Node,
  visitor: (node: Node) => void,
): void {
  if (!node) {
    return;
  }

  visitor(node);

  for (let key of t.VISITOR_KEYS[node.type] || []) {
    // @ts-expect-error - TS7053 - Element implicitly has an 'any' type because expression of type 'any' can't be used to index type 'Node'.
    let subNode: Node | Array<Node> = node[key];
    if (Array.isArray(subNode)) {
      for (let i = 0; i < subNode.length; i++) {
        traverseAll(t, subNode[i], visitor);
      }
    } else {
      traverseAll(t, subNode, visitor);
    }
  }
}
