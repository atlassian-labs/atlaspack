/* eslint-env jest */
const assert = require('assert');
const sinon = require('sinon');
const {
  checkForRustPackageBump,
  validateChangesets,
  isChangesetCheckboxTicked,
  hasNoChangesetAnnotation,
} = require('../validate-changesets.js');

describe('validate-changesets', () => {
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

  // Add missing mock methods for validation tests
  const setupMockComments = (comments = []) => {
    mockOctokit.rest.issues.listComments.resolves({
      data: comments,
    });
  };

  const setupMockPR = (body = '') => {
    mockOctokit.rest.pulls.get.resolves({
      data: {body},
    });
  };

  beforeEach(() => {
    mockOctokit.rest.issues = {
      listComments: sinon.stub(),
      createComment: sinon.stub(),
      updateComment: sinon.stub(),
      deleteComment: sinon.stub(),
    };
    mockOctokit.rest.pulls.get = sinon.stub();
    process.exitCode = 0;
  });

  it('should validate both general and Rust changesets in one pass', async () => {
    setupMockFiles(['.changeset/seven-pens-beg.md']);
    setupMockComments([]);
    setupMockPR('');

    await validateChangesets({
      octokit: mockOctokit,
      owner: 'test',
      repo: 'test',
      pullNumber: 1,
    });

    assert.equal(process.exitCode, 0);
    sinon.assert.notCalled(mockOctokit.rest.issues.createComment);
  });

  it('should fail validation when Rust changes have no changeset and no explanation', async () => {
    setupMockFiles(['src/main.rs']);
    setupMockComments([]);
    setupMockPR('Regular PR description');

    try {
      await validateChangesets({
        octokit: mockOctokit,
        owner: 'test',
        repo: 'test',
        pullNumber: 1,
      });
      assert.fail('Expected function to throw');
    } catch (error) {
      assert.equal(error.message, 'No changeset found in PR');
      // Should create comments for both general and Rust validation
      sinon.assert.calledTwice(mockOctokit.rest.issues.createComment);
    }
  });

  it('should pass validation when Rust changes have no-changeset tag', async () => {
    setupMockFiles(['src/main.rs']);
    setupMockComments([]);
    setupMockPR('[no-changeset]: Internal Rust refactoring');

    await validateChangesets({
      octokit: mockOctokit,
      owner: 'test',
      repo: 'test',
      pullNumber: 1,
    });

    assert.equal(process.exitCode, 0);
    sinon.assert.notCalled(mockOctokit.rest.issues.createComment);
  });

  describe('isChangesetCheckboxTicked', () => {
    it('should return true when checkbox is ticked with lowercase x', () => {
      const body =
        '- [x] There is a changeset for this change, or one is not required';
      assert.equal(isChangesetCheckboxTicked(body), true);
    });

    it('should return true when checkbox is ticked with uppercase X', () => {
      const body =
        '- [X] There is a changeset for this change, or one is not required';
      assert.equal(isChangesetCheckboxTicked(body), true);
    });

    it('should return false when checkbox is not ticked', () => {
      const body =
        '- [ ] There is a changeset for this change, or one is not required';
      assert.equal(isChangesetCheckboxTicked(body), false);
    });

    it('should return false when there is no changeset checkbox', () => {
      const body = '- [x] Some other task\n- [ ] Another task';
      assert.equal(isChangesetCheckboxTicked(body), false);
    });

    it('should return true with asterisk list marker', () => {
      const body =
        '* [x] There is a changeset for this change, or one is not required';
      assert.equal(isChangesetCheckboxTicked(body), true);
    });

    it('should return true with plus list marker', () => {
      const body =
        '+ [x] There is a changeset for this change, or one is not required';
      assert.equal(isChangesetCheckboxTicked(body), true);
    });

    it('should return true when checkbox is among other items', () => {
      const body = [
        '## Checklist',
        '- [x] Existing or new tests cover this change',
        '- [x] There is a changeset for this change, or one is not required',
        '- [ ] Added documentation for any new features',
      ].join('\n');
      assert.equal(isChangesetCheckboxTicked(body), true);
    });

    it('should return false for empty body', () => {
      assert.equal(isChangesetCheckboxTicked(''), false);
    });
  });

  describe('hasNoChangesetAnnotation', () => {
    it('should return true when [no-changeset] tag is present', () => {
      assert.equal(
        hasNoChangesetAnnotation('[no-changeset]: Not needed'),
        true,
      );
    });

    it('should return false when [no-changeset] is inside an HTML comment', () => {
      assert.equal(
        hasNoChangesetAnnotation(
          '<!-- [no-changeset]: Internal refactoring -->',
        ),
        false,
      );
    });

    it('should return false when [no-changeset] is inside a multi-line HTML comment', () => {
      assert.equal(
        hasNoChangesetAnnotation(
          '<!--\n[no-changeset]: Internal refactoring\n-->',
        ),
        false,
      );
    });

    it('should return false for the default PR template with commented-out tag', () => {
      const templateBody = [
        '## Checklist',
        '- [ ] There is a changeset for this change, or one is not required',
        '<!-- If this change does not require a changeset, uncomment the tag and explain why -->',
        '<!-- [no-changeset]: -->',
      ].join('\n');
      assert.equal(hasNoChangesetAnnotation(templateBody), false);
    });

    it('should return true when [no-changeset] exists both in and outside an HTML comment', () => {
      const body = [
        '<!-- [no-changeset]: -->',
        '[no-changeset]: Actually I do want to skip',
      ].join('\n');
      assert.equal(hasNoChangesetAnnotation(body), true);
    });

    it('should return false when no tag is present', () => {
      assert.equal(hasNoChangesetAnnotation('Regular PR description'), false);
    });

    it('should return false for empty body', () => {
      assert.equal(hasNoChangesetAnnotation(''), false);
    });
  });

  describe('changeset checkbox cross-validation', () => {
    it('should fail with specific message when checkbox is ticked but no changeset file exists', async () => {
      setupMockFiles(['src/index.js']);
      setupMockComments([]);
      setupMockPR(
        [
          '## Checklist',
          '- [x] There is a changeset for this change, or one is not required',
        ].join('\n'),
      );

      try {
        await validateChangesets({
          octokit: mockOctokit,
          owner: 'test',
          repo: 'test',
          pullNumber: 1,
        });
        assert.fail('Expected function to throw');
      } catch (error) {
        assert.equal(
          error.message,
          'Changeset checkbox is ticked but no changeset file was found',
        );
      }
    });

    it('should pass when checkbox is ticked and changeset file exists', async () => {
      setupMockFiles(['.changeset/happy-cats-run.md', 'src/index.js']);
      setupMockComments([]);
      setupMockPR(
        [
          '## Checklist',
          '- [x] There is a changeset for this change, or one is not required',
        ].join('\n'),
      );

      await validateChangesets({
        octokit: mockOctokit,
        owner: 'test',
        repo: 'test',
        pullNumber: 1,
      });

      assert.equal(process.exitCode, 0);
    });

    it('should pass when checkbox is ticked and uncommented [no-changeset] annotation exists', async () => {
      setupMockFiles(['src/index.js']);
      setupMockComments([]);
      setupMockPR(
        [
          '## Checklist',
          '- [x] There is a changeset for this change, or one is not required',
          '[no-changeset]: Internal refactoring',
        ].join('\n'),
      );

      await validateChangesets({
        octokit: mockOctokit,
        owner: 'test',
        repo: 'test',
        pullNumber: 1,
      });

      assert.equal(process.exitCode, 0);
    });

    it('should fail when checkbox is ticked and [no-changeset] is only in an HTML comment', async () => {
      setupMockFiles(['src/index.js']);
      setupMockComments([]);
      setupMockPR(
        [
          '## Checklist',
          '- [x] There is a changeset for this change, or one is not required',
          '<!-- [no-changeset]: Internal refactoring -->',
        ].join('\n'),
      );

      try {
        await validateChangesets({
          octokit: mockOctokit,
          owner: 'test',
          repo: 'test',
          pullNumber: 1,
        });
        assert.fail('Expected function to throw');
      } catch (error) {
        assert.equal(
          error.message,
          'Changeset checkbox is ticked but no changeset file was found',
        );
      }
    });

    it('should fail with generic message when checkbox is not ticked and no changeset exists', async () => {
      setupMockFiles(['src/index.js']);
      setupMockComments([]);
      setupMockPR(
        [
          '## Checklist',
          '- [ ] There is a changeset for this change, or one is not required',
        ].join('\n'),
      );

      try {
        await validateChangesets({
          octokit: mockOctokit,
          owner: 'test',
          repo: 'test',
          pullNumber: 1,
        });
        assert.fail('Expected function to throw');
      } catch (error) {
        assert.equal(error.message, 'No changeset found in PR');
      }
    });

    it('should delete stale error comment when changeset file is added', async () => {
      const staleCommentId = 42;
      setupMockFiles(['.changeset/happy-cats-run.md', 'src/index.js']);
      setupMockComments([
        {
          id: staleCommentId,
          body: '## Missing Changeset\nNo changeset found in PR.',
          user: {login: 'github-actions[bot]'},
        },
      ]);
      setupMockPR(
        [
          '## Checklist',
          '- [x] There is a changeset for this change, or one is not required',
        ].join('\n'),
      );

      await validateChangesets({
        octokit: mockOctokit,
        owner: 'test',
        repo: 'test',
        pullNumber: 1,
      });

      assert.equal(process.exitCode, 0);
      sinon.assert.calledWith(mockOctokit.rest.issues.deleteComment, {
        owner: 'test',
        repo: 'test',
        comment_id: staleCommentId,
      });
    });

    it('should delete stale error comment when [no-changeset] annotation is added', async () => {
      const staleCommentId = 99;
      setupMockFiles(['src/index.js']);
      setupMockComments([
        {
          id: staleCommentId,
          body: '## Missing Changeset\nNo changeset found in PR.',
          user: {login: 'github-actions[bot]'},
        },
      ]);
      setupMockPR(
        [
          '## Checklist',
          '- [x] There is a changeset for this change, or one is not required',
          '[no-changeset]: Internal refactoring',
        ].join('\n'),
      );

      await validateChangesets({
        octokit: mockOctokit,
        owner: 'test',
        repo: 'test',
        pullNumber: 1,
      });

      assert.equal(process.exitCode, 0);
      sinon.assert.calledWith(mockOctokit.rest.issues.deleteComment, {
        owner: 'test',
        repo: 'test',
        comment_id: staleCommentId,
      });
    });

    it('should update existing error comment instead of creating a duplicate when validation still fails', async () => {
      const existingCommentId = 77;
      setupMockFiles(['src/index.js']);
      setupMockComments([
        {
          id: existingCommentId,
          body: '## Missing Changeset\nNo changeset found in PR.',
          user: {login: 'github-actions[bot]'},
        },
      ]);
      setupMockPR(
        [
          '## Checklist',
          '- [x] There is a changeset for this change, or one is not required',
        ].join('\n'),
      );

      try {
        await validateChangesets({
          octokit: mockOctokit,
          owner: 'test',
          repo: 'test',
          pullNumber: 1,
        });
        assert.fail('Expected function to throw');
      } catch (error) {
        assert.equal(
          error.message,
          'Changeset checkbox is ticked but no changeset file was found',
        );
        sinon.assert.notCalled(mockOctokit.rest.issues.createComment);
        sinon.assert.calledWith(mockOctokit.rest.issues.updateComment, {
          owner: 'test',
          repo: 'test',
          comment_id: existingCommentId,
          body: sinon.match.string,
        });
      }
    });

    it('should not call deleteComment when there is no stale comment', async () => {
      setupMockFiles(['.changeset/happy-cats-run.md', 'src/index.js']);
      setupMockComments([]);
      setupMockPR(
        [
          '## Checklist',
          '- [x] There is a changeset for this change, or one is not required',
        ].join('\n'),
      );

      await validateChangesets({
        octokit: mockOctokit,
        owner: 'test',
        repo: 'test',
        pullNumber: 1,
      });

      assert.equal(process.exitCode, 0);
      sinon.assert.notCalled(mockOctokit.rest.issues.deleteComment);
    });
  });
});
