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
    return /\.changeset\/\w+-\w+\w+\.md$/.test(filename);
  });

  console.log('hasChangeset', hasChangeset);

  if (hasChangeset) {
    // TODO: Change this to whatever actually makes the action pass
    return true;
  }
}
