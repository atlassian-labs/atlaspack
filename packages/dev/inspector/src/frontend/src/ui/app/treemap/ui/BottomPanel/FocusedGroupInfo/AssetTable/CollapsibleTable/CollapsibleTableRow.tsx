import {Fragment, useEffect} from 'react';
import {observer} from 'mobx-react-lite';
import {runInAction} from 'mobx';
import {BitbucketIcon} from '@atlaskit/logo';

export const CollapsibleTableRow = observer(({node, level = 0, model}: any) => {
  useEffect(() => {
    if (model.focusedNodeId === node.id) {
      const row = document.querySelector(
        `[data-nodeid="${model.focusedNodeId}"]`,
      );
      if (row) {
        (row as HTMLElement).focus();
      }
    }
  }, [model, node]);

  return (
    <Fragment>
      <tr
        style={{height: '20px'}}
        data-nodeid={node.id}
        tabIndex={0}
        autoFocus={model.focusedNodeId === node.id}
      >
        <td
          style={{
            verticalAlign: 'baseline',
            paddingLeft: level * 16,
            display: 'flex',
            gap: 8,
          }}
        >
          {node.children.length > 0 ? (
            <button
              onClick={() => {
                runInAction(() => {
                  node.isExpanded = !node.isExpanded;
                });
              }}
              style={{border: 'none', background: 'none', width: '16px'}}
            >
              {node.isExpanded ? '▼' : '▶'}
            </button>
          ) : (
            <span style={{width: '16px'}} />
          )}

          {node.path}
        </td>
        <td>
          <a
            href={`https://bitbucket.org/atlassian/atlassian-frontend-monorepo/src/master/jira/${node.path}`}
            rel="noopener noreferrer"
            target="_blank"
          >
            <BitbucketIcon size="small" />
          </a>
        </td>
      </tr>
    </Fragment>
  );
});
