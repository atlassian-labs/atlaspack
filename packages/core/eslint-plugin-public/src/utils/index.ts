import {ESLintUtils} from '@typescript-eslint/utils';

export interface ExamplePluginDocs {
  description: string;
  recommended?: boolean;
}

export const createRule = ESLintUtils.RuleCreator<ExamplePluginDocs>(
  (name) =>
    `https://github.com/atlassian-labs/atlaspack/tree/main/packages/core/eslint-plugin-public/src/rules/${name}/README.md`,
);
