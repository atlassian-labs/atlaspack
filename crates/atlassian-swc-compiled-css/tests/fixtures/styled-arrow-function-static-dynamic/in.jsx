import { styled } from '@compiled/react';

const gridSize = 8;

const Container = styled.div(({ hideDropdownLabel }) => ({
  display: 'flex',
  justifyContent: 'center',
  alignItems: 'center',
  minHeight: `${gridSize * (hideDropdownLabel ? 14 : 17)}px`,
  overflow: 'hidden',
}));

export const Component = ({ hideDropdownLabel }) => (
  <Container hideDropdownLabel={hideDropdownLabel} />
);
