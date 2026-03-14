const ts = require('typescript');
const fs = require('fs');
const path = require('path');

let source = fs.readFileSync(path.join(__dirname, '..', 'agenticpdf.ts'), 'utf-8');
source += '\n(globalThis).AgenticPDF = AgenticPDF;\n';

const result = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ESNext, module: ts.ModuleKind.CommonJS, esModuleInterop: true, strict: false }
});

const tempPath = path.join(__dirname, '_temp_annot_trace.cjs');
fs.writeFileSync(tempPath, result.outputText);
require(tempPath);
const AgenticPDF = globalThis.AgenticPDF;

(async () => {
  const buf = fs.readFileSync(path.join(__dirname, '..', 'demos', 'sample.pdf'));
  const pdf = await AgenticPDF.fromBuffer(buf.buffer);

  for (let p = 1; p <= 8; p++) {
    const annots = await pdf.getAnnotations(p);
    const links = annots.filter(a => a.type === 'Link' && a.destination);
    if (!links.length) continue;
    console.log(`\nPage ${p}:`);
    for (const link of links) {
      const d = link.destination;
      const dtype = typeof d;
      const isArr = Array.isArray(d);
      const display = isArr ? JSON.stringify(d) : d;
      const isUrl = dtype === 'string' && d.startsWith('http');
      console.log(`  dest type=${dtype}, isArray=${isArr}, isUrl=${isUrl}: ${display}`);
    }
  }

  pdf.close();
  fs.unlinkSync(tempPath);
})().catch(e => { console.error(e.stack); process.exit(1); });
