function echo(...messages) {
  let stdout = process.stdout;
  for (let message of messages) {
    stdout.write(String(message))
  }
}

echo(1, 2, 3);
