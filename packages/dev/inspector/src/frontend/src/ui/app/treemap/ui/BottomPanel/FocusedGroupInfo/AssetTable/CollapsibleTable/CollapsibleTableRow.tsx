import {Fragment, useEffect} from 'react';
import {observer} from 'mobx-react-lite';
import {runInAction} from 'mobx';
import {BitbucketIcon} from '@atlaskit/logo';

import styles from './CollapsibleTableRow.module.css';
import {
  CollapsibleTableModel,
  CollapsibleTableNode,
} from './CollapsibleTableModel';

interface CollapsibleTableRowProps {
  model: CollapsibleTableModel;
  node: CollapsibleTableNode;
  level: number;
}

export const CollapsibleTableRow = observer(
  ({node, level, model}: CollapsibleTableRowProps) => {
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
          className={styles.collapsibleTableRow}
          data-nodeid={node.id}
          tabIndex={0}
          autoFocus={model.focusedNodeId === node.id}
        >
          <td
            className={styles.collapsibleTableRowPath}
            style={{paddingLeft: level * 16}}
          >
            {node.children.length > 0 ? (
              <button
                onClick={() => {
                  runInAction(() => {
                    node.isExpanded = !node.isExpanded;
                  });
                }}
                className={styles.collapsibleTableRowPathButton}
              >
                {node.isExpanded ? '▼' : '▶'}
              </button>
            ) : (
              <span className={styles.collapsibleTableRowPathPlaceholder} />
            )}

            {node.path}
          </td>
          <td>
            <a
              href={node.sourceCodeUrl}
              rel="noopener noreferrer"
              target="_blank"
            >
              <BitbucketIcon size="small" />
            </a>
          </td>
        </tr>
      </Fragment>
    );
  },
);
