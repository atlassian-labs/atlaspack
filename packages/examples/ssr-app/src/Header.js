function Header({title = 'SSR App'}) {
  return (
    <header className="header">
      <h1>{title}</h1>
    </header>
  );
}

module.exports = Header;
