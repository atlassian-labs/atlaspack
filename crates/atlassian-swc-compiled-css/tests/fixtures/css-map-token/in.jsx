import { cssMap } from '@compiled/react';
import { token } from '@atlaskit/tokens';

const styles = cssMap({
  root: {
    paddingBlockStart: token('space.0'),
    paddingBlockEnd: token('space.0'),
  },
});

export const Component = () => <div className={styles.root}>Hi</div>;
