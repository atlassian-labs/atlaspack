'use strict';

const {RuleTester} = require('eslint');
const rule = require('../../src/rules/feature-flag-author');

// Set test environment variables for consistent testing
process.env.ESLINT_TEST_USER_NAME = 'Test User';
process.env.ESLINT_TEST_USER_EMAIL = 'test.user@atlassian.com';

const ruleTester = new RuleTester({
  parserOptions: {
    ecmaVersion: 2018,
    sourceType: 'module',
  },
});

ruleTester.run('feature-flag-author', rule, {
  valid: [
    // Valid feature flag with proper @author
    {
      code: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * Test feature flag
           * @author John Doe <jdoe@atlassian.com>
           */
          testFeature: false,
        };
      `,
    },
    // Example feature flag (should be ignored)
    {
      code: `
        export const DEFAULT_FEATURE_FLAGS = {
          exampleFeature: false,
        };
      `,
    },
    // Example feature flag with @author (should be ignored)
    {
      code: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * @author John Doe <jdoe@atlassian.com>
           */
          exampleFeature: false,
        };
      `,
    },
    // Multiple valid feature flags
    {
      code: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * @author Jane Smith <jsmith@atlassian.com>
           */
          feature1: true,
          /**
           * @author Bob Johnson <bjohnson@atlassian.com>
           */
          feature2: 'NEW',
        };
      `,
    },
    // Non-feature flag object (should be ignored)
    {
      code: `
        const regularObject = {
          someProperty: 'value',
        };
      `,
    },
  ],
  invalid: [
    // Missing @author
    {
      code: `
        export const DEFAULT_FEATURE_FLAGS = {
          testFeature: false,
        };
      `,
      errors: [
        {
          message:
            'Feature flag "testFeature" is missing @author documentation. Add a comment with "@author Name <email@atlassian.com>" before the property.',
        },
      ],
      output: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * @author Test User <test.user@atlassian.com>
           */
          testFeature: false,
        };
      `,
    },
    // Missing @author in existing JSDoc comment
    {
      code: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * This is a feature flag description
           */
          testFeatureWithComment: false,
        };
      `,
      errors: [
        {
          message:
            'Feature flag "testFeatureWithComment" is missing @author documentation. Add a comment with "@author Name <email@atlassian.com>" before the property.',
        },
      ],
      output: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * This is a feature flag description
           * @author Test User <test.user@atlassian.com>
           */
          testFeatureWithComment: false,
        };
      `,
    },
    // Wrong email domain
    {
      code: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * @author Test User <test.user@invalid.com>
           */
          testFeature: false,
        };
      `,
      errors: [
        {
          message:
            'Feature flag "testFeature" @author email must end with @atlassian.com, got: "test.user@invalid.com"',
        },
      ],
      output: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * @author Test User <test.user@atlassian.com>
           */
          testFeature: false,
        };
      `,
    },
    // Empty name
    {
      code: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * @author <test.user@atlassian.com>
           */
          testFeature: false,
        };
      `,
      errors: [
        {
          message:
            'Feature flag "testFeature" @author format is invalid. Expected format: "@author Name <email@atlassian.com>"',
        },
      ],
      output: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * @author Test User <test.user@atlassian.com>
           */
          testFeature: false,
        };
      `,
    },
    // Missing email brackets
    {
      code: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * @author Test User test.user@atlassian.com
           */
          testFeature: false,
        };
      `,
      errors: [
        {
          message:
            'Feature flag "testFeature" @author format is invalid. Expected format: "@author Name <email@atlassian.com>"',
        },
      ],
      output: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * @author Test User <test.user@atlassian.com>
           */
          testFeature: false,
        };
      `,
    },
    // Mixed valid and invalid flags
    {
      code: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * Correct flag
           * @author Joe Bloggs <jbloggs@atlassian.com>
           */
          correctFlag: true,
          incorrectFlag: false,
        };
      `,
      errors: [
        {
          message:
            'Feature flag "incorrectFlag" is missing @author documentation. Add a comment with "@author Name <email@atlassian.com>" before the property.',
        },
      ],
      output: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * Correct flag
           * @author Joe Bloggs <jbloggs@atlassian.com>
           */
          correctFlag: true,
          /**
           * @author Test User <test.user@atlassian.com>
           */
          incorrectFlag: false,
        };
      `,
    },
  ],
});

module.exports = {ruleTester};
