import { HighlightedCode } from "@/components/highlighted-code";

export default function CliPage() {
  return (
    <div className="mx-auto max-w-4xl px-4 sm:px-6 py-12">
      <h1 className="text-4xl font-bold tracking-tight mb-2" style={{ color: "var(--text)" }}>
        CLI Reference
      </h1>
      <p className="mb-10" style={{ color: "var(--text-muted)" }}>
        AgenticPDF includes a full-featured command-line interface for PDF processing.
      </p>

      {/* Installation */}
      <section className="mb-10">
        <h2 className="text-2xl font-bold mb-4" style={{ color: "var(--text)" }}>Installation</h2>
        <HighlightedCode code={`# Global install
npm install -g agenticpdf

# Or use npx
npx agenticpdf --help`} language="bash" />
      </section>

      {/* Commands */}
      <section className="mb-10">
        <h2 className="text-2xl font-bold mb-6 pb-2 border-b" style={{ color: "var(--text)", borderColor: "var(--border)" }}>
          Commands
        </h2>

        {[
          {
            name: "info",
            desc: "Display PDF metadata and document information.",
            example: `agenticpdf info -i document.pdf\nagenticpdf info -i document.pdf --pretty`,
          },
          {
            name: "extract",
            desc: "Extract text content from a PDF.",
            example: `agenticpdf extract -i document.pdf\nagenticpdf extract -i document.pdf -p 1-5 -o output.txt\nagenticpdf extract -i document.pdf --stream`,
          },
          {
            name: "convert",
            desc: "Convert a PDF to text, JSON, HTML, or Markdown.",
            example: `agenticpdf convert -i document.pdf -f markdown -o output.md\nagenticpdf convert -i document.pdf -f json --pretty\nagenticpdf convert -i document.pdf -f html -o output.html`,
          },
          {
            name: "analyze",
            desc: "Run AI-powered analysis including structural analysis, NER, and summarization.",
            example: `agenticpdf analyze -i document.pdf\nagenticpdf analyze -i document.pdf --ai`,
          },
          {
            name: "chunk",
            desc: "Generate semantic chunks for RAG pipelines.",
            example: `agenticpdf chunk -i document.pdf\nagenticpdf chunk -i document.pdf --chunk-size 1000 -o chunks.json`,
          },
          {
            name: "images",
            desc: "Extract embedded images from a PDF.",
            example: `agenticpdf images -i document.pdf -o ./images/`,
          },
          {
            name: "forms",
            desc: "Extract form fields and their values.",
            example: `agenticpdf forms -i document.pdf`,
          },
          {
            name: "ingest",
            desc: "Unified agentic ingestion — metadata, structure, and semantic chunks in one call.",
            example: `agenticpdf ingest -i document.pdf -o result.json\nagenticpdf ingest -i document.pdf --ndjson\nagenticpdf ingest -i document.pdf --include-text -o full.json`,
          },
          {
            name: "tool-schema",
            desc: "Output JSON tool/function schemas for LLM agents.",
            example: `agenticpdf tool-schema --tool-schema openai\nagenticpdf tool-schema --tool-schema anthropic\nagenticpdf tool-schema --tool-schema mcp`,
          },
          {
            name: "generate",
            desc: "Generate an aPDF binary file (PDF + metadata envelope).",
            example: `agenticpdf generate -i paper.pdf -o paper.apdf`,
          },
          {
            name: "typeset",
            desc: "Typeset PDF for web display with themed HTML output.",
            example: `agenticpdf typeset -i document.pdf -o output.html`,
          },
        ].map((cmd) => (
          <div key={cmd.name} className="mb-8">
            <h3 className="text-lg font-semibold mb-2" style={{ color: "var(--text)" }}>
              <code style={{ color: "var(--accent)" }}>{cmd.name}</code>
            </h3>
            <p className="text-sm mb-3" style={{ color: "var(--text-muted)" }}>{cmd.desc}</p>
            <HighlightedCode code={cmd.example} language="bash" />
          </div>
        ))}
      </section>

      {/* Global Options */}
      <section className="mb-10">
        <h2 className="text-2xl font-bold mb-4 pb-2 border-b" style={{ color: "var(--text)", borderColor: "var(--border)" }}>
          Global Options
        </h2>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr style={{ borderBottom: "1px solid var(--border)" }}>
                <th className="text-left py-2 pr-4 font-semibold" style={{ color: "var(--text)" }}>Flag</th>
                <th className="text-left py-2 pr-4 font-semibold" style={{ color: "var(--text)" }}>Description</th>
              </tr>
            </thead>
            <tbody style={{ color: "var(--text-muted)" }}>
              {[
                ["-i, --input <file>", "Input PDF file path"],
                ["-o, --output <path>", "Output file or directory"],
                ["-p, --pages <range>", "Page range (e.g., 1-5, 1,3,5)"],
                ["-f, --format <fmt>", "Output format (text, json, html, markdown)"],
                ["-v, --verbose", "Enable verbose output"],
                ["--pretty", "Pretty-print JSON output"],
                ["--stream", "Enable streaming mode for large files"],
                ["--ndjson", "Output newline-delimited JSON (ingest command)"],
                ["--include-text", "Include full text in ingestion output"],
                ["--tool-schema <fmt>", "Tool schema format: openai, anthropic, generic, mcp"],
                ["--metadata", "Include metadata in output"],
              ].map(([flag, desc]) => (
                <tr key={flag} style={{ borderBottom: "1px solid var(--border)" }}>
                  <td className="py-2 pr-4"><code style={{ color: "var(--accent)" }}>{flag}</code></td>
                  <td className="py-2">{desc}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* Examples */}
      <section>
        <h2 className="text-2xl font-bold mb-4 pb-2 border-b" style={{ color: "var(--text)", borderColor: "var(--border)" }}>
          Workflow Examples
        </h2>
        <HighlightedCode code={`# Extract text, then analyze with AI
agenticpdf extract -i paper.pdf -o text.txt
agenticpdf analyze -i paper.pdf --ai

# Batch convert directory
for f in *.pdf; do
  agenticpdf convert -i "$f" -f markdown -o "output/$(basename "$f" .pdf).md"
done

# Stream large document to chunks
agenticpdf chunk -i large-doc.pdf --stream --chunk-size 1500 -o chunks.json

# Quick info + images in one pass  
agenticpdf info -i report.pdf --pretty
agenticpdf images -i report.pdf -o ./report-images/`} language="bash" />
      </section>
    </div>
  );
}
