"use client";

import { useState, useCallback, useRef } from "react";
import { DemoShell, CodeBlock, ConsolePanel, Tabs } from "@/components/ui";

const samplePages = [
  { page: 1, chars: 2840, text: "A Mathematical Theory of Communication — Claude E. Shannon\n\nThe recent development of various methods of modulation such as PCM and PPM which exchange bandwidth for signal-to-noise ratio has intensified the interest in a general theory of communication..." },
  { page: 2, chars: 3510, text: "The fundamental problem of communication is that of reproducing at one point either exactly or approximately a message selected at another point. Frequently the messages have meaning..." },
  { page: 3, chars: 3250, text: "Part I: Discrete Noiseless Systems\n\n1. The Discrete Noiseless Channel. We consider a class of channels with the following properties..." },
  { page: 4, chars: 3100, text: "If we have a set of possible events whose probabilities of occurrence are p₁, p₂, … , pₙ, is there a measure of how much 'choice' is involved in the selection of the event..." },
  { page: 5, chars: 3620, text: "Theorem 1: Let H(p₁, p₂, … , pₙ) = −Σ pᵢ log pᵢ. Then H is the only function satisfying the three conditions above. This theorem ensures entropy is the unique information measure..." },
  { page: 6, chars: 3400, text: "The Ergodic property guarantees that sufficiently long sequences of the process will be close in statistical properties to the ensemble average. For any ε > 0..." },
  { page: 7, chars: 3900, text: "Part II: The Discrete Channel with Noise\n\nThe system considered may be represented as a noisy channel connected to a source and destination through a transmitter and receiver..." },
  { page: 8, chars: 4100, text: "Fundamental Theorem: Let a discrete channel have the capacity C and a discrete source the entropy per second H. If H ≤ C there exists a coding system such that the source output can be transmitted with an arbitrarily small frequency of errors..." },
  { page: 9, chars: 3750, text: "Part III: Mathematical Preliminaries for the Continuous Case\n\nThe concepts of information theory extend naturally to continuous distributions. We define the entropy of a continuous distribution..." },
  { page: 10, chars: 2800, text: "The capacity of a continuous channel subject to additive white Gaussian noise of power N, with signal power limited to P, and bandwidth W is: C = W log₂(1 + P/N). This is the Shannon-Hartley theorem..." },
];

