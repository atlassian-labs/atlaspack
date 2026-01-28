const http = require('http');
const fs = require('fs');
const path = require('path');
const url = require('url');

const PORT = 3000;
const TESSERACT_URL = 'http://localhost:8080';

const server = http.createServer((req, res) => {
  const parsedUrl = url.parse(req.url, true);

  // Serve viewer.html
  if (parsedUrl.pathname === '/' || parsedUrl.pathname === '/viewer.html') {
    const filePath = path.join(__dirname, 'viewer.html');
    fs.readFile(filePath, 'utf8', (err, data) => {
      if (err) {
        res.writeHead(500);
        res.end('Error loading viewer.html');
        return;
      }
      res.writeHead(200, {'Content-Type': 'text/html'});
      res.end(data);
    });
    return;
  }

  // Serve viewer.css
  if (parsedUrl.pathname === '/viewer.css') {
    const filePath = path.join(__dirname, 'viewer.css');
    fs.readFile(filePath, 'utf8', (err, data) => {
      if (err) {
        res.writeHead(500);
        res.end('Error loading viewer.css');
        return;
      }
      res.writeHead(200, {'Content-Type': 'text/css'});
      res.end(data);
    });
    return;
  }

  // Proxy requests to Tesseract server
  if (parsedUrl.pathname === '/render') {
    // Handle CORS preflight
    if (req.method === 'OPTIONS') {
      res.writeHead(200, {
        'Access-Control-Allow-Origin': '*',
        'Access-Control-Allow-Methods': 'POST, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type',
      });
      res.end();
      return;
    }

    // Proxy POST request to Tesseract
    if (req.method === 'POST') {
      let body = '';
      req.on('data', (chunk) => {
        body += chunk.toString();
      });

      req.on('end', () => {
        const options = {
          hostname: 'localhost',
          port: 8080,
          path: '/render',
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'Content-Length': Buffer.byteLength(body),
          },
        };

        const proxyReq = http.request(options, (proxyRes) => {
          res.writeHead(proxyRes.statusCode, {
            'Access-Control-Allow-Origin': '*',
            'Content-Type': 'application/json',
          });
          proxyRes.pipe(res);
        });

        proxyReq.on('error', (err) => {
          res.writeHead(500, {
            'Access-Control-Allow-Origin': '*',
            'Content-Type': 'application/json',
          });
          res.end(JSON.stringify({error: err.message}));
        });

        proxyReq.write(body);
        proxyReq.end();
      });
      return;
    }
  }

  // 404 for other paths
  res.writeHead(404);
  res.end('Not found');
});

server.listen(PORT, () => {
  console.log(`Viewer server running at http://localhost:${PORT}`);
  console.log(`Make sure Tesseract server is running at ${TESSERACT_URL}`);
});
