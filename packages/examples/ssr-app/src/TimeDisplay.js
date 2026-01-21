const React = require('react');

function TimeDisplay({timestamp}) {
  const time = timestamp ? new Date(timestamp) : new Date();
  const formattedTime = time.toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: true,
  });

  return (
    <div className="time-display">
      <h3>Current Time</h3>
      <p>{formattedTime}</p>
    </div>
  );
}

module.exports = TimeDisplay;
