import rule, {RULE_NAME} from '../src/rules/importcond-type-annotations/index';

import {outdent} from 'outdent';
import {tsRuleTester} from './utils';

tsRuleTester.run(RULE_NAME, rule, {
  valid: [
    {
      name: 'Correct type annotation is present',
      code: outdent`
        importCond<typeof import('./new.tsx'), typeof import('./old.tsx')>(
          'gate_name',
          './new.tsx',
          './old.tsx'
        );
      `,
    },
    {
      name: 'Correct type annotation with extra spaces',
      code: outdent`
        importCond<
          typeof import('./new.tsx'),
          typeof import('./old.tsx'),
        >(
          'gate_name',
          './new.tsx',
          './old.tsx',
        );
      `,
    },
    {
      name: 'Non-importCond function call',
      code: outdent`
        otherFunction('gate_name', './new.tsx', './old.tsx');
      `,
    },
    {
      name: 'importCond with fewer than 3 arguments',
      code: outdent`
        importCond('gate_name', './new.tsx');
      `,
    },
  ],

  invalid: [
    {
      name: 'Missing type annotation',
      code: outdent`
        importCond('gate_name', './new.tsx', './old.tsx');
      `,
      output: outdent`
        importCond<typeof import('./new.tsx'), typeof import('./old.tsx')>('gate_name', './new.tsx', './old.tsx');
      `,
      errors: [{messageId: 'addTypeAnnotation'}],
    },
    {
      name: 'Incorrect type annotation order',
      code: outdent`
        importCond<
          typeof import('./old.tsx'),
          typeof import('./new.tsx')
        >(
          'gate_name',
          './new.tsx',
          './old.tsx'
        );
      `,
      errors: [
        {
          messageId: 'wrongTypeAnnotation',
          suggestions: [
            {
              messageId: 'suggestTypeAnnotation',
              output: outdent`
                importCond<typeof import('./new.tsx'), typeof import('./old.tsx')>('gate_name', './new.tsx', './old.tsx');
              `,
            },
          ],
        },
      ],
    },
    {
      name: 'Missing type annotation with extra spaces',
      code: outdent`
        importCond(
          'gate_name',
          './new.tsx',
          './old.tsx'
        );
      `,
      output: outdent`
        importCond<typeof import('./new.tsx'), typeof import('./old.tsx')>('gate_name', './new.tsx', './old.tsx');
      `,
      errors: [{messageId: 'addTypeAnnotation'}],
    },
    {
      name: 'Incorrect type annotation with inline arguments',
      code: outdent`
        importCond<typeof import('./old.tsx'), typeof import('./new.tsx')>('gate_name', './new.tsx', './old.tsx');
      `,
      errors: [
        {
          messageId: 'wrongTypeAnnotation',
          suggestions: [
            {
              messageId: 'suggestTypeAnnotation',
              output: outdent`
                importCond<typeof import('./new.tsx'), typeof import('./old.tsx')>('gate_name', './new.tsx', './old.tsx');
              `,
            },
          ],
        },
      ],
    },
  ],
});
