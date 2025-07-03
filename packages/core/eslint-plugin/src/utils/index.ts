import {ESLintUtils} from '@typescript-eslint/utils';

// eslint-disable-next-line flowtype/no-types-missing-file-annotation
export interface PluginDocs {
  description: string;
  recommended?: boolean;
}

export const createRule = ESLintUtils.RuleCreator<PluginDocs>(
  (name) =>
    `https://github.com/atlassian-labs/atlaspack/tree/main/packages/core/eslint-plugin/src/rules/${name}/README.md`,
);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type SomeType = any; // replace with actual type export
