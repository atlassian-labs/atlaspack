/** @jsx jsx */
import { jsx, cssMap } from '@compiled/react';

const styles = cssMap({
  root: {
    gridArea: 'banner',
    height: 'var(--banner-height)',
    insetBlockStart: 0,
    position: 'sticky',
    zIndex: 100,
    overflow: 'hidden',
  },
});

function Banner({ children, xcss, height = 48, testId }) {
  return (
    <div 
      data-layout-slot 
      css={styles.root} 
      data-testid={testId}
    >
      {children}
    </div>
  );
}

export default Banner;