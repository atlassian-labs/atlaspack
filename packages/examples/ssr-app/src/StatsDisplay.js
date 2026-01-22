const React = require('react');

function StatsDisplay({timestamp}) {
  const now = timestamp ? new Date(timestamp) : new Date();

  const stats = {
    year: now.getFullYear(),
    month: now.getMonth() + 1,
    day: now.getDate(),
    hour: now.getHours(),
    minute: now.getMinutes(),
    second: now.getSeconds(),
    dayOfWeek: now.getDay(),
    dayOfYear: Math.floor(
      (now - new Date(now.getFullYear(), 0, 0)) / 1000 / 60 / 60 / 24,
    ),
    weekOfYear: Math.ceil(now.getDate() / 7),
    timestamp: now.getTime(),
  };

  return (
    <div className="stats-display">
      <h3>Date & Time Statistics</h3>
      <div className="stats-grid">
        <div className="stat-item">
          <span className="stat-label">Year:</span>
          <span className="stat-value">{stats.year}</span>
        </div>
        <div className="stat-item">
          <span className="stat-label">Month:</span>
          <span className="stat-value">{stats.month}</span>
        </div>
        <div className="stat-item">
          <span className="stat-label">Day:</span>
          <span className="stat-value">{stats.day}</span>
        </div>
        <div className="stat-item">
          <span className="stat-label">Hour:</span>
          <span className="stat-value">{stats.hour}</span>
        </div>
        <div className="stat-item">
          <span className="stat-label">Minute:</span>
          <span className="stat-value">{stats.minute}</span>
        </div>
        <div className="stat-item">
          <span className="stat-label">Second:</span>
          <span className="stat-value">{stats.second}</span>
        </div>
        <div className="stat-item">
          <span className="stat-label">Day of Year:</span>
          <span className="stat-value">{stats.dayOfYear}</span>
        </div>
        <div className="stat-item">
          <span className="stat-label">Unix Timestamp:</span>
          <span className="stat-value">{stats.timestamp}</span>
        </div>
      </div>
    </div>
  );
}

module.exports = StatsDisplay;
