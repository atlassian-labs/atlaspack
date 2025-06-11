import {observer} from 'mobx-react-lite';
import {viewModel} from '../../../../model/ViewModel';
import {formatBytes} from '../../../../util/formatBytes';
import styles from './TreemapTooltip.module.css';

export const TreemapTooltip = observer(() => {
  if (!viewModel.tooltipState) {
    return null;
  }

  return (
    <div
      className={styles.treemapTooltip}
      style={{
        left: viewModel.mouseState.x + 10,
        top: viewModel.mouseState.y + 10,
      }}
    >
      {viewModel.tooltipState.group.label}
      <br />
      {formatBytes(viewModel.tooltipState.group.weight)}
    </div>
  );
});
