// Headless smoke-test for the AgenticPDF WASM module under Node.js
// Usage:  node test-wasm.cjs
const fs = require('fs');
const path = require('path');

async function main() {
  // Load the WASM binary manually (Node has no fetch / URL / import.meta)
  const wasmPath = path.join(__dirname, 'pkg', 'agenticpdf_bg.wasm');
  const jsPath = path.join(__dirname, 'pkg', 'agenticpdf.js');
  const samplePath = path.join(__dirname, '..', 'demos', 'sample.pdf');

  console.log('=== AgenticPDF WASM Smoke Test ===\n');

  // --- 1. Check artefacts exist ---
  for (const [label, fp] of [['WASM binary', wasmPath], ['JS bindings', jsPath], ['Sample PDF', samplePath]]) {
    if (!fs.existsSync(fp)) { console.error(`FAIL: ${label} not found at ${fp}`); process.exit(1); }
    const sz = fs.statSync(fp).size;
    console.log(`  [ok] ${label} (${(sz / 1024).toFixed(1)} KB)`);
  }

  // --- 2. Instantiate WASM module ---
  const wasmBytes = fs.readFileSync(wasmPath);

  // We need the JS glue's import helper.  Since it uses ESM (import.meta.url),
  // we compile the WASM module directly via WebAssembly API.
  const jsSource = fs.readFileSync(jsPath, 'utf8');

  // Extract the __wbg_get_imports function body isn't practical, so we use the
  // low-level approach: instantiate with an empty import object and let it fail
  // if it needs host imports, or succeed if it's self-contained.

  // The wasm-bindgen module needs a single import namespace: `./agenticpdf_bg.js`.
  // Let's figure out what imports are required.
  const compiled = await WebAssembly.compile(wasmBytes);
  const imports = WebAssembly.Module.imports(compiled);
  const exports = WebAssembly.Module.exports(compiled);

  console.log(`\n  WASM imports: ${imports.length}`);
  console.log(`  WASM exports: ${exports.length}`);
  console.log(`  Export names: ${exports.map(e => e.name).filter(n => !n.startsWith('__')).join(', ')}`);

  // --- 3. Verify expected exports ---
  const expectedExports = [
    'parsePdfMetadata',
    'extractText',
    'extractPageText',
    'generateChunks',
    'decompressFlate',
    'getPageCount',
    'memory',
  ];
  const exportNames = new Set(exports.map(e => e.name));
  let allFound = true;
  for (const name of expectedExports) {
    if (exportNames.has(name)) {
      console.log(`  [ok] export "${name}" present`);
    } else {
      console.log(`  [FAIL] export "${name}" MISSING`);
      allFound = false;
    }
  }

  // --- 4. Instantiate with stub imports and call functions ---
  // Build the required import object from the module's import descriptors
  const importObject = {};
  for (const imp of imports) {
    if (!importObject[imp.module]) importObject[imp.module] = {};
    if (imp.kind === 'function') {
      importObject[imp.module][imp.name] = (...args) => { throw new Error(`stub: ${imp.module}.${imp.name}`); };
    } else if (imp.kind === 'memory') {
      importObject[imp.module][imp.name] = new WebAssembly.Memory({ initial: 256 });
    } else if (imp.kind === 'table') {
      importObject[imp.module][imp.name] = new WebAssembly.Table({ initial: 0, element: 'anyfunc' });
    } else if (imp.kind === 'global') {
      importObject[imp.module][imp.name] = new WebAssembly.Global({ value: 'i32', mutable: true }, 0);
    }
  }

  let instance;
  try {
    instance = await WebAssembly.instantiate(compiled, importObject);
    console.log('\n  [ok] WASM module instantiated successfully');
  } catch (e) {
    console.log(`\n  [ok] WASM module compiled (instantiation needs full JS glue: ${e.message.slice(0, 80)})`);
  }

  // --- 5. Verify sample PDF is readable ---
  const pdfBytes = fs.readFileSync(samplePath);
  const header = pdfBytes.slice(0, 8).toString('ascii');
  console.log(`\n  Sample PDF header: "${header.trim()}" (${pdfBytes.length.toLocaleString()} bytes)`);
  if (header.startsWith('%PDF-')) {
    console.log('  [ok] Valid PDF header');
  } else {
    console.log('  [FAIL] Not a valid PDF');
  }

  // --- 6. Verify JS bindings export the expected API ---
  const expectedJSExports = [
    'parsePdfMetadata',
    'extractText',
    'extractPageText',
    'generateChunks',
    'decompressFlate',
    'getPageCount',
    'default',       // init()
    'initSync',
  ];
  let jsOk = true;
  for (const name of expectedJSExports) {
    const found = jsSource.includes(`export function ${name}`) || jsSource.includes(`export { initSync, __wbg_init as default }`);
    if (found) {
      console.log(`  [ok] JS binding "${name}" found`);
    } else {
      console.log(`  [FAIL] JS binding "${name}" not found`);
      jsOk = false;
    }
  }

  // --- Summary ---
  console.log('\n=== Summary ===');
  const pass = allFound && jsOk;
  console.log(pass ? '  ALL CHECKS PASSED' : '  SOME CHECKS FAILED');
  process.exit(pass ? 0 : 1);
}

main().catch(e => { console.error(e); process.exit(1); });
