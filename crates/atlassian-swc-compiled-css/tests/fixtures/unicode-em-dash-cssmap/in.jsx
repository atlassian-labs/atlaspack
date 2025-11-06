import { SimpleTag as Tag } from '@atlaskit/tag';
import React from 'react';
import { cssMap } from '@atlaskit/css';
// eslint-disable-next-line @atlaskit/design-system/no-emotion-primitives – to be migrated to @atlaskit/primitives/compiled

const styles = cssMap({
	tag: {
		display: 'inline-block',
		padding: '4px 8px',
		borderRadius: '3px',
		fontSize: '12px',
		fontWeight: 'bold',
		textTransform: 'uppercase',
		// Unicode character – causing boundary issues
		border: '1px solid #ccc',
	},
});

export function TagComponent({ children, color }) {
	return (
		<Tag 
			css={styles.tag}
			color={color}
		>
			{children}
		</Tag>
	);
}