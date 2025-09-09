'use strict';

const {RuleTester} = require('eslint');
const rule = require('../../src/rules/feature-flag-author');

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
            'Feature flag "testFeature" is missing @author documentation. Add a comment with @author "Name <email@atlassian.com>" before the property.',
        },
      ],
    },
    // Wrong email domain
    {
      code: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * @author John Doe <jdoe@example.com>
           */
          testFeature: false,
        };
      `,
      errors: [
        {
          message:
            'Feature flag "testFeature" @author email must end with @atlassian.com, got: "jdoe@example.com"',
        },
      ],
    },
    // Empty name
    {
      code: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * @author <jdoe@atlassian.com>
           */
          testFeature: false,
        };
      `,
      errors: [
        {
          message:
            'Feature flag "testFeature" @author format is invalid. Expected: Name <email@atlassian.com>, got: "<jdoe@atlassian.com>"',
        },
      ],
    },
    // Missing email brackets
    {
      code: `
        export const DEFAULT_FEATURE_FLAGS = {
          /**
           * @author John Doe jdoe@atlassian.com
           */
          testFeature: false,
        };
      `,
      errors: [
        {
          message:
            'Feature flag "testFeature" @author format is invalid. Expected: Name <email@atlassian.com>, got: "John Doe jdoe@atlassian.com"',
        },
      ],
    },
  ],
});

module.exports = {ruleTester};
