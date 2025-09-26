/*
 * Enforces that feature flags have proper @author and @since documentation.
 *
 * Rules:
 * - Every feature flag (except those starting with "example") must have an @author line
 * - @author format must be "Name <email@atlassian.com>"
 * - Email must end with @atlassian.com
 * - Every feature flag must have a @since line with format YYYY-MM-DD
 */

'use strict';

const EXPECTED_AUTHOR_FORMAT = `"@author Name <email@atlassian.com>"`;
const EXPECTED_SINCE_FORMAT = `"@since YYYY-MM-DD"`;

module.exports = {
  meta: {
    description: 'Enforce @author and @since documentation for feature flags',
    fixable: 'code',
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

        // Look for JSDoc comment with @author and @since
        const jsdocComment = findJSDocComment(node, context);

        if (!jsdocComment) {
          reportMissingDocumentation(node, flagName, context);
          return;
        }

        // Check both @author and @since
        const authorValidation = validateAuthorInComment(
          jsdocComment,
          flagName,
        );
        const sinceValidation = validateSinceInComment(jsdocComment, flagName);

        // Check if they actually exist in the comment
        const hasAuthor = /@author/m.test(jsdocComment);
        const hasSince = /@since/m.test(jsdocComment);

        // If both are completely missing, report missing documentation
        if (!hasAuthor && !hasSince) {
          reportMissingDocumentation(node, flagName, context);
          return;
        }

        // Otherwise, report specific issues
        if (!authorValidation.isValid) {
          reportInvalidAuthor(
            node,
            flagName,
            authorValidation.message,
            context,
          );
          return;
        }

        if (!sinceValidation.isValid) {
          reportInvalidSince(node, flagName, sinceValidation.message, context);
          return;
        }
      },
    };
  },
};

/**
 * Report missing documentation with auto-fix if possible
 */
function reportMissingDocumentation(node, flagName, context) {
  const message = `Feature flag "${flagName}" is missing @author and @since documentation. Add a comment with ${EXPECTED_AUTHOR_FORMAT} and ${EXPECTED_SINCE_FORMAT} before the property.`;

  reportWithAutoFix(node, message, context);
}

/**
 * Report invalid @author with auto-fix if possible
 */
function reportInvalidAuthor(node, flagName, errorMessage, context) {
  reportWithAutoFix(node, errorMessage, context);
}

/**
 * Report invalid @since with auto-fix if possible
 */
function reportInvalidSince(node, flagName, errorMessage, context) {
  reportWithAutoFix(node, errorMessage, context);
}

/**
 * Report error with auto-fix if git config is available
 */
function reportWithAutoFix(node, message, context) {
  try {
    getCurrentUserDetails();
    context.report({
      node,
      message,
      fix: (fixer) => addDocumentationToComment(fixer, node, context),
    });
  } catch (error) {
    context.report({
      node,
      message: error.message,
    });
  }
}

/**
 * Validate @author in JSDoc comment
 */
function validateAuthorInComment(commentValue, flagName) {
  const COMMENT_AUTHOR_REGEX = /@author (?<name>[^<]+) <(?<email>[^>]+)>/m;

  // Check if @author line exists and matches the expected format
  const authorMatch = commentValue.match(COMMENT_AUTHOR_REGEX);
  if (!authorMatch) {
    return {
      isValid: false,
      message: `Feature flag "${flagName}" @author format is invalid. Expected format: ${EXPECTED_AUTHOR_FORMAT}`,
    };
  }

  const {name, email} = authorMatch.groups;

  // Validate email ends with @atlassian.com
  if (!email.endsWith('@atlassian.com')) {
    return {
      isValid: false,
      message: `Feature flag "${flagName}" @author email must end with @atlassian.com, got: "${email}"`,
    };
  }

  // Validate name is not empty
  if (!name.trim()) {
    return {
      isValid: false,
      message: `Feature flag "${flagName}" @author name cannot be empty`,
    };
  }

  return {isValid: true};
}

/**
 * Validate @since in JSDoc comment
 */
function validateSinceInComment(commentValue, flagName) {
  const COMMENT_SINCE_REGEX = /@since (\d{4}-\d{2}-\d{2})/m;

  // Check if @since line exists and matches the expected format
  const sinceMatch = commentValue.match(COMMENT_SINCE_REGEX);
  if (!sinceMatch) {
    return {
      isValid: false,
      message: `Feature flag "${flagName}" is missing @since or format is invalid. Expected format: ${EXPECTED_SINCE_FORMAT}`,
    };
  }

  const dateStr = sinceMatch[1];

  // Validate the date format more strictly
  const dateRegex = /^(\d{4})-(\d{2})-(\d{2})$/;
  const match = dateStr.match(dateRegex);
  if (!match) {
    return {
      isValid: false,
      message: `Feature flag "${flagName}" @since format is invalid. Expected format: ${EXPECTED_SINCE_FORMAT}`,
    };
  }

  const [, year, month, day] = match;
  const date = new Date(parseInt(year), parseInt(month) - 1, parseInt(day));

  // Check if the date is valid (e.g., not Feb 30)
  if (
    date.getFullYear() != year ||
    date.getMonth() != month - 1 ||
    date.getDate() != day
  ) {
    return {
      isValid: false,
      message: `Feature flag "${flagName}" @since date is invalid: ${dateStr}`,
    };
  }

  return {isValid: true};
}

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
 * Find JSDoc comment before the property
 */
