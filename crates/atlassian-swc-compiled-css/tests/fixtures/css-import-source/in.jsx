import { css, cssMap } from '@atlaskit/css';

const styles = css({ color: 'red', backgroundColor: 'blue' });
const stylesMap = cssMap({
	primary: { color: 'red', backgroundColor: 'blue' },
	secondary: { color: 'blue', backgroundColor: 'red' },
});

const div = <div css={[styles, stylesMap.primary]} />;
