import { styled } from '@compiled/react';
import { token } from '@atlaskit/tokens';

const DotStart = styled.div({
  position: 'absolute',
  top: (props) => `${props.y}px`,
  left: (props) => `${props.x}px`,
  borderRadius: token('radius.full'),
  width: '10px',
  height: '10px',
  transform: 'translate(-5px, -5px)',
  backgroundColor: token('color.background.accent.blue.subtler'),
});

const DotEnd = styled(DotStart)({
  backgroundColor: token('color.background.accent.red.subtler'),
});

export const Example = () => <DotEnd x={10} y={20} />;
