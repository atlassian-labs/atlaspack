import { styled } from '@compiled/react';

const padding = '8px';
const large = 8;
const small = 4;

const Cell = styled.div`
  padding: ${padding};
  padding-left: ${(props) => (props.first ? large : small)}px;
  padding-right: ${(props) => (props.last ? large : small)}px;
`;

export const Component = ({ first, last }) => (
  <Cell first={first} last={last}>
    Content
  </Cell>
);
