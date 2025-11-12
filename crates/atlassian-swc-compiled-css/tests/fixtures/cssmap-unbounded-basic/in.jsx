import { cssMap } from '@compiled/react';

const styles = cssMap({
  container: {
    display: 'inline-flex',
    borderRadius: '3px',
    blockSize: 'min-content',
    position: 'static',
    overflow: 'hidden',
    paddingInline: '4px',
    boxSizing: 'border-box',
  },
  text: {
    fontFamily: 'ui-sans-serif',
    fontSize: '11px',
    fontStyle: 'normal',
    fontWeight: 'bold',
    lineHeight: '16px',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    textTransform: 'uppercase',
    whiteSpace: 'nowrap',
  },
});

export const Component = () => (
  <div css={styles.container}>
    <span css={styles.text}>Hello</span>
  </div>
);