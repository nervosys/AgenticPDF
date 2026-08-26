"use client";

import { useState, useCallback } from "react";
import { DemoShell, CodeBlock, ConsolePanel, Tabs } from "@/components/ui";

/* ── Simulated AI pipeline data — based on Shannon (1948) ── */
const sampleChunks = [
  {
    id: "chunk-1",
    type: "Header",
    content: "A Mathematical Theory of Communication — Claude E. Shannon",
    pageNumbers: [1],
    metadata: { confidence: 0.99, importance: 0.97, tokenCount: 9, keywords: ["information theory", "communication", "Shannon"] },
  },
  {
    id: "chunk-2",
    type: "Paragraph",
    content:
      "The fundamental problem of communication is that of reproducing at one point either exactly or approximately a message selected at another point. Frequently the messages have meaning; that is they refer to or are correlated according to some system with certain physical or conceptual entities.",
    pageNumbers: [1, 2],
    metadata: { confidence: 0.96, importance: 0.93, tokenCount: 48, keywords: ["communication", "message", "information", "meaning"] },
  },
  {
    id: "chunk-3",
    type: "Paragraph",
    content:
      "If the number of messages in the set is finite then this number or any monotonic function of this number can be regarded as a measure of the information produced when one message is chosen from the set, all choices being equally likely. The logarithmic measure is more convenient for various reasons.",
    pageNumbers: [3, 4],
    metadata: { confidence: 0.95, importance: 0.90, tokenCount: 52, keywords: ["entropy", "logarithm", "information measure", "probability"] },
  },
  {
    id: "chunk-4",
    type: "Equation",
    content:
      "H = −Σ pᵢ log pᵢ\n\nThe quantity H has a number of interesting properties which further substantiate it as a reasonable measure of choice or information: H = 0 if and only if all the pᵢ but one are zero. This is the condition of no uncertainty.",
    pageNumbers: [10, 11],
    metadata: { confidence: 0.98, importance: 0.96, tokenCount: 44, keywords: ["entropy formula", "H", "probability", "uncertainty"] },
  },
  {
    id: "chunk-5",
    type: "Paragraph",
    content:
      "The fundamental theorem for a noiseless channel demonstrates that the capacity C of such a channel equals the logarithm of the largest real root of the determinant equation. For a noisy channel, the rate of transmission R satisfies R ≤ C where C = max(H(x) − Hy(x)).",
    pageNumbers: [18, 19],
    metadata: { confidence: 0.97, importance: 0.95, tokenCount: 50, keywords: ["channel capacity", "noisy channel", "fundamental theorem", "transmission rate"] },
  },
];

const sampleAnalysis = {
  documentType: "academic_paper",
  sections: [
    { title: "Introduction", pageRange: "1-3", level: 1 },
    { title: "Discrete Noiseless Systems", pageRange: "3-14", level: 1 },
    { title: "The Discrete Channel with Noise", pageRange: "14-24", level: 1 },
    { title: "The Continuous Channel", pageRange: "24-45", level: 1 },
    { title: "The Rate for a Continuous Source", pageRange: "45-55", level: 1 },
  ],
  entities: [
    { text: "Claude E. Shannon", type: "Person", count: 2 },
    { text: "Bell Telephone Laboratories", type: "Organization", count: 1 },
    { text: "Nyquist", type: "Person", count: 3 },
    { text: "Hartley", type: "Person", count: 2 },
    { text: "Boltzmann", type: "Person", count: 1 },
  ],
  keywords: ["information theory", "entropy", "channel capacity", "noisy channel", "Markov process", "coding theorem"],
  summary:
    "Shannon's foundational 1948 paper establishes the mathematical framework for information theory, defining entropy as a measure of information, proving the noisy channel coding theorem, and deriving fundamental limits on data compression and reliable communication.",
};

