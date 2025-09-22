import path from 'path';
import fs from 'fs';
import childProcess from 'child_process';

const PROJECT_ROOT_DIRS = [
  '.parcelrc',
  '.atlaspackrc',
  'yarn.lock',
  'package-lock.json',
  'pnpm-lock.yaml',
];

/**
 * Finds a root directory that contains a marker directory.
 *
 * @param target - The target directory to search from.
 * @param candidate - The marker directory to search for.
 */
export function findRoot(target: string, candidate: string): string | null {
  let projectRoot = path.resolve(process.cwd(), target);
  let exists = false;
  while (projectRoot !== '/' && projectRoot !== '.') {
    if (fs.existsSync(path.join(projectRoot, candidate))) {
      exists = true;
      break;
    }
    projectRoot = path.dirname(projectRoot);
  }

  if (!exists) {
    return null;
  }

  return projectRoot;
}

/**
 * Finds a root directory that contains a `.git` directory.
 * This is not quite matching `@atlaspack/core`'s logic.
 *
 * @param target - The target directory to search from.
 */
export function findRepositoryRoot(target: string): string | null {
  return findRoot(target, '.git');
}

/**
 * Finds a root directory that contains a project root directory.
 *
 * @param target - The target directory to search from.
 */
export function findProjectRoot(target: string): string | null {
  for (const candidate of PROJECT_ROOT_DIRS) {
    const root = findRoot(target, candidate);
    if (root) {
      return root;
    }
  }
  return null;
}

/**
 * A parsed remote repository URL, for either GitHub or BitBucket repositories.
 *
 * @example
 * ```ts
 * // github.com/owner/repo
 * {
 *   owner: 'owner',
 *   repo: 'repo',
 *   type: 'github',
 * }
 * ```
 *
 * @example
 * ```ts
 * // bitbucket.org/owner/repo
 * {
 *   owner: 'owner',
 *   repo: 'repo',
 *   type: 'bitbucket',
 * }
 * ```
 */
export interface SourceCodeURL {
  owner: string;
  repo: string;
  type: 'github' | 'bitbucket';
}

/**
 * Based on the directory path, find a source code URL for this project.
 *
 * This is based on parsing `git remote` URLs.
 *
 * Both SSH and HTTP URLs should be supported for both GitHub and BitBucket
 * Cloud.
 *
 * If a repository has multiple remotes, the first GitHub/BitBucket remote
 * will be used.
 *
 * This might not work for repositories using BitBucket Server, or mirror
 * URLs as remotes.
 */
export function findSourceCodeURL(target: string): SourceCodeURL | null {
  const repositoryRoot = findRepositoryRoot(target);
  const projectRoot = findProjectRoot(target);

  if (!repositoryRoot || !projectRoot) {
    return null;
  }

  const remotes = childProcess
    .execSync('git remote -v', {
      cwd: repositoryRoot,
    })
    .toString()
    .split('\n')
    .filter(Boolean)
    .map((line) => line.split('\t'))
    .map(([name, url]) => ({name, url}));

  const remote = remotes.find(
    ({url}) => url.includes('bitbucket.org') || url.includes('github.com'),
  )?.url;

  if (!remote) {
    return null;
  }

  const remoteUrl = remote.split(' ')[0];
  const type = remoteUrl.includes('bitbucket.org') ? 'bitbucket' : 'github';

  // get owner and repo from HTTP or SSH urls
  const regex =
    /^(?:https?:\/\/|git@)(github|bitbucket)\.(com|org)[:/]([^/\s.]+)\/([^/\s.]+)(\.git)?/;
  const match = remoteUrl.match(regex);

  if (!match) {
    return null;
  }

  const [, , , owner, repo] = match;

  return {type, owner, repo};
}
