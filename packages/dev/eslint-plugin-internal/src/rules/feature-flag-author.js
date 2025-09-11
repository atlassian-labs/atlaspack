/*
 * Enforces that feature flags have proper @author documentation.
 *
 * Rules:
 * - Every feature flag (except those starting with "example") must have an @author line
 * - @author format must be "Name <email@atlassian.com>"
 * - Email must end with @atlassian.com
 */

'use strict';

module.exports = {
  meta: {
    description: 'Enforce @author documentation for feature flags',
    fixable: null,
    schema: [],
  },
  create(context) {
    return {
      Property(node) {
        // Check if this is a feature flag property
        if (!isFeatureFlagProperty(node)) {
          return;
        }

        const flagName = getPropertyName(node);

        // Skip example feature flags
        if (flagName && flagName.startsWith('example')) {
          return;
        }

        // Look for @author comment in the JSDoc comment
        const authorComment = findAuthorComment(node, context);

        if (!authorComment) {
          context.report({
            node,
            message: `Feature flag "${flagName}" is missing @author documentation. Add a comment with @author "Name <email@atlassian.com>" before the property.`,
          });
          return;
        }

        // Validate @author format
        const authorMatch = authorComment.match(/@author\s+(.+)/);
        if (!authorMatch) {
          context.report({
            node,
            message: `Feature flag "${flagName}" has malformed @author documentation. Expected format: @author Name <email@atlassian.com>`,
          });
          return;
        }

        const authorText = authorMatch[1].trim();

        // Check if it matches the expected format: "Name <email@atlassian.com>"
        const emailMatch = authorText.match(/^(.+?)\s*<(.+?)>$/);
        if (!emailMatch) {
          context.report({
            node,
            message: `Feature flag "${flagName}" @author format is invalid. Expected: Name <email@atlassian.com>, got: "${authorText}"`,
          });
          return;
        }

        const [, name, email] = emailMatch;

        // Validate email ends with @atlassian.com
        if (!email.endsWith('@atlassian.com')) {
          context.report({
            node,
            message: `Feature flag "${flagName}" @author email must end with @atlassian.com, got: "${email}"`,
          });
          return;
        }

        // Validate name is not empty
        if (!name.trim()) {
          context.report({
            node,
            message: `Feature flag "${flagName}" @author name cannot be empty`,
          });
          return;
        }
      },
    };
  },
};

/**
 * Check if a property node represents a feature flag
 */
function isFeatureFlagProperty(node) {
  // Only apply to properties in the DEFAULT_FEATURE_FLAGS object
  const parent = node.parent;

  // Check if parent is an object expression
  if (parent && parent.type === 'ObjectExpression') {
    const grandParent = parent.parent;

    // Check if the object is assigned to DEFAULT_FEATURE_FLAGS
    if (grandParent && grandParent.type === 'VariableDeclarator') {
      const varName = grandParent.id.name;
      return varName === 'DEFAULT_FEATURE_FLAGS';
    }
  }

  return false;
}

/**
 * Get the property name from a property node
 */
function getPropertyName(node) {
  if (node.key.type === 'Identifier') {
    return node.key.name;
  }
  if (node.key.type === 'Literal') {
    return node.key.value;
  }
  return null;
}

/**
 * Find @author comment in JSDoc comments before the property
 */
function findAuthorComment(node, context) {
  const sourceCode = context.getSourceCode();
  const comments = sourceCode.getCommentsBefore(node);

  // Look through comments for JSDoc with @author
  for (const comment of comments) {
    if (comment.type === 'Block' && comment.value.includes('@author')) {
      return comment.value;
    }
  }

  return null;
}
