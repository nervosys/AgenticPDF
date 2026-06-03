// Open a visible (headed) Firefox window rendering demos/sample.pdf with the
// AgenticPDF WebGL2 renderer, and keep it open. Run:
//   node agenticpdf-rs/render/show-firefox.mjs
import { firefox } from "playwright";
import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const MIME = {
  ".js": "text/javascript", ".mjs": "text/javascript", ".wasm": "application/wasm",
  ".html": "text/html", ".pdf": "application/pdf", ".json": "application/json", ".css": "text/css",
};

const server = http.createServer((req, res) => {
  const rel = decodeURIComponent(req.url.split("?")[0]);
  const file = path.join(ROOT, rel);
  fs.readFile(file, (err, data) => {
    if (err) { res.writeHead(404); res.end("not found"); return; }
    res.writeHead(200, { "Content-Type": MIME[path.extname(file)] || "application/octet-stream" });
    res.end(data);
  });
});
await new Promise((r) => server.listen(0, r));
const url = `http://localhost:${server.address().port}/agenticpdf-rs/render/viewer.html`;

const browser = await firefox.launch({
  headless: false,
  firefoxUserPrefs: { "webgl.force-enabled": true, "webgl.disabled": false },
});
const page = await browser.newPage({ viewport: { width: 1100, height: 1300 } });
page.on("pageerror", (e) => console.log("pageerror:", String(e)));
await page.goto(url, { waitUntil: "load" });
console.log("Firefox window open at", url, "— close the window to exit.");

// Exit when the user closes the browser window.
await new Promise((resolve) => browser.on("disconnected", resolve));
server.close();
