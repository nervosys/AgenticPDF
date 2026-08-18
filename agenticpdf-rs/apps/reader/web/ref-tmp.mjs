import { firefox } from "playwright";
const browser = await firefox.launch({
  timeout: 60000,
  firefoxUserPrefs: { "pdfjs.disabled": false, "browser.download.open_pdf_attachments_inline": true },
});
const page = await browser.newPage({ viewport: { width: 900, height: 1200 }, deviceScaleFactor: 1 });
const url = `file:///${process.env.APDF_WEB_PDF.split("\\").join("/")}#page=${process.env.APDF_PAGE ?? 1}`;
await page.goto(url, { waitUntil: "load", timeout: 60000 });
await page.waitForTimeout(9000);
await page.screenshot({ path: process.env.APDF_SHOT });
await browser.close();
