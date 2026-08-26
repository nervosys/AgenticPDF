import Link from "next/link";
import { DocsSidebar } from "@/components/ui";
import { HighlightedCode } from "@/components/highlighted-code";

const sidebar = [
  {
    title: "Getting Started",
    links: [
      { href: "/docs/", label: "Introduction" },
      { href: "/docs/#installation", label: "Installation" },
      { href: "/docs/#quick-start", label: "Quick Start" },
    ],
  },
  {
    title: "Reference",
    links: [
      { href: "/docs/api/", label: "API Reference" },
      { href: "/docs/cli/", label: "CLI Reference" },
      { href: "/docs/guides/", label: "Guides" },
    ],
  },
  {
    title: "Demos",
    links: [
      { href: "/demos/ai/", label: "Agentic AI" },
      { href: "/demos/streaming/", label: "Streaming" },
      { href: "/demos/viewer/", label: "PDF Viewer" },
      { href: "/demos/pretext/", label: "Pretext Layout" },
    ],
  },
];

export default function DocsPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 sm:px-6 py-12 flex gap-0">
      <DocsSidebar sections={sidebar} />

      <article className="min-w-0 max-w-3xl">
        <h1 className="text-4xl font-bold tracking-tight mb-4" style={{ color: "var(--text)" }}>
          Documentation
        </h1>
        <p className="text-lg mb-10 leading-relaxed" style={{ color: "var(--text-muted)" }}>
          AgenticPDF is a comprehensive, production-ready PDF processing library with
          first-class support for streaming and AI systems. Everything is in a single
          TypeScript file with zero runtime dependencies.
        </p>

        {/* Installation */}
        <section id="installation" className="mb-12">
          <h2 className="text-2xl font-bold mb-4" style={{ color: "var(--text)" }}>
            Installation
          </h2>
          <HighlightedCode code="npm install agenticpdf" language="bash" />

          <p className="mt-4 text-sm" style={{ color: "var(--text-muted)" }}>
            Or include the browser bundle directly:
          </p>
          <HighlightedCode
            code={`<script src="https://unpkg.com/agenticpdf/agenticpdf-browser.js"></script>`}
            language="html"
          />
        </section>

        {/* Quick Start */}
        <section id="quick-start" className="mb-12">
          <h2 className="text-2xl font-bold mb-4" style={{ color: "var(--text)" }}>
            Quick Start
          </h2>

          <h3 className="text-lg font-semibold mb-3" style={{ color: "var(--text)" }}>
            Load a PDF
          </h3>
          <HighlightedCode
            code={`import { AgenticPDF } from 'agenticpdf';

// From file (browser)
const pdf = await AgenticPDF.fromFile(file);

// From URL
const pdf = await AgenticPDF.fromUrl('https://example.com/doc.pdf');

// From buffer (Node.js)
const pdf = await AgenticPDF.fromBuffer(buffer);

// From stream
const pdf = AgenticPDF.fromStream(readableStream);`}
            filename="loading.ts"
          />

          <h3 className="text-lg font-semibold mt-8 mb-3" style={{ color: "var(--text)" }}>
            Extract Text
          </h3>
          <HighlightedCode
            code={`const pages = await pdf.extractText({
  preserveFormatting: true,
  extractTables: true,
  pageRange: { start: 1, end: 10 }
});

for (const page of pages) {
  console.log(\`Page \${page.pageNumber}: \${page.text}\`);
}

// Or stream for large documents
for await (const page of pdf.streamText()) {
  process.stdout.write(page.text);
}`}
            filename="extract-text.ts"
          />

          <h3 className="text-lg font-semibold mt-8 mb-3" style={{ color: "var(--text)" }}>
            AI &amp; Semantic Chunking
          </h3>
          <HighlightedCode
            code={`// Generate semantic chunks for RAG
for await (const chunk of pdf.streamSemanticChunks({
  strategy: 'semantic',
  maxChunkSize: 1000,
  preserveParagraphs: true
})) {
  await vectorStore.add({
    content: chunk.content,
    embedding: await embed(chunk.content),
    metadata: { pages: chunk.pageNumbers, type: chunk.type }
  });
}

// Full AI analysis
const ai = await pdf.getAIFeatures({
  enableStructuralAnalysis: true,
  enableSemanticChunking: true,
  enableNER: true
});
console.log(ai.structuralAnalysis.sections);
console.log(ai.nlpReady.keywords);`}
            filename="ai-features.ts"
          />

          <h3 className="text-lg font-semibold mt-8 mb-3" style={{ color: "var(--text)" }}>
            Agentic Ingestion
          </h3>
          <HighlightedCode
            code={`// Single-call ingestion — metadata + structure + chunks
const result = await pdf.ingest({ chunkSize: 1000 });
console.log(result.metadata.title);
console.log(result.stats.totalChunks);

// Or stream as NDJSON for real-time pipelines
for await (const line of pdf.streamIngest()) {
  const record = JSON.parse(line);
  if (record.type === 'chunk') process.stdout.write('.');
}`}
            filename="ingest.ts"
          />

          <h3 className="text-lg font-semibold mt-8 mb-3" style={{ color: "var(--text)" }}>
            Render to Canvas
          </h3>
          <HighlightedCode
            code={`const canvas = document.getElementById('pdf-canvas');
await pdf.renderPage(1, canvas, {
  scale: 2,
  fitToWidth: true,
  darkMode: true
});

// Or create a full viewer
const viewer = pdf.createOptimalViewer(container, {
  continuousScrolling: true,
  enableThemeToggle: true,
  defaultTheme: 'dark'
});`}
            filename="render.ts"
          />

          <h3 className="text-lg font-semibold mt-8 mb-3" style={{ color: "var(--text)" }}>
            Memory Management
          </h3>
          <HighlightedCode
            code={`const pdf = await AgenticPDF.fromFile(file, {
  lazyLoad: true,
  maxMemoryUsage: 100 * 1024 * 1024 // 100MB
});

try {
  // ... process document
} finally {
  pdf.close(); // Always clean up
}`}
            filename="memory.ts"
          />
        </section>

        {/* Architecture overview */}
        <section id="architecture" className="mb-12">
          <h2 className="text-2xl font-bold mb-4" style={{ color: "var(--text)" }}>
            Architecture
          </h2>
          <p className="mb-4" style={{ color: "var(--text-muted)" }}>
            AgenticPDF processes PDFs through a layered pipeline:
          </p>
          <div
            className="rounded-xl p-6 font-mono text-sm leading-loose"
            style={{ background: "var(--bg-card)", border: "1px solid var(--border)", color: "var(--text)" }}
          >
            PDF File → Stream → Lexer → Parser → XRef → Catalog → Page → Renderer
          </div>
          <ul className="mt-4 space-y-2 text-sm" style={{ color: "var(--text-muted)" }}>
            <li><strong style={{ color: "var(--text)" }}>Stream</strong> — Byte-by-byte reading with position tracking</li>
            <li><strong style={{ color: "var(--text)" }}>Lexer</strong> — Tokenizes bytes into PDF tokens</li>
            <li><strong style={{ color: "var(--text)" }}>Parser</strong> — Constructs objects from tokens</li>
            <li><strong style={{ color: "var(--text)" }}>XRef</strong> — Cross-reference table resolution</li>
            <li><strong style={{ color: "var(--text)" }}>Catalog</strong> — Document structure navigation</li>
            <li><strong style={{ color: "var(--text)" }}>Page</strong> — Individual page content</li>
            <li><strong style={{ color: "var(--text)" }}>CanvasGraphics</strong> — Executes PDF operators on canvas</li>
          </ul>
        </section>

        {/* Next pages */}
        <div className="flex gap-4 pt-8 border-t" style={{ borderColor: "var(--border)" }}>
          <Link
            href="/docs/api/"
            className="flex-1 rounded-xl p-4 transition-colors"
            style={{ background: "var(--bg-card)", border: "1px solid var(--border)" }}
          >
            <span className="text-sm" style={{ color: "var(--text-muted)" }}>Next</span>
            <span className="block font-semibold" style={{ color: "var(--accent)" }}>API Reference →</span>
          </Link>
          <Link
            href="/docs/cli/"
            className="flex-1 rounded-xl p-4 transition-colors"
            style={{ background: "var(--bg-card)", border: "1px solid var(--border)" }}
          >
            <span className="text-sm" style={{ color: "var(--text-muted)" }}>Reference</span>
            <span className="block font-semibold" style={{ color: "var(--accent)" }}>CLI Reference →</span>
          </Link>
        </div>
      </article>
    </div>
  );
}
