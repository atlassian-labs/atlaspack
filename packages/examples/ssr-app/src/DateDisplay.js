function DateDisplay({timestamp}) {
  const date = timestamp ? new Date(timestamp) : new Date();
  const formattedDate = date.toLocaleDateString('en-US', {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });

  return (
    <div className="date-display">
      <h2>Current Date</h2>
      <p>{formattedDate}</p>
    </div>
  );
}

module.exports = DateDisplay;
