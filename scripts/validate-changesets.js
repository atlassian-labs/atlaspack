/* eslint-disable no-console */

/**
 * @typedef ValidateChangesetsOptions
 * @property number pullNumber
 * @property string owner
 * @property string repo
 * @property {import('@actions/github-script').AsyncFunctionArguments} octokit
 */

const generalCommentTitle = '## Missing Changeset';
const rustCommentTitle = `## Rust Package Changeset Check`;
const debugMode = process.env.DEBUG_VALIDATE_CHANGESETS === 'true';

function debugLog(message) {
  if (debugMode) {
    console.log(message);
  }
}

async function getCommentId(
  {octokit, owner, repo, pullNumber},
  commentTitle,
  botLogin,
) {
  const comments = await octokit.rest.issues.listComments({
    owner,
    repo,
    issue_number: pullNumber,
  });

  const comment = comments.data.find(
    (comment) =>
      comment.body.includes(commentTitle) && comment.user.login === botLogin,
  );

  if (comment) {
    debugLog('Existing changeset validation comment found in PR');
  } else {
    debugLog('No changeset validation comment found in PR');
  }

  return comment?.id;
}

async function checkForRustFileChanges({octokit, owner, repo, pullNumber}) {
  const files = await octokit.rest.pulls.listFiles({
    owner,
    repo,
    pull_number: pullNumber,
  });

  const hasRustFiles = files.data.some(({filename}) =>
    filename.endsWith('.rs'),
  );

  if (hasRustFiles) {
    debugLog('Rust files found in PR');
  } else {
    debugLog('No Rust files found in PR');
  }

  return hasRustFiles;
}

async function checkForRustPackageBump({octokit, owner, repo, pullNumber}) {
  const files = await octokit.rest.pulls.listFiles({
    owner,
    repo,
    pull_number: pullNumber,
  });

  // Check for changeset files
  const changesetFiles = files.data.filter(
    ({filename}) =>
      filename.startsWith('.changeset/') && filename.endsWith('.md'),
  );

  if (changesetFiles.length === 0) {
    debugLog('No changeset files found in PR');
    return false;
  }

  // Get the content of each changeset file to check for @atlaspack/rust bumps
  const changesetContents = await Promise.all(
    changesetFiles.map(async ({filename}) => {
      const content = await octokit.rest.repos.getContent({
        owner,
        repo,
        path: filename,
        ref: `pull/${pullNumber}/head`,
      });

      // Decode the content (it's base64 encoded)
      const decodedContent = Buffer.from(
        content.data.content,
        'base64',
      ).toString('utf-8');
      return decodedContent;
    }),
  );

  // Check if any changeset contains a bump for @atlaspack/rust in the frontmatter
  const hasRustBump = changesetContents.some((content) => {
    // Extract the frontmatter section (between --- markers)
    const frontmatterMatch = content.match(/^---\s*\n([\s\S]*?)\n---\s*\n/);
    if (!frontmatterMatch) {
      return false;
    }

    const frontmatter = frontmatterMatch[1];

    // Look for @atlaspack/rust in the frontmatter only
    return frontmatter.includes('@atlaspack/rust');
  });

  if (hasRustBump) {
    debugLog('@atlaspack/rust bump found in changeset files');
  } else {
    debugLog('No @atlaspack/rust bump found in changeset files');
  }

  return hasRustBump;
}

const changesetFileRegex = /\.changeset\/\w+-\w+-\w+\.md$/;

async function checkForChangesetFile({octokit, owner, repo, pullNumber}) {
  const files = await octokit.rest.pulls.listFiles({
    owner,
    repo,
    pull_number: pullNumber,
  });

  const hasChangesetFile = files.data.some(({filename}) =>
    changesetFileRegex.test(filename),
  );

  if (!hasChangesetFile) {
    debugLog('No changeset file found in PR');
  } else {
    debugLog('Changeset file found in PR');
  }

  return hasChangesetFile;
}

const noChangesetRegex = /\[no-changeset\]/;

async function checkForExplanationTag({octokit, owner, repo, pullNumber}) {
  const prDetails = await octokit.rest.pulls.get({
    owner,
    repo,
    pull_number: pullNumber,
  });

  const hasExplanationTag = noChangesetRegex.test(prDetails.data.body);

  if (!hasExplanationTag) {
    debugLog('No explanation tag found in PR description');
  } else {
    debugLog('Explanation tag found in PR description');
  }

  return hasExplanationTag;
}