function findJSDocComment(node, context) {
  const sourceCode = context.getSourceCode();
  const comments = sourceCode.getCommentsBefore(node);

  // Look through comments for JSDoc (Block comment starting with *)
  for (const comment of comments) {
    if (comment.type === 'Block') {
      return comment.value;
    }
  }

  return null;
}

/**
 * Get current user details for @author
 */
function getCurrentUserDetails() {
  let gitConfigMessage =
    'Please run: git config --global user.name "Your Name" && git config --global user.email "your.email@atlassian.com"';

  // For testing, use environment variables if set
  if (process.env.ESLINT_TEST_USER_NAME && process.env.ESLINT_TEST_USER_EMAIL) {
    return {
      name: process.env.ESLINT_TEST_USER_NAME,
      email: process.env.ESLINT_TEST_USER_EMAIL,
    };
  }

  // Get user details from git config (try global first, then local)
  const {execSync} = require('child_process');

  let name, email;

  try {
    name = execSync('git config --global user.name', {encoding: 'utf8'}).trim();
    email = execSync('git config --global user.email', {
      encoding: 'utf8',
    }).trim();
  } catch {
    try {
      name = execSync('git config user.name', {encoding: 'utf8'}).trim();
      email = execSync('git config user.email', {encoding: 'utf8'}).trim();
    } catch {
      throw new Error(
        `Unable to get user details from git config. ${gitConfigMessage}`,
      );
    }
  }

  // Validate that email ends with @atlassian.com
  if (!email.endsWith('@atlassian.com')) {
    throw new Error(
      `Your Git email "${email}" does not end with @atlassian.com. ${gitConfigMessage}`,
    );
  }

  return {name, email};
}

/**
 * Add @author and @since to JSDoc comment
 */
function addDocumentationToComment(fixer, node, context) {
  const sourceCode = context.getSourceCode();
  const comments = sourceCode.getCommentsBefore(node);

  // Find the JSDoc comment (Block comment)
  let jsdocComment = null;
  for (const comment of comments) {
    if (comment.type === 'Block') {
      jsdocComment = comment;
      break;
    }
  }

  const {name, email} = getCurrentUserDetails();

  if (jsdocComment) {
    // Case 1: Existing JSDoc comment
    // Add or replace @author and @since in existing JSDoc comment
    const commentText = sourceCode.getText(jsdocComment);
    const lines = commentText.split('\n');

    // Determine the indentation by looking at existing comment lines
    let indentation = ' * ';
    for (const line of lines) {
      if (line.trim().startsWith('*') && line.trim() !== '*/') {
        // Extract the indentation pattern from existing lines
        const match = line.match(/^(\s*)\*/);
        if (match) {
          indentation = match[1] + '* ';
          break;
        }
      }
    }

    // Create the @author and @since lines with proper indentation
    const today = new Date().toISOString().split('T')[0]; // YYYY-MM-DD format
    const indentedAuthorLine = indentation + `@author ${name} <${email}>`;
    const indentedSinceLine = indentation + `@since ${today}`;

    // Check what needs to be fixed
    const authorValidation = validateAuthorInComment(jsdocComment.value, '');
    const sinceValidation = validateSinceInComment(jsdocComment.value, '');

    // Check if there's already an @author line and replace it if needed
    let authorLineFound = false;
    let sinceLineFound = false;
    for (let i = 0; i < lines.length; i++) {
      if (lines[i].includes('@author') && !authorValidation.isValid) {
        lines[i] = indentedAuthorLine;
        authorLineFound = true;
      } else if (lines[i].includes('@author')) {
        authorLineFound = true;
      } else if (lines[i].includes('@since') && !sinceValidation.isValid) {
        lines[i] = indentedSinceLine;
        sinceLineFound = true;
      } else if (lines[i].includes('@since')) {
        sinceLineFound = true;
      }
    }

    // Find where to insert new lines (before the closing */)
    let insertIndex = lines.length - 1;
    for (let i = lines.length - 1; i >= 0; i--) {
      if (lines[i].trim() === '*/') {
        insertIndex = i;
        break;
      }
    }

    // Add missing lines (add @author first, then @since)
    if (!sinceLineFound) {
      lines.splice(insertIndex, 0, indentedSinceLine);
    }
    if (!authorLineFound) {
      lines.splice(insertIndex, 0, indentedAuthorLine);
    }

    const newCommentText = lines.join('\n');
    return fixer.replaceText(jsdocComment, newCommentText);
  } else {
    // Case 2: No existing JSDoc comment
    // Create new JSDoc comment with @author and @since

    // Get the indentation from the source code
    const sourceCode = context.getSourceCode();
    const nodeStart = node.range[0];
    const sourceLines = sourceCode.getText().split('\n');

    // Find which line the node starts on
    let currentPos = 0;
    let lineIndex = 0;
    for (let i = 0; i < sourceLines.length; i++) {
      if (currentPos + sourceLines[i].length + 1 >= nodeStart) {
        lineIndex = i;
        break;
      }
      currentPos += sourceLines[i].length + 1; // +1 for newline
    }

    const propertyLine = sourceLines[lineIndex];
    const match = propertyLine.match(/^(\s*)/);
    const baseIndentation = match ? match[1] : '';

    const today = new Date().toISOString().split('T')[0]; // YYYY-MM-DD format
    const newComment = `/**\n${baseIndentation} * @author ${name} <${email}>\n${baseIndentation} * @since ${today}\n${baseIndentation} */\n${baseIndentation}`;
    const originalPropertyText = sourceCode.getText(node);
    const newPropertyText = newComment + originalPropertyText;

    return fixer.replaceText(node, newPropertyText);
  }
}
