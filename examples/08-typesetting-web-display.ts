/**
 * aPDF Typesetting & Web Display Examples
 *
 * Demonstrates how to use aPDF display hints, font metadata, and structural
 * analysis to drive web rendering, CSS generation, responsive layouts,
 * accessible HTML, and print-ready typesetting.
 */

import AgenticPDF from '../agenticpdf';
import type { APDFDocument } from '../agenticpdf';

// ============================================================================
// Example 1: Generate CSS Stylesheet from aPDF Display Hints
//
// Reads the aPDF display envelope (fonts, theme, reading order, math flag)
// and emits a scoped CSS stylesheet to faithfully render the document on web.
// ============================================================================

function generateCSS(apdf: APDFDocument, scopeSelector = '.apdf-document'): string {
  const d = apdf.display;
  const { width: pageW, height: pageH } = d.pageDimensions;
  const aspectRatio = (pageW / pageH).toFixed(4);

  const lines: string[] = [];

  // Root scope
  lines.push(`${scopeSelector} {`);
  lines.push(`  --apdf-page-width: ${pageW}pt;`);
  lines.push(`  --apdf-page-height: ${pageH}pt;`);
  lines.push(`  --apdf-aspect-ratio: ${aspectRatio};`);
  lines.push(`  max-width: 900px;`);
  lines.push(`  margin: 0 auto;`);
  lines.push(`  line-height: 1.6;`);

  // Column layout
  if (d.readingOrder === 'multi-column') {
    lines.push(`  column-count: 2;`);
    lines.push(`  column-gap: 2rem;`);
    lines.push(`  column-rule: 1px solid var(--apdf-rule-color, #ddd);`);
  }

  lines.push(`}`);
  lines.push('');

  // Theme palette
  const themeVars: Record<string, Record<string, string>> = {
    academic: {
      '--apdf-bg': '#fffff8',
      '--apdf-fg': '#1a1a1a',
      '--apdf-heading': '#222',
      '--apdf-link': '#1a5276',
      '--apdf-rule-color': '#ccc',
      '--apdf-code-bg': '#f5f2e8',
    },
    technical: {
      '--apdf-bg': '#fafafa',
      '--apdf-fg': '#333',
      '--apdf-heading': '#0d47a1',
      '--apdf-link': '#1565c0',
      '--apdf-rule-color': '#e0e0e0',
      '--apdf-code-bg': '#f0f4f8',
    },
    general: {
      '--apdf-bg': '#ffffff',
      '--apdf-fg': '#222',
      '--apdf-heading': '#111',
      '--apdf-link': '#0645ad',
      '--apdf-rule-color': '#ddd',
      '--apdf-code-bg': '#f6f6f6',
    },
  };

  const theme = d.suggestedTheme || 'general';
  const vars = themeVars[theme];
  lines.push(`${scopeSelector} {`);
  for (const [prop, val] of Object.entries(vars)) {
    lines.push(`  ${prop}: ${val};`);
  }
  lines.push(`  background: var(--apdf-bg);`);
  lines.push(`  color: var(--apdf-fg);`);
  lines.push(`}`);
  lines.push('');

  // Dark mode override
  lines.push(`@media (prefers-color-scheme: dark) {`);
  lines.push(`  ${scopeSelector} {`);
  lines.push(`    --apdf-bg: #1e1e1e;`);
  lines.push(`    --apdf-fg: #d4d4d4;`);
  lines.push(`    --apdf-heading: #e0e0e0;`);
  lines.push(`    --apdf-link: #6cb4ee;`);
  lines.push(`    --apdf-rule-color: #444;`);
  lines.push(`    --apdf-code-bg: #2d2d2d;`);
  lines.push(`  }`);
  lines.push(`}`);
  lines.push('');

  // Font stacks
  for (const font of d.fonts) {
    const selector =
      font.role === 'body' ? `${scopeSelector} p, ${scopeSelector} li` :
      font.role === 'heading' ? `${scopeSelector} h1, ${scopeSelector} h2, ${scopeSelector} h3, ${scopeSelector} h4` :
      font.role === 'mono' ? `${scopeSelector} code, ${scopeSelector} pre` :
      null;

    if (selector) {
      const fallback =
        font.role === 'mono' ? ', "Fira Code", "Consolas", monospace' :
        font.role === 'heading' ? ', "Georgia", serif' :
        ', "Charter", "Source Serif Pro", serif';

      lines.push(`${selector} {`);
      lines.push(`  font-family: "${font.name}"${fallback};`);
      lines.push(`}`);
      lines.push('');
    }
  }

  // Headings
  lines.push(`${scopeSelector} h1, ${scopeSelector} h2, ${scopeSelector} h3, ${scopeSelector} h4 {`);
  lines.push(`  color: var(--apdf-heading);`);
  lines.push(`  margin-top: 1.5em;`);
  lines.push(`  margin-bottom: 0.5em;`);
  lines.push(`}`);
  lines.push('');

  // Math
  if (d.hasMath) {
    lines.push(`/* Math typesetting enabled — include KaTeX or MathJax stylesheet */`);
    lines.push(`${scopeSelector} .math-inline { display: inline; }`);
    lines.push(`${scopeSelector} .math-block {`);
    lines.push(`  display: block;`);
    lines.push(`  text-align: center;`);
    lines.push(`  margin: 1em 0;`);
    lines.push(`  overflow-x: auto;`);
    lines.push(`}`);
    lines.push('');
  }

  // Links
  lines.push(`${scopeSelector} a {`);
  lines.push(`  color: var(--apdf-link);`);
  lines.push(`  text-decoration: none;`);
  lines.push(`}`);
  lines.push(`${scopeSelector} a:hover { text-decoration: underline; }`);
  lines.push('');

  // Code blocks
  lines.push(`${scopeSelector} code {`);
  lines.push(`  background: var(--apdf-code-bg);`);
  lines.push(`  padding: 0.15em 0.3em;`);
  lines.push(`  border-radius: 3px;`);
  lines.push(`  font-size: 0.9em;`);
  lines.push(`}`);
  lines.push('');

  // Print stylesheet
  lines.push(`@media print {`);
  lines.push(`  ${scopeSelector} {`);
  lines.push(`    max-width: none;`);
  lines.push(`    font-size: 10pt;`);
  if (d.readingOrder === 'multi-column') {
    lines.push(`    column-count: 2;`);
    lines.push(`    column-gap: 1.5cm;`);
  }
  lines.push(`  }`);
  lines.push(`  @page {`);
  lines.push(`    size: ${pageW}pt ${pageH}pt;`);
  lines.push(`    margin: 2cm;`);
  lines.push(`  }`);
  lines.push(`}`);

  return lines.join('\n');
}

