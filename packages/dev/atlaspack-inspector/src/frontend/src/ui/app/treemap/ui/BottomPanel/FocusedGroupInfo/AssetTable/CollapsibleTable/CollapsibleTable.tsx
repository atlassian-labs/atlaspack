import {observer} from 'mobx-react-lite';
import {CollapsibleTableModel} from './CollapsibleTableModel';
import {runInAction} from 'mobx';
import {CollapsibleTableRow} from './CollapsibleTableRow';

import * as styles from './CollapsibleTable.module.css';

function limit(value: number, len: number) {
  if (value < 0) {
    return len - (Math.abs(value) % len);
  }
  return value % len;
}

interface CollapsibleTableProps {
  model: CollapsibleTableModel;
}

export const CollapsibleTable = observer(({model}: CollapsibleTableProps) => {
  function focusOnNode(nodeId: string) {
    runInAction(() => {
      model.focusedNodeId = nodeId;
    });

    const node = document.querySelector(
      `[data-nodeid="${model.focusedNodeId}"]`,
    );
    if (node) {
      node.scrollIntoView({behavior: 'smooth', block: 'center'});
      (node as HTMLElement).focus();
    }
  }

  /**
   * Handles keyboard navigation in 4 directions.
   */
  function onKeyDown(e: React.KeyboardEvent<HTMLTableElement>) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      e.stopPropagation();

      runInAction(() => {
        const current = model.flatNodeList.findIndex(
          (node: any) => node.id === model.focusedNodeId,
        );
        if (current === -1) {
          focusOnNode(model.flatNodeList[0].id);
        } else {
          const increment = e.key === 'ArrowDown' ? 1 : -1;
          focusOnNode(
            model.flatNodeList[
              limit(current + increment, model.flatNodeList.length)
            ].id,
          );
        }
      });
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      e.stopPropagation();

      const current = model.flatNodeList.findIndex(
        (node: any) => node.id === model.focusedNodeId,
      );
      runInAction(() => {
        if (current !== -1) {
          model.flatNodeList[current].isExpanded = true;

          const newNode =
            model.flatNodeList[limit(current + 1, model.flatNodeList.length)];
          if (newNode.level > model.flatNodeList[current].level) {
            focusOnNode(newNode.id);
          }
        }
      });
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      e.stopPropagation();

      const current = model.flatNodeList.find(
        (node: any) => node.id === model.focusedNodeId,
      );
      if (current && current.isExpanded) {
        runInAction(() => {
          current.isExpanded = false;
        });
      } else if (current && current.parent) {
        focusOnNode(current.parent);
      }
    }
  }

  /**
   * When a node is focused, walk up until we find the `data-nodeid` of the focused row,
   * then update view model to focus on that node.
   */
  function onFocus(e: React.FocusEvent<HTMLTableElement>) {
    let current = e.target as HTMLElement;
    while (current && !current.getAttribute('data-nodeid')) {
      current = current.parentElement as HTMLElement;
    }

    runInAction(() => {
      model.focusedNodeId = current.getAttribute('data-nodeid');
    });
  }

  return (
    <div className={styles.collapsibleTable}>
      <table onKeyDown={onKeyDown} onFocus={onFocus}>
        <tbody>
          {model.flatNodeList.map((node, i) => (
            <CollapsibleTableRow
              key={i}
              node={node}
              level={node.level}
              model={model}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
});
