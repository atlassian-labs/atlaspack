import React from 'react';
import { styled } from '@compiled/react';

const selectors = {
  helper: 'helper-class',
  id: 'target-id',
};

const Component = styled.div({
  color: 'rebeccapurple',
  backgroundColor: ({ isActive }) => (isActive ? 'lime' : 'cyan'),
  [`.${selectors.helper}`]: {
    padding: '4px',
    color: 'tomato',
  },
  [`#${selectors.id}`]: {
    marginTop: '8px',
    '&:hover': {
      opacity: 0.5,
    },
  },
});

export const Styled = () => (
  <Component isActive>
    <span className="helper-class">Helper</span>
    <span id="target-id">Target</span>
  </Component>
);
