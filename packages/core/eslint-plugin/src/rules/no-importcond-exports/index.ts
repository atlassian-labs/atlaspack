/**
 * @file Bans directly exporting Atlaspack conditional imports (importCond) from a file, as this is not expressly supported and will break in tests.
 */
import {createRule} from '../../utils/index';

export const RULE_NAME = 'no-importcond-exports';

export const messages = {
  noImportCondExports:
    'Directly exporting Atlaspack conditional imports (importCond) from a file is not supported and will break in tests. If you need the component for testing, please import the old and new versions of the components or modules directly in the test file instead.\n\n' +
    'Does your use case require exporting the component? Reach out to #atlaspack-contextual-imports so we can help find a workaround.',
}; // satisfies Record<string, string>;   // Uncomment this line once prettier is updated to a version that supports it

export type Options = [];

const rule = createRule<Options, keyof typeof messages>({
  name: RULE_NAME,
  meta: {
    docs: {
      description:
        'Bans directly exporting Atlaspack conditional imports (importCond) from a file, as this is not expressly supported and will break in tests.',
      recommended: true,
    },
    messages,
    schema: [],
    type: 'problem',
  },
  defaultOptions: [],
  create(context) {
    // Track variables initialized with `importCond`
    const importCondVariables = new Set<string>();

    return {
      VariableDeclarator(node) {
        if (
          node.init &&
          node.init.type === 'CallExpression' &&
          node.init.callee.type === 'Identifier' &&
          node.init.callee.name === 'importCond' &&
          node.id.type === 'Identifier'
        ) {
          // Add the variable name to the tracked set
          importCondVariables.add(node.id.name);
        }
      },
      ExportNamedDeclaration(node) {
        // Check if any named export references a tracked variable
        if (node.specifiers) {
          node.specifiers.forEach((specifier) => {
            if (
              specifier.type === 'ExportSpecifier' &&
              specifier.local.type === 'Identifier' &&
              importCondVariables.has(specifier.local.name)
            ) {
              context.report({
                node: specifier,
                messageId: 'noImportCondExports',
              });
            }
          });
        }

        // Check if a variable declaration is directly exported
        if (
          node.declaration &&
          node.declaration.type === 'VariableDeclaration' &&
          node.declaration.declarations
        ) {
          node.declaration.declarations.forEach((declaration) => {
            if (
              declaration.init &&
              declaration.init.type === 'CallExpression' &&
              declaration.init.callee.type === 'Identifier' &&
              declaration.init.callee.name === 'importCond'
            ) {
              context.report({
                node: declaration,
                messageId: 'noImportCondExports',
              });
            }
          });
        }
      },
      ExportDefaultDeclaration(node) {
        // Check if the default export is a tracked variable or an `importCond` call
        if (
          node.declaration &&
          ((node.declaration.type === 'Identifier' &&
            importCondVariables.has(node.declaration.name)) ||
            (node.declaration.type === 'CallExpression' &&
              node.declaration.callee.type === 'Identifier' &&
              node.declaration.callee.name === 'importCond'))
        ) {
          context.report({
            node,
            messageId: 'noImportCondExports',
          });
        }
      },
    };
  },
});

export default rule;
