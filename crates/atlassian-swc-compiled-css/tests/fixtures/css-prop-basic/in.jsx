import { jsx } from '@compiled/react';

function Component() {
  return (
    <div
      css={{
        color: 'red',
        backgroundColor: 'blue',
        padding: '8px',
        '&:hover': {
          color: 'green'
        }
      }}
    >
      Hello World
    </div>
  );
}