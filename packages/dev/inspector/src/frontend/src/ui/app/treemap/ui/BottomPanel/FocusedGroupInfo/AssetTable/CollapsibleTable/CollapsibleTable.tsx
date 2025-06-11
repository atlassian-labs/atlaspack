import styles from '../../../../../../../App.module.css';
import {observer} from 'mobx-react-lite';
import {CollapsibleTableModel} from './CollapsibleTableModel';
import {runInAction} from 'mobx';
import {CollapsibleTableRow} from './CollapsibleTableRow';

function limit(value: number, len: number) {
  if (value < 0) {
    return len - (Math.abs(value) % len);
  }
  return value % len;
}

export const CollapsibleTable = observer(
  ({model}: {model: CollapsibleTableModel}) => {
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

    return (
      <div style={{width: '100%'}}>
        <table
          style={{width: '100%'}}
          className={styles.treemapTable}
          onKeyDown={(e) => {
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
                    model.flatNodeList[
                      limit(current + 1, model.flatNodeList.length)
                    ];
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
          }}
          onFocus={(e) => {
            let current = e.target as HTMLElement;
            while (current && !current.getAttribute('data-nodeid')) {
              current = current.parentElement as HTMLElement;
            }

            runInAction(() => {
              model.focusedNodeId = current.getAttribute('data-nodeid');
            });
          }}
        >
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
  },
);
