import {css} from '@compiled/react';
import ReactDOM from 'react-dom';

const stylesRed = css({color: 'red'});
const stylesGreen = css({color: 'greefn'});

const App = () => {
  return (
    <div>
      <p css={stylesRed}>I should be red</p>
      <p css={stylesGreen}>Hello from React</p>
    </div>
  );
};

ReactDOM.render(<App />, document.getElementById('container'));
