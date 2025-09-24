import {getFileURL} from './getFileURL';

describe('getFileURL', () => {
  it('should return the correct URL for a file in the project', () => {
    const fileURL = getFileURL(
      'jira/src/ui/app/treemap/ui/BottomPanel/FocusedGroupInfo/AssetTable/AssetTable.tsx',
      {
        owner: 'atlassian',
        repo: 'atlassian-frontend',
        type: 'bitbucket',
      },
    );

    expect(fileURL).toEqual({
      type: 'bitbucket',
      url: 'https://bitbucket.org/atlassian/atlassian-frontend/src/master/jira/src/ui/app/treemap/ui/BottomPanel/FocusedGroupInfo/AssetTable/AssetTable.tsx',
    });
  });
});
