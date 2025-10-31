import { cssMap } from '@atlaskit/css';

const styles = cssMap({
  linkField: {
    display: 'flex',
    alignItems: 'baseline',
    justifyContent: 'start',
    gap: 'var(--ds-space-050,4px)',
    color: 'var(--ds-text-subtle,#44546f)',
    paddingTop: 'var(--ds-space-100,8px)',
    paddingBottom: 'var(--ds-space-100,8px)',
    '> button': {
      alignSelf: 'end',
    },
  },
});

export const Component = () => <div xcss={styles.linkField}>content</div>;
