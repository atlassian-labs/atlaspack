import { css, jsx } from '@compiled/react';
import { padding, outline } from './external';

const styles = css({
  padding: `${padding} ${outline} 0 0`,
});

export const Component = () => <div css={styles} />;