async function cssFromDisplayHints(file: File): Promise<string> {
  console.log('=== Example 1: CSS from aPDF Display Hints ===\n');

  const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

  try {
    const apdf = await pdf.generateAPDFMetadata();

    console.log('Display hints:');
    console.log(`  Reading order: ${apdf.display.readingOrder}`);
    console.log(`  Page size:     ${apdf.display.pageDimensions.width}×${apdf.display.pageDimensions.height} pt`);
    console.log(`  Has math:      ${apdf.display.hasMath}`);
    console.log(`  Has color:     ${apdf.display.hasColor}`);
    console.log(`  Theme:         ${apdf.display.suggestedTheme}`);
    console.log(`  Fonts:         ${apdf.display.fonts.map(f => `${f.name} (${f.role})`).join(', ')}\n`);

    const css = generateCSS(apdf);

    console.log(`Generated ${css.length} chars of CSS`);
    console.log(`\nPreview:\n${css.substring(0, 500)}\n...`);

    return css;
  } finally {
    pdf.close();
  }
}

// ============================================================================
// Example 2: Responsive HTML Article from aPDF
//
// Builds a self-contained HTML document that adapts to screen size, using
// aPDF structure (TOC, sections, figures, tables) and display hints.
// ============================================================================

async function responsiveHTMLArticle(file: File): Promise<string> {
  console.log('\n=== Example 2: Responsive HTML Article ===\n');

  const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

  try {
    const apdf = await pdf.generateAPDFMetadata();
    const css = generateCSS(apdf, '.article');

    const html: string[] = [];

    html.push('<!DOCTYPE html>');
    html.push('<html lang="' + escapeAttr(apdf.metadata.language) + '">');
    html.push('<head>');
    html.push('  <meta charset="utf-8">');
    html.push('  <meta name="viewport" content="width=device-width, initial-scale=1">');
    html.push('  <title>' + escapeHTML(apdf.metadata.title) + '</title>');

    // Schema.org metadata
    if (apdf.metadata.identifiers.doi) {
      html.push('  <meta name="citation_doi" content="' + escapeAttr(apdf.metadata.identifiers.doi) + '">');
    }
    for (const author of apdf.authors) {
      html.push('  <meta name="citation_author" content="' + escapeAttr(author.name) + '">');
    }
    if (apdf.metadata.datePublished) {
      html.push('  <meta name="citation_publication_date" content="' + escapeAttr(apdf.metadata.datePublished.substring(0, 10)) + '">');
    }

    // KaTeX if needed
    if (apdf.display.hasMath) {
      html.push('  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css" integrity="sha384-n8MVd4RsNIU0tQ2/19gy7bdV4CSjkCEQAo3GrJpc7b/dQSzuqoDP9pwp1SUbJkm8" crossorigin="anonymous">');
      html.push('  <script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.js" integrity="sha384-XjKyOOlGwcjNTAIQHIpgOno0Ola1kmFsp8Ro+2LQWtuGCicnlao/VUfR8rLJ33Eo" crossorigin="anonymous"></script>');
      html.push('  <script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/contrib/auto-render.min.js" integrity="sha384-+VBxd8OG0CnQQRXaISmihC2MG4qXMPSC4e+un7F1GA1iqtVrAmGnDnI9ow3NfJut" crossorigin="anonymous"></script>');
    }

    html.push('  <style>');
    html.push(css);
    html.push('  /* Responsive overrides */');
    html.push('  @media (max-width: 600px) {');
    html.push('    .article { column-count: 1 !important; padding: 1rem; font-size: 0.95rem; }');
    html.push('    .article h1 { font-size: 1.4rem; }');
    html.push('  }');
    html.push('  .toc { background: var(--apdf-code-bg); padding: 1rem 1.5rem; border-radius: 6px; margin: 1.5rem 0; }');
    html.push('  .toc ul { list-style: none; padding-left: 1.2em; }');
    html.push('  .toc > ul { padding-left: 0; }');
    html.push('  .figure-ref, .table-ref { background: var(--apdf-code-bg); padding: 0.75rem; border-radius: 4px; margin: 1rem 0; }');
    html.push('  .bib-entry { margin-bottom: 0.5em; }');
    html.push('  .bib-entry .bib-num { font-weight: bold; margin-right: 0.5em; }');
    html.push('  </style>');
    html.push('</head>');
    html.push('<body>');
    html.push('<article class="article" role="article">');

    // Title block
    html.push('  <header>');
    html.push('    <h1>' + escapeHTML(apdf.metadata.title) + '</h1>');
    if (apdf.authors.length > 0) {
      html.push('    <p class="authors">');
      const authorParts = apdf.authors.map(a => {
        let s = '<span class="author">' + escapeHTML(a.name);
        if (a.orcid) {
          s += ' <a href="https://orcid.org/' + escapeAttr(a.orcid) + '" title="ORCID" aria-label="ORCID profile">\u{1F517}</a>';
        }
        s += '</span>';
        return s;
      });
      html.push('      ' + authorParts.join(', '));
      html.push('    </p>');
    }
    if (apdf.metadata.datePublished) {
      html.push('    <time datetime="' + escapeAttr(apdf.metadata.datePublished) + '">' + escapeHTML(apdf.metadata.datePublished.substring(0, 10)) + '</time>');
    }
    html.push('  </header>');

    // Abstract
    if (apdf.metadata.abstract) {
      html.push('  <section class="abstract" aria-label="Abstract">');
      html.push('    <h2>Abstract</h2>');
      html.push('    <p>' + escapeHTML(apdf.metadata.abstract) + '</p>');
      html.push('  </section>');
    }

    // TOC
    if (apdf.structure.tableOfContents.length > 0) {
      html.push('  <nav class="toc" aria-label="Table of Contents">');
      html.push('    <strong>Contents</strong>');
      html.push('    <ul>');
      for (const entry of apdf.structure.tableOfContents) {
        const anchor = entry.title.toLowerCase().replace(/[^a-z0-9]+/g, '-');
        html.push('      <li><a href="#' + escapeAttr(anchor) + '">' + escapeHTML(entry.title) + '</a></li>');
      }
      html.push('    </ul>');
      html.push('  </nav>');
    }

    // Body content from semantic chunks
    for (const chunk of apdf.aiContent.chunks) {
      html.push('  <p>' + escapeHTML(chunk.content.trim()) + '</p>');
    }

    // Figures
    for (const fig of apdf.structure.figures) {
      html.push('  <div class="figure-ref" role="figure" aria-label="' + escapeAttr(fig.caption || fig.id) + '">');
      html.push('    <strong>' + escapeHTML(fig.id) + '</strong> (p.\u00A0' + fig.pageNumber + ')');
      if (fig.caption) {
        html.push('    <p>' + escapeHTML(fig.caption) + '</p>');
      }
      html.push('  </div>');
    }

    // Tables
    for (const table of apdf.structure.tables) {
      html.push('  <div class="table-ref" role="table" aria-label="' + escapeAttr(table.caption || table.id) + '">');
      html.push('    <strong>' + escapeHTML(table.id) + '</strong> (' + table.rows + '\u00D7' + table.columns + ', p.\u00A0' + table.pageNumber + ')');
      if (table.caption) {
        html.push('    <p>' + escapeHTML(table.caption) + '</p>');
      }
      html.push('  </div>');
    }

    // Bibliography
    if (apdf.structure.bibliography.length > 0) {
      html.push('  <section class="bibliography" aria-label="References">');
      html.push('    <h2>References</h2>');
      html.push('    <ol>');
      for (const bib of apdf.structure.bibliography) {
        const authors = bib.authors?.join(', ') || 'Unknown';
        const year = bib.year ? ' (' + bib.year + ')' : '';
        let entry = escapeHTML(authors + year + '. "' + bib.title + '"');
        if (bib.venue) entry += '. <em>' + escapeHTML(bib.venue) + '</em>';
        if (bib.doi) entry += '. <a href="https://doi.org/' + escapeAttr(bib.doi) + '">doi:' + escapeHTML(bib.doi) + '</a>';
        html.push('      <li class="bib-entry">' + entry + '</li>');
      }
      html.push('    </ol>');
      html.push('  </section>');
    }

    html.push('</article>');

    // Auto-render math
    if (apdf.display.hasMath) {
      html.push('<script>');
      html.push('  document.addEventListener("DOMContentLoaded", function() {');
      html.push('    if (typeof renderMathInElement === "function") {');
      html.push('      renderMathInElement(document.querySelector(".article"), {');
      html.push('        delimiters: [');
      html.push('          { left: "$$", right: "$$", display: true },');
      html.push('          { left: "$", right: "$", display: false },');
      html.push('          { left: "\\\\(", right: "\\\\)", display: false },');
      html.push('          { left: "\\\\[", right: "\\\\]", display: true }');
      html.push('        ]');
      html.push('      });');
      html.push('    }');
      html.push('  });');
      html.push('</script>');
    }

    html.push('</body>');
    html.push('</html>');

    const result = html.join('\n');

    console.log(`Document: "${apdf.metadata.title}"`);
    console.log(`Generated ${result.length} chars of HTML`);
    console.log(`  Theme:     ${apdf.display.suggestedTheme}`);
    console.log(`  Columns:   ${apdf.display.readingOrder}`);
    console.log(`  Math:      ${apdf.display.hasMath ? 'KaTeX included' : 'none'}`);
    console.log(`  TOC items: ${apdf.structure.tableOfContents.length}`);
    console.log(`  Figures:   ${apdf.structure.figures.length}`);
    console.log(`  Tables:    ${apdf.structure.tables.length}`);
    console.log(`  Bib refs:  ${apdf.structure.bibliography.length}\n`);

    return result;
  } finally {
    pdf.close();
  }
}

