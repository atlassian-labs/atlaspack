/* eslint-env jest */
const assert = require('assert');
const sinon = require('sinon');
const {
  checkForRustPackageBump,
  enforceChangeset,
  validateChangesets,
} = require('../validate-changesets.js');

describe('check-rust-changes test', () => {
  let mockOctokit;

  beforeEach(() => {
    mockOctokit = {
      rest: {
        pulls: {
          listFiles: sinon.stub(),
        },
        repos: {
          getContent: sinon.stub(),
        },
      },
    };
  });

  afterEach(() => {
    sinon.restore();
  });

  const createMockChangesetContent = (frontmatter, summary = '') => {
    return `---
${frontmatter}
---

${summary}`;
  };

  const setupMockFiles = (files) => {
    mockOctokit.rest.pulls.listFiles.resolves({
      data: files.map((filename) => ({filename})),
    });
  };

  const setupMockContent = (contents) => {
    mockOctokit.rest.repos.getContent.resolves({
      data: {
        content: Buffer.from(contents).toString('base64'),
      },
    });
  };

  it('should return true when @atlaspack/rust is in frontmatter', async () => {
    const changesetContent = createMockChangesetContent(
      "'@atlaspack/rust': minor\n'@atlaspack/core': minor",
    );

    setupMockFiles(['.changeset/test.md']);
    setupMockContent(changesetContent);

    const result = await checkForRustPackageBump({
      octokit: mockOctokit,
      owner: 'test',
      repo: 'test',
      pullNumber: 1,
    });

    assert.equal(result, true);
  });

  it('should return false when @atlaspack/rust is only in summary', async () => {
    const changesetContent = createMockChangesetContent(
      "'@atlaspack/core': minor",
      "This changeset doesn't bump the `@atlaspack/rust` package",
    );

    setupMockFiles(['.changeset/test.md']);
    setupMockContent(changesetContent);

    const result = await checkForRustPackageBump({
      octokit: mockOctokit,
      owner: 'test',
      repo: 'test',
      pullNumber: 1,
    });

    assert.equal(result, false);
  });

  it('should return false when no changeset files exist', async () => {
    setupMockFiles([]);

    const result = await checkForRustPackageBump({
      octokit: mockOctokit,
      owner: 'test',
      repo: 'test',
      pullNumber: 1,
    });

    assert.equal(result, false);
  });

  it('should return false when changeset has no frontmatter', async () => {
    const changesetContent = 'This is a changeset without frontmatter';

    setupMockFiles(['.changeset/test.md']);
    setupMockContent(changesetContent);

    const result = await checkForRustPackageBump({
      octokit: mockOctokit,
      owner: 'test',
      repo: 'test',
      pullNumber: 1,
    });

    assert.equal(result, false);
  });

  it('should return false when changeset has malformed frontmatter', async () => {
    const changesetContent = `---
'@atlaspack/rust': minor
This is malformed frontmatter without closing ---`;

    setupMockFiles(['.changeset/test.md']);
    setupMockContent(changesetContent);

    const result = await checkForRustPackageBump({
      octokit: mockOctokit,
      owner: 'test',
      repo: 'test',
      pullNumber: 1,
    });

    assert.equal(result, false);
  });

  it('should return true when multiple changeset files exist and one has @atlaspack/rust', async () => {
    const changesetContent1 = createMockChangesetContent(
      "'@atlaspack/core': minor",
    );
    const changesetContent2 = createMockChangesetContent(
      "'@atlaspack/rust': minor",
    );

    setupMockFiles(['.changeset/test1.md', '.changeset/test2.md']);

    // Mock the content for both files
    mockOctokit.rest.repos.getContent
      .onFirstCall()
      .resolves({
        data: {content: Buffer.from(changesetContent1).toString('base64')},
      })
      .onSecondCall()
      .resolves({
        data: {content: Buffer.from(changesetContent2).toString('base64')},
      });

    const result = await checkForRustPackageBump({
      octokit: mockOctokit,
      owner: 'test',
      repo: 'test',
      pullNumber: 1,
    });

    assert.equal(result, true);
  });

  it('should return false when multiple changeset files exist but none have @atlaspack/rust', async () => {
    const changesetContent1 = createMockChangesetContent(
      "'@atlaspack/core': minor",
    );
    const changesetContent2 = createMockChangesetContent(
      "'@atlaspack/utils': minor",
    );

    setupMockFiles(['.changeset/test1.md', '.changeset/test2.md']);

    // Mock the content for both files
    mockOctokit.rest.repos.getContent
      .onFirstCall()
      .resolves({
        data: {content: Buffer.from(changesetContent1).toString('base64')},
      })
      .onSecondCall()
      .resolves({
        data: {content: Buffer.from(changesetContent2).toString('base64')},
      });

    const result = await checkForRustPackageBump({
      octokit: mockOctokit,
      owner: 'test',
      repo: 'test',
      pullNumber: 1,
    });

    assert.equal(result, false);
  });

  it('should ignore non-changeset files', async () => {
    setupMockFiles(['package.json', 'README.md']);

    await checkForRustPackageBump({
      octokit: mockOctokit,
      owner: 'test',
      repo: 'test',
      pullNumber: 1,
    });

    // Should not call getContent for non-changeset files
    sinon.assert.notCalled(mockOctokit.rest.repos.getContent);
  });

  it('should only process changeset files', async () => {
    setupMockFiles(['.changeset/test.md', 'package.json', 'README.md']);
    setupMockContent('---\n---\n');

    await checkForRustPackageBump({
      octokit: mockOctokit,
      owner: 'test',
      repo: 'test',
      pullNumber: 1,
    });

    // Should only call getContent once for the changeset file
    sinon.assert.calledOnce(mockOctokit.rest.repos.getContent);
    sinon.assert.calledWith(mockOctokit.rest.repos.getContent, {
      owner: 'test',
      repo: 'test',
      path: '.changeset/test.md',
      ref: 'pull/1/head',
    });
  });
});
