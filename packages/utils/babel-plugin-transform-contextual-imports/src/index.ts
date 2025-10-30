import type {PluginObj, types as BabelTypes} from '@babel/core';
import type {Binding} from '@babel/traverse';
import {declare} from '@babel/helper-plugin-utils';

interface Opts {
  /** Use node safe import cond syntax */
  node?: boolean;
}

interface State {
  /** Plugin options */
  opts: Opts;
  /** Set of bindings that need to be mutated after import was transformed */
  conditionalImportBindings?: Set<Binding>;
  /** Set of identifiers that have been visited in the exit pass, to avoid adding the load property multiple times */
  visitedIdentifiers?: Set<BabelTypes.Identifier | BabelTypes.JSXIdentifier>;
}

const isNode = (opts: Opts): boolean => !!('node' in opts && opts.node);

export default declare((api): PluginObj<State> => {
  const {types: t} = api;

  const isImportCondCallExpression = (
    node: BabelTypes.Node,
  ): node is BabelTypes.CallExpression & {
    arguments: [
      BabelTypes.StringLiteral,
      BabelTypes.StringLiteral,
      BabelTypes.StringLiteral,
    ];
  } => {
    if (
      node.type === 'CallExpression' &&
      node.callee.type === 'Identifier' &&
      node.callee.name === 'importCond'
    ) {
      if (
        node.arguments.length === 3 &&
        node.arguments.every(
          (arg): arg is BabelTypes.StringLiteral =>
            arg.type === 'StringLiteral',
        )
      ) {
        return true;
      } else {
        // Simple error for incorrect syntax (since it's documented with the type)
        throw new Error('importCond must have three string literal arguments');
      }
    }

    return false;
  };

  const buildCondFunction = (
    cond: BabelTypes.StringLiteral,
    ifTrue: BabelTypes.StringLiteral,
    ifFalse: BabelTypes.StringLiteral,
  ) =>
    t.conditionalExpression(
      t.logicalExpression(
        '&&',
        t.memberExpression(t.identifier('globalThis'), t.identifier('__MCOND')),
        t.callExpression(
          t.memberExpression(
            t.identifier('globalThis'),
            t.identifier('__MCOND'),
          ),
          [cond],
        ),
      ),
      t.memberExpression(
        t.callExpression(t.identifier('require'), [ifTrue]),
        t.identifier('default'),
      ),
      t.memberExpression(
        t.callExpression(t.identifier('require'), [ifFalse]),
        t.identifier('default'),
      ),
    );

  const buildNodeObject = (
    identifier: BabelTypes.Identifier,
    cond: BabelTypes.StringLiteral,
    ifTrue: BabelTypes.StringLiteral,
    ifFalse: BabelTypes.StringLiteral,
  ) => [
    // Create object containing imports
    t.variableDeclaration('const', [
      t.variableDeclarator(
        identifier,
        t.objectExpression([
          t.objectProperty(
            t.identifier('ifTrue'),
            t.memberExpression(
              t.callExpression(t.identifier('require'), [ifTrue]),
              t.identifier('default'),
            ),
          ),
          t.objectProperty(
            t.identifier('ifFalse'),
            t.memberExpression(
              t.callExpression(t.identifier('require'), [ifFalse]),
              t.identifier('default'),
            ),
          ),
        ]),
      ),
    ]),

    // Create lazy getter via the load property on the object.
    // This is node module resolution safe because each time the import is accessed, we re-evaluate the condition.
    t.expressionStatement(
      t.callExpression(
        t.memberExpression(
          t.identifier('Object'),
          t.identifier('defineProperty'),
        ),
        [
          identifier,
          t.stringLiteral('load'),
          t.objectExpression([
            t.objectProperty(
              t.identifier('get'),
              t.arrowFunctionExpression(
                [],
                t.conditionalExpression(
                  t.logicalExpression(
                    '&&',
                    t.memberExpression(
                      t.identifier('globalThis'),
                      t.identifier('__MCOND'),
                    ),
                    t.callExpression(
                      t.memberExpression(
                        t.identifier('globalThis'),
                        t.identifier('__MCOND'),
                      ),
                      [cond],
                    ),
                  ),
                  t.memberExpression(identifier, t.identifier('ifTrue')),
                  t.memberExpression(identifier, t.identifier('ifFalse')),
                ),
              ),
            ),
          ]),
        ],
      ),
    ),
  ];

  return {
    name: '@atlaspack/babel-plugin-transform-contextual-imports',
    visitor: {
      CallExpression: {
        enter(path, state) {
          const node = path.node;
          if (isImportCondCallExpression(node)) {
            const [cond, ifTrue, ifFalse] = node.arguments;
            if (!isNode(state.opts)) {
              // Replace the importCond call with a conditional require import, as a fallback for environments that don't support Atlaspack
              path.replaceWith(buildCondFunction(cond, ifTrue, ifFalse));
            }
          }
        },
      },
      VariableDeclaration: {
        enter(path, state) {
          if (isNode(state.opts)) {
            if (
              path.node.declarations.length === 1 &&
              path.node.declarations[0].type === 'VariableDeclarator' &&
              path.node.declarations[0].id.type === 'Identifier'
            ) {
              const importId = path.node.declarations[0].id;
              const call = path.node.declarations[0].init;

              // Mark identifier for object so we don't add the load property to it
              state.visitedIdentifiers?.add(importId);

              if (call && isImportCondCallExpression(call)) {
                const [cond, ifTrue, ifFalse] = call.arguments;

                // Replace with object containing imports and lazy getter, which allows us to load the correct import based on the condition at runtime
                path.replaceWithMultiple(
                  buildNodeObject(importId, cond, ifTrue, ifFalse),
                );

                // Add the binding to set so we can mutate all references to this binding in the exit pass
                const binding = path.scope.getBinding(importId.name);
                if (binding) {
                  state.conditionalImportBindings?.add(binding);
                }
              }
            }
          }
        },
      },
      ReferencedIdentifier: {
        exit(path, state) {
          if (!isNode(state.opts)) {
            return;
          }

          if (path.parentPath.isTSType()) {
            return;
          }

          if (state.visitedIdentifiers?.has(path.node)) {
            return;
          }

          const binding = path.scope.getBinding(path.node.name);
          if (binding && state.conditionalImportBindings?.has(binding)) {
            if (path.isJSXIdentifier()) {
              // Add load property to the import usage
              const newIdentifer = t.jsxIdentifier(path.node.name);
              path.replaceWith(
                t.jsxMemberExpression(newIdentifer, t.jsxIdentifier('load')),
              );
              state.visitedIdentifiers?.add(newIdentifer);
            } else {
              // Add load property to the import usage
              const newIdentifer = t.identifier(path.node.name);
              path.replaceWith(
                t.memberExpression(newIdentifer, t.identifier('load')),
              );
              state.visitedIdentifiers?.add(newIdentifer);
            }
          }
        },
      },
      Program: {
        enter(_, state) {
          state.conditionalImportBindings = new Set();
          state.visitedIdentifiers = new Set();
        },
      },
    },
  };
});
