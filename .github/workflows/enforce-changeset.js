/**
 * @typedef EnforceChangesetOptions
 * @property number pullNumber
 * @property string owner
 * @property string repo
 * @property {import('@actions/github-script').AsyncFunctionArguments} octokit
 */

const checkboxMessage = '- [x] This change does not require a changeset';

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
  // const comments = await octokit.rest.issues.listComments({
  //   owner,
  //   repo,
  //   issue_number: pullNumber,
  // });

  const prDetails = await octokit.rest.pulls.get({
    owner,
    repo,
    pull_number: pullNumber,
  });

  if (prDetails.data.body.includes(checkboxMessage)) {
    process.exitCode = 0;
    return;
  }

  // Add a comment
  throw new Error('No changeset found in PR');
}
