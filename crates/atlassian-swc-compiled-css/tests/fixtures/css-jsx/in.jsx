import { css } from '@compiled/react';

<>
	<div css={{ color: 'blue' }} />
	<div css={css({ color: 'blue' })} className={cx('A', 'B')} />
</>;
