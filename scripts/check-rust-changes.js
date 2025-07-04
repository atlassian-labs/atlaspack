/* eslint-disable no-console */

/**
 * @typedef CheckRustChangesOptions
 * @property number pullNumber
 * @property string owner
 * @property string repo
 * @property {import('@actions/github-script').AsyncFunctionArguments} octokit
 */

const commentTitle = `## 🦀 Ferris' Rust Changeset Check`;

async function getCommentId({octokit, owner, repo, pullNumber}) {
  const comments = await octokit.rest.issues.listComments({
    owner,
    repo,
    issue_number: pullNumber,
  });

  const comment = comments.data.find(
    (comment) =>
      comment.body.includes(commentTitle) &&
      comment.user.login === 'ferris-atlaspack-bot[bot]',
  );

  if (comment) {
    console.log('Existing ferris-atlaspack-bot comment found in PR');
  } else {
    console.log('No ferris-atlaspack-bot comment found in PR');
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
    console.log('Rust files found in PR');
  } else {
    console.log('No Rust files found in PR');
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
    console.log('No changeset files found in PR');
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

  // Check if any changeset contains a bump for @atlaspack/rust
  const hasRustBump = changesetContents.some((content) =>
    content.includes('@atlaspack/rust'),
  );

  if (hasRustBump) {
    console.log('@atlaspack/rust bump found in changeset files');
  } else {
    console.log('No @atlaspack/rust bump found in changeset files');
  }

  return hasRustBump;
}

/**
 * Check if Rust files have been changed and if the @atlaspack/rust package is bumped
 * @param CheckRustChangesOptions options
 */
export async function checkRustChanges(prOptions) {
  const {octokit, owner, repo, pullNumber} = prOptions;

  const [hasRustFiles, commentId, hasRustBump] = await Promise.all([
    checkForRustFileChanges(prOptions),
    getCommentId(prOptions),
    checkForRustPackageBump(prOptions),
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

      console.log(
        'Detected existing ferris-atlaspack-bot comment in PR but now there are no Rust files, so deleting it',
      );
    }

    return;
  }

  // If Rust files changed and rust package is bumped, no need for PR comment
  if (hasRustBump) {
    process.exitCode = 0;

    // If we previously left a PR comment, delete it
    if (commentId) {
      await octokit.rest.issues.updateComment({
        owner,
        repo,
        comment_id: commentId,
        body: `
${commentTitle}
I can see you have now included \`@atlaspack/rust\` in your changeset. This means your Rust changes will be published.
Now I'm a [happy crab](https://youtu.be/LDU_Txk06tM?si=L80HlbKGtjXAmi6R&t=71) 🦀🎉
`.trim(),
      });

      console.log(
        'Detected existing ferris-atlaspack-bot comment in PR but now there is a Rust bump, so updating it',
      );
    }

    return;
  }

  // If comment already exists, just leave it in place
  if (commentId) {
    process.exitCode = 1;
    console.log(
      'Rust files changed but @atlaspack/rust package not bumped. Comment already exists.',
    );
  }

  // Add the comment
  await octokit.rest.issues.createComment({
    owner,
    repo,
    issue_number: pullNumber,
    body: `
${commentTitle}
Ferris says: Hi! I noticed you changed some \`.rs\` files but you didn't bump the Rust package.

If you want your Rust changes published, you will need to bump the \`@atlaspack/rust\` package in your changeset.
`.trim(),
  });

  console.log(
    'Rust files changed but @atlaspack/rust package not bumped. Left a comment.',
  );
}
