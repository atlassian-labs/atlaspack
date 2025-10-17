import ReactDOM from 'react-dom';
import {jsx, css} from '@compiled/react';

const styles = css({color: 'red'});

const App = () => {
  return <div css={styles}>Hello world!</div>;
};

ReactDOM.render(<App />, document.getElementById('container'));
