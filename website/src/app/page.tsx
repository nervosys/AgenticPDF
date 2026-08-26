import Link from "next/link";
import { FeatureCard } from "@/components/ui";
import { HighlightedCode } from "@/components/highlighted-code";

const features = [
  {
    icon: (
      <svg width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.5" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" d="M9.75 3.104v5.714a2.25 2.25 0 0 1-.659 1.591L5 14.5M9.75 3.104c-.251.023-.501.05-.75.082m.75-.082a24.301 24.301 0 0 1 4.5 0m0 0v5.714a2.25 2.25 0 0 0 .659 1.591L19 14.5m-4.75-11.396c.251.023.501.05.75.082M19 14.5l-2.47-2.47" />
      </svg>
    ),
    title: "AI-Native",
    description:
      "Built-in semantic chunking, structural analysis, entity extraction, and embedding support for RAG pipelines.",
  },
  {
    icon: (
      <svg width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.5" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" d="m3.75 13.5 10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75Z" />
      </svg>
    ),
    title: "Streaming-First",
    description:
      "Process gigabyte PDFs with constant memory. Every major operation supports async generators and ReadableStream.",
  },
  {
    icon: (
      <svg width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.5" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" d="M9.53 16.122a3 3 0 0 0-5.78 1.128 2.25 2.25 0 0 1-2.4 2.245 4.5 4.5 0 0 0 8.4-2.245c0-.399-.078-.78-.22-1.128Zm0 0a15.998 15.998 0 0 0 3.388-1.62m-5.043-.025a15.994 15.994 0 0 1 1.622-3.395m3.42 3.42a15.995 15.995 0 0 0 4.764-4.648l3.876-5.814a1.151 1.151 0 0 0-1.597-1.597L14.146 6.32a15.996 15.996 0 0 0-4.649 4.763m3.42 3.42a6.776 6.776 0 0 0-3.42-3.42" />
      </svg>
    ),
    title: "Canvas Rendering",
    description:
      "Full PDF rendering to HTML Canvas with text layers, theme support, continuous scrolling, and High DPI.",
  },
  {
    icon: (
      <svg width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.5" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" d="M3.75 6A2.25 2.25 0 0 1 6 3.75h2.25A2.25 2.25 0 0 1 10.5 6v2.25a2.25 2.25 0 0 1-2.25 2.25H6a2.25 2.25 0 0 1-2.25-2.25V6ZM3.75 15.75A2.25 2.25 0 0 1 6 13.5h2.25a2.25 2.25 0 0 1 2.25 2.25V18a2.25 2.25 0 0 1-2.25 2.25H6A2.25 2.25 0 0 1 3.75 18v-2.25ZM13.5 6a2.25 2.25 0 0 1 2.25-2.25H18A2.25 2.25 0 0 1 20.25 6v2.25A2.25 2.25 0 0 1 18 10.5h-2.25a2.25 2.25 0 0 1-2.25-2.25V6ZM13.5 15.75a2.25 2.25 0 0 1 2.25-2.25H18a2.25 2.25 0 0 1 2.25 2.25V18A2.25 2.25 0 0 1 18 20.25h-2.25a2.25 2.25 0 0 1-2.25-2.25v-2.25Z" />
      </svg>
    ),
    title: "Pretext Layout",
    description:
      "Native zero-dependency text measurement and line-breaking engine for precise typography and web typesetting.",
  },
  {
    icon: (
      <svg width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.5" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" d="M16.5 10.5V6.75a4.5 4.5 0 1 0-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 0 0 2.25-2.25v-6.75a2.25 2.25 0 0 0-2.25-2.25H6.75a2.25 2.25 0 0 0-2.25 2.25v6.75a2.25 2.25 0 0 0 2.25 2.25Z" />
      </svg>
    ),
    title: "Zero Dependencies",
    description:
      "Single TypeScript file with no runtime dependencies. Tree-shakeable, auditable, and fully self-contained.",
  },
  {
    icon: (
      <svg width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.5" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" d="m21 7.5-9-5.25L3 7.5m18 0-9 5.25m9-5.25v9l-9 5.25M3 7.5l9 5.25M3 7.5v9l9 5.25m0-9v9" />
      </svg>
    ),
    title: "Universal",
    description:
      "Works in Node.js, browsers, and edge runtimes. ESM, CommonJS, and IIFE bundles included.",
  },
  {
    icon: (
      <svg width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.5" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" d="M12 18v-5.25m0 0a6.01 6.01 0 0 0 1.5-.189m-1.5.189a6.01 6.01 0 0 1-1.5-.189m3.75 7.478a12.06 12.06 0 0 1-4.5 0m3.75 2.383a14.406 14.406 0 0 1-3 0M14.25 18v-.192c0-.983.658-1.823 1.508-2.316a7.5 7.5 0 1 0-7.517 0c.85.493 1.509 1.333 1.509 2.316V18" />
      </svg>
    ),
    title: "Agent Discovery",
    description:
      "Built-in ontology and introspection API. AI agents can programmatically discover capabilities and generate code.",
  },
  {
    icon: (
      <svg width="20" height="20" fill="none" stroke="currentColor" strokeWidth="1.5" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z" />
      </svg>
    ),
    title: "Full Extraction",
    description:
      "Text, images, forms, annotations, bookmarks, metadata — everything extractable from a PDF, with positioning data.",
  },
];