// ============================================================================
// Example 3: Font Audit & Web Font Loader
//
// Analyzes fonts used in the PDF via aPDF display hints and generates
// a Google Fonts / custom @font-face snippet to match them on the web.
// ============================================================================

/** Known mappings from common PDF font names to web equivalents */
const PDF_TO_WEB_FONTS: Record<string, { family: string; googleFonts?: string }> = {
  'TimesNewRomanPSMT': { family: 'Times New Roman', googleFonts: 'Noto+Serif' },
  'Times-Roman': { family: 'Times New Roman', googleFonts: 'Noto+Serif' },
  'Times-Bold': { family: 'Times New Roman', googleFonts: 'Noto+Serif:wght@700' },
  'Times-Italic': { family: 'Times New Roman', googleFonts: 'Noto+Serif:ital@1' },
  'Helvetica': { family: 'Helvetica, Arial, sans-serif', googleFonts: 'Inter' },
  'ArialMT': { family: 'Arial, sans-serif', googleFonts: 'Inter' },
  'Arial-BoldMT': { family: 'Arial, sans-serif', googleFonts: 'Inter:wght@700' },
  'CourierNewPSMT': { family: 'Courier New, monospace', googleFonts: 'Fira+Code' },
  'Courier': { family: 'Courier New, monospace', googleFonts: 'Fira+Code' },
  'CMR10': { family: 'Computer Modern', googleFonts: 'Noto+Serif' },
  'CMR12': { family: 'Computer Modern', googleFonts: 'Noto+Serif' },
  'CMBX10': { family: 'Computer Modern', googleFonts: 'Noto+Serif:wght@700' },
  'CMTI10': { family: 'Computer Modern', googleFonts: 'Noto+Serif:ital@1' },
  'CMTT10': { family: 'Computer Modern Typewriter', googleFonts: 'Fira+Code' },
  'NimbusRomNo9L-Regu': { family: 'Nimbus Roman', googleFonts: 'Noto+Serif' },
  'NimbusSanL-Regu': { family: 'Nimbus Sans', googleFonts: 'Inter' },
};

