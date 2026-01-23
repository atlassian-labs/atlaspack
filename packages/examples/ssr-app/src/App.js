const Header = require('./Header');
const DateDisplay = require('./DateDisplay');
const TimeDisplay = require('./TimeDisplay');
const InfoCard = require('./InfoCard');

function App({input}) {
  return (
    <div className="app">
      <Header title={input?.title || 'SSR App'} />

      <div className="app-content">
        <div className="app-section">
          <InfoCard title="Date & Time Information">
            <DateDisplay timestamp={input?.timestamp} />
            <TimeDisplay timestamp={input?.timestamp} />
          </InfoCard>
        </div>
      </div>
    </div>
  );
}

module.exports = App;
