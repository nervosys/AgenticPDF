import Link from "next/link";

const demos = [
  {
    href: "/demos/ai/",
    title: "Agentic AI",
    desc: "Interactive RAG pipeline demo with semantic chunking, document analysis, entity extraction, and AI feature exploration.",
    tags: ["RAG", "Semantic Chunking", "NER", "Embeddings"],
  },
  {
    href: "/demos/streaming/",
    title: "Streaming",
    desc: "Real-time PDF processing with async generators. Watch text extraction, chunking, and export happen page-by-page.",
    tags: ["AsyncGenerator", "Progress", "Memory-Efficient", "AbortSignal"],
  },
  {
    href: "/demos/viewer/",
    title: "PDF Viewer",
    desc: "Full-featured canvas-based PDF viewer with dark/light themes, zoom, continuous scrolling, and text selection.",
    tags: ["Canvas", "Themes", "High DPI", "Text Layer"],
  },
  {
    href: "/demos/pretext/",
    title: "Pretext Layout",
    desc: "Precision text measurement and line-breaking engine. See word-wrap, CJK support, and whitespace modes in real-time.",
    tags: ["Typography", "Line Breaking", "CJK", "Measurement"],
  },
];

export default function DemosPage() {
  return (
    <div className="mx-auto max-w-5xl px-4 sm:px-6 py-12">
      <h1 className="text-4xl font-bold tracking-tight mb-2" style={{ color: "var(--text)" }}>
        Interactive Demos
      </h1>
      <p className="mb-10" style={{ color: "var(--text-muted)" }}>
        Explore AgenticPDF capabilities with live, interactive demonstrations — powered by Shannon&apos;s
        1948 paper <em>&quot;A Mathematical Theory of Communication.&quot;</em>
      </p>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
        {demos.map((d) => (
          <Link
            key={d.href}
            href={d.href}
            className="rounded-xl p-6 transition-all group"
            style={{
              background: "var(--bg-card)",
              border: "1px solid var(--border)",
            }}
          >
            <h2
              className="text-xl font-bold mb-2 group-hover:underline"
              style={{ color: "var(--accent)" }}
            >
              {d.title} →
            </h2>
            <p className="text-sm mb-4" style={{ color: "var(--text-muted)" }}>
              {d.desc}
            </p>
            <div className="flex flex-wrap gap-2">
              {d.tags.map((t) => (
                <span
                  key={t}
                  className="px-2 py-0.5 rounded text-xs font-medium"
                  style={{
                    background: "var(--accent-soft)",
                    color: "var(--accent)",
                  }}
                >
                  {t}
                </span>
              ))}
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}
