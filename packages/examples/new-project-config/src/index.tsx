import React from 'react';
import ReactDOM from 'react-dom';

function logRender(target: any) {
  const originalRender = target.prototype.render;
  target.prototype.render = function () {
    console.log('Component is being rendered');
    return originalRender.apply(this, arguments);
  };
  return target;
}

@logRender
class Component extends React.Component {
  render() {
    return <div>Component</div>;
  }
}

const App = () => {
  return (
    <>
      <h1>Config demo</h1>
      <Component />
    </>
  );
};

ReactDOM.render(<App />, document.getElementById('container'));
