// Headless in-browser validation of the WebGL2 renderer.
// Serves the repo, renders demos/sample.pdf in a real browser (Firefox, then
// Chromium/SwiftShader — Brave's engine — as fallback), reads back GL pixels,
// and asserts content was drawn. Run: node agenticpdf-rs/render/test-render.mjs
import { firefox, chromium } from "playwright";
import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const MIME = {
  ".js": "text/javascript", ".mjs": "text/javascript", ".wasm": "application/wasm",
  ".html": "text/html", ".pdf": "application/pdf", ".json": "application/json",
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
const url = `http://localhost:${server.address().port}/agenticpdf-rs/render/harness.html`;

async function run(launch, name, opts) {
  const browser = await launch(opts);
  try {
    const page = await browser.newPage();
    const errors = [];
    page.on("pageerror", (e) => errors.push(String(e)));
    page.on("console", (m) => { if (m.type() === "error") errors.push("console: " + m.text()); });
    await page.goto(url, { waitUntil: "load" });
    await page.waitForFunction("window.__ready === true", { timeout: 20000 });
    const res = await page.evaluate(() => window.runDemo(1, 1.5));
    // Page 6 carries a raster figure — validates the image-texture path.
    const img = await page.evaluate(() => window.runDemo(6, 1.5));
    await page.screenshot({ path: path.join(ROOT, `agenticpdf-rs/render/out-${name}.png`) });
    return { name, res, img, errors };
  } finally {
    await browser.close();
  }
}

let out = null;
for (const [launch, name, opts] of [
  [firefox, "firefox", { firefoxUserPrefs: { "webgl.force-enabled": true, "webgl.disabled": false } }],
  [chromium, "chromium", { args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] }],
]) {
  try {
    out = await run(launch.launch.bind(launch), name, opts);
    if (out.res && out.res.nonwhite > 0) break;
    console.log(`[${name}] rendered nothing / errors:`, out.errors);
  } catch (e) {
    console.log(`[${name}] failed: ${e.message}`);
  }
}
server.close();

if (out && out.res && out.res.nonwhite > 0) {
  console.log(`OK [${out.name}] page1: ${out.res.ops} ops -> ${out.res.width}x${out.res.height}, ${out.res.nonwhite} non-white px`);
  const imgOk = out.img && out.img.nonwhite > 1000;
  console.log(`   page6 (image): ${out.img?.ops} ops, ${out.img?.nonwhite} non-white px -> image texture ${imgOk ? "DREW" : "not detected"}`);
  if (out.errors?.length) console.log("   pageerrors:", out.errors);
  process.exit(0);
} else {
  console.log("FAIL: renderer produced no content.", out?.errors ?? "");
  process.exit(1);
}
