/**
 * @typedef EnforceChangesetOptions
 * @property number pullNumber
 * @property string owner
 * @property string repo
 * @property {import('@actions/github-script').AsyncFunctionArguments} octokit
 */

const noChangesetRegex = /^ ?\[no-changeset]: ?\S/;

/**
 * Enforce that a changeset is present in a PR
 * @param EnforceChangesetOptions options
 */
export async function enforceChangeset({pullNumber, owner, repo, octokit}) {
  // Check for a changeset file in the PR
  const files = await octokit.rest.pulls.listFiles({
    owner,
    repo,
    pull_number: pullNumber,
  });

  const hasChangeset = files.data.some(({filename}) => {
    console.log('filename', filename);
    return /\.changeset\/\w+-\w+-\w+\.md$/.test(filename);
  });

  if (hasChangeset) {
    process.exitCode = 0;
    return;
  }

  const prDetails = await octokit.rest.pulls.get({
    owner,
    repo,
    pull_number: pullNumber,
  });

  // Explanation already provided
  if (noChangesetRegex.test(prDetails.data.body)) {
    process.exitCode = 0;
    return;
  }

  // Check to see if comment already exists
  const comments = await octokit.rest.issues.listComments({
    owner,
    repo,
    issue_number: pullNumber,
  });

  const existingComment = comments.data.find((comment) => {
    return (
      comment.body.includes('## Missing changeset') &&
      comment.user.login === 'github-actions[bot]'
    );
  });

  if (existingComment) {
    throw new Error('No changeset or explanation found in PR');
  }

  // Add the comment
  await octokit.rest.issues.createComment({
    owner,
    repo,
    issue_number: pullNumber,
    body: `
## Missing changeset
No changeset found in PR.
Please add a changeset file (\`yarn changeset\`), or add a '[no-changeset]' tag with explanation to the PR description.

E.g.
> [no-changeset]: This PR is a refactor and does not require a changeset
`.trim(),
  });

  throw new Error('No changeset found in PR');
}
