// Simplified build script - just use TypeScript compiler
const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

console.log('📦 Building ModernPDF Core with TypeScript compiler...');

try {
    // Compile TypeScript to JavaScript
    execSync('npx tsc modernpdf-core.ts --target ES2020 --module ESNext --outDir demos --declaration false', {
        stdio: 'inherit'
    });

    // Read the generated file
    const jsPath = path.join(__dirname, '..', 'demos', 'modernpdf-core.js');
    let jsCode = fs.readFileSync(jsPath, 'utf8');

    // Remove export statements (browser doesn't support them in regular script tags)
    jsCode = jsCode.replace(/export /g, '');

    // Add browser global export at the end
    jsCode += `
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

    // Write back
    fs.writeFileSync(jsPath, jsCode);

    console.log('✅ Built: demos/modernpdf-core.js');
    console.log(`📊 Size: ${Math.round(jsCode.length / 1024)}KB`);
} catch (error) {
    console.error('❌ Build failed:', error.message);
    process.exit(1);
}