interface FontAuditResult {
  pdfFont: string;
  role: string;
  webFamily: string;
  googleFontsParam?: string;
}

async function fontAuditAndWebLoader(file: File): Promise<{ audit: FontAuditResult[]; loaderHTML: string }> {
  console.log('\n=== Example 3: Font Audit & Web Font Loader ===\n');

  const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

  try {
    const apdf = await pdf.generateAPDFMetadata();

    const audit: FontAuditResult[] = [];
    const googleFontsParams = new Set<string>();

    for (const font of apdf.display.fonts) {
      const mapping = PDF_TO_WEB_FONTS[font.name];
      const webFamily = mapping?.family || `"${font.name}", sans-serif`;
      const googleParam = mapping?.googleFonts;

      audit.push({
        pdfFont: font.name,
        role: font.role,
        webFamily,
        googleFontsParam: googleParam,
      });

      if (googleParam) {
        googleFontsParams.add(googleParam);
      }
    }

    // Generate loader snippet
    const loaderLines: string[] = [];

    if (googleFontsParams.size > 0) {
      const families = [...googleFontsParams].map(f => 'family=' + f).join('&');
      loaderLines.push(`<!-- Google Fonts loader generated from aPDF font audit -->`);
      loaderLines.push(`<link rel="preconnect" href="https://fonts.googleapis.com">`);
      loaderLines.push(`<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>`);
      loaderLines.push(`<link rel="stylesheet" href="https://fonts.googleapis.com/css2?${families}&display=swap">`);
    } else {
      loaderLines.push(`<!-- No Google Fonts matches — using system font stack -->`);
    }

    const loaderHTML = loaderLines.join('\n');

    console.log('Font Audit:');
    for (const entry of audit) {
      const gf = entry.googleFontsParam ? ` → Google: ${entry.googleFontsParam}` : ' (system fallback)';
      console.log(`  [${entry.role}] ${entry.pdfFont} → ${entry.webFamily}${gf}`);
    }
    console.log(`\nWeb Font Loader:\n${loaderHTML}\n`);

    return { audit, loaderHTML };
  } finally {
    pdf.close();
  }
}

// ============================================================================
// Example 4: Accessible Reading View
//
// Generates semantic HTML with ARIA landmarks, reading-level indicators,
// and skip-navigation based on aPDF structure, suitable for screen readers.
// ============================================================================

