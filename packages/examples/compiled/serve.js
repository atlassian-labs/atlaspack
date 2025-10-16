const express = require('express');
const fs = require('node:fs');
const path = require('node:path');

const app = express();

app.get('/', (req, res, next) => {
  let index = fs.readFileSync('dist/index.html', 'utf-8');
  res.contentType = 'text/html';
  res.send(index);
});
app.use(express.static('dist'));
app.listen(3000, () => {
  console.log('Server is running on http://localhost:3000');
});
