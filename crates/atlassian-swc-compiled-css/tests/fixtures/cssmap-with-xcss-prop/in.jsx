import React from 'react';
import { cssMap } from '@atlaskit/css';
import { Box } from '@atlaskit/primitives/compiled';

const styles = cssMap({
	avatarItemWrapper: {
		marginLeft: '-6px',
		paddingRight: '8px',
	},
	container: {
		display: 'flex',
		alignItems: 'center',
		backgroundColor: '#f4f5f7',
	},
	text: {
		fontSize: '14px',
		fontWeight: 'bold',
		color: '#172b4d',
	},
});

export const Component = ({ name, picture }) => {
	return (
		<Box xcss={styles.avatarItemWrapper}>
			<div className={styles.container()}>
				<img src={picture} alt={name} />
				<span className={styles.text()}>{name}</span>
			</div>
		</Box>
	);
};

export default Component;