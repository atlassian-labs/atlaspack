import {observer} from 'mobx-react-lite';
import {viewModel} from '../../../../model/ViewModel';
import {formatBytes} from '../../../../util/formatBytes';
import * as styles from './TreemapTooltip.module.css';
import {Inline, Stack} from '@atlaskit/primitives';
import {Code} from '@atlaskit/code';
import {ImpactScore} from './ImpactScore';

export const TreemapTooltip = observer(() => {
  const position = {
    left: viewModel.mouseState.x + 30,
    top: viewModel.mouseState.y,
  };

  if (!viewModel.tooltipState) {
    return null;
  }

  const focusedGroup =
    viewModel.focusedGroup?.id !== viewModel.tooltipState.group.id
      ? viewModel.focusedGroup
      : null;

  const focusedBundle =
    viewModel.focusedBundle?.id !== focusedGroup?.id
      ? viewModel.focusedBundle
      : null;

  return (
    <div
      className={styles.treemapTooltip}
      style={{
        left: position.left,
        top: position.top,
      }}
    >
      <Stack space="space.100">
        <Stack space="space.100">
          <Inline space="space.100">
            <strong>{viewModel.tooltipState.group.label}</strong>
            <div>(group)</div>
          </Inline>
          <Inline space="space.100">
            <div>Unminified size</div>
            <Code>{formatBytes(viewModel.tooltipState.group.weight)}</Code>
          </Inline>
        </Stack>

        {focusedGroup && (
          <div className={styles.parent}>
            <Stack space="space.100">
              <Inline space="space.100">
                <strong>{focusedGroup.label}</strong>
                <div>(focused group)</div>
              </Inline>

              <Inline space="space.100">
                <div>Unminified size</div>
                <Code>
                  {formatBytes(
                    focusedGroup.assetTreeSize ?? focusedGroup.weight,
                  )}
                </Code>
              </Inline>

              {focusedGroup.assetTreeSize != null && (
                <Inline space="space.100">
                  <div>Output size on disk</div>
                  <Code>{formatBytes(focusedGroup.weight)}</Code>
                </Inline>
              )}

              <ImpactScore
                parentSize={focusedGroup.weight}
                groupSize={viewModel.tooltipState.group.weight}
                message={`${viewModel.tooltipState.group.label} is ${Math.round(Math.min(1, viewModel.tooltipState.group.weight / focusedGroup.weight) * 100)}% of ${focusedGroup.label}`}
              />
            </Stack>
          </div>
        )}

        {focusedBundle && (
          <div className={styles.parent}>
            <Stack space="space.100">
              <Inline space="space.100">
                <strong>{focusedBundle.label}</strong>
                <div>(focused bundle)</div>
              </Inline>

              <Inline space="space.100">
                <div>Unminified size</div>
                <Code>{formatBytes(focusedBundle.assetTreeSize ?? 0)}</Code>
              </Inline>

              <Inline space="space.100">
                <div>Output size on disk</div>
                <Code>{formatBytes(focusedBundle.weight)}</Code>
              </Inline>

              <ImpactScore
                parentSize={focusedBundle.assetTreeSize ?? 0}
                groupSize={viewModel.tooltipState.group.weight}
                message={`${viewModel.tooltipState.group.label} is ${Math.round(Math.min(1, viewModel.tooltipState.group.weight / (focusedBundle.assetTreeSize ?? 0)) * 100)}% of ${focusedBundle.label}`}
              />
            </Stack>
          </div>
        )}
      </Stack>
    </div>
  );
});
