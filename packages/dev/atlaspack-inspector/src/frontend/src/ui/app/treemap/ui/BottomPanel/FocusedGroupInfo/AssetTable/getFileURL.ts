import {SourceCodeURL} from '../SourceCodeURL';

export function getFileURL(
  filePath: string,
  sourceCodeURL: SourceCodeURL | null,
): {url: string; type: 'github' | 'bitbucket'} | null {
  if (!sourceCodeURL) {
    return null;
  }

  if (sourceCodeURL.type === 'github') {
    return {
      url: `https://github.com/${sourceCodeURL.owner}/${sourceCodeURL.repo}/blob/master/${filePath}`,
      type: 'github',
    };
  } else if (sourceCodeURL.type === 'bitbucket') {
    return {
      url: `https://bitbucket.org/${sourceCodeURL.owner}/${sourceCodeURL.repo}/src/master/${filePath}`,
      type: 'bitbucket',
    };
  }

  return null;
}
