const React = require('react');

function InputDisplay({input}) {
  if (!input || Object.keys(input).length === 0) {
    return (
      <div className="input-display">
        <h3>Request Input</h3>
        <p className="no-input">No input provided</p>
      </div>
    );
  }

  return (
    <div className="input-display">
      <h3>Request Input</h3>
      <div className="input-content">
        <pre>{JSON.stringify(input, null, 2)}</pre>
      </div>
    </div>
  );
}

module.exports = InputDisplay;
