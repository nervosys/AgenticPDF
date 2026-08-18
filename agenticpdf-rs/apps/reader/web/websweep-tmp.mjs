// Render page one of each document in the web shell and save the canvas.
import { firefox } from "playwright";
import http from "node:http";
import fs from "node:fs";
import path from "node:path";

const ROOT = path.resolve(".");
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm" };
const server = http.createServer((req, res) => {
  const file = path.join(ROOT, decodeURIComponent(req.url.split("?")[0]));
  fs.readFile(file, (err, data) => {
    if (err) { res.writeHead(404); res.end("no"); return; }
    res.writeHead(200, { "Content-Type": MIME[path.extname(file)] ?? "application/octet-stream" });
    res.end(data);
  });
});
await new Promise((r) => server.listen(8141, r));

const files = JSON.parse(fs.readFileSync(process.env.APDF_LIST, "utf8"));
const out = process.env.APDF_OUT;
const browser = await firefox.launch({ timeout: 60000 });
for (const file of files) {
  const name = path.basename(file).replace(/[^\w.-]/g, "_");
  const page = await browser.newPage({ viewport: { width: 760, height: 1000 }, deviceScaleFactor: 1 });
  try {
    await page.goto("http://localhost:8141/web/index.html", { waitUntil: "load" });
    await page.waitForFunction("window.apdf !== undefined", { timeout: 30000 });
    await page.setInputFiles("#file", file);
    await page.waitForTimeout(2500);
    await page.locator("#page").screenshot({ path: path.join(out, `${name}.web.png`) });
    console.log("ok", name);
  } catch (why) {
    console.log("skip", name, String(why).split("\n")[0].slice(0, 60));
  }
  await page.close();
}
await browser.close();
server.close();