async function accessibleReadingView(file: File): Promise<string> {
  console.log('\n=== Example 4: Accessible Reading View ===\n');

  const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

  try {
    const apdf = await pdf.generateAPDFMetadata();
    const html: string[] = [];

    // Compute reading level description
    const stats = apdf.aiContent.stats;
    const readingLevel = stats.readingLevel != null
      ? (stats.readingLevel <= 8 ? 'Easy' : stats.readingLevel <= 12 ? 'Moderate' : 'Advanced')
      : 'Unknown';

    html.push('<!DOCTYPE html>');
    html.push('<html lang="' + escapeAttr(apdf.metadata.language) + '">');
    html.push('<head>');
    html.push('  <meta charset="utf-8">');
    html.push('  <meta name="viewport" content="width=device-width, initial-scale=1">');
    html.push('  <title>' + escapeHTML(apdf.metadata.title) + ' — Accessible View</title>');
    html.push('  <style>');
    html.push('    body { font-family: system-ui, sans-serif; max-width: 45em; margin: 2rem auto; padding: 0 1rem; line-height: 1.7; }');
    html.push('    .skip-link { position: absolute; top: -40px; left: 0; background: #000; color: #fff; padding: 8px; z-index: 100; }');
    html.push('    .skip-link:focus { top: 0; }');
    html.push('    .doc-info { background: #f5f5f5; padding: 1rem; border-radius: 6px; margin-bottom: 2rem; }');
    html.push('    .doc-info dt { font-weight: bold; }');
    html.push('    .doc-info dd { margin: 0 0 0.5em 1em; }');
    html.push('    .chunk { margin-bottom: 1em; }');
    html.push('    .chunk[data-importance="high"] { border-left: 3px solid #4CAF50; padding-left: 1em; }');
    html.push('    @media (prefers-color-scheme: dark) { body { background: #1a1a1a; color: #d4d4d4; } .doc-info { background: #2a2a2a; } }');
    html.push('  </style>');
    html.push('</head>');
    html.push('<body>');

    // Skip navigation
    html.push('  <a class="skip-link" href="#main-content">Skip to content</a>');

    // Document info banner
    html.push('  <aside class="doc-info" role="complementary" aria-label="Document information">');
    html.push('    <dl>');
    html.push('      <dt>Title</dt><dd>' + escapeHTML(apdf.metadata.title) + '</dd>');
    html.push('      <dt>Pages</dt><dd>' + apdf.metadata.pageCount + '</dd>');
    html.push('      <dt>Reading level</dt><dd>' + readingLevel + (stats.readingLevel != null ? ' (grade ' + stats.readingLevel + ')' : '') + '</dd>');
    html.push('      <dt>Estimated reading time</dt><dd>' + Math.ceil(stats.tokenCount / 250) + ' min</dd>');
    html.push('      <dt>Language</dt><dd>' + escapeHTML(apdf.metadata.language) + '</dd>');
    html.push('    </dl>');
    html.push('  </aside>');

    // TOC navigation
    if (apdf.structure.tableOfContents.length > 0) {
      html.push('  <nav aria-label="Table of contents">');
      html.push('    <h2>Contents</h2>');
      html.push('    <ol>');
      for (const entry of apdf.structure.tableOfContents) {
        const anchor = entry.title.toLowerCase().replace(/[^a-z0-9]+/g, '-');
        html.push('      <li><a href="#' + escapeAttr(anchor) + '">' + escapeHTML(entry.title) + '</a></li>');
      }
      html.push('    </ol>');
      html.push('  </nav>');
    }

    // Main content
    html.push('  <main id="main-content" role="main">');

    for (const chunk of apdf.aiContent.chunks) {
      const importance = chunk.importance >= 0.7 ? 'high' : chunk.importance >= 0.4 ? 'medium' : 'low';
      html.push('    <div class="chunk" data-importance="' + importance + '" data-pages="' + chunk.pageNumbers.join(',') + '">');
      html.push('      <p>' + escapeHTML(chunk.content.trim()) + '</p>');
      html.push('    </div>');
    }

    html.push('  </main>');

    // Footer
    html.push('  <footer role="contentinfo">');
    html.push('    <p><small>Generated from aPDF v' + escapeHTML(apdf.apdfVersion) + ' by ' + escapeHTML(apdf.provenance.generator) + '</small></p>');
    html.push('  </footer>');

    html.push('</body>');
    html.push('</html>');

    const result = html.join('\n');

    console.log(`Document:     "${apdf.metadata.title}"`);
    console.log(`Language:     ${apdf.metadata.language}`);
    console.log(`Reading level: ${readingLevel}`);
    console.log(`Reading time: ~${Math.ceil(stats.tokenCount / 250)} min`);
    console.log(`Chunks:       ${apdf.aiContent.chunks.length}`);
    console.log(`High-importance chunks: ${apdf.aiContent.chunks.filter(c => c.importance >= 0.7).length}`);
    console.log(`HTML size:    ${result.length} chars\n`);

    return result;
  } finally {
    pdf.close();
  }
}

// ============================================================================
// Example 5: Print-Ready Typesetting Stylesheet
//
// Generates a CSS stylesheet optimized for print/PDF output with precise
// page dimensions, margins, orphan/widow control, and figure placement.
// ============================================================================

