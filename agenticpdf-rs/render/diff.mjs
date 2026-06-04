// Quantify how closely the WebGL renderer matches the Canvas2D reference for
// each page of demos/sample.pdf, saving a red diff heatmap per page.
//   node agenticpdf-rs/render/diff.mjs [pages...]   (default: 1 5 6)
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

let totalDiff = 0, totalInk = 0;
for (const p of PAGES) {
  const r = await page.evaluate((pp) => window.diffPage(pp, 1), p);
  await page.locator("#diff").screenshot({ path: path.join(OUT, `p${p}-diff.png`) });
  totalDiff += r.diffPx; totalInk += r.inkPx;
  const box = r.hardBox ? ` hardBox=[${r.hardBox.join(",")}]` : "";
  console.log(`page ${p}: ${r.diffPx} diff (${r.hardPx} hard) / ${r.inkPx} inked = ${(r.pctInk * 100).toFixed(2)}% differs, ${((r.hardPx / r.inkPx) * 100).toFixed(2)}% hard (${(r.pctAll * 100).toFixed(3)}% of page)${box}`);
  if (r.samples?.length && process.env.SAMPLES) console.log("  samples (x,y,d,A,B):", r.samples.map((s) => `(${s[0]},${s[1]},d${s[2]},${s[3]}vs${s[4]})`).join(" "));
}
console.log(`OVERALL: ${(totalInk ? (totalDiff / totalInk) * 100 : 0).toFixed(2)}% of inked pixels differ`);
await browser.close();
server.close();
console.log("diff heatmaps in", OUT);
