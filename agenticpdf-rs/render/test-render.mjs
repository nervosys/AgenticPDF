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

// A reflowable document, served from memory. It has no geometry of its own, so
// rendering it exercises the typesetter end to end: parse -> semantic model ->
// typeset -> display list -> WebGL. Markdown keeps the fixture a plain string,
// so no binary is committed to make this run.
const MARKDOWN_FIXTURE = `# Quarterly Report

Revenue grew by **12%** across all *regions* this quarter, and the pipeline for
next quarter looks materially stronger than it did three months ago.

## Highlights

- Hiring is on plan
- Churn is down
  - Enterprise churn down 3pp

| Region | Growth |
| --- | --- |
| EMEA | 8% |
| APAC | 17% |

> Best quarter on record.
`;
const FIXTURE_PATH = "/__fixture__/report.md";

const server = http.createServer((req, res) => {
  const rel = decodeURIComponent(req.url.split("?")[0]);
  if (rel === FIXTURE_PATH) {
    res.writeHead(200, { "Content-Type": "text/markdown" });
    res.end(MARKDOWN_FIXTURE);
    return;
  }
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
    // A reflowable document, which only has pixels because the typesetter gave
    // it some.
    const typeset = await page.evaluate(
      (src) => window.runDemo(1, 1.5, src),
      FIXTURE_PATH,
    );
    await page.screenshot({ path: path.join(ROOT, `agenticpdf-rs/render/out-${name}.png`) });
    return { name, res, img, typeset, errors };
  } finally {
    await browser.close();
  }
}

let out = null;
for (const [launch, name, opts] of [
  // `timeout` matters: a chromium that cannot reach a GL backend hangs in
  // `launch()` rather than failing, which stalls the whole run instead of
  // falling through to the next browser.
  [firefox, "firefox", { timeout: 60000, firefoxUserPrefs: { "webgl.force-enabled": true, "webgl.disabled": false } }],
  [chromium, "chromium", { timeout: 60000, args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] }],
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

// Report the two surfaces separately. They fail independently — text goes to
// the 2D overlay and vectors/images to WebGL — so a combined total can stay
// healthy while one of them draws nothing at all.
const surfaces = (r) => `${r?.vector ?? 0} vector + ${r?.glyphs ?? 0} glyph px`;

if (out && out.res && out.res.nonwhite > 0) {
  console.log(`OK [${out.name}] pdf page1: ${out.res.ops} ops -> ${out.res.width}x${out.res.height}, ${surfaces(out.res)}`);
  const imgOk = out.img && out.img.vector > 1000;
  console.log(`   pdf page6 (image): ${out.img?.ops} ops, ${surfaces(out.img)} -> image texture ${imgOk ? "DREW" : "not detected"}`);

  // A typeset page is text, so glyphs are the thing that must be non-zero;
  // checking the total would let it pass on vector rulings alone.
  const typesetOk = out.typeset && out.typeset.glyphs > 0 && out.typeset.ops > 0;
  console.log(`   typeset markdown: ${out.typeset?.ops} ops -> ${out.typeset?.width}x${out.typeset?.height}, ${surfaces(out.typeset)} -> ${typesetOk ? "DREW" : "NOTHING"}`);
  if (out.errors?.length) console.log("   pageerrors:", out.errors);

  if (!typesetOk) {
    console.log("FAIL: a typeset document rendered nothing.");
    process.exit(1);
  }
  process.exit(0);
} else {
  console.log("FAIL: renderer produced no content.", out?.errors ?? "");
  process.exit(1);
}