async function printReadyStylesheet(file: File): Promise<string> {
  console.log('\n=== Example 5: Print-Ready Typesetting Stylesheet ===\n');

  const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

  try {
    const apdf = await pdf.generateAPDFMetadata();
    const d = apdf.display;
    const { width: pageW, height: pageH } = d.pageDimensions;

    // Determine page size name
    const pageSizeName =
      (Math.abs(pageW - 612) < 5 && Math.abs(pageH - 792) < 5) ? 'letter' :
      (Math.abs(pageW - 595) < 5 && Math.abs(pageH - 842) < 5) ? 'A4' :
      `${pageW}pt ${pageH}pt`;

    const lines: string[] = [];

    lines.push('/* aPDF Print-Ready Typesetting Stylesheet */');
    lines.push(`/* Source: ${escapeCSS(apdf.metadata.title)} */`);
    lines.push(`/* Page size: ${pageSizeName} (${pageW}×${pageH} pt) */`);
    lines.push('');

    // Page setup
    lines.push('@page {');
    lines.push(`  size: ${pageSizeName};`);
    lines.push('  margin: 2.5cm 2cm;');
    lines.push('  @bottom-center {');
    lines.push('    content: counter(page);');
    lines.push('    font-size: 9pt;');
    lines.push('  }');
    lines.push('}');
    lines.push('');

    // First page — no page number, extra top margin
    lines.push('@page :first {');
    lines.push('  margin-top: 4cm;');
    lines.push('  @bottom-center { content: none; }');
    lines.push('}');
    lines.push('');

    // Body
    const bodyFont = d.fonts.find(f => f.role === 'body');
    const headingFont = d.fonts.find(f => f.role === 'heading');
    const monoFont = d.fonts.find(f => f.role === 'mono');

    lines.push('body {');
    lines.push(`  font-family: "${bodyFont?.name || 'Georgia'}", "Noto Serif", serif;`);
    lines.push('  font-size: 10pt;');
    lines.push('  line-height: 1.5;');
    lines.push('  color: #000;');
    lines.push('  orphans: 3;');
    lines.push('  widows: 3;');
    lines.push('  hyphens: auto;');
    lines.push('  text-align: justify;');
    lines.push('}');
    lines.push('');

    // Column layout
    if (d.readingOrder === 'multi-column') {
      lines.push('.content {');
      lines.push('  column-count: 2;');
      lines.push('  column-gap: 0.6cm;');
      lines.push('  column-rule: 0.5pt solid #ccc;');
      lines.push('}');
      lines.push('');
    }

    // Headings
    lines.push(`h1, h2, h3, h4 {`);
    lines.push(`  font-family: "${headingFont?.name || bodyFont?.name || 'Georgia'}", serif;`);
    lines.push('  page-break-after: avoid;');
    lines.push('  break-after: avoid;');
    lines.push('}');
    lines.push('');
    lines.push('h1 { font-size: 16pt; margin-top: 0; }');
    lines.push('h2 { font-size: 13pt; margin-top: 1.5em; }');
    lines.push('h3 { font-size: 11pt; margin-top: 1.2em; }');
    lines.push('');

    // Figures and tables — avoid breaks
    lines.push('figure, table, .figure-ref, .table-ref {');
    lines.push('  page-break-inside: avoid;');
    lines.push('  break-inside: avoid;');
    lines.push('  margin: 1em 0;');
    lines.push('}');
    lines.push('');

    lines.push('figcaption, .table-caption {');
    lines.push('  font-size: 9pt;');
    lines.push('  text-align: center;');
    lines.push('  margin-top: 0.5em;');
    lines.push('}');
    lines.push('');

    // Code blocks
    lines.push('pre, code {');
    lines.push(`  font-family: "${monoFont?.name || 'Courier New'}", "Fira Code", monospace;`);
    lines.push('  font-size: 8.5pt;');
    lines.push('}');
    lines.push('');
    lines.push('pre {');
    lines.push('  background: #f8f8f8;');
    lines.push('  padding: 0.5em;');
    lines.push('  border: 0.5pt solid #ddd;');
    lines.push('  page-break-inside: avoid;');
    lines.push('  break-inside: avoid;');
    lines.push('  overflow-x: auto;');
    lines.push('}');
    lines.push('');

    // Math
    if (d.hasMath) {
      lines.push('/* Math — ensure equations don\'t break across pages */');
      lines.push('.math-block, .katex-display {');
      lines.push('  page-break-inside: avoid;');
      lines.push('  break-inside: avoid;');
      lines.push('  margin: 0.8em 0;');
      lines.push('}');
      lines.push('');
    }

    // Bibliography
    lines.push('.bibliography {');
    lines.push('  font-size: 9pt;');
    lines.push('  line-height: 1.4;');
    lines.push('}');
    lines.push('');
    lines.push('.bibliography li {');
    lines.push('  margin-bottom: 0.3em;');
    lines.push('}');
    lines.push('');

    // Links — show URL in print
    lines.push('a[href^="http"]::after {');
    lines.push('  content: " (" attr(href) ")";');
    lines.push('  font-size: 8pt;');
    lines.push('  color: #666;');
    lines.push('  word-break: break-all;');
    lines.push('}');
    lines.push('');

    // Abstract
    lines.push('.abstract {');
    lines.push('  font-size: 9.5pt;');
    lines.push('  font-style: italic;');
    lines.push('  margin: 1em 2em;');
    lines.push('}');

    const result = lines.join('\n');

    console.log(`Document:   "${apdf.metadata.title}"`);
    console.log(`Page size:  ${pageSizeName}`);
    console.log(`Columns:    ${d.readingOrder}`);
    console.log(`Body font:  ${bodyFont?.name || '(default)'}`);
    console.log(`Heading:    ${headingFont?.name || '(default)'}`);
    console.log(`Mono:       ${monoFont?.name || '(default)'}`);
    console.log(`Math:       ${d.hasMath}`);
    console.log(`CSS size:   ${result.length} chars\n`);

    return result;
  } finally {
    pdf.close();
  }
}

// ============================================================================
// Example 6: OG / Twitter Card Meta Tags
//
// Generates Open Graph and Twitter Card metadata from aPDF for social
// sharing previews when the document is published on the web.
// ============================================================================

