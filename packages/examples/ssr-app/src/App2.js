const Header = require('./Header');
const StatsDisplay = require('./StatsDisplay');
const InfoCard = require('./InfoCard');

function App2({input}) {
  return (
    <div className="app">
      <Header title={input?.title || 'SSR App 2'} />

      <div className="app-content">
        <div className="app-section">
          <InfoCard title="Detailed Statistics">
            <StatsDisplay timestamp={input?.timestamp} />
          </InfoCard>
        </div>
      </div>
    </div>
  );
}

module.exports = App2;
