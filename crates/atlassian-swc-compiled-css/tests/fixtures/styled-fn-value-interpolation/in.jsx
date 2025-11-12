import { styled } from '@compiled/react';

const Component = styled.button`
	color: ${(props) => (props.isPrimary ? 'blue' : 'red')};
	/* annoying-comment */
	text-decoration-line: ${({ isDone }) => (isDone ? 'line-through' : 'none')};
	-webkit-line-clamp: ${({ isClamped }) => (isClamped ? 3 : 1)};
	font-size: 30px;
`;

export const View = () => <Component />;
