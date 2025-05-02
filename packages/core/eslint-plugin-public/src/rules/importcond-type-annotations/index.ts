/**
 * @file Automatically adds type annotations for conditional import (importCond) usages
 */
import type {TSESTree} from '@typescript-eslint/utils';
import {createRule} from '../../utils/index';

export const RULE_NAME = 'importcond-type-annotations';

export const messages = {
  addTypeAnnotation:
    'This conditional import usage needs a type annotation. Please apply the autofix so that the type annotation is correct.',
  wrongTypeAnnotation:
    'The type annotation for this conditional import usage is incorrect. Please apply the suggestion to fix it.',
  suggestTypeAnnotation:
    'Fix the type annotation for this conditional import usage.',
}; // satisfies Record<string, string>;   // Uncomment this line once prettier is updated to a version that supports it

export type Options = [];

const generateFix = (
  firstArgText: string,
  secondArgText: string,
  thirdArgText: string,
): string => {
  const correctTypeAnnotation = `<typeof import(${secondArgText}), typeof import(${thirdArgText})>`;
  return `importCond${correctTypeAnnotation}(${firstArgText}, ${secondArgText}, ${thirdArgText})`;
};

const rule = createRule<Options, keyof typeof messages>({
  name: RULE_NAME,
  meta: {
    docs: {
      description:
        'Ensures that importCond function calls have correct type annotations.',
      recommended: true,
    },
    messages,
    schema: [],
    type: 'problem',
    fixable: 'code',
    hasSuggestions: true,
  },
  defaultOptions: [],
  create(context) {
    return {
      CallExpression(node) {
        // Check if the function being called is `importCond`
        // Note that this is a global, so we don't need to check the imports
        if (
          node.callee.type === 'Identifier' &&
          node.callee.name === 'importCond'
        ) {
          const args = node.arguments;

          // Ensure there are at least 3 arguments
          // If not, then there will be type-checking errors anyway
          if (args.length < 3) {
            return;
          }

          const [firstArg, secondArg, thirdArg] = args;

          // Get the source code for the arguments
          const {sourceCode} = context;
          const firstArgText = sourceCode.getText(firstArg);
          const secondArgText = sourceCode.getText(secondArg);
          const thirdArgText = sourceCode.getText(thirdArg);

          // Check if the type annotation is missing or incorrect
          // Whether it's called typeArguments or typeParameters depends on the version of typescript-eslint
          const typeArguments:
            | TSESTree.TSTypeParameterInstantiation
            | undefined = node.typeArguments ?? (node as any).typeParameters;
          if (!typeArguments) {
            // No type annotation exists at all
            context.report({
              node,
              messageId: 'addTypeAnnotation',
              // Apply autofix instead of suggestion, because there was no type annotation in the first place.
              fix: (fixer) => {
                const fixedCode = generateFix(
                  firstArgText,
                  secondArgText,
                  thirdArgText,
                );
                return fixer.replaceText(node, fixedCode);
              },
            });
          } else if (
            typeArguments.params.length !== 2 ||
            sourceCode.getText(typeArguments.params[0]) !==
              `typeof import(${secondArgText})` ||
            sourceCode.getText(typeArguments.params[1]) !==
              `typeof import(${thirdArgText})`
          ) {
            // Type annotation is present, but incorrect
            context.report({
              node,
              messageId: 'wrongTypeAnnotation',
              // We should be more cautious here, so we make it a suggestion instead of an autofix
              suggest: [
                {
                  messageId: 'suggestTypeAnnotation',
                  fix: (fixer) => {
                    const fixedCode = generateFix(
                      firstArgText,
                      secondArgText,
                      thirdArgText,
                    );
                    return fixer.replaceText(node, fixedCode);
                  },
                },
              ],
            });
          }
        }
      },
    };
  },
});

export default rule;
