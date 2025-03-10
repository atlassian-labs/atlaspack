import type {PluginObj} from '@babel/core';
import {declare} from '@babel/helper-plugin-utils';
import type {StringLiteral} from '@babel/types';

interface Opts {
  // Use node safe import cond syntax
  server?: boolean;
}

interface State {
  opts: Opts;
  importMap?: Map<string, string>;
}

const isServer = (opts: Opts): boolean => !!('server' in opts && opts.server);

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
    name: '@atlaspack/babel-plugin-transform-contextual-imports',
    visitor: {
      CallExpression: {
        enter(path, state) {
          if (
            path.node.callee.type === 'Identifier' &&
            path.node.callee.name === 'importCond'
          ) {
            if (
              path.node.arguments.length === 3 &&
              path.node.arguments.every((arg) => arg.type === 'StringLiteral')
            ) {
              const [cond, ifTrue, ifFalse] = path.node.arguments;

              if (!isServer(state.opts)) {
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
      VariableDeclaration: {
        enter(path, state) {
          if (isServer(state.opts)) {
            if (
              path.node.declarations.length === 1 &&
              path.node.declarations[0].type === 'VariableDeclarator' &&
              path.node.declarations[0].id.type === 'Identifier'
            ) {
              const importId = path.node.declarations[0].id;
              const call = path.node.declarations[0].init;

              if (call?.type === 'CallExpression') {
                if (
                  call.callee.type === 'Identifier' &&
                  call.callee.name === 'importCond'
                ) {
                  if (
                    call.arguments.length === 3 &&
                    call.arguments.every((arg) => arg.type === 'StringLiteral')
                  ) {
                    const [cond, ifTrue, ifFalse] = call.arguments;

                    // Make module pass lazy in ssr
                    const identUid = path.scope.generateUid(
                      `${cond.value}$${ifTrue.value}$${ifFalse.value}`,
                    );

                    path.replaceWithMultiple(
                      buildServerObject(identUid, cond, ifTrue, ifFalse),
                    );

                    state.importMap?.set(importId.name, identUid);
                  } else {
                    // Simple error for incorrect syntax (since it's documented with the type)
                    throw new Error(
                      'importCond must have three string literal arguments',
                    );
                  }
                }
              }
            }
          }
        },
      },
      Identifier: {
        exit(path, state) {
          const newImportId = state.importMap?.get(path.node.name);
          if (newImportId) {
            path.replaceWith(
              t.memberExpression(
                t.identifier(newImportId),
                t.identifier('load'),
              ),
            );
          }
        },
      },
      Program: {
        enter(_, state) {
          state.importMap = new Map();
        },
      },
    },
  };
});
