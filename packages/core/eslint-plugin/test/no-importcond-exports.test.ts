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
          typeof import('./old.tsx'),
          typeof import('./new.tsx')
        >(
          'gate-name',
          './old.tsx',
          './new.tsx',
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
            typeof import('./old.tsx'),
            typeof import('./new.tsx')
          >(
            'gate-name',
            './old.tsx',
            './new.tsx',
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
          typeof import('./old.tsx'),
          typeof import('./new.tsx')
        >(
          'gate-name',
          './old.tsx',
          './new.tsx',
        );
      `,
      errors: [{messageId: 'noImportCondExports'}],
    },
    {
      name: 'importCond is indirectly exported',
      code: `
        const MyComponent = importCond<
          typeof import('./old.tsx'),
          typeof import('./new.tsx')
        >(
          'gate-name',
          './old.tsx',
          './new.tsx',
        );
        export default MyComponent;
      `,
      errors: [{messageId: 'noImportCondExports'}],
    },
    {
      name: 'Directly exporting importCond as default',
      code: `
        export default importCond({ old: OldComponent, new: NewComponent });
      `,
      errors: [{messageId: 'noImportCondExports'}],
    },
    {
      name: 'Named export of a variable initialized with importCond',
      code: `
        const MyComponent = importCond<
          typeof import('./old.tsx'),
          typeof import('./new.tsx')
        >(
          'gate-name',
          './old.tsx',
          './new.tsx',
        );
        export { MyComponent };
      `,
      errors: [{messageId: 'noImportCondExports'}],
    },
  ],
});