const installCode = "npm install agenticpdf";

const quickCode = `import { AgenticPDF } from 'agenticpdf';

const pdf = await AgenticPDF.fromFile(file);
const text = await pdf.extractText({ preserveFormatting: true });
console.log(text[0].text);

// Stream semantic chunks for RAG
for await (const chunk of pdf.streamSemanticChunks()) {
  await vectorStore.add(chunk);
}

pdf.close();`;

const comparisonRows: ({ cap: string; header: true } | { cap: string; a: boolean; b: boolean })[] = [
  { cap: "Agentic AI & LLM Integration", header: true },
  { cap: "Semantic chunking for RAG", a: true, b: false },
  { cap: "Streaming agentic ingestion", a: true, b: false },
  { cap: "JSON tool schemas (MCP/OpenAI)", a: true, b: false },
  { cap: "Agent discovery ontology (JSON-LD)", a: true, b: false },
  { cap: "Embedding provider interface", a: true, b: false },
  { cap: "Token-aware context splitting", a: true, b: false },
  { cap: "Pre-built agent workflow templates", a: true, b: false },
  { cap: "Unified agentic ingestion API", a: true, b: false },
  { cap: "Document type auto-detection", a: true, b: false },
  { cap: "Document Analysis", header: true },
  { cap: "Structural analysis (tables, figures)", a: true, b: false },
  { cap: "Summarization & NER", a: true, b: false },
  { cap: "Form field extraction & filling", a: true, b: false },
  { cap: "Annotation extraction", a: true, b: false },
  { cap: "Rendering & Output", header: true },
  { cap: "Canvas PDF rendering", a: true, b: true },
  { cap: "Text extraction", a: true, b: true },
  { cap: "Multi-format export (HTML, MD, JSON)", a: true, b: false },
  { cap: "Architecture & Developer Experience", header: true },
  { cap: "Streaming-first with progress callbacks", a: true, b: false },
  { cap: "Constant-memory processing mode", a: true, b: false },
  { cap: "AbortSignal cancellation support", a: true, b: false },
  { cap: "Zero runtime dependencies", a: true, b: false },
  { cap: "Single-file deployment", a: true, b: false },
  { cap: "Native TypeScript (no build step)", a: true, b: false },
];

