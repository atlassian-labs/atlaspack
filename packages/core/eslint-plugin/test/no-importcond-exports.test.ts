import {tsRuleTester} from './utils';
import rule, {RULE_NAME} from '../src/rules/no-importcond-exports/index';

tsRuleTester.run(RULE_NAME, rule, {
  valid: [
    {
      name: 'No importCond usage',
      code: `
        import { something } from 'module';
        export const myFunction = () => {};
      `,
    },
    {
      name: 'importCond is used inside a function, not directly exported',
      code: `
        const MyComponent = importCond<
          typeof import('./new.tsx'),
          typeof import('./old.tsx')
        >(
          'gate-name',
          './new.tsx',
          './old.tsx',
        );
        export function getComponent() {
          return MyComponent;
        }
      `,
    },
    {
      name: 'importCond is used inside a function, not directly exported',
      code: `
        export function getComponent() {
          const MyComponent = importCond<
            typeof import('./new.tsx'),
            typeof import('./old.tsx')
          >(
            'gate-name',
            './new.tsx',
            './old.tsx',
          );
        }
      `,
    },
  ],

  invalid: [
    {
      name: 'Directly exporting importCond',
      code: `
        export const MyComponent = importCond<
          typeof import('./new.tsx'),
          typeof import('./old.tsx')
        >(
          'gate-name',
          './new.tsx',
          './old.tsx',
        );
      `,
      errors: [{messageId: 'noImportCondExports'}],
    },
    {
      name: 'importCond is indirectly exported',
      code: `
        const MyComponent = importCond<
          typeof import('./new.tsx'),
          typeof import('./old.tsx')
        >(
          'gate-name',
          './new.tsx',
          './old.tsx',
        );
        export default MyComponent;
      `,
      errors: [{messageId: 'noImportCondExports'}],
    },
    {
      name: 'importCond is indirectly exported, with export before definition',
      code: `
        export default MyComponent;
        const MyComponent = importCond<
          typeof import('./new.tsx'),
          typeof import('./old.tsx')
        >(
          'gate-name',
          './new.tsx',
          './old.tsx',
        );
      `,
      errors: [{messageId: 'noImportCondExports'}],
    },
    {
      name: 'Directly exporting importCond as default',
      code: `
        export default importCond<typeof import('./new.tsx'), typeof import('./old.tsx')>('gate-name', './new.tsx', './old.tsx');
      `,
      errors: [{messageId: 'noImportCondExports'}],
    },
    {
      name: 'Named export of a variable initialized with importCond',
      code: `
        const MyComponent = importCond<
          typeof import('./new.tsx'),
          typeof import('./old.tsx')
        >(
          'gate-name',
          './new.tsx',
          './old.tsx',
        );
        export { MyComponent };
      `,
      errors: [{messageId: 'noImportCondExports'}],
    },
    {
      name: 'Named export of a variable initialized with importCond, with export before definition',
      code: `
        export { MyComponent };
        const MyComponent = importCond<
          typeof import('./new.tsx'),
          typeof import('./old.tsx')
        >(
          'gate-name',
          './new.tsx',
          './old.tsx',
        );
      `,
      errors: [{messageId: 'noImportCondExports'}],
    },
  ],
});
