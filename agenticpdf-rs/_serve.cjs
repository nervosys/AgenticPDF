const http = require('http');
const fs = require('fs');
const path = require('path');
const root = 'c:/Users/adamm/dev/nervosys/utilities/ModernPDF';
const mimes = {
  '.html': 'text/html',
  '.js': 'application/javascript',
  '.wasm': 'application/wasm',
  '.pdf': 'application/pdf',
  '.json': 'application/json',
};
const srv = http.createServer((req, res) => {
  const url = decodeURIComponent(req.url.split('?')[0]);
  let fp;
  if (url.startsWith('/../demos/')) {
    fp = path.join(root, 'demos', path.basename(url));
  } else {
    fp = path.join(root, 'agenticpdf-rs', url);
  }
  fs.readFile(fp, (err, data) => {
    if (err) { res.writeHead(404); res.end('Not found: ' + fp); return; }
    const ext = path.extname(fp);
    res.writeHead(200, { 'Content-Type': mimes[ext] || 'application/octet-stream' });
    res.end(data);
  });
});
srv.listen(8080, '127.0.0.1', () => {
  console.log('Serving at http://127.0.0.1:8080/demo.html');
});
