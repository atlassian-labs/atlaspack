import path from 'path';
import fs from 'fs';
import childProcess from 'child_process';

export function findProjectRoot(target: string): string | null {
  let projectRoot = path.resolve(process.cwd(), target);
  let exists = false;
  while (projectRoot !== '/' && projectRoot !== '.') {
    const gitDirectory = path.join(projectRoot, '.git');
    if (fs.existsSync(gitDirectory)) {
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

export interface SourceCodeURL {
  owner: string;
  repo: string;
  type: 'github' | 'bitbucket';
}

/**
 * Based on the directory path, find a source code URL for this project.
 */
export function findSourceCodeURL(target: string): SourceCodeURL | null {
  const projectRoot = findProjectRoot(target);
  if (!projectRoot) {
    return null;
  }

  const remotes = childProcess
    .execSync('git remote -v', {
      cwd: projectRoot,
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
    /(?:https?:\/\/|git@)(github|bitbucket)\.(com|org)[:/]([^/\s.]+)\/([^/\s.]+)(\.git)?/;
  const match = remoteUrl.match(regex);

  if (!match) {
    return null;
  }

  const [, , , owner, repo] = match;

  return {type, owner, repo};
}
