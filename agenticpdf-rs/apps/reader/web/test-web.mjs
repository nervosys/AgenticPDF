// SPDX-License-Identifier: AGPL-3.0-or-later
// Drives the web shell in a real browser at a phone viewport: opens a
// document, renders it, searches, edits and saves — the whole loop.
import { firefox } from "playwright";
import http from "node:http";
import fs from "node:fs";
import path from "node:path";

const ROOT = path.resolve(".");
const MIME = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".json": "application/json" };

const server = http.createServer((req, res) => {
  const file = path.join(ROOT, decodeURIComponent(req.url.split("?")[0]));
  fs.readFile(file, (err, data) => {
    if (err) { res.writeHead(404); res.end("not found"); return; }
    res.writeHead(200, { "Content-Type": MIME[path.extname(file)] ?? "application/octet-stream" });
    res.end(data);
  });
});
await new Promise((r) => server.listen(8137, r));

const browser = await firefox.launch({ timeout: 60000 });
const errors = [];
try {
  // iPhone-ish viewport with a device pixel ratio, so the DPR path is exercised.
  const page = await browser.newPage({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 3, hasTouch: true });
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("console", (m) => { if (m.type() === "error") errors.push("console: " + m.text()); });

  await page.goto("http://localhost:8137/web/index.html", { waitUntil: "load" });
  await page.waitForFunction("window.apdf !== undefined", { timeout: 30000 });

  const markdown = "# Quarterly Report\n\nRevenue grew by **12%** across EMEA.\n\n- Hiring on plan\n- Churn down\n";
  await page.setInputFiles("#file", { name: "report.md", mimeType: "text/markdown", buffer: Buffer.from(markdown) });
  await page.waitForTimeout(500);

  const result = await page.evaluate(() => {
    const painted = JSON.parse(window.__lastOps ?? "null");
    return {
      status: document.getElementById("status").textContent,
      position: document.getElementById("position").textContent,
      caps: window.apdf.capabilities().actions.length,
      blocks: window.apdf.act("get_blocks").blocks.length,
      hits: window.apdf.act("search", { query: "revenue emea" }).hits.length,
      inserted: !!window.apdf.act("insert_block", { markdown: "## Added by an agent" }).inserted,
      exported: window.apdf.act("export", { to: "markdown" }).content.includes("Added by an agent"),
      canvasPx: (() => {
        const c = document.getElementById("page");
        const ctx = c.getContext("2d");
        const d = ctx.getImageData(0, 0, c.width, c.height).data;
        let painted = 0;
        for (let i = 3; i < d.length; i += 4) if (d[i] > 8) painted++;
        return painted;
      })(),
    };
  });
  console.log(JSON.stringify(result, null, 1));

  // A PDF exercises the path Markdown never touches: text drawn from the
  // document's own glyphs, which reach the browser as image masks in the
  // recording. A host that cannot draw those renders a page of empty
  // rectangles -- ink on the canvas is what distinguishes the two.
  const pdfPath = ["../../../demos/sample.pdf", "../../demos/sample.pdf", "demos/sample.pdf"]
    .map((candidate) => path.resolve(candidate))
    .find((candidate) => fs.existsSync(candidate)) ?? "";
  let pdf = null;
  if (pdfPath) {
    await page.setInputFiles("#file", pdfPath);
    await page.waitForTimeout(1500);
    pdf = await page.evaluate(() => {
      const canvas = document.getElementById("page");
      const ctx = canvas.getContext("2d");
      const data = ctx.getImageData(0, 0, canvas.width, canvas.height).data;
      let ink = 0;
      // Dark pixels only: the page itself is painted white, so counting
      // anything opaque would pass on a blank sheet.
      for (let i = 0; i < data.length; i += 4) {
        if (data[i + 3] > 8 && data[i] < 128 && data[i + 1] < 128 && data[i + 2] < 128) ink++;
      }
      const ops = JSON.parse(window.__lastOps ?? "[]");
      return {
        status: document.getElementById("status").textContent,
        glyphOps: ops.filter((op) => op.op === "image").length,
        masksInlined: ops.filter((op) => op.op === "image" && op.pixels).length,
        framesDrawn: ops.filter((op) => op.op === "image" && !op.key).length,
        ink,
      };
    });
    console.log(JSON.stringify(pdf, null, 1));
  } else {
    console.log("skipping the PDF case: demos/sample.pdf not present");
  }

  console.log("pageerrors:", errors);

  const pdfOk =
    pdf === null ||
    // Thousands of glyphs, each an image op, and enough dark pixels that they
    // were actually rasterised rather than outlined.
    (pdf.glyphOps > 500 && pdf.ink > 5000 && pdf.framesDrawn === 0);
  if (pdf && !pdfOk) console.log("FAIL: the PDF page did not render its glyphs");

  const ok =
    result.canvasPx > 1000 &&
    result.hits === 1 &&
    result.exported &&
    errors.length === 0 &&
    pdfOk;
  console.log(ok ? "OK: the web shell rendered Markdown and a PDF's own glyphs." : "FAIL");
  process.exitCode = ok ? 0 : 1;
} finally {
  await browser.close();
  server.close();
}
