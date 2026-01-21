const React = require('react');
const Header = require('./Header');
const DateDisplay = require('./DateDisplay');
const TimeDisplay = require('./TimeDisplay');

function App({input}) {
  return (
    <div className="app">
      <Header title={input?.title || 'SSR App'} />
      <DateDisplay timestamp={input?.timestamp} />
      <TimeDisplay timestamp={input?.timestamp} />
    </div>
  );
}

module.exports = App;
