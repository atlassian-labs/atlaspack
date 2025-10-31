import { css } from '@compiled/react';

const styles = css({
  transition:
    'background-color 0.5s cubic-bezier(0.15, 1.0, 0.3, 1.0), outline-color 0.5s cubic-bezier(0.15, 1.0, 0.3, 1.0)',
});

export const Component = () => <div css={styles} />;