export default function AiDemoPage() {
  const [logs, setLogs] = useState<string[]>([]);
  const [chunks, setChunks] = useState<typeof sampleChunks>([]);
  const [analysis, setAnalysis] = useState<typeof sampleAnalysis | null>(null);
  const [running, setRunning] = useState(false);
  const [activeChunk, setActiveChunk] = useState<string | null>(null);

  const log = useCallback((msg: string) => {
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);
  }, []);

  const runPipeline = useCallback(async () => {
    setRunning(true);
    setChunks([]);
    setAnalysis(null);
    setLogs([]);
    setActiveChunk(null);

    log("Loading shannon1948.pdf...");
    await delay(600);
    log("✓ Document loaded — 55 pages, 360 KB — \"A Mathematical Theory of Communication\"");

    log("Running structural analysis...");
    await delay(800);
    setAnalysis(sampleAnalysis);
    log(`✓ Detected document type: ${sampleAnalysis.documentType}`);
    log(`✓ Found ${sampleAnalysis.sections.length} sections, ${sampleAnalysis.entities.length} entities`);

    log("Generating semantic chunks (strategy: semantic, maxSize: 1000)...");
    for (let i = 0; i < sampleChunks.length; i++) {
      await delay(400 + Math.random() * 300);
      const chunk = sampleChunks[i];
      setChunks((prev) => [...prev, chunk]);
      log(
        `✓ Chunk ${i + 1}/${sampleChunks.length} [${chunk.type}] — ${chunk.metadata.tokenCount} tokens, confidence: ${chunk.metadata.confidence}`
      );
    }

    log("Generating embeddings (mock: 1536 dimensions)...");
    await delay(500);
    log("✓ Embeddings generated for all 5 chunks");
    log("✓ Pipeline complete — 5 chunks ready for vector store");
    setRunning(false);
  }, [log]);

  return (
    <DemoShell
      title="Agentic AI Demo"
      description="Interactive demonstration of AgenticPDF's AI features: semantic chunking, structural analysis, entity extraction, and RAG pipeline preparation."
    >
      {/* Controls */}
      <div className="flex gap-3 mb-8">
        <button
          onClick={runPipeline}
          disabled={running}
          className="px-5 py-2.5 rounded-lg text-sm font-semibold text-white transition-colors disabled:opacity-50"
          style={{ background: "var(--accent)" }}
        >
          {running ? "Processing..." : "▶ Run AI Pipeline"}
        </button>
        <button
          onClick={() => { setLogs([]); setChunks([]); setAnalysis(null); setActiveChunk(null); }}
          className="px-4 py-2.5 rounded-lg text-sm font-medium transition-colors"
          style={{ border: "1px solid var(--border)", color: "var(--text-muted)" }}
        >
          Clear
        </button>
      </div>

      <Tabs
        tabs={[
          {
            label: "Pipeline Output",
            content: (
              <div className="space-y-6">
                <ConsolePanel lines={logs} />

                {/* Chunks display */}
                {chunks.length > 0 && (
                  <div>
                    <h3 className="font-semibold mb-3" style={{ color: "var(--text)" }}>
                      Semantic Chunks ({chunks.length})
                    </h3>
                    <div className="space-y-3">
                      {chunks.map((c) => (
                        <div
                          key={c.id}
                          className="rounded-lg p-4 cursor-pointer transition-colors"
                          onClick={() => setActiveChunk(activeChunk === c.id ? null : c.id)}
                          style={{
                            background: activeChunk === c.id ? "var(--accent-soft)" : "var(--bg-card)",
                            border: `1px solid ${activeChunk === c.id ? "var(--accent)" : "var(--border)"}`,
                          }}
                        >
                          <div className="flex items-center justify-between mb-2">
                            <div className="flex items-center gap-2">
                              <span
                                className="text-xs font-medium px-2 py-0.5 rounded"
                                style={{ background: "var(--accent-soft)", color: "var(--accent)" }}
                              >
                                {c.type}
                              </span>
                              <span className="text-xs" style={{ color: "var(--text-muted)" }}>
                                Pages {c.pageNumbers.join(", ")}
                              </span>
                            </div>
                            <span className="text-xs font-mono" style={{ color: "var(--text-muted)" }}>
                              {c.metadata.tokenCount} tokens
                            </span>
                          </div>
                          <p className="text-sm" style={{ color: "var(--text)" }}>
                            {c.content.slice(0, 120)}{c.content.length > 120 ? "..." : ""}
                          </p>
                          {activeChunk === c.id && (
                            <div className="mt-3 pt-3 border-t text-xs space-y-1" style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}>
                              <div>Confidence: {c.metadata.confidence}</div>
                              <div>Importance: {c.metadata.importance}</div>
                              <div>Keywords: {c.metadata.keywords.join(", ")}</div>
                              <div className="font-mono pt-1" style={{ color: "var(--text)" }}>
                                {c.content}
                              </div>
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {/* Analysis results */}
                {analysis && (
                  <div>
                    <h3 className="font-semibold mb-3" style={{ color: "var(--text)" }}>
                      Structural Analysis
                    </h3>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                      <div className="rounded-lg p-4" style={{ background: "var(--bg-card)", border: "1px solid var(--border)" }}>
                        <h4 className="text-sm font-semibold mb-2" style={{ color: "var(--text)" }}>Sections</h4>
                        {analysis.sections.map((s) => (
                          <div key={s.title} className="text-sm py-1" style={{ color: "var(--text-muted)", paddingLeft: `${(s.level - 1) * 16}px` }}>
                            {s.title} <span className="text-xs">({s.pageRange})</span>
                          </div>
                        ))}
                      </div>
                      <div className="rounded-lg p-4" style={{ background: "var(--bg-card)", border: "1px solid var(--border)" }}>
                        <h4 className="text-sm font-semibold mb-2" style={{ color: "var(--text)" }}>Named Entities</h4>
                        {analysis.entities.map((e) => (
                          <div key={e.text} className="text-sm py-1 flex justify-between" style={{ color: "var(--text-muted)" }}>
                            <span>{e.text} <span className="text-xs">({e.type})</span></span>
                            <span className="text-xs">×{e.count}</span>
                          </div>
                        ))}
                      </div>
                    </div>
                    <div className="rounded-lg p-4 mt-4" style={{ background: "var(--bg-card)", border: "1px solid var(--border)" }}>
                      <h4 className="text-sm font-semibold mb-2" style={{ color: "var(--text)" }}>Summary</h4>
                      <p className="text-sm" style={{ color: "var(--text-muted)" }}>{analysis.summary}</p>
                    </div>
                  </div>
                )}
              </div>
            ),
          },
          {
            label: "Source Code",
            content: (
              <CodeBlock
                filename="ai-pipeline.ts"
                code={`import { AgenticPDF, EmbeddingProvider } from 'agenticpdf';

// 1. Load Shannon's 1948 paper
const pdf = await AgenticPDF.fromUrl('/shannon1948.pdf', {
  lazyLoad: true,
  maxMemoryUsage: 100 * 1024 * 1024
});

// 2. Run AI analysis
const ai = await pdf.getAIFeatures({
  enableStructuralAnalysis: true,
  enableSemanticChunking: true,
  enableNER: true,
  enableSummarization: true,
  chunkSize: 1000
});

console.log('Type:', ai.structuralAnalysis.documentType);
console.log('Sections:', ai.structuralAnalysis.sections);
console.log('Entities:', ai.nlpReady.keywords);
console.log('Summary:', ai.nlpReady.summary);

// 3. Stream semantic chunks to vector store
for await (const chunk of pdf.streamSemanticChunks({
  strategy: 'semantic',
  maxChunkSize: 1000,
  preserveParagraphs: true
})) {
  const embedding = await embedModel.generate(chunk.content);
  await vectorStore.upsert({
    id: chunk.id,
    vector: Array.from(embedding),
    content: chunk.content,
    metadata: {
      pages: chunk.pageNumbers,
      type: chunk.type,
      keywords: chunk.metadata.keywords
    }
  });
}

pdf.close();`}
              />
            ),
          },
        ]}
      />
    </DemoShell>
  );
}

function delay(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}
