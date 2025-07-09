import * as mocha from 'mocha';
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import tseslint from 'typescript-eslint';
import {RuleTester} from '@typescript-eslint/rule-tester';

RuleTester.afterAll = mocha.after;

export const tsRuleTester = new RuleTester();
