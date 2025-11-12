import { css } from '@compiled/react';

const styles = css({ color: 'green' });

const cond = true;

<>
	<div xcss={styles} />
	<div xcss={cond ? styles : css({ color: 'blue' })} />
	<div xcss={Math.random() > 0.5 ? styles : styles} />
</>;
