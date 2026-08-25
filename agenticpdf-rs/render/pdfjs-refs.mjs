// Render pages of a corpus of PDFs with Firefox's built-in PDF.js and save each
// page's pixels as a PPM. This is the ground truth the crate's renderer is
// measured against by the `compare_corpus` test in apps/reader.
//
//   node pdfjs-refs.mjs <list-file-or-dir> <outdir> [pages-per-file] [width]
//
// `width` is an upper bound, not a target: the viewer's own canvas width wins
// when it is smaller. Do not lower it to save disk. Downscaling the reference
// inflates the scores, because our side is then rendered at the same reduced
// width and thin strokes land differently on both sides -- one page measured
// 0.111 against a 640-wide reference and 0.031 against the native one.
//
// One Firefox instance is reused across the whole corpus; a per-file launch
// costs more than the rendering does.
import { firefox } from "playwright";
import fs from "node:fs";
import path from "node:path";

const [input, outdir, perFileArg, widthArg] = process.argv.slice(2);
if (!input || !outdir) {
  console.error("usage: node corpus-tmp.mjs <list-file-or-dir> <outdir> [pages] [width]");
  process.exit(2);
}
const perFile = Number(perFileArg) || 3;
const target = Number(widthArg) || 640;
fs.mkdirSync(outdir, { recursive: true });

const files = fs.statSync(input).isDirectory()
  ? fs
      .readdirSync(input)
      .filter((f) => f.toLowerCase().endsWith(".pdf"))
      .map((f) => path.join(input, f))
  : fs
      .readFileSync(input, "utf8")
      .split("\n")
      .map((s) => s.trim())
      .filter(Boolean);
console.log(files.length, "files");

const browser = await firefox.launch({
  headless: true,
  firefoxUserPrefs: { "pdfjs.disabled": false },
});
const page = await browser.newPage({ viewport: { width: 1200, height: 1600 } });
page.on("pageerror", () => {});

let done = 0;
for (const file of files) {
  done++;
  const slug =
    path
      .basename(file)
      .replace(/\.[Pp][Dd][Ff]$/, "")
      .replace(/[^A-Za-z0-9]+/g, "_")
      .slice(0, 60) +
    "_" +
    done;
  const d = path.join(outdir, slug);
  const url = "file:///" + path.resolve(file).split(String.fromCharCode(92)).join("/");
  try {
    await page.goto(url, { waitUntil: "load", timeout: 60000 });
    await page.waitForSelector(".page canvas", { timeout: 60000 });
    const count = await page.evaluate(() => window.PDFViewerApplication.pagesCount);
    const last = Math.min(count, perFile);
    fs.mkdirSync(d, { recursive: true });
    fs.writeFileSync(path.join(d, "source.txt"), file);
    for (let n = 1; n <= last; n++) {
      await page.evaluate((p) => {
        window.PDFViewerApplication.page = p;
      }, n);
      await page.waitForFunction(
        (p) => {
          const el = document.querySelector(`.page[data-page-number="${p}"] canvas`);
          return el && el.width > 10;
        },
        n,
        { timeout: 60000 },
      );
      await page.waitForTimeout(700);
      // Composite over white and downscale in the page, then hand back base64:
      // a multi-million-element JSON array across the bridge costs more than
      // the rendering does.
      const out = await page.evaluate(
        ([p, target]) => {
          const el = document.querySelector(`.page[data-page-number="${p}"] canvas`);
          const w = Math.min(target, el.width);
          const h = Math.max(1, Math.round((el.height * w) / el.width));
          const c = document.createElement("canvas");
          c.width = w;
          c.height = h;
          const g = c.getContext("2d");
          g.fillStyle = "#fff";
          g.fillRect(0, 0, w, h);
          g.drawImage(el, 0, 0, w, h);
          const data = g.getImageData(0, 0, w, h).data;
          let s = "";
          for (let i = 0; i < data.length; i += 4) {
            s += String.fromCharCode(data[i], data[i + 1], data[i + 2]);
          }
          return { w, h, b64: btoa(s) };
        },
        [n, target],
      );
      fs.writeFileSync(
        path.join(d, `p${n}.ref.ppm`),
        Buffer.concat([
          Buffer.from(`P6\n${out.w} ${out.h}\n255\n`),
          Buffer.from(out.b64, "base64"),
        ]),
      );
    }
    console.log(`ok ${done}/${files.length} ${slug} ${last}/${count}`);
  } catch (e) {
    console.log(`skip ${done}/${files.length} ${slug} ${String(e).split("\n")[0].slice(0, 90)}`);
  }
}
await browser.close();
