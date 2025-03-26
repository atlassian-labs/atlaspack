/**
 * @typedef EnforceChangesetOptions
 * @property number pullNumber
 * @property string owner
 * @property string repo
 * @property {import('@actions/github-script').AsyncFunctionArguments} octokit
 */

const commentTitle = '## Missing Changeset';

async function getCommentId({octokit, owner, repo, pullNumber}) {
  const comments = await octokit.rest.issues.listComments({
    owner,
    repo,
    issue_number: pullNumber,
  });

  const comment = comments.data.find(
    (comment) =>
      comment.body.includes(commentTitle) &&
      comment.user.login === 'github-actions[bot]',
  );

  return comment?.id;
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
    console.log('No changeset file found in PR');
  }

  return hasChangesetFile;
}

const noChangesetRegex = /^ ?\[no-changeset]: ?\S/;
async function checkForExplanationTag({octokit, owner, repo, pullNumber}) {
  const prDetails = await octokit.rest.pulls.get({
    owner,
    repo,
    pull_number: pullNumber,
  });

  const hasExplanationTag = noChangesetRegex.test(prDetails.data.body);

  if (!hasExplanationTag) {
    console.log('No explanation tag found in PR description');
  }

  return hasExplanationTag;
}

/**
 * Enforce that a changeset is present in a PR
 * @param EnforceChangesetOptions options
 */
export async function enforceChangeset(prOptions) {
  const {octokit, owner, repo, pullNumber} = prOptions;

  const [hasChangeset, commentId, hasExplanationTag] = await Promise.all([
    checkForChangesetFile(prOptions),
    getCommentId(prOptions),
    checkForExplanationTag(prOptions),
  ]);

  if (hasChangeset || hasExplanationTag) {
    process.exitCode = 0;

    // If requirements are satisfied, delete the comment
    if (commentId) {
      await octokit.rest.issues.deleteComment({
        owner,
        repo,
        comment_id: commentId,
      });
    }

    return;
  }

  // If comment already exists, just leave it in place
  if (commentId) {
    process.exitCode = 1;
    throw new Error('No changeset or explanation found in PR');
  }

  // Add the comment
  await octokit.rest.issues.createComment({
    owner,
    repo,
    issue_number: pullNumber,
    body: `
${commentTitle}
No changeset found in PR.
Please add a changeset file (\`yarn changeset\`), or add a '[no-changeset]' tag with explanation to the PR description.

<details>
<summary>Example</summary>
<blockquote>[no-changeset]: This PR is a refactor and does not require a changeset</blockquote>
</details>
`.trim(),
  });

  throw new Error('No changeset found in PR');
}
