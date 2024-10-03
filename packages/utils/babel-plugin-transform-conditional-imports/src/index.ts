import {PluginObj} from '@babel/core';
import {declare} from '@babel/helper-plugin-utils';
import {
  ExpressionStatement,
  Node,
  Statement,
  StringLiteral,
  VariableDeclaration,
} from '@babel/types';

interface State {
  server?: boolean;
  importNodes?: any[]; // Statement types didn't work so using any
}

const isServer = (state: State) => {
  return 'server' in state && state.server;
};

// @ts-expect-error Types don't seem to work, manually make them what we need
export default declare((api): PluginObj<State> => {
  const {types: t} = api;

  const buildCondFunction = (
    cond: StringLiteral,
    ifTrue: StringLiteral,
    ifFalse: StringLiteral,
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

  const buildServerObject = (
    identUid: string,
    cond: StringLiteral,
    ifTrue: StringLiteral,
    ifFalse: StringLiteral,
  ) => [
    // Create object containing imports
    t.variableDeclaration('const', [
      t.variableDeclarator(
        t.identifier(identUid),
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

    // Create lazy getter via the load property on the object
    t.expressionStatement(
      t.callExpression(
        t.memberExpression(
          t.identifier('Object'),
          t.identifier('defineProperty'),
        ),
        [
          t.identifier(identUid),
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
                  t.memberExpression(
                    t.identifier(identUid),
                    t.identifier('ifTrue'),
                  ),
                  t.memberExpression(
                    t.identifier(identUid),
                    t.identifier('ifFalse'),
                  ),
                ),
              ),
            ),
          ]),
        ],
      ),
    ),
  ];

  return {
    name: '@atlaspack/babel-plugin-transform-conditional-imports',
    visitor: {
      CallExpression: {
        enter(path, state) {
          if (
            path.node.callee.type === 'Identifier' &&
            path.node.callee.name === 'importCond'
          ) {
            if (
              path.node.arguments.length == 3 &&
              path.node.arguments.every(arg => arg.type === 'StringLiteral')
            ) {
              const [cond, ifTrue, ifFalse] = path.node.arguments;

              if (isServer(state)) {
                // Make module pass lazy in ssr
                const identUid = path.scope.generateUid(
                  `${cond.value}$${ifTrue.value}$${ifFalse.value}`,
                );

                state.importNodes ??= [];
                state.importNodes.push(
                  ...buildServerObject(identUid, cond, ifTrue, ifFalse),
                );

                // Replace call expression with reference to lazy object getter
                path.replaceWith(
                  t.memberExpression(
                    t.identifier(identUid),
                    t.identifier('load'),
                  ),
                );
              } else {
                path.replaceWith(buildCondFunction(cond, ifTrue, ifFalse));
              }
            } else {
              // Simple error for incorrect syntax (since it's documented with the type)
              throw new Error(
                'importCond must have three string literal arguments',
              );
            }
          }
        },
      },
      Program: {
        exit(path, state) {
          if (state.importNodes) {
            // If there's an import
            for (const node of state.importNodes) {
              path.unshiftContainer('body', node);
            }
          }
        },
      },
    },
  };
});