export default function StreamingDemoPage() {
  const [logs, setLogs] = useState<string[]>([]);
  const [pages, setPages] = useState<typeof samplePages>([]);
  const [progress, setProgress] = useState(0);
  const [running, setRunning] = useState(false);
  const [mode, setMode] = useState<"text" | "chunks" | "export">("text");
  const abortRef = useRef<AbortController | null>(null);

  const log = useCallback((msg: string) => {
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);
  }, []);

  const abort = useCallback(() => {
    abortRef.current?.abort();
    abortRef.current = null;
  }, []);

  const runStream = useCallback(async () => {
    setRunning(true);
    setPages([]);
    setLogs([]);
    setProgress(0);
    const controller = new AbortController();
    abortRef.current = controller;

    const total = samplePages.length;
    log(`Starting ${mode} streaming (${total} pages)...`);

    if (mode === "text") {
      log("pdf.streamText({ normalizeWhitespace: true })");
      let totalChars = 0;
      for (let i = 0; i < total; i++) {
        if (controller.signal.aborted) { log("⚠ Aborted by user"); break; }
        await delay(300 + Math.random() * 200);
        const pg = samplePages[i];
        totalChars += pg.chars;
        setPages((prev) => [...prev, pg]);
        setProgress(((i + 1) / total) * 100);
        log(`✓ Page ${pg.page}: ${pg.chars} chars extracted`);
      }
      if (!controller.signal.aborted) {
        log(`✓ Stream complete — ${totalChars.toLocaleString()} total characters`);
      }
    } else if (mode === "chunks") {
      log("pdf.streamSemanticChunks({ strategy: 'semantic', maxChunkSize: 1000 })");
      let chunkId = 0;
      for (let i = 0; i < total; i++) {
        if (controller.signal.aborted) { log("⚠ Aborted by user"); break; }
        await delay(400 + Math.random() * 300);
        chunkId++;
        const pg = samplePages[i];
        setPages((prev) => [...prev, pg]);
        setProgress(((i + 1) / total) * 100);
        log(`✓ Chunk ${chunkId}: pages [${pg.page}] — ${Math.round(pg.chars * 0.27)} tokens`);
      }
      if (!controller.signal.aborted) {
        log(`✓ Chunking complete — ${chunkId} semantic chunks generated`);
      }
    } else {
      log("pdf.exportAs('markdown', { includeImages: true })");
      for (let i = 0; i < total; i++) {
        if (controller.signal.aborted) { log("⚠ Aborted by user"); break; }
        await delay(250 + Math.random() * 150);
        const pg = samplePages[i];
        setPages((prev) => [...prev, pg]);
        setProgress(((i + 1) / total) * 100);
        log(`✓ Page ${pg.page} exported to Markdown`);
      }
      if (!controller.signal.aborted) {
        log("✓ Export complete — output.md ready");
      }
    }

    setRunning(false);
    abortRef.current = null;
  }, [mode, log]);

  return (
    <DemoShell
      title="Streaming Demo"
      description="Watch AgenticPDF process Shannon's 1948 paper in real-time using async generators. Supports abort signals for cancellation."
    >
      {/* Controls */}
      <div className="flex flex-wrap gap-3 mb-8">
        {(["text", "chunks", "export"] as const).map((m) => (
          <button
            key={m}
            onClick={() => setMode(m)}
            disabled={running}
            className="px-4 py-2 rounded-lg text-sm font-medium transition-colors"
            style={{
              background: mode === m ? "var(--accent-soft)" : "transparent",
              color: mode === m ? "var(--accent)" : "var(--text-muted)",
              border: `1px solid ${mode === m ? "var(--accent)" : "var(--border)"}`,
            }}
          >
            {m === "text" ? "Stream Text" : m === "chunks" ? "Stream Chunks" : "Stream Export"}
          </button>
        ))}
        <button
          onClick={running ? abort : runStream}
          className="px-5 py-2 rounded-lg text-sm font-semibold text-white transition-colors"
          style={{ background: running ? "#ef4444" : "var(--accent)" }}
        >
          {running ? "⏹ Abort" : "▶ Start Stream"}
        </button>
      </div>

      {/* Progress bar */}
      <div className="mb-6 rounded-full h-2 overflow-hidden" style={{ background: "var(--bg-card)" }}>
        <div
          className="h-full rounded-full transition-all duration-300"
          style={{ width: `${progress}%`, background: "var(--accent)" }}
        />
      </div>
      <div className="text-xs mb-6 font-mono" style={{ color: "var(--text-muted)" }}>
        {Math.round(progress)}% — {pages.length}/{samplePages.length} pages
      </div>

      <Tabs
        tabs={[
          {
            label: "Live Output",
            content: (
              <div className="space-y-4">
                <ConsolePanel lines={logs} />
                {pages.length > 0 && (
                  <div className="space-y-2">
                    {pages.map((pg) => (
                      <div
                        key={pg.page}
                        className="rounded-lg p-3 animate-in"
                        style={{ background: "var(--bg-card)", border: "1px solid var(--border)" }}
                      >
                        <div className="flex justify-between items-center mb-1">
                          <span className="text-xs font-semibold" style={{ color: "var(--accent)" }}>
                            Page {pg.page}
                          </span>
                          <span className="text-xs font-mono" style={{ color: "var(--text-muted)" }}>
                            {pg.chars} chars
                          </span>
                        </div>
                        <p className="text-xs font-mono truncate" style={{ color: "var(--text-muted)" }}>
                          {pg.text.slice(0, 100)}...
                        </p>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ),
          },
          {
            label: "Source Code",
            content: (
              <CodeBlock
                filename="streaming.ts"
                code={`import { AgenticPDF } from 'agenticpdf';

const pdf = await AgenticPDF.fromUrl('/shannon1948.pdf', {
  lazyLoad: true,
  streamOptions: {
    chunkSize: 1024 * 1024,
    progressCallback: (p) => {
      updateProgress(p.bytesRead / p.totalBytes);
    },
    abortSignal: abortController.signal
  }
});

// Stream text
for await (const page of pdf.streamText({
  normalizeWhitespace: true
})) {
  console.log(\`Page \${page.pageNumber}: \${page.text.length} chars\`);
}

// Stream semantic chunks
for await (const chunk of pdf.streamSemanticChunks({
  strategy: 'semantic',
  maxChunkSize: 1000
})) {
  await vectorStore.add(chunk);
}

// Stream export
const markdown = await pdf.exportAs('markdown', {
  includeImages: true,
  imageFormat: 'webp'
});

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
