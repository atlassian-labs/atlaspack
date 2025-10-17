import { styled } from '@compiled/react';

export const BadgeSkeleton = styled.span({
	width: ({ width }) => width,
	minWidth: ({ width: w }) => w,
	maxWidth: (propz) => propz.width,
});

export const View = () => <BadgeSkeleton />;
