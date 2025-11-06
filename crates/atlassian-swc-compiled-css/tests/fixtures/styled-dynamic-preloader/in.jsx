import React from 'react';
import { styled } from '@compiled/react';

const Preloader = styled.div((props) => ({
  display: 'flex',
  justifyContent: 'center',
  alignItems: 'center',
  overflow: 'hidden',
  minHeight: props.hideLabel ? '200px' : '120px',
}));

export const Component = ({ hideLabel }) => (
  <Preloader hideLabel={hideLabel}>
    <div>content</div>
  </Preloader>
);
