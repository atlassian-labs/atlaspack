import * as mocha from 'mocha';
import tseslint from 'typescript-eslint';
import {RuleTester} from '@typescript-eslint/rule-tester';
import path from 'node:path';

RuleTester.afterAll = mocha.after;

export const tsRuleTester = new RuleTester({
  languageOptions: {
    parser: tseslint.parser,
    parserOptions: {
      projectService: {
        allowDefaultProject: ['*.ts*'],
        defaultProject: 'tsconfig.json',
      },
      tsconfigRootDir: path.join(__dirname, '..'),
    },
  },
});

// Dummy filename, needed for every test to make ESLint tests work
// This file needs to exist but its contents don't matter
export const filename = path.join(__dirname, 'test.ts');