async function socialMetaTags(file: File, pageUrl: string): Promise<string> {
  console.log('\n=== Example 6: Open Graph & Twitter Card Meta Tags ===\n');

  const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

  try {
    const apdf = await pdf.generateAPDFMetadata();

    const title = apdf.metadata.title;
    const description = apdf.metadata.abstract
      || apdf.aiContent.summary
      || apdf.aiContent.chunks[0]?.content.substring(0, 200)
      || '';
    const authors = apdf.authors.map(a => a.name).join(', ');

    const tags: string[] = [];

    // Open Graph
    tags.push('<!-- Open Graph -->');
    tags.push('<meta property="og:type" content="article">');
    tags.push('<meta property="og:title" content="' + escapeAttr(title) + '">');
    tags.push('<meta property="og:description" content="' + escapeAttr(truncate(description, 300)) + '">');
    tags.push('<meta property="og:url" content="' + escapeAttr(pageUrl) + '">');
    if (apdf.metadata.datePublished) {
      tags.push('<meta property="article:published_time" content="' + escapeAttr(apdf.metadata.datePublished) + '">');
    }
    for (const author of apdf.authors) {
      tags.push('<meta property="article:author" content="' + escapeAttr(author.name) + '">');
    }
    for (const kw of apdf.aiContent.keywords.slice(0, 5)) {
      tags.push('<meta property="article:tag" content="' + escapeAttr(kw) + '">');
    }

    tags.push('');

    // Twitter Card
    tags.push('<!-- Twitter Card -->');
    tags.push('<meta name="twitter:card" content="summary_large_image">');
    tags.push('<meta name="twitter:title" content="' + escapeAttr(title) + '">');
    tags.push('<meta name="twitter:description" content="' + escapeAttr(truncate(description, 200)) + '">');

    tags.push('');

    // Citation metadata
    tags.push('<!-- Citation metadata -->');
    tags.push('<meta name="citation_title" content="' + escapeAttr(title) + '">');
    for (const author of apdf.authors) {
      tags.push('<meta name="citation_author" content="' + escapeAttr(author.name) + '">');
    }
    if (apdf.metadata.identifiers.doi) {
      tags.push('<meta name="citation_doi" content="' + escapeAttr(apdf.metadata.identifiers.doi) + '">');
    }
    if (apdf.metadata.identifiers.arxivId) {
      tags.push('<meta name="citation_arxiv_id" content="' + escapeAttr(apdf.metadata.identifiers.arxivId) + '">');
    }
    if (apdf.metadata.datePublished) {
      tags.push('<meta name="citation_publication_date" content="' + escapeAttr(apdf.metadata.datePublished.substring(0, 10)) + '">');
    }
    if (apdf.metadata.venue) {
      tags.push('<meta name="citation_journal_title" content="' + escapeAttr(apdf.metadata.venue) + '">');
    }

    tags.push('');

    // JSON-LD structured data
    const jsonLd = {
      '@context': 'https://schema.org',
      '@type': apdf['@type'],
      headline: title,
      author: apdf.authors.map(a => ({
        '@type': 'Person',
        name: a.name,
        ...(a.orcid ? { url: 'https://orcid.org/' + a.orcid } : {}),
      })),
      datePublished: apdf.metadata.datePublished,
      description: truncate(description, 300),
      keywords: apdf.aiContent.keywords.slice(0, 10).join(', '),
      url: pageUrl,
      ...(apdf.metadata.identifiers.doi ? { sameAs: 'https://doi.org/' + apdf.metadata.identifiers.doi } : {}),
    };

    tags.push('<script type="application/ld+json">');
    tags.push(JSON.stringify(jsonLd, null, 2));
    tags.push('</script>');

    const result = tags.join('\n');

    console.log(`Title:       "${title}"`);
    console.log(`Authors:     ${authors}`);
    console.log(`Description: ${truncate(description, 100)}`);
    console.log(`Keywords:    ${apdf.aiContent.keywords.slice(0, 5).join(', ')}`);
    console.log(`Tags:        ${tags.filter(t => t.startsWith('<meta')).length} meta tags + JSON-LD`);
    console.log(`\nPreview:\n${result.substring(0, 600)}\n...`);

    return result;
  } finally {
    pdf.close();
  }
}

// ============================================================================
// Example 7: Canvas PDF Page Render with Theme & Overlay
//
// Renders a PDF page to an HTML canvas with dark-mode support, then overlays
// aPDF structural annotations (figure/table bounding regions, reading order).
// ============================================================================

async function canvasRenderWithOverlay(
  file: File,
  pageNumber: number = 1,
): Promise<void> {
  console.log('\n=== Example 7: Canvas Render with Structural Overlay ===\n');

  // This example requires a browser environment with HTMLCanvasElement.
  // It demonstrates the API calls; in Node.js it logs what would happen.
  const isBrowser = typeof HTMLCanvasElement !== 'undefined';

  const pdf = await AgenticPDF.fromFile(file);

  try {
    const apdf = await pdf.generateAPDFMetadata();
    const metadata = pdf.getMetadata();
    const totalPages = metadata?.pageCount || 1;

    console.log(`Document: "${apdf.metadata.title}" (${totalPages} pages)`);
    console.log(`Rendering page ${pageNumber} with structural overlay\n`);

    if (isBrowser) {
      // Create canvas
      const canvas = document.createElement('canvas');
      const scale = 2.0;

      // Render the PDF page
      await pdf.renderPage(pageNumber, canvas, {
        scale,
        darkMode: apdf.display.suggestedTheme === 'general' ? false : undefined,
        renderAnnotations: true,
      });

      console.log(`  Page rendered at ${scale}x (${canvas.width}×${canvas.height})`);

      // Build selectable text layer
      const textContainer = document.createElement('div');
      textContainer.style.position = 'absolute';
      textContainer.style.top = '0';
      textContainer.style.left = '0';

      await pdf.buildTextLayer(pageNumber, textContainer, {
        width: canvas.width,
        height: canvas.height,
        scale,
      });

      console.log('  Text layer built for selection/search');

      // Draw structural overlay on a separate canvas
      const overlayCanvas = document.createElement('canvas');
      overlayCanvas.width = canvas.width;
      overlayCanvas.height = canvas.height;
      const ctx = overlayCanvas.getContext('2d')!;

      const pageScale = canvas.width / apdf.display.pageDimensions.width;

      // Highlight figures
      ctx.strokeStyle = 'rgba(76, 175, 80, 0.6)';
      ctx.lineWidth = 2;
      for (const fig of apdf.structure.figures.filter(f => f.pageNumber === pageNumber)) {
        // Use approximate positions based on page layout
        ctx.strokeRect(50 * pageScale, 100 * pageScale, 500 * pageScale, 300 * pageScale);
        ctx.fillStyle = 'rgba(76, 175, 80, 0.1)';
        ctx.fillRect(50 * pageScale, 100 * pageScale, 500 * pageScale, 300 * pageScale);

        // Label
        ctx.fillStyle = 'rgba(76, 175, 80, 0.9)';
        ctx.font = `${12 * pageScale}px system-ui`;
        ctx.fillText(fig.caption || fig.id, 55 * pageScale, 95 * pageScale);
      }

      // Highlight tables
      ctx.strokeStyle = 'rgba(33, 150, 243, 0.6)';
      for (const table of apdf.structure.tables.filter(t => t.pageNumber === pageNumber)) {
        ctx.strokeRect(50 * pageScale, 450 * pageScale, 500 * pageScale, 200 * pageScale);
        ctx.fillStyle = 'rgba(33, 150, 243, 0.1)';
        ctx.fillRect(50 * pageScale, 450 * pageScale, 500 * pageScale, 200 * pageScale);

        ctx.fillStyle = 'rgba(33, 150, 243, 0.9)';
        ctx.fillText(
          `${table.caption || table.id} (${table.rows}×${table.columns})`,
          55 * pageScale, 445 * pageScale,
        );
      }

      console.log(`  Overlay: ${apdf.structure.figures.filter(f => f.pageNumber === pageNumber).length} figures, ` +
        `${apdf.structure.tables.filter(t => t.pageNumber === pageNumber).length} tables highlighted`);

    } else {
      // Node.js: demonstrate the API path without actual rendering
      console.log('  [Node.js] Canvas rendering requires a browser environment.');
      console.log('  API calls that would execute:');
      console.log('    pdf.renderPage(' + pageNumber + ', canvas, { scale: 2.0 })');
      console.log('    pdf.buildTextLayer(' + pageNumber + ', container, viewport)');
      console.log('  Structural elements on page ' + pageNumber + ':');

      const pageFigures = apdf.structure.figures.filter(f => f.pageNumber === pageNumber);
      const pageTables = apdf.structure.tables.filter(t => t.pageNumber === pageNumber);
      const pageEquations = apdf.structure.equations.filter(e => e.pageNumber === pageNumber);

      console.log(`    Figures:   ${pageFigures.length}${pageFigures.length > 0 ? ' — ' + pageFigures.map(f => f.caption || f.id).join(', ') : ''}`);
      console.log(`    Tables:    ${pageTables.length}${pageTables.length > 0 ? ' — ' + pageTables.map(t => t.caption || t.id).join(', ') : ''}`);
      console.log(`    Equations: ${pageEquations.length}`);
    }

  } finally {
    pdf.close();
  }
}

