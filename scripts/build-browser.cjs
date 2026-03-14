// Build script for agenticpdf-browser.js
// Compiles agenticpdf.ts to browser-compatible JavaScript

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

console.log('🔨 Building AgenticPDF browser bundle...');

// Create temp directory for build
const tempDir = path.join(__dirname, '.temp-build');
if (!fs.existsSync(tempDir)) {
    fs.mkdirSync(tempDir);
}

// Create tsconfig for browser build
const browserTsConfig = {
    compilerOptions: {
        target: "ES2020",
        lib: ["ES2020", "DOM"],
        module: "ES2020",
        moduleResolution: "node",
        strict: true,
        esModuleInterop: true,
        skipLibCheck: true,
        declaration: false,
        outDir: tempDir,
        rootDir: __dirname + "/..",
        removeComments: false,
        sourceMap: false
    },
    include: [path.join(__dirname, "../agenticpdf.ts")],
    exclude: ["node_modules", "dist", "coverage", "tests", "examples"]
};

const configPath = path.join(tempDir, 'tsconfig.browser.json');
fs.writeFileSync(configPath, JSON.stringify(browserTsConfig, null, 2));

console.log('📝 Compiling TypeScript...');

try {
    // Compile TypeScript
    execSync(`npx tsc --project "${configPath}"`, { stdio: 'inherit' });

    console.log('✅ TypeScript compiled successfully');

    // Read compiled JavaScript
    const compiledPath = path.join(tempDir, 'agenticpdf.js');
    let jsContent = fs.readFileSync(compiledPath, 'utf-8');

    console.log('🔧 Processing for browser compatibility...');

    // Remove Node.js specific imports and exports
    jsContent = jsContent.replace(/^import\s+.*from\s+['"]node:.*['"];?$/gm, '');
    jsContent = jsContent.replace(/^import\s+.*from\s+['"]fs['"];?$/gm, '');
    jsContent = jsContent.replace(/^import\s+.*from\s+['"]path['"];?$/gm, '');
    jsContent = jsContent.replace(/^import\s+.*from\s+['"]stream['"];?$/gm, '');
    jsContent = jsContent.replace(/^import\s+.*from\s+['"]worker_threads['"];?$/gm, '');

    // Remove all export statements for browser compatibility
    jsContent = jsContent.replace(/^export\s+/gm, '');
    jsContent = jsContent.replace(/^export\s*{\s*[^}]*\s*};?$/gm, '');
    jsContent = jsContent.replace(/^export\s+default\s+/gm, '');

    // Remove standalone export blocks (malformed exports)
    jsContent = jsContent.replace(/^{\s*\w+\s*};?\s*$/gm, '');
    jsContent = jsContent.replace(/^default\s+\w+;?\s*$/gm, '');

    // Replace Node.js Buffer with Uint8Array for browser
    jsContent = jsContent.replace(/\bBuffer\b/g, 'Uint8Array');

    // Wrap in IIFE and expose to window
    const browserBundle = `/**
 * AgenticPDF - Browser Bundle
 * Modern, TypeScript-native PDF processing library
 * Version: 1.0.1
 * Compiled: ${new Date().toISOString()}
 */

(function(global) {
    'use strict';
    
${jsContent}

    // Export to global scope
    if (typeof window !== 'undefined') {
        window.AgenticPDF = AgenticPDF;
        window.TextExtractor = TextExtractor;
        window.ImageExtractor = ImageExtractor;
        window.FormExtractor = FormExtractor;
        window.AnnotationExtractor = AnnotationExtractor;
        window.renderTextLayer = renderTextLayer;
        window.TextLayerBuilder = TextLayerBuilder;
        window.ThemeManager = ThemeManager;
    }
    if (typeof global !== 'undefined') {
        global.AgenticPDF = AgenticPDF;
        global.TextExtractor = TextExtractor;
        global.ImageExtractor = ImageExtractor;
        global.FormExtractor = FormExtractor;
        global.AnnotationExtractor = AnnotationExtractor;
        global.renderTextLayer = renderTextLayer;
        global.TextLayerBuilder = TextLayerBuilder;
        global.ThemeManager = ThemeManager;
    }
    
})(typeof window !== 'undefined' ? window : typeof global !== 'undefined' ? global : this);
`;

    // Write browser bundle
    const outputPath = path.join(__dirname, 'agenticpdf-browser.js');
    fs.writeFileSync(outputPath, browserBundle);

    console.log('✅ Browser bundle created: agenticpdf-browser.js');

    // Copy to project root
    const rootPath = path.join(__dirname, '..', 'agenticpdf-browser.js');
    fs.copyFileSync(outputPath, rootPath);
    console.log('✅ Copied to project root');

    // Copy to demos folder for testing
    const demosPath = path.join(__dirname, '..', 'demos', 'agenticpdf-browser.js');
    fs.copyFileSync(outputPath, demosPath);
    console.log('✅ Copied to demos folder');

    // Clean up temp directory
    fs.rmSync(tempDir, { recursive: true, force: true });

    console.log('🎉 Build complete!');

} catch (error) {
    console.error('❌ Build failed:', error.message);
    // Clean up on error
    if (fs.existsSync(tempDir)) {
        fs.rmSync(tempDir, { recursive: true, force: true });
    }
    process.exit(1);
}
