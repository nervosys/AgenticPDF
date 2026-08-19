// PDF.js reference shots, page-fit so a landscape page is not cropped.
import { firefox } from "playwright";
import fs from "node:fs";
import path from "node:path";

const jobs = JSON.parse(fs.readFileSync(process.env.APDF_LIST, "utf8"));
const out = process.env.APDF_OUT;
const browser = await firefox.launch({
  timeout: 60000,
  firefoxUserPrefs: {
    "pdfjs.disabled": false,
    "browser.download.open_pdf_attachments_inline": true,
  },
});
for (const [file, pages] of jobs) {
  const name = path.basename(file).replace(/[^\w.-]/g, "_");
  const page = await browser.newPage({ viewport: { width: 1800, height: 1600 } });
  try {
    await page.goto(`file:///${file.split("\\").join("/")}#page=1&zoom=page-fit`, {
      waitUntil: "load",
      timeout: 45000,
    });
    await page.waitForSelector(".page canvas", { timeout: 30000 });
    for (const n of pages) {
      const target = path.join(out, `${name}.p${n}.ref.png`);
      if (fs.existsSync(target) && !process.env.APDF_FORCE) continue;
      await page.fill("#pageNumber", String(n));
      await page.press("#pageNumber", "Enter");
      await page.waitForTimeout(3500);
      const canvas = page.locator(`.page[data-page-number="${n}"] canvas`).first();
      if ((await canvas.count()) === 0) continue;
      await canvas.screenshot({ path: target });
    }
    console.log("ok", name);
  } catch (why) {
    console.log("skip", name, String(why).split("\n")[0].slice(0, 70));
  }
  await page.close();
}
await browser.close();
