import {declare} from '@babel/helper-plugin-utils';

export default declare(({types: t}) => {
  return {
    visitor: {
      CallExpression(path) {
        if (
          path.node.callee.type === 'Identifier' &&
          path.node.callee.name === 'importCond'
        ) {
          if (
            path.node.arguments.length !== 3 ||
            !path.node.arguments.every(arg => arg.type === 'StringLiteral')
          ) {
            // Simple error for incorrect syntax (since it's documented with the type)
            throw new Error(
              'importCond must have three string literal arguments',
            );
          } else {
            const [cond, ifTrue, ifFalse] = path.node.arguments;
            path.replaceWith(
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
                  t.callExpression(t.identifier('require'), [ifTrue]),
                  t.identifier('default'),
                ),
                t.memberExpression(
                  t.callExpression(t.identifier('require'), [ifFalse]),
                  t.identifier('default'),
                ),
              ),
            );
          }
        }
      },
    },
  };
});