// ============================================================================
// Example 8: Multi-Format Export Pipeline with Display Awareness
//
// Uses aPDF display hints to choose optimal export settings per format,
// then exports the document in HTML, Markdown, and plain text.
// ============================================================================

interface ExportResult {
  format: string;
  size: number;
  preview: string;
}

async function displayAwareExportPipeline(file: File): Promise<ExportResult[]> {
  console.log('\n=== Example 8: Display-Aware Multi-Format Export ===\n');

  const pdf = await AgenticPDF.fromFile(file);

  try {
    const apdf = await pdf.generateAPDFMetadata();
    const results: ExportResult[] = [];

    console.log(`Document: "${apdf.metadata.title}"`);
    console.log(`Display:  ${apdf.display.readingOrder}, theme=${apdf.display.suggestedTheme}, math=${apdf.display.hasMath}\n`);

    // HTML export — include images for color documents, metadata always
    const htmlBlob = await pdf.exportAs('html', {
      includeMetadata: true,
      includeAnnotations: true,
      includeImages: apdf.display.hasColor,
      imageFormat: 'webp',
    });
    const htmlText = typeof htmlBlob === 'string' ? htmlBlob : await htmlBlob.text();
    results.push({
      format: 'HTML',
      size: htmlText.length,
      preview: htmlText.substring(0, 200),
    });

    // Markdown export — include images for technical docs
    const mdBlob = await pdf.exportAs('markdown', {
      includeMetadata: true,
      includeImages: apdf.display.suggestedTheme === 'technical',
      imageFormat: 'png',
    });
    const mdText = typeof mdBlob === 'string' ? mdBlob : await mdBlob.text();
    results.push({
      format: 'Markdown',
      size: mdText.length,
      preview: mdText.substring(0, 200),
    });

    // Plain text — always useful for indexing
    const textBlob = await pdf.exportAs('text', { includeMetadata: true });
    const textStr = typeof textBlob === 'string' ? textBlob : await textBlob.text();
    results.push({
      format: 'Text',
      size: textStr.length,
      preview: textStr.substring(0, 200),
    });

    // aPDF JSON — full structured envelope
    const apdfJson = await pdf.exportAs('apdf');
    const apdfStr = typeof apdfJson === 'string' ? apdfJson : await apdfJson.text();
    results.push({
      format: 'aPDF JSON',
      size: apdfStr.length,
      preview: apdfStr.substring(0, 200),
    });

    console.log('Export Results:');
    for (const r of results) {
      console.log(`  ${r.format}: ${(r.size / 1024).toFixed(1)} KB`);
    }
    console.log('');

    return results;
  } finally {
    pdf.close();
  }
}

// ============================================================================
// Utilities
// ============================================================================

function escapeHTML(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

function escapeAttr(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/"/g, '&quot;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

function escapeCSS(text: string): string {
  return text.replace(/[\\/"']/g, '\\$&');
}

function truncate(text: string, maxLen: number): string {
  const cleaned = text.replace(/\s+/g, ' ').trim();
  if (cleaned.length <= maxLen) return cleaned;
  return cleaned.substring(0, maxLen - 1) + '\u2026';
}

// ============================================================================
// Runner
// ============================================================================

export async function typesettingWebDisplayExamples(file: File, pageUrl?: string): Promise<void> {
  await cssFromDisplayHints(file);
  await responsiveHTMLArticle(file);
  await fontAuditAndWebLoader(file);
  await accessibleReadingView(file);
  await printReadyStylesheet(file);
  await socialMetaTags(file, pageUrl || 'https://example.com/papers/document');
  await canvasRenderWithOverlay(file);
  await displayAwareExportPipeline(file);

  console.log('\n=== All typesetting & web display examples completed ===');
}
