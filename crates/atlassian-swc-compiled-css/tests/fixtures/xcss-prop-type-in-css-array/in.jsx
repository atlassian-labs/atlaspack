import { cssMap } from '@compiled/react';

const styles = cssMap({
  root: {
    display: 'grid',
    minHeight: '100vh',
    gridTemplateAreas: `
      "banner"
      "top-bar"
      "main"
      "aside"
    `,
    gridTemplateColumns: 'minmax(0, 1fr)',
    gridTemplateRows: 'auto auto 1fr auto',
    '@media (min-width: 64rem)': {
      gridTemplateAreas: `
        "banner banner banner"
        "top-bar top-bar top-bar"
        "side-nav main aside"
      `,
      gridTemplateRows: 'auto auto 3fr',
      gridTemplateColumns: 'auto minmax(0,1fr) auto',
    },
    '> :not([data-layout-slot])': {
      display: 'none !important',
    },
  },
});

function Root({ xcss }) {
  return (
    <div
      css={styles.root}
    >
      Content
    </div>
  );
}