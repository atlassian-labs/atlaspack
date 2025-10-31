import { css, cssMap } from '@atlaskit/css';

const styles = cssMap({
  container: {
    paddingTop: 'var(--ds-space-100,8px)',
    paddingRight: 'var(--ds-space-100,8px)',
    paddingBottom: 'var(--ds-space-100,8px)',
    '&:hover': {
      backgroundColor: 'var(--ds-background-neutral-hovered,#091e4224)',
      cursor: 'pointer',
    },
  },
  fieldName: {
    flexGrow: 1,
  },
});

const labelStyles = css({
  display: 'block',
  width: '100%',
  cursor: 'pointer',
});

export const Component = () => (
  <label css={labelStyles}>
    <div xcss={styles.container}>content</div>
  </label>
);