export default function HomePage() {
  let dataIdx = 0;

  return (
    <>
      {/* Hero */}
      <section className="relative overflow-hidden">
        <div className="grid-overlay absolute inset-0" />
        <div
          className="absolute top-0 left-1/2 -translate-x-1/2 w-[600px] h-[400px] rounded-full opacity-30 blur-3xl pointer-events-none"
          style={{ background: "radial-gradient(circle, var(--accent) 0%, transparent 70%)" }}
        />
        <div className="relative mx-auto max-w-7xl px-4 sm:px-6 pt-28 pb-20 text-center">
          <div className="animate-in">
            <div
              className="inline-flex items-center gap-2 px-3 py-1.5 rounded-md text-xs font-mono mb-6"
              style={{
                background: "var(--accent-soft)",
                border: "1px solid var(--border-accent)",
                color: "var(--accent)",
              }}
            >
              <span className="pulse-dot" />
              v1.0.0 — OPERATIONAL
            </div>
            <h1
              className="font-sans text-5xl sm:text-6xl lg:text-7xl font-bold mb-6 leading-[1.1] tracking-tight"
              style={{ color: "var(--text)" }}
            >
              AI-Native PDF
              <br />
              <span style={{ color: "var(--accent)" }}>Processing</span>
            </h1>
            <p
              className="text-lg sm:text-xl max-w-2xl mx-auto mb-10 leading-relaxed"
              style={{ color: "var(--text-muted)" }}
            >
              Streaming-first TypeScript library with semantic chunking, canvas
              rendering, agent discovery, and zero runtime dependencies. Built
              for mission-critical AI applications.
            </p>

            <div className="flex flex-col sm:flex-row items-center justify-center gap-3">
              <Link
                href="/docs/"
                className="px-6 py-3 rounded-md font-mono font-semibold text-sm transition-colors"
                style={{
                  background: "var(--accent)",
                  color: "var(--bg)",
                  boxShadow: "0 0 20px var(--glow)",
                }}
              >
                Get Started
              </Link>
              <Link
                href="/demos/"
                className="px-6 py-3 rounded-md font-mono font-semibold text-sm transition-colors"
                style={{
                  border: "1px solid var(--border)",
                  color: "var(--text)",
                }}
              >
                View Demos
              </Link>
            </div>
          </div>

          {/* Install banner */}
          <div
            className="mt-12 inline-flex items-center gap-3 px-5 py-3 rounded-md font-mono text-sm"
            style={{
              background: "var(--bg-card)",
              border: "1px solid var(--border)",
              color: "var(--text-muted)",
            }}
          >
            <span style={{ color: "var(--accent)" }}>$</span>
            <span style={{ color: "var(--text)" }}>{installCode}</span>
          </div>
        </div>
      </section>

      {/* Quick code preview */}
      <section className="mx-auto max-w-3xl px-4 sm:px-6 pb-20">
        <HighlightedCode code={quickCode} language="typescript" />
      </section>

      {/* Features grid */}
      <section className="mx-auto max-w-7xl px-4 sm:px-6 pb-24">
        <div className="text-center mb-12">
          <span
            className="font-mono text-xs uppercase tracking-[0.2em] mb-3 block"
            style={{ color: "var(--accent)" }}
          >
            CAPABILITIES
          </span>
          <h2 className="font-tactical text-3xl" style={{ color: "var(--text)" }}>
            Everything you need for PDF processing
          </h2>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 stagger-in">
          {features.map((f) => (
            <FeatureCard key={f.title} {...f} />
          ))}
        </div>
      </section>

      {/* Feature comparison */}
      <section className="mx-auto max-w-5xl px-4 sm:px-6 pb-24">
        <div className="text-center mb-10">
          <span
            className="font-mono text-xs uppercase tracking-[0.2em] mb-3 block"
            style={{ color: "var(--accent)" }}
          >
            COMPARISON
          </span>
          <h2 className="font-tactical text-3xl mb-4" style={{ color: "var(--text)" }}>
            AgenticPDF vs Others
          </h2>
          <p style={{ color: "var(--text-muted)" }}>
            Purpose-built for agentic AI workflows — not retrofitted.
          </p>
        </div>
        <div
          className="card-glow rounded-lg overflow-hidden"
          style={{ border: "1px solid var(--border)" }}
        >
          <table className="w-full text-sm" style={{ borderCollapse: "collapse" }}>
            <thead>
              <tr style={{ background: "var(--bg-card)" }}>
                <th
                  className="text-left px-5 py-3 font-mono text-xs uppercase tracking-[0.1em] font-semibold"
                  style={{ color: "var(--text)", borderBottom: "1px solid var(--border)" }}
                >
                  Capability
                </th>
                <th
                  className="text-center px-5 py-3 font-mono text-xs uppercase tracking-[0.1em] font-semibold"
                  style={{ color: "var(--accent)", borderBottom: "1px solid var(--border)" }}
                >
                  AgenticPDF
                </th>
                <th
                  className="text-center px-5 py-3 font-mono text-xs uppercase tracking-[0.1em] font-semibold"
                  style={{ color: "var(--text-muted)", borderBottom: "1px solid var(--border)" }}
                >
                  Others
                </th>
              </tr>
            </thead>
            <tbody>
              {comparisonRows.map((row) => {
                if ("header" in row && row.header) {
                  return (
                    <tr
                      key={row.cap}
                      style={{
                        background: "var(--bg-card)",
                        borderBottom: "1px solid var(--border)",
                      }}
                    >
                      <td
                        colSpan={3}
                        className="px-5 py-2 font-mono text-xs font-semibold uppercase tracking-[0.15em]"
                        style={{ color: "var(--accent)" }}
                      >
                        {row.cap}
                      </td>
                    </tr>
                  );
                }
                const dataRow = row as { cap: string; a: boolean; b: boolean };
                const isEven = dataIdx % 2 === 0;
                dataIdx++;
                return (
                  <tr
                    key={row.cap}
                    style={{
                      background: isEven ? "var(--bg)" : "var(--bg-card)",
                      borderBottom: "1px solid var(--border)",
                    }}
                  >
                    <td className="px-5 py-3" style={{ color: "var(--text)" }}>
                      {row.cap}
                    </td>
                    <td className="text-center px-5 py-3">
                      {dataRow.a ? (
                        <span style={{ color: "var(--success)" }}>&#10003;</span>
                      ) : (
                        <span style={{ color: "var(--text-muted)" }}>—</span>
                      )}
                    </td>
                    <td className="text-center px-5 py-3">
                      {dataRow.b ? (
                        <span style={{ color: "var(--success)" }}>&#10003;</span>
                      ) : (
                        <span style={{ color: "var(--text-muted)" }}>—</span>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </section>

      {/* Demos CTA */}
      <section className="border-y py-20" style={{ borderColor: "var(--border)" }}>
        <div className="mx-auto max-w-4xl px-4 sm:px-6 text-center">
          <span
            className="font-mono text-xs uppercase tracking-[0.2em] mb-3 block"
            style={{ color: "var(--accent)" }}
          >
            LIVE DEMOS
          </span>
          <h2 className="font-tactical text-3xl mb-4" style={{ color: "var(--text)" }}>
            Interactive Demos
          </h2>
          <p className="mb-8 text-sm" style={{ color: "var(--text-muted)" }}>
            See AgenticPDF in action — agentic AI pipelines, real-time
            streaming, PDF rendering, and precision text layout.
          </p>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            {[
              {
                href: "/demos/ai/",
                label: "Agentic AI",
                desc: "RAG pipeline & semantic chunking",
              },
              {
                href: "/demos/streaming/",
                label: "Streaming",
                desc: "Real-time PDF processing",
              },
              {
                href: "/demos/viewer/",
                label: "PDF Viewer",
                desc: "Canvas rendering & themes",
              },
              {
                href: "/demos/pretext/",
                label: "Pretext Layout",
                desc: "Text measurement & line-breaking",
              },
            ].map((d) => (
              <Link
                key={d.href}
                href={d.href}
                className="card-glow rounded-lg p-5 text-left transition-colors group"
                style={{
                  background: "var(--bg-card)",
                  border: "1px solid var(--border)",
                }}
              >
                <span className="font-mono text-xs uppercase tracking-wider block mb-2" style={{ color: "var(--accent)" }}>
                  {d.label}
                </span>
                <p className="text-sm" style={{ color: "var(--text-muted)" }}>
                  {d.desc}
                </p>
                <span
                  className="inline-block mt-3 text-xs font-mono transition-transform group-hover:translate-x-1"
                  style={{ color: "var(--accent)" }}
                >
                  Explore →
                </span>
              </Link>
            ))}
          </div>
        </div>
      </section>

      {/* Stats */}
      <section className="mx-auto max-w-5xl px-4 sm:px-6 py-20">
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          {[
            { value: "950", label: "Tests Passing" },
            { value: "0", label: "Runtime Deps" },
            { value: "100%", label: "TypeScript" },
            { value: "21K+", label: "Lines of Code" },
          ].map((s) => (
            <div
              key={s.label}
              className="card-glow rounded-lg p-6 text-center"
              style={{ border: "1px solid var(--border)", background: "var(--bg-card)" }}
            >
              <div className="font-tactical text-3xl" style={{ color: "var(--accent)" }}>
                {s.value}
              </div>
              <div
                className="font-mono text-xs uppercase tracking-wider mt-1"
                style={{ color: "var(--text-muted)" }}
              >
                {s.label}
              </div>
            </div>
          ))}
        </div>
      </section>
    </>
  );
}