async function enforceChangeset(prOptions) {
  const {octokit, owner, repo, pullNumber} = prOptions;

  const [hasChangeset, hasExplanationTag] = await Promise.all([
    checkForChangesetFile(prOptions),
    checkForExplanationTag(prOptions),
  ]);

  if (hasChangeset || hasExplanationTag) {
    process.exitCode = 0;
    return;
  }

  await octokit.rest.issues.createComment({
    owner,
    repo,
    issue_number: pullNumber,
    body: `
${generalCommentTitle}
No changeset found in PR.
Please add a changeset file (\`yarn changeset\`), or add a '[no-changeset]' tag with explanation to the PR description.
`.trim(),
  });

  throw new Error('No changeset found in PR');
}

async function validateChangesets(prOptions) {
  // Run both validations concurrently
  // The Rust validation will override general validation if [no-changeset] is used for Rust changes
  const rustOptions = {
    ...prOptions,
    rustBotLogin: 'github-actions[bot]',
  };

  // Check if there are Rust files and [no-changeset] tag
  const [hasRustFiles, hasExplanationTag] = await Promise.all([
    checkForRustFileChanges(prOptions),
    checkForExplanationTag(prOptions),
  ]);

  // If there are Rust files with [no-changeset] tag, only run Rust validation
  if (hasRustFiles && hasExplanationTag) {
    await checkRustChanges(rustOptions);
    return;
  }

  // Otherwise, run both validations
  await Promise.all([
    enforceChangeset(prOptions),
    checkRustChanges(rustOptions),
  ]);
}

async function checkRustChanges(prOptions) {
  const {
    octokit,
    owner,
    repo,
    pullNumber,
    rustBotLogin = 'github-actions[bot]',
  } = prOptions;

  const [hasRustFiles, commentId] = await Promise.all([
    checkForRustFileChanges(prOptions),
    getCommentId(prOptions, rustCommentTitle, rustBotLogin),
  ]);

  // If no Rust files changed, we don't need to do anything
  if (!hasRustFiles) {
    process.exitCode = 0;

    // If no Rust files changed, delete any existing comment
    if (commentId) {
      await octokit.rest.issues.deleteComment({
        owner,
        repo,
        comment_id: commentId,
      });

      debugLog(
        'Detected existing changeset validation comment in PR but now there are no Rust files, so deleting it',
      );
    }

    return;
  }

  const [hasRustBump, hasExplanationTag] = await Promise.all([
    checkForRustPackageBump(prOptions),
    checkForExplanationTag(prOptions),
  ]);

  // If Rust files changed and rust package is bumped, no need for PR comment
  if (hasRustBump) {
    process.exitCode = 0;

    // If we previously left a PR comment, update it
    if (commentId) {
      await octokit.rest.issues.updateComment({
        owner,
        repo,
        comment_id: commentId,
        body: `
${rustCommentTitle}
✅ The \`@atlaspack/rust\` package has been included in your changeset. Your Rust changes will be published.
`.trim(),
      });

      debugLog(
        'Detected existing changeset validation comment in PR but now there is a Rust bump, so updating it',
      );
    }

    return;
  }

  // If Rust files changed but [no-changeset] tag is present, allow it to pass
  if (hasExplanationTag) {
    process.exitCode = 0;

    // If we previously left a PR comment, update it to acknowledge the no-changeset tag
    if (commentId) {
      await octokit.rest.issues.updateComment({
        owner,
        repo,
        comment_id: commentId,
        body: `
${rustCommentTitle}
✅ A \`[no-changeset]\` tag has been detected in your PR description. Since this change doesn't require a changeset, your Rust changes will pass without requiring a bump to \`@atlaspack/rust\`.
`.trim(),
      });

      debugLog(
        'Detected existing changeset validation comment in PR but now there is a [no-changeset] tag, so updating it',
      );
    }

    return;
  }

  // If comment already exists, just leave it in place
  if (commentId) {
    process.exitCode = 1;
    debugLog(
      'Rust files changed but @atlaspack/rust package not bumped and no [no-changeset] tag. Comment already exists.',
    );

    return;
  }

  // Add the comment
  await octokit.rest.issues.createComment({
    owner,
    repo,
    issue_number: pullNumber,
    body: `
${rustCommentTitle}
⚠️ Rust files have been changed but the \`@atlaspack/rust\` package was not bumped in your changeset.

**Options:**
1. **If you want your Rust changes published:** Add \`@atlaspack/rust\` to your changeset
2. **If this change doesn't require publishing:** Add a \`[no-changeset]\` tag to your PR description

Example: \`[no-changeset]: Internal refactoring that doesn't affect the public API\`
`.trim(),
  });

  debugLog(
    'Rust files changed but @atlaspack/rust package not bumped and no [no-changeset] tag. Left a comment.',
  );
}

module.exports = {
  validateChangesets,
  // Keep these for testing purposes
  checkForRustPackageBump,
};
