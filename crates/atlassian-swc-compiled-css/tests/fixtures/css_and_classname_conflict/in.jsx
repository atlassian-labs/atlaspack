import { cssMap, jsx } from '@compiled/react';

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

export function Banner({ xcss, testId, id }) {
    return (
        <div 
            id={id} 
            css={styles.root} 
            className={xcss} 
            data-testid={testId}
        >
            Content
        </div>
    );
}