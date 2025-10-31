import { cssMap, cx } from '@atlaskit/css';

const styles = cssMap({
  base: {
    color: 'var(--ds-text-subtle,#44546f)',
    paddingTop: 'var(--ds-space-100,8px)',
  },
  extra: {
    marginBottom: 'var(--ds-space-200,1pc)',
  },
});

export const Component = ({ showExtra }) => (
  <div xcss={cx(styles.base, showExtra ? styles.extra : null)}>content</div>
);
