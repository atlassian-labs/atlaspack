/**
 * Post benchmark results as a GitHub comment
 * This module is intended to be required and called from actions/github-script@v7
 * which provides 'github' and 'context' globals.
 *
 * Usage in workflow:
 *   const postComment = require('./scripts/benchmarks/post-github-comment.js');
 *   await postComment(github, context);
 */

const fs = require('fs');
const path = require('path');

const COMMENT_PATH = path.join(
  __dirname,
  '../../packages/core/e2e-tests/benchmark-results/github-comment.md',
);

/**
 * Post benchmark results as a GitHub comment
 * Updates existing benchmark comments or creates a new one
 *
 * @param {Object} github - GitHub API client from actions/github
 * @param {Object} context - GitHub Actions context from actions/github
 * @throws If unable to read the comment file or GitHub API fails
 */
async function postGitHubComment(github, context) {
  if (!fs.existsSync(COMMENT_PATH)) {
    return;
  }

  const comment = fs.readFileSync(COMMENT_PATH, 'utf8');

  // Find existing benchmark comment
  const comments = await github.rest.issues.listComments({
    owner: context.repo.owner,
    repo: context.repo.repo,
    issue_number: context.issue.number,
  });

  const existingComment = comments.data.find((c) =>
    c.body?.includes('📊 Benchmark Results'),
  );

  if (existingComment) {
    // Update existing comment
    await github.rest.issues.updateComment({
      owner: context.repo.owner,
      repo: context.repo.repo,
      comment_id: existingComment.id,
      body: comment,
    });
  } else {
    // Create new comment
    await github.rest.issues.createComment({
      owner: context.repo.owner,
      repo: context.repo.repo,
      issue_number: context.issue.number,
      body: comment,
    });
  }
}

module.exports = postGitHubComment;
