import { styled } from '@compiled/react';

const Item = styled.div({
  cursor: ({ isClickable }) => (isClickable ? 'pointer' : 'auto'),
  '&:hover, &:focus': {
    outline: 'none',
    backgroundColor: ({ isClickable }) => (isClickable ? 'red' : 'initial'),
  },
  '> *': {
    margin: ({ spaced }) => (spaced ? '0 3px' : '0'),
  },
});

export const Component = ({ isClickable, spaced }) => (
  <Item isClickable={isClickable} spaced={spaced}>
    Content
  </Item>
);
