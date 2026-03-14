// Build script for modernpdf-core.ts
const fs = require('fs');
const path = require('path');

console.log('📦 Building ModernPDF Core...');

// Read the TypeScript source
const sourcePath = path.join(__dirname, '..', 'modernpdf-core.ts');
const source = fs.readFileSync(sourcePath, 'utf8');

// Simple transpilation: remove type annotations and export statements
let jsCode = source
    // Remove type annotations from function parameters
    .replace(/\(([^)]+)\):\s*[A-Za-z<>\[\]|]+(\s*{|\s*=>)/g, '($1)$2')
    // Remove return type annotations
    .replace(/\):\s*Promise<[^>]+>/g, ')')
    .replace(/\):\s*[A-Za-z<>\[\]|]+\s*{/g, ') {')
    // Remove type annotations from variable declarations
    .replace(/:\s*[A-Za-z<>\[\]|]+(\s*=)/g, '$1')
    .replace(/:\s*[A-Za-z<>\[\]|]+;/g, ';')
    // Remove interface and enum declarations
    .replace(/export\s+interface\s+\w+\s*{[^}]*}/g, '')
    .replace(/export\s+enum\s+\w+\s*{[^}]*}/g, '')
    // Remove type imports
    .replace(/import\s+type\s+{[^}]+}\s+from\s+['"][^'"]+['"]\s*;?/g, '')
    // Keep export statements
    .replace(/export\s+/g, '');

// Add browser global
jsCode = `
// ModernPDF Core - Compiled from TypeScript
${jsCode}

// Browser global export
if (typeof window !== 'undefined') {
    window.ModernPDF = {
        getDocument,
        renderPage,
        Stream,
        Lexer,
        Parser,
        XRef,
        Catalog,
        Page,
        CanvasGraphics,
        PDFDocument,
        Obj,
        ObjType
    };
}
`;

// Write output
const outputPath = path.join(__dirname, '..', 'demos', 'modernpdf-core.js');
fs.writeFileSync(outputPath, jsCode);

console.log('✅ Built: demos/modernpdf-core.js');
console.log(`📊 Size: ${Math.round(jsCode.length / 1024)}KB`);
