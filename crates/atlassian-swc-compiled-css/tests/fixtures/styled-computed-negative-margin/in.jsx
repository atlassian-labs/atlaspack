/** @jsx jsx */
import { jsx, styled } from '@compiled/react';

const gridSize = 8;

const IssueContainer = styled.div({
	marginLeft: `${-gridSize * 2.25}px`,
	height: '100%',
});

const Component = () => (
	<IssueContainer>
		<div>Content</div>
	</IssueContainer>
);

export default Component;