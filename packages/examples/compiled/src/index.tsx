import {css} from '@compiled/react';
import ReactDOM from 'react-dom';

const stylesRed = css({color: 'red'});
const stylesGreen = css({color: 'green'});

const App = () => {
  return (
    <div>
      <p css={stylesGreen}>Hello from React</p>
      <p css={stylesRed}>I should be red</p>
    </div>
  );
};

ReactDOM.render(<App />, document.getElementById('container'));
