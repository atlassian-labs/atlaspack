import { styled } from '@compiled/react';
import { gridSize } from './constants';

const Container = styled.div({
  minWidth: `${gridSize * 10}px`,
  maxWidth: `${gridSize * 20}px`,
  marginLeft: 'auto',
  marginRight: 'auto',
  'td:first-child': {
    position: 'relative',
  },
  td: {
    paddingTop: `${gridSize}px`,
    paddingRight: `${gridSize * 2}px`,
    paddingBottom: `${gridSize * 3}px`,
    paddingLeft: `${gridSize * 4}px`,
  },
});

const Logo = styled.div({
  width: `${gridSize * 2}px`,
  height: `${gridSize * 3}px`,
  flexShrink: 0,
});

export const Component = () => (
  <Container>
    <Logo />
    <table>
      <tbody>
        <tr>
          <td>First</td>
          <td>Second</td>
        </tr>
      </tbody>
    </table>
  </Container>
);

