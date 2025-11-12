import { styled } from '@compiled/react';

const Dot = styled.div({
  top: `${(props) => props.y}px`,
  left: `${(props) => props.x}px`,
  position: 'absolute',
  borderRadius: '9999px',
  width: '10px',
  height: '10px',
  transform: 'translate(-5px, -5px)',
  backgroundColor: 'blue',
});

export const Component = () => <Dot x={0} y={0} />;
