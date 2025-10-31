import { css as compiledCss } from '@compiled/react';
import { color } from './tokens';

const styles = compiledCss({
  color,
});

export const Component = () => <div css={styles}>Hello</div>;
