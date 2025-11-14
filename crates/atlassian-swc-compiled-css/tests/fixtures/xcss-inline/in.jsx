import { css } from '@compiled/react';

const PADDING = 8;
const BLUE = 'blue';

<>
	<div xcss={{ color: BLUE, padding: PADDING }} />
	<button data-test xCss={{ '&:hover': { color: 'red' } }} />
</>;
