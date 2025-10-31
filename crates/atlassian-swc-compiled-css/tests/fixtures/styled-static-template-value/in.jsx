import { styled } from '@compiled/react';

const GRID = 8;
const SIZE = GRID * 2;

const Icon = styled.div({
  width: `${SIZE}px`,
  minWidth: `${SIZE}px`,
  height: `${SIZE}px`,
  flexBasis: `${SIZE}px`,
});

export const Component = () => <Icon />;
