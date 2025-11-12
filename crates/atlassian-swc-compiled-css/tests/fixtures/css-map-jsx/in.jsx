import { css, cssMap } from '@compiled/react';

const isActive = true;
const count = 1;
const themes = cssMap({
	primary: { color: 'red', '&:hover': { color: 'blue' } },
	secondary: { backgroundColor: '#eee' },
	danger: { color: 'crimson' },
});

<>
	<div css={themes.primary} />
	<div css={[themes.primary, isActive && themes.secondary]} />
	<div css={[false && themes.secondary, themes.primary]} className={cx('A')} />
	<div css={[themes.secondary, themes.danger, count > 0 && themes.primary]} />
</>;
