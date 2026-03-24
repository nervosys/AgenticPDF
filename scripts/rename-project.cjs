/**
 * Rename project from ModernPDF to AgenticPDF (aPDF)
 * Handles all source files, configs, docs, and demos.
 * NOTE: This script was used for the initial rename and is kept for reference.
 */
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');

// Files to process (relative to ROOT) — skip dist/, coverage/, node_modules/, .git/
const sourceFiles = [
  'modernpdf.ts',
  'package.json',
  'tsconfig.json',
  'jest.config.cjs',
  'README.md',
  'CONTRIBUTING.md',
  'ARCHITECTURE.md',
  'cli.ts',
  'cli.js',
  '.github/copilot-instructions.md',
  '.github/SECURITY_SCANNING.md',
  '.github/security-config.yml',
  '.github/codeql/codeql-config.yml',
  '.github/ISSUE_TEMPLATE/bug_report.yml',
  '.github/ISSUE_TEMPLATE/feature_request.yml',
  '.github/ISSUE_TEMPLATE/question.yml',
  '.github/workflows/ci.yml',
  '.github/workflows/release.yml',
  'demos/pdf-viewer.html',
  'demos/render-engine-demo.html',
  'demos/README.md',
  'docs/README.md',
  'docs/cli/CLI_QUICKSTART.md',
  'docs/cli/CLI_REFERENCE.md',
  'docs/cli/CLI.md',
  'docs/development/EXAMPLES_SOLUTION.md',
  'docs/development/INSTALL.md',
  'docs/implementation/CLIPPING_PATHS_IMPLEMENTATION.md',
  'docs/implementation/COLOR_SPACE_IMPLEMENTATION.md',
  'docs/implementation/GLYPH_WIDTH_IMPLEMENTATION.md',
  'docs/implementation/GRAPHICS_STATE_STACK_IMPLEMENTATION.md',
  'docs/implementation/IMAGE_RENDERING_IMPLEMENTATION.md',
  'docs/implementation/OPTIMAL_CONFIG_IMPLEMENTATION.md',
  'docs/implementation/PDFJS_INTEGRATION.md',
  'docs/implementation/PDFJS_WRAPPER_README.md',
  'docs/implementation/PERFORMANCE_OPTIMIZATION_IMPLEMENTATION.md',
  'docs/implementation/TEXTEXTRACTOR_EXPORT_FIX.md',
  'docs/implementation/THEME_TOGGLE_IMPLEMENTATION.md',
  'docs/security/CODE_REVIEW.md',
  'docs/security/SECURITY_PATCHES.md',
  'examples/01-basic-processing.ts',
  'examples/02-ai-integration.ts',
  'examples/03-streaming-to-llm.ts',
  'examples/04-batch-processing.ts',
  'examples/index.ts',
  'examples/README.md',
  'tests/setup.ts',
  'tests/fixtures/index.ts',
  'tests/mocks/index.ts',
  'tests/integration/modernpdf.test.ts',
  'tests/unit/ai-features.test.ts',
  'tests/unit/constructor-coverage.test.ts',
  'tests/unit/error-handling.test.ts',
  'tests/unit/extraction.test.ts',
  'tests/unit/pdf-parser.test.ts',
  'tests/unit/streaming.test.ts',
  'tests/unit/utility-coverage.test.ts',
  'scripts/build-browser.cjs',
];

let totalReplacements = 0;
let filesModified = 0;

for (const relPath of sourceFiles) {
  const absPath = path.join(ROOT, relPath);
  if (!fs.existsSync(absPath)) {
    console.log(`  SKIP (not found): ${relPath}`);
    continue;
  }

  let content = fs.readFileSync(absPath, 'utf8');
  const original = content;

  // Class name / API: ModernPDF → AgenticPDF
  content = content.replace(/ModernPDF/g, 'AgenticPDF');
  
  // Package name / filenames: modernpdf → agenticpdf
  content = content.replace(/modernpdf/g, 'agenticpdf');

  // Fix package.json bin entry — keep cli.js as-is
  // The bin entry "modernpdf" CLI command → "apdf"
  // This will have been changed to "agenticpdf" above, which is fine for npm name,
  // but the CLI command should be "apdf"
  if (relPath === 'package.json') {
    content = content.replace(/"agenticpdf": ".\/cli.js"/, '"apdf": "./cli.js"');
  }

  if (content !== original) {
    fs.writeFileSync(absPath, content, 'utf8');
    const count = (original.length - content.replace(/AgenticPDF|agenticpdf/g, '').length) -
                  (original.length - original.replace(/ModernPDF|modernpdf/g, '').length);
    filesModified++;
    console.log(`  ✅ ${relPath}`);
  }
}

console.log(`\n✅ Renamed in ${filesModified} files`);

// Now rename physical files
const fileRenames = [
  ['modernpdf.ts', 'agenticpdf.ts'],
  ['modernpdf.d.ts', 'agenticpdf.d.ts'],
  ['modernpdf-browser.js', 'agenticpdf-browser.js'],
  ['demos/modernpdf-browser.js', 'demos/agenticpdf-browser.js'],
];

for (const [oldName, newName] of fileRenames) {
  const oldPath = path.join(ROOT, oldName);
  const newPath = path.join(ROOT, newName);
  if (fs.existsSync(oldPath)) {
    fs.renameSync(oldPath, newPath);
    console.log(`  📁 ${oldName} → ${newName}`);
  }
}

console.log('\n🎉 Project renamed to AgenticPDF (aPDF)');
