import assert from 'assert';
import * as sinon from 'sinon';
import path from 'path';
import fs from 'fs';
import childProcess from 'child_process';
import {findProjectRoot, findSourceCodeURL} from './findSourceCodeUrl';

describe('findSourceCodeUrl', function () {
  let sandbox: sinon.SinonSandbox;

  beforeEach(() => {
    sandbox = sinon.createSandbox();
  });

  afterEach(() => {
    sandbox.restore();
  });

  describe('findProjectRoot', function () {
    it('should find project root when .git directory exists', function () {
      const mockTarget = '/path/to/project/subdir';
      const mockResolvedPath = '/resolved/path/to/project/subdir';
      const mockProjectRoot = '/resolved/path/to/project';

      sandbox.stub(path, 'resolve').returns(mockResolvedPath);
      sandbox.stub(path, 'dirname').returns(mockProjectRoot);
      sandbox.stub(path, 'join').returns('/resolved/path/to/project/.git');
      sandbox.stub(fs, 'existsSync').returns(true);

      const result = findProjectRoot(mockTarget);

      assert.equal(result, mockResolvedPath);
      assert(
        (path.resolve as sinon.SinonStub).calledWith(process.cwd(), mockTarget),
      );
      assert(
        (fs.existsSync as sinon.SinonStub).calledWith(
          '/resolved/path/to/project/.git',
        ),
      );
    });

    it('should return null when no .git directory is found', function () {
      const mockTarget = '/path/to/project';
      const mockResolvedPath = '/resolved/path/to/project';

      sandbox.stub(path, 'resolve').returns(mockResolvedPath);
      sandbox.stub(path, 'dirname').callsFake((p: string) => {
        if (p === mockResolvedPath) return '/resolved/path/to';
        if (p === '/resolved/path/to') return '/resolved/path';
        if (p === '/resolved/path') return '/resolved';
        if (p === '/resolved') return '/';
        return '/';
      });
      sandbox
        .stub(path, 'join')
        .callsFake((p1: string, p2: string) => `${p1}/${p2}`);
      sandbox.stub(fs, 'existsSync').returns(false);

      const result = findProjectRoot(mockTarget);

      assert.equal(result, null);
    });

    it('should traverse up directories until finding .git', function () {
      const mockTarget = '/path/to/project/deep/subdir';
      const mockResolvedPath = '/resolved/path/to/project/deep/subdir';

      sandbox.stub(path, 'resolve').returns(mockResolvedPath);
      sandbox.stub(path, 'dirname').callsFake((p: string) => {
        if (p === mockResolvedPath) return '/resolved/path/to/project/deep';
        if (p === '/resolved/path/to/project/deep')
          return '/resolved/path/to/project';
        if (p === '/resolved/path/to/project') return '/resolved/path/to';
        return '/';
      });
      sandbox
        .stub(path, 'join')
        .callsFake((p1: string, _p2: string) => `${p1}/.git`);

      const existsStub = sandbox.stub(fs, 'existsSync');
      existsStub
        .withArgs('/resolved/path/to/project/deep/subdir/.git')
        .returns(false);
      existsStub.withArgs('/resolved/path/to/project/deep/.git').returns(false);
      existsStub.withArgs('/resolved/path/to/project/.git').returns(true);

      const result = findProjectRoot(mockTarget);

      assert.equal(result, '/resolved/path/to/project');
    });
  });

  describe('findSourceCodeURL', function () {
    it('should return null when no project root is found', function () {
      const mockTarget = '/path/to/project';

      sandbox.stub(path, 'resolve').returns('/resolved/path');
      sandbox.stub(path, 'dirname').returns('/');
      sandbox.stub(path, 'join').returns('/.git');
      sandbox.stub(fs, 'existsSync').returns(false);

      const result = findSourceCodeURL(mockTarget);

      assert.equal(result, null);
    });

    it('should parse GitHub SSH URL correctly', function () {
      const mockTarget = '/path/to/project';

      sandbox.stub(path, 'resolve').returns('/resolved/path');
      sandbox.stub(path, 'dirname').returns('/');
      sandbox.stub(path, 'join').returns('/resolved/path/.git');
      sandbox.stub(fs, 'existsSync').returns(true);

      const mockGitOutput =
        'origin\tgit@github.com:owner/repo.git (fetch)\norigin\tgit@github.com:owner/repo.git (push)';
      sandbox
        .stub(childProcess, 'execSync')
        .returns(Buffer.from(mockGitOutput));

      const result = findSourceCodeURL(mockTarget);

      assert.deepEqual(result, {
        type: 'github',
        owner: 'owner',
        repo: 'repo',
      });
    });

    it('should parse GitHub HTTPS URL correctly', function () {
      const mockTarget = '/path/to/project';

      sandbox.stub(path, 'resolve').returns('/resolved/path');
      sandbox.stub(path, 'dirname').returns('/');
      sandbox.stub(path, 'join').returns('/resolved/path/.git');
      sandbox.stub(fs, 'existsSync').returns(true);

      const mockGitOutput = 'origin\thttps://github.com/owner/repo.git (fetch)';
      sandbox
        .stub(childProcess, 'execSync')
        .returns(Buffer.from(mockGitOutput));

      const result = findSourceCodeURL(mockTarget);

      assert.deepEqual(result, {
        type: 'github',
        owner: 'owner',
        repo: 'repo',
      });
    });

    it('should parse Bitbucket SSH URL correctly', function () {
      const mockTarget = '/path/to/project';

      sandbox.stub(path, 'resolve').returns('/resolved/path');
      sandbox.stub(path, 'dirname').returns('/');
      sandbox.stub(path, 'join').returns('/resolved/path/.git');
      sandbox.stub(fs, 'existsSync').returns(true);

      const mockGitOutput = 'origin\tgit@bitbucket.org:owner/repo.git (fetch)';
      sandbox
        .stub(childProcess, 'execSync')
        .returns(Buffer.from(mockGitOutput));

      const result = findSourceCodeURL(mockTarget);

      assert.deepEqual(result, {
        type: 'bitbucket',
        owner: 'owner',
        repo: 'repo',
      });
    });

    it('should return null when no valid remote is found', function () {
      const mockTarget = '/path/to/project';

      sandbox.stub(path, 'resolve').returns('/resolved/path');
      sandbox.stub(path, 'dirname').returns('/');
      sandbox.stub(path, 'join').returns('/resolved/path/.git');
      sandbox.stub(fs, 'existsSync').returns(true);

      const mockGitOutput = 'origin\tgit@example.com:owner/repo.git (fetch)';
      sandbox
        .stub(childProcess, 'execSync')
        .returns(Buffer.from(mockGitOutput));

      const result = findSourceCodeURL(mockTarget);

      assert.equal(result, null);
    });

    it('should return null when git command fails', function () {
      const mockTarget = '/path/to/project';

      sandbox.stub(path, 'resolve').returns('/resolved/path');
      sandbox.stub(path, 'dirname').returns('/');
      sandbox.stub(path, 'join').returns('/resolved/path/.git');
      sandbox.stub(fs, 'existsSync').returns(true);

      sandbox
        .stub(childProcess, 'execSync')
        .throws(new Error('Git command failed'));

      assert.throws(() => {
        findSourceCodeURL(mockTarget);
      }, Error);
    });

    it('should return null when URL format is invalid', function () {
      const mockTarget = '/path/to/project';

      sandbox.stub(path, 'resolve').returns('/resolved/path');
      sandbox.stub(path, 'dirname').returns('/');
      sandbox.stub(path, 'join').returns('/resolved/path/.git');
      sandbox.stub(fs, 'existsSync').returns(true);

      const mockGitOutput = 'origin\tinvalid-url-format (fetch)';
      sandbox
        .stub(childProcess, 'execSync')
        .returns(Buffer.from(mockGitOutput));

      const result = findSourceCodeURL(mockTarget);

      assert.equal(result, null);
    });
  });
});
