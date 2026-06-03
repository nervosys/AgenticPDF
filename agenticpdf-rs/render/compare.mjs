// Render selected pages of demos/sample.pdf with BOTH the WebGL renderer and the
// Canvas2D reference, saving screenshots side by side for comparison.
//   node agenticpdf-rs/render/compare.mjs [pages...]   (default: 1 5 6)
import { firefox } from "playwright";
import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const OUT = path.join(ROOT, "agenticpdf-rs", "render", "cmp");
fs.mkdirSync(OUT, { recursive: true });
const MIME = { ".js": "text/javascript", ".mjs": "text/javascript", ".wasm": "application/wasm", ".html": "text/html", ".pdf": "application/pdf" };
const pages = process.argv.slice(2).map(Number).filter(Boolean);
const PAGES = pages.length ? pages : [1, 5, 6];

const server = http.createServer((req, res) => {
  const file = path.join(ROOT, decodeURIComponent(req.url.split("?")[0]));
  fs.readFile(file, (err, data) => {
    if (err) { res.writeHead(404); res.end(); return; }
    res.writeHead(200, { "Content-Type": MIME[path.extname(file)] || "application/octet-stream" });
    res.end(data);
  });
});
await new Promise((r) => server.listen(0, r));
const url = `http://localhost:${server.address().port}/agenticpdf-rs/render/compare.html`;

const browser = await firefox.launch({ headless: true, firefoxUserPrefs: { "webgl.force-enabled": true } });
const page = await browser.newPage({ viewport: { width: 700, height: 950 }, deviceScaleFactor: 1 });
page.on("pageerror", (e) => console.log("pageerror:", String(e)));
page.on("console", (m) => { if (m.type() === "error") console.log("console.error:", m.text()); });
await page.goto(url, { waitUntil: "load" });
await page.evaluate(() => window.boot());
await page.waitForFunction("window.__ready === true", { timeout: 30000 });
const count = await page.evaluate(() => window.pageCount);
console.log("pageCount =", count);

for (const p of PAGES) {
  try {
    const w = await page.evaluate((pp) => window.renderWebGL(pp, 1), p);
    await page.locator("#stage").screenshot({ path: path.join(OUT, `p${p}-webgl.png`) });
    let c = null;
    try { c = await page.evaluate((pp) => window.renderC2D(pp, 1), p); }
    catch (e) { console.log(`  page ${p} C2D error: ${e.message}`); }
    if (c) await page.locator("#c2d").screenshot({ path: path.join(OUT, `p${p}-c2d.png`) });
    console.log(`page ${p}: webgl ${w.ops} ops ${Math.round(w.w)}x${Math.round(w.h)}; c2d ${c ? "ok" : "FAILED"}`);
  } catch (e) {
    console.log(`page ${p} failed: ${e.message}`);
  }
}
await browser.close();
server.close();
console.log("screenshots in", OUT);
