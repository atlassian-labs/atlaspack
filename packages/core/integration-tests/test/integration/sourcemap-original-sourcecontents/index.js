import * as React from "react";
import * as ReactDOM from "react-dom";

function App(props) {
  return <div>{props.bar}</div>;
}

ReactDOM.render(<App bar="bar" />, document.getElementById("root"));
