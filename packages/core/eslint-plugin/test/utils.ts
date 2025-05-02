import * as mocha from 'mocha';
import tseslint from 'typescript-eslint';
import {RuleTester} from '@typescript-eslint/rule-tester';
import path from 'node:path';

RuleTester.afterAll = mocha.after;

export const tsRuleTester = new RuleTester();
