/**
 * @typedef EnforceChangesetOptions
 * @property number pullNumber
 * @property string owner
 * @property string repo
 * @property {import('@actions/github-script').AsyncFunctionArguments} octokit
 */

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

  // Get all comments on PR
  const comments = await octokit.rest.issues.listComments({
    owner,
    repo,
    issue_number: pullNumber,
  });

  console.log('comments', JSON.stringify(comments.data, null, 2));
  throw new Error('No changeset found in PR');
}
