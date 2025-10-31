import { styled } from '@compiled/react';

const gridSize = 8;

const HeadingCellWrapper = styled.div`
  display: inline-block;
  padding: 0;
  padding-left: ${props => (props.first ? gridSize : gridSize / 2)}px;
  padding-right: ${props => (props.last ? gridSize : gridSize / 2)}px;
`;

export const Component = ({ first, last }) => (
  <HeadingCellWrapper first={first} last={last}>
    content
  </HeadingCellWrapper>
);
