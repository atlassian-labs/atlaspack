import * as nodeTest from 'node:test';

// eslint-disable-next-line @typescript-eslint/no-unused-vars
import tseslint from 'typescript-eslint';
import {RuleTester} from '@typescript-eslint/rule-tester';

// @ts-expect-error globalThis is undefined
RuleTester.afterAll = globalThis.after || nodeTest.after;
// @ts-expect-error globalThis is undefined
RuleTester.it = globalThis.it || nodeTest.it;
// @ts-expect-error globalThis is undefined
RuleTester.describe = globalThis.describe || nodeTest.describe;

export const tsRuleTester = new RuleTester();
