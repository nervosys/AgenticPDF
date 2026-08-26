"use client";

import { useState, useRef, useEffect } from "react";
import { DemoShell, CodeBlock, Tabs } from "@/components/ui";

/* eslint-disable @typescript-eslint/no-explicit-any */
declare global {
  interface Window {
    AgenticPDF: any;
  }
}

export default function ViewerDemoPage() {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [totalPages, setTotalPages] = useState(0);
  const [zoom, setZoom] = useState(1);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");
  const [errorMsg, setErrorMsg] = useState("");
  const [metrics, setMetrics] = useState({ renderMs: 0, fileKB: 0 });
  const [showMeta, setShowMeta] = useState(false);
  const [metadata, setMetadata] = useState<Record<string, string>>({});
  const [extraction, setExtraction] = useState<Record<string, string>>({});
  const [aiData, setAiData] = useState<Record<string, string>>({});
  const [sampleText, setSampleText] = useState("");
  const [processing, setProcessing] = useState(false);
  const processedRef = useRef(false);
  const pdfRef = useRef<any>(null);
  const genRef = useRef(0); // render generation to cancel stale renders
  const canvasInfoRef = useRef<{ canvas: HTMLCanvasElement; pageW: number; pageH: number }[]>([]);
  const fitZoomRef = useRef(1); // fit-to-width zoom stored for initial sizing
  const viewerRef = useRef<HTMLDivElement>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);

  /* ── Load library + PDF (once) ── */
  useEffect(() => {
    let cancelled = false;

    function loadScript(src: string): Promise<void> {
      if (document.querySelector(`script[src="${src}"]`)) return Promise.resolve();
      return new Promise((resolve, reject) => {
        const s = document.createElement("script");
        s.src = src;
        s.onload = () => resolve();
        s.onerror = () => reject(new Error(`Failed to load ${src}`));
        document.head.appendChild(s);
      });
    }

    (async () => {
      try {
        // Fetch script and PDF in parallel
        const [, resp] = await Promise.all([
          loadScript("/agenticpdf-browser.js"),
          fetch("/shannon1948.pdf"),
        ]);
        if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
        const buf = await resp.arrayBuffer();
        if (cancelled) return;

        const pdf = await window.AgenticPDF.fromBuffer(buf, { lazyLoad: false });
        if (cancelled) { pdf.close(); return; }

        pdfRef.current = pdf;
        const meta = pdf.getMetadata();
        setTotalPages(meta?.pageCount || 1);
        setMetrics((m) => ({ ...m, fileKB: Math.round(buf.byteLength / 1024) }));

        // Extract metadata table
        if (meta) {
          const page1 = await pdf.getPage(1);
          const entries: Record<string, string> = {};
          if (meta.title) entries["Title"] = meta.title;
          if (meta.author) entries["Author"] = meta.author;
          if (meta.subject) entries["Subject"] = meta.subject;
          if (meta.keywords) entries["Keywords"] = meta.keywords;
          if (meta.creator) entries["Creator"] = meta.creator;
          if (meta.producer) entries["Producer"] = meta.producer;
          entries["PDF Version"] = meta.version || "—";
          entries["Pages"] = String(meta.pageCount);
          if (page1) entries["Page Size"] = `${page1.width} × ${page1.height} pt`;
          entries["File Size"] = `${Math.round(buf.byteLength / 1024)} KB`;
          entries["Encrypted"] = meta.isEncrypted ? "Yes" : "No";
          entries["Linearized"] = meta.isLinearized ? "Yes" : "No";
          if (meta.creationDate) entries["Created"] = new Date(meta.creationDate).toLocaleDateString();
          if (meta.modificationDate) entries["Modified"] = new Date(meta.modificationDate).toLocaleDateString();
          setMetadata(entries);
        }

        setStatus("ready");
      } catch (e: any) {
        if (!cancelled) { setStatus("error"); setErrorMsg(e?.message || String(e)); }
      }
    })();

    return () => { cancelled = true; };
  }, []);

  /* ── Render all pages once at high resolution ── */
  useEffect(() => {
    const container = scrollRef.current;
    const pdf = pdfRef.current;
    if (!container || !pdf || status !== "ready") return;

    const gen = ++genRef.current;
    let cancelled = false;

    (async () => {
      const meta = pdf.getMetadata();
      const pages = meta?.pageCount || 1;
      const renderScale = 3; // 3x backing — crisp at common zooms, smooth CSS scaling beyond
      let totalMs = 0;
      const infos: typeof canvasInfoRef.current = [];

      // Clear container
      while (container.firstChild) container.removeChild(container.firstChild);

      // Compute fit-to-width from first page
      const page1 = await pdf.getPage(1);
      if (page1) {
        const availW = container.clientWidth;
        fitZoomRef.current = +(Math.min(5, Math.max(0.5, availW / page1.width)).toFixed(2));
        setZoom(fitZoomRef.current);
      }

      for (let pageNum = 1; pageNum <= pages; pageNum++) {
        if (cancelled || gen !== genRef.current) return;

        const page = await pdf.getPage(pageNum);
        if (!page) continue;

        const canvas = document.createElement("canvas");

        const t0 = performance.now();
        try {
          await pdf.renderPage(pageNum, canvas, {
            scale: 1,
            renderScale,
            background: "white",
          });
        } catch (err: any) {
          const w = Math.round(page.width * renderScale);
          const h = Math.round(page.height * renderScale);
          canvas.width = w;
          canvas.height = h;
          const ctx = canvas.getContext("2d");
          if (ctx) {
            ctx.fillStyle = "#fff";
            ctx.fillRect(0, 0, w, h);
            ctx.fillStyle = "#ef4444";
            ctx.font = `${14 * renderScale}px system-ui`;
            ctx.textAlign = "center";
            ctx.fillText(`Page ${pageNum} — ${err?.message || "render error"}`, w / 2, h / 2);
          }
        }
        totalMs += performance.now() - t0;

        if (cancelled || gen !== genRef.current) return;

        const curZoom = fitZoomRef.current;
        canvas.style.width = `${page.width * curZoom}px`;
        canvas.style.height = `${page.height * curZoom}px`;

        infos.push({ canvas, pageW: page.width, pageH: page.height });

        const wrapper = document.createElement("div");
        wrapper.style.cssText = "margin-bottom:2px;";
        wrapper.appendChild(canvas);
        container.appendChild(wrapper);

        setMetrics((m) => ({ ...m, renderMs: Math.round(totalMs) }));

        // Yield to browser so each page paints immediately
        await new Promise((r) => requestAnimationFrame(r));
      }

      canvasInfoRef.current = infos;
      if (gen === genRef.current) {
        setMetrics((m) => ({ ...m, renderMs: Math.round(totalMs) }));
      }
    })();

    return () => { cancelled = true; };
  }, [status]);

  /* ── Instant CSS-only zoom ── */
  useEffect(() => {
    for (const info of canvasInfoRef.current) {
      info.canvas.style.width = `${info.pageW * zoom}px`;
      info.canvas.style.height = `${info.pageH * zoom}px`;
    }
  }, [zoom]);

  /* ── Recompute fit-to-width when sidebar toggles ── */
  useEffect(() => {
    const container = scrollRef.current;
    if (!container || canvasInfoRef.current.length === 0) return;
    // Wait one frame for layout to settle after sidebar show/hide
    requestAnimationFrame(() => {
      const availW = container.clientWidth;
      const pageW = canvasInfoRef.current[0].pageW;
      if (pageW > 0) {
        const newFit = +(Math.min(5, Math.max(0.5, availW / pageW)).toFixed(2));
        fitZoomRef.current = newFit;
        setZoom(newFit);
      }
    });
  }, [showMeta]);

  /* ── Track current page from scroll ── */
  useEffect(() => {
    const container = scrollRef.current;
    if (!container) return;

    function handleScroll() {
      const children = container!.children;
      const scrollTop = container!.scrollTop + 100;
      for (let i = 0; i < children.length; i++) {
        const child = children[i] as HTMLElement;
        if (child.offsetTop + child.offsetHeight > scrollTop) {
          setCurrentPage(i + 1);
          break;
        }
      }
    }
    container.addEventListener("scroll", handleScroll);
    return () => container.removeEventListener("scroll", handleScroll);
  }, []);

  /* ── Fullscreen toggle (CSS-based fixed positioning) ── */
  function toggleFullscreen() {
    setIsFullscreen((v) => !v);
  }

  // Recalculate fit-to-width when entering/exiting fullscreen
  useEffect(() => {
    requestAnimationFrame(() => {
      const container = scrollRef.current;
      if (!container || canvasInfoRef.current.length === 0) return;
      const availW = container.clientWidth;
      const pageW = canvasInfoRef.current[0].pageW;
      if (pageW > 0) {
        const newFit = +(Math.min(5, Math.max(0.5, availW / pageW)).toFixed(2));
        fitZoomRef.current = newFit;
        setZoom(newFit);
      }
    });
  }, [isFullscreen]);

  // Escape key exits fullscreen
  useEffect(() => {
    if (!isFullscreen) return;
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") setIsFullscreen(false);
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [isFullscreen]);

  // Prevent body scroll while fullscreen
  useEffect(() => {
    if (isFullscreen) {
      document.body.style.overflow = "hidden";
      return () => { document.body.style.overflow = ""; };
    }
  }, [isFullscreen]);

  /* ── Jump to page N ── */
  function goToPage(n: number) {
    const container = scrollRef.current;
    if (!container) return;
    const idx = Math.max(0, Math.min(n - 1, container.children.length - 1));
    const child = container.children[idx] as HTMLElement | undefined;
    if (child) child.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  /* ── Run extraction / AI analysis lazily (on Info click) ── */
  async function runProcessing() {
    if (processedRef.current || !pdfRef.current) return;
    processedRef.current = true;
    setProcessing(true);
    const pdf = pdfRef.current;

    try {
      const ext: Record<string, string> = {};

      // Text extraction
      const t0 = performance.now();
      const textItems = await pdf.extractText({ preserveFormatting: true });
      const textMs = Math.round(performance.now() - t0);
      const allText = textItems.map((t: any) => t.text || "").join(" ");
      const wordCount = allText.split(/\s+/).filter(Boolean).length;
      const charCount = allText.length;
      ext["Words"] = wordCount.toLocaleString();
      ext["Characters"] = charCount.toLocaleString();
      ext["Text Pages"] = `${new Set(textItems.map((t: any) => t.pageNumber)).size}`;
      ext["Extract Time"] = `${textMs} ms`;
      setSampleText(allText.slice(0, 300).trim() + (allText.length > 300 ? "\u2026" : ""));

      // Annotations
      const annotations = await pdf.getAnnotations();
      ext["Annotations"] = String(annotations.length);

      // Form fields
      const fields = await pdf.getFormFields();
      ext["Form Fields"] = String(fields.length);

      // Named destinations
      const dests = pdf.getNamedDestinations();
      ext["Named Dests"] = String(dests.size);

      setExtraction(ext);

      // AI analysis
      const ai: Record<string, string> = {};
      const at0 = performance.now();
      const chunks = await pdf.generateSemanticChunks({ strategy: "semantic", maxChunkSize: 1000 });
      const chunkMs = Math.round(performance.now() - at0);
      ai["Semantic Chunks"] = String(chunks.length);
      ai["Chunking Time"] = `${chunkMs} ms`;
      ai["Avg Chunk Size"] = chunks.length ? `${Math.round(chunks.reduce((s: number, c: any) => s + (c.content?.length || 0), 0) / chunks.length)} chars` : "\u2014";

      const af0 = performance.now();
      const features = await pdf.getAIFeatures({ enableStructuralAnalysis: true });
      const aiMs = Math.round(performance.now() - af0);
      ai["Analysis Time"] = `${aiMs} ms`;
      if (features.documentType) ai["Doc Type"] = features.documentType;
      if (features.structuralAnalysis) {
        const sa = features.structuralAnalysis;
        if (sa.sections) ai["Sections"] = String(sa.sections.length);
        if (sa.tables) ai["Tables"] = String(sa.tables.length);
        if (sa.figures) ai["Figures"] = String(sa.figures.length);
      }
      if (features.nlpReady?.keywords?.length) {
        ai["Keywords"] = features.nlpReady.keywords.slice(0, 8).join(", ");
      }

      setAiData(ai);
    } catch { /* extraction best-effort */ }

    setProcessing(false);
  }

  return (
    <DemoShell
      title="PDF Viewer Demo"
      description="Shannon's 1948 paper rendered by the AgenticPDF engine — real PDF parsing, canvas rendering, and continuous scrolling."
    >
      <Tabs
        tabs={[
          {
            label: "Viewer",
            content: (
              <div
                ref={viewerRef}
                style={isFullscreen ? {
                  position: "fixed",
                  inset: 0,
                  zIndex: 9999,
                  background: "var(--bg)",
                } : {
                  position: "relative",
                  background: "var(--bg)",
                }}
              >
                {/* Toolbar — overlaid on viewport */}
                <div
                  className="flex items-center justify-between gap-4 px-3 py-2 flex-wrap"
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    right: showMeta ? 300 : 0,
                    zIndex: 10,
                    background: "color-mix(in srgb, var(--bg-card) 85%, transparent)",
                    backdropFilter: "blur(8px)",
                    borderBottom: "1px solid var(--border)",
                    borderRadius: isFullscreen ? 0 : "12px 12px 0 0",
                  }}
                >
                  {/* Status + page nav */}
                  <div className="flex items-center gap-3">
                    <span
                      className="w-2 h-2 rounded-full"
                      style={{
                        background:
                          status === "ready" ? "#22c55e" : status === "loading" ? "#f59e0b" : "#ef4444",
                      }}
                    />
                    {status === "loading" ? (
                      <span className="text-sm font-mono" style={{ color: "var(--text)" }}>Processing PDF...</span>
                    ) : status === "error" ? (
                      <span className="text-sm font-mono" style={{ color: "#ef4444" }}>Error: {errorMsg}</span>
                    ) : (
                      <span className="text-sm font-mono flex items-center gap-1" style={{ color: "var(--text)" }}>
                        Page{" "}
                        <input
                          type="text"
                          inputMode="numeric"
                          defaultValue={currentPage}
                          key={currentPage}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") {
                              const n = parseInt((e.target as HTMLInputElement).value, 10);
                              if (n >= 1 && n <= totalPages) {
                                setCurrentPage(n);
                                goToPage(n);
                              }
                              (e.target as HTMLInputElement).blur();
                            }
                          }}
                          onBlur={(e) => {
                            const n = parseInt(e.target.value, 10);
                            if (n >= 1 && n <= totalPages) {
                              setCurrentPage(n);
                              goToPage(n);
                            }
                          }}
                          onFocus={(e) => e.target.select()}
                          className="font-mono text-sm text-center rounded"
                          style={{
                            width: `${String(totalPages).length + 1.5}ch`,
                            padding: "1px 4px",
                            background: "var(--bg)",
                            border: "1px solid var(--border)",
                            color: "var(--text)",
                          }}
                        />
                        {" / "}{totalPages}
                      </span>
                    )}
                  </div>

                  {/* Zoom */}
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => setZoom((z) => Math.max(0.25, +(z - 0.25).toFixed(2)))}
                      disabled={status !== "ready"}
                      className="px-3 py-1.5 rounded text-sm font-medium disabled:opacity-40"
                      style={{ background: "var(--bg)", border: "1px solid var(--border)", color: "var(--text)" }}
                    >
                      −
                    </button>
                    <span className="text-xs font-mono w-14 text-center" style={{ color: "var(--text-muted)" }}>
                      {Math.round(zoom * 100)}%
                    </span>
                    <button
                      onClick={() => setZoom((z) => Math.min(5, +(z + 0.25).toFixed(2)))}
                      disabled={status !== "ready"}
                      className="px-3 py-1.5 rounded text-sm font-medium disabled:opacity-40"
                      style={{ background: "var(--bg)", border: "1px solid var(--border)", color: "var(--text)" }}
                    >
                      +
                    </button>
                    <button
                      onClick={() => setZoom(fitZoomRef.current)}
                      disabled={status !== "ready"}
                      className="px-3 py-1.5 rounded text-xs font-medium disabled:opacity-40"
                      style={{ background: "var(--bg)", border: "1px solid var(--border)", color: "var(--text)" }}
                    >
                      Fit
                    </button>
                  </div>

                  {/* Metrics + Info toggle */}
                  <div className="flex items-center gap-4 text-xs font-mono" style={{ color: "var(--text-muted)" }}>
                    {metrics.fileKB > 0 && <span>{metrics.fileKB} KB</span>}
                    {metrics.renderMs > 0 && <span>{metrics.renderMs} ms</span>}
                    <button
                      onClick={() => { setShowMeta((v) => !v); if (!processedRef.current) requestAnimationFrame(() => runProcessing()); }}
                      disabled={status !== "ready"}
                      className={`px-3 py-1.5 rounded text-xs font-medium disabled:opacity-40 ${showMeta ? "" : "hover:opacity-80 active:scale-95"}`}
                      style={{
                        background: showMeta ? "var(--accent)" : "var(--bg)",
                        border: showMeta ? "1px solid var(--accent)" : "1px solid var(--border)",
                        color: showMeta ? "#fff" : "var(--text)",
                        boxShadow: showMeta ? "0 0 0 2px var(--accent-soft)" : "none",
                        transition: "all 0.15s ease",
                      }}
                    >
                      Info
                    </button>
                    <button
                      onClick={toggleFullscreen}
                      disabled={status !== "ready"}
                      className="px-3 py-1.5 rounded text-xs font-medium disabled:opacity-40 hover:opacity-80 active:scale-95"
                      style={{
                        background: isFullscreen ? "var(--accent)" : "var(--bg)",
                        border: isFullscreen ? "1px solid var(--accent)" : "1px solid var(--border)",
                        color: isFullscreen ? "#fff" : "var(--text)",
                        boxShadow: isFullscreen ? "0 0 0 2px var(--accent-soft)" : "none",
                        transition: "all 0.15s ease",
                      }}
                      title={isFullscreen ? "Exit fullscreen" : "Fullscreen"}
                    >
                      {isFullscreen ? "\u2716" : "\u26F6"}
                    </button>
                  </div>
                </div>

                {/* Main area: viewport + optional metadata sidebar */}
                <div className="flex" style={{ height: isFullscreen ? "calc(100dvh - 44px)" : "70vh" }}>
                {/* Scroll viewport */}
                <div
                  ref={scrollRef}
                  className="overflow-auto rounded-l-xl px-0 py-0 pt-12 flex flex-col items-center"
                  style={{
                    background: "var(--bg)",
                    border: "1px solid var(--border)",
                    flex: 1,
                    minWidth: 0,
                    borderRadius: isFullscreen ? 0 : showMeta ? "12px 0 0 12px" : "12px",
                  }}
                >
                  {status === "loading" && (
                    <div className="flex flex-col items-center justify-center h-full gap-4">
                      <div
                        className="w-8 h-8 border-3 rounded-full animate-spin"
                        style={{ borderColor: "var(--border)", borderTopColor: "var(--accent)" }}
                      />
                      <span className="text-sm" style={{ color: "var(--text-muted)" }}>
                        Processing shannon1948.pdf...
                      </span>
                    </div>
                  )}
                  {status === "error" && (
                    <div className="flex items-center justify-center h-full">
                      <span className="text-sm" style={{ color: "#ef4444" }}>
                        Failed to load PDF: {errorMsg}
                      </span>
                    </div>
                  )}
                </div>

                {/* Metadata sidebar */}
                {showMeta && (
                  <div
                    className="overflow-auto rounded-br-xl p-4"
                    style={{
                      width: 300,
                      flexShrink: 0,
                      background: "var(--bg-card)",
                      borderTop: "1px solid var(--border)",
                      borderRight: "1px solid var(--border)",
                      borderBottom: "1px solid var(--border)",
                    }}
                  >
                    {/* Document Info */}
                    <h3 className="text-xs font-semibold uppercase tracking-wider mb-2" style={{ color: "var(--accent)" }}>Document Info</h3>
                    <table className="w-full text-xs mb-4" style={{ borderCollapse: "collapse" }}>
                      <tbody>
                        {Object.entries(metadata).map(([key, val]) => (
                          <tr key={key} style={{ borderBottom: "1px solid var(--border)" }}>
                            <td className="py-1 pr-2 font-medium align-top" style={{ color: "var(--text-muted)", whiteSpace: "nowrap" }}>{key}</td>
                            <td className="py-1 font-mono" style={{ color: "var(--text)", wordBreak: "break-word" }}>{val}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>

                    {/* Extracted Data */}
                    {processing && Object.keys(extraction).length === 0 && (
                      <p className="text-xs animate-pulse mb-4" style={{ color: "var(--text-muted)" }}>Processing extraction&hellip;</p>
                    )}
                    {Object.keys(extraction).length > 0 && (
                      <>
                        <h3 className="text-xs font-semibold uppercase tracking-wider mb-2" style={{ color: "var(--accent)" }}>Extracted Data</h3>
                        <table className="w-full text-xs mb-2" style={{ borderCollapse: "collapse" }}>
                          <tbody>
                            {Object.entries(extraction).map(([key, val]) => (
                              <tr key={key} style={{ borderBottom: "1px solid var(--border)" }}>
                                <td className="py-1 pr-2 font-medium align-top" style={{ color: "var(--text-muted)", whiteSpace: "nowrap" }}>{key}</td>
                                <td className="py-1 font-mono" style={{ color: "var(--text)", wordBreak: "break-word" }}>{val}</td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                        {sampleText && (
                          <div className="text-xs p-2 rounded mb-4" style={{ background: "var(--bg)", color: "var(--text-muted)", lineHeight: 1.5 }}>
                            <span className="font-semibold" style={{ color: "var(--text)" }}>Sample: </span>{sampleText}
                          </div>
                        )}
                      </>
                    )}

                    {/* AI Analysis */}
                    {processing && Object.keys(aiData).length === 0 && (
                      <p className="text-xs animate-pulse mb-4" style={{ color: "var(--text-muted)" }}>Running AI analysis&hellip;</p>
                    )}
                    {Object.keys(aiData).length > 0 && (
                      <>
                        <h3 className="text-xs font-semibold uppercase tracking-wider mb-2" style={{ color: "var(--accent)" }}>AI Analysis</h3>
                        <table className="w-full text-xs" style={{ borderCollapse: "collapse" }}>
                          <tbody>
                            {Object.entries(aiData).map(([key, val]) => (
                              <tr key={key} style={{ borderBottom: "1px solid var(--border)" }}>
                                <td className="py-1 pr-2 font-medium align-top" style={{ color: "var(--text-muted)", whiteSpace: "nowrap" }}>{key}</td>
                                <td className="py-1 font-mono" style={{ color: "var(--text)", wordBreak: "break-word" }}>{val}</td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </>
                    )}
                  </div>
                )}
                </div>
              </div>
            ),
          },
          {
            label: "Source Code",
            content: (
              <CodeBlock
                filename="viewer.ts"
                code={`import { AgenticPDF } from 'agenticpdf';

// Load Shannon's 1948 paper
const pdf = await AgenticPDF.fromUrl('/shannon1948.pdf');
const container = document.getElementById('viewer');

// Create full-featured viewer
const viewer = pdf.createOptimalViewer(container, {
  scale: 2,
  fitToWidth: true,
  maintainAspectRatio: true,
  darkMode: true,
  continuousScrolling: true,
  enableThemeToggle: true,
  persistTheme: true,
  defaultTheme: 'dark'
});

// Or render individual pages
const canvas = document.getElementById('canvas');
await pdf.renderPage(1, canvas, {
  scale: window.devicePixelRatio,
  fitToWidth: true,
  darkMode: true
});

// Add selectable text layer
await pdf.buildTextLayer(1, textLayerDiv, {
  width: canvas.width,
  height: canvas.height
});

// Export page as image
const blob = await pdf.renderPageToImage(1, 'png', {
  scale: 3 // High-res export
});

// Cleanup
viewer.destroy();
pdf.close();`}
              />
            ),
          },
        ]}
      />
    </DemoShell>
  );
}
