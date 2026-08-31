const http = require('http');
const fs = require('fs');
const path = require('path');

const PORT = 8080;

const mimeTypes = {
    '.html': 'text/html',
    '.js': 'text/javascript',
    '.css': 'text/css',
    '.pdf': 'application/pdf',
    '.json': 'application/json',
    '.png': 'image/png',
    '.jpg': 'image/jpeg',
    '.gif': 'image/gif',
    '.svg': 'image/svg+xml',
};

const server = http.createServer((req, res) => {
    console.log(`${req.method} ${req.url}`);

    let filePath = '.' + req.url;
    if (filePath === './') {
        filePath = './demos/core-demo.html';
    }

    // Path traversal protection: resolve and confine to serving directory
    const resolvedPath = path.resolve(filePath);
    const servingRoot = path.resolve('.');
    if (!resolvedPath.startsWith(servingRoot + path.sep) && resolvedPath !== servingRoot) {
        res.writeHead(403, { 'Content-Type': 'text/html' });
        res.end('<h1>403 - Forbidden</h1>', 'utf-8');
        return;
    }

    const extname = String(path.extname(filePath)).toLowerCase();
    const contentType = mimeTypes[extname] || 'application/octet-stream';

    fs.readFile(resolvedPath, (error, content) => {
        if (error) {
            if (error.code === 'ENOENT') {
                res.writeHead(404, { 'Content-Type': 'text/html' });
                res.end('<h1>404 - File Not Found</h1>', 'utf-8');
            } else {
                res.writeHead(500);
                res.end('Internal Server Error', 'utf-8');
            }
        } else {
            res.writeHead(200, {
                'Content-Type': contentType,
                'X-Content-Type-Options': 'nosniff',
                'X-Frame-Options': 'SAMEORIGIN',
                'Referrer-Policy': 'strict-origin-when-cross-origin',
                // The demos are self-contained: they load no third-party
                // script, style or font, and they talk to nothing. Saying so
                // in a policy costs nothing here and means a stray injected
                // <script src> fails loudly during development rather than
                // quietly in production. 'unsafe-inline' for style is what
                // the demo pages' inline styles need; scripts do not get it.
                'Content-Security-Policy': [
                    "default-src 'self'",
                    "script-src 'self'",
                    "style-src 'self' 'unsafe-inline'",
                    "img-src 'self' data: blob:",
                    "font-src 'self' data:",
                    "connect-src 'self'",
                    "object-src 'none'",
                    "base-uri 'none'",
                    "frame-ancestors 'self'",
                    "form-action 'self'"
                ].join('; ')
            });
            res.end(content, 'utf-8');
        }
    });
});

server.listen(PORT, () => {
    console.log(`\n🚀 Server running at http://localhost:${PORT}/`);
    console.log(`📄 Core Demo: http://localhost:${PORT}/demos/core-demo.html\n`);
});
