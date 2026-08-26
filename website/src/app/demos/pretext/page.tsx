"use client";

import { useState, useRef, useEffect, useCallback } from "react";
import { DemoShell, CodeBlock, Tabs } from "@/components/ui";

/* ── Simple text measurement using canvas ── */
function measureText(text: string, font: string): number[] {
  if (typeof document === "undefined") return text.split("").map(() => 8);
  const c = document.createElement("canvas").getContext("2d")!;
  c.font = font;
  return text.split("").map((ch) => c.measureText(ch).width);
}

/* ── Tiny line-breaking engine (mirrors PretextLayout behavior) ── */
interface LayoutLine {
  text: string;
  width: number;
  y: number;
}

function layoutText(
  text: string,
  font: string,
  maxWidth: number,
  lineHeight: number,
  whiteSpace: "normal" | "pre" | "pre-wrap" | "nowrap" = "normal"
): LayoutLine[] {
  const widths = measureText(text, font);
  const lines: LayoutLine[] = [];
  let lineStart = 0;
  let x = 0;
  let lastBreak = 0;
  let y = 0;

  if (whiteSpace === "nowrap") {
    return [{ text, width: widths.reduce((a, b) => a + b, 0), y: 0 }];
  }

  if (whiteSpace === "pre") {
    for (const raw of text.split("\n")) {
      const w = measureText(raw, font).reduce((a, b) => a + b, 0);
      lines.push({ text: raw, width: w, y });
      y += lineHeight;
    }
    return lines;
  }

  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    const w = widths[i];

    if (ch === "\n" && (whiteSpace === "pre-wrap")) {
      lines.push({ text: text.slice(lineStart, i), width: x, y });
      y += lineHeight;
      lineStart = i + 1;
      x = 0;
      lastBreak = lineStart;
      continue;
    }

    if (ch === " " || ch === "\t") lastBreak = i + 1;

    if (x + w > maxWidth && lineStart < i) {
      const breakAt = lastBreak > lineStart ? lastBreak : i;
      const lineText = text.slice(lineStart, breakAt);
      const lineW = measureText(lineText, font).reduce((a, b) => a + b, 0);
      lines.push({ text: lineText, width: lineW, y });
      y += lineHeight;
      lineStart = breakAt;
      x = 0;
      for (let j = lineStart; j <= i; j++) x += widths[j];
      lastBreak = lineStart;
    } else {
      x += w;
    }
  }

  if (lineStart < text.length) {
    const rest = text.slice(lineStart);
    const w = measureText(rest, font).reduce((a, b) => a + b, 0);
    lines.push({ text: rest, width: w, y });
  }

  return lines;
}

const sampleTexts: Record<string, string> = {
  english:
    "The quick brown fox jumps over the lazy dog. Typography is the art and technique of arranging type to make written language legible, readable, and appealing when displayed. The arrangement of type involves selecting typefaces, point sizes, line lengths, line-spacing, and letter-spacing.",
  cjk: "吾輩は猫である。名前はまだ無い。どこで生まれたかとんと見当がつかぬ。何でも薄暗いじめじめした所でニャーニャー泣いていた事だけは記憶している。",
  mixed:
    "AgenticPDF v1.0.0 — AI-native PDF処理ライブラリ。Streaming-first architecture with 零依存関係 (zero dependencies). Supports 日本語、中文、한국어 and more.",
  code: "function fibonacci(n: number): number {\n  if (n <= 1) return n;\n  return fibonacci(n - 1) + fibonacci(n - 2);\n}\n\nconsole.log(fibonacci(10)); // 55",
};

export default function PretextDemoPage() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [text, setText] = useState(sampleTexts.english);
  const [maxWidth, setMaxWidth] = useState(400);
  const [fontSize, setFontSize] = useState(16);
  const [lineHeight, setLineHeight] = useState(24);
  const [whiteSpace, setWhiteSpace] = useState<"normal" | "pre" | "pre-wrap" | "nowrap">("normal");
  const [showGuides, setShowGuides] = useState(true);
  const [lines, setLines] = useState<LayoutLine[]>([]);

  const render = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d")!;
    const font = `${fontSize}px system-ui, sans-serif`;

    const result = layoutText(text, font, maxWidth, lineHeight, whiteSpace);
    setLines(result);

    // Read theme-aware colors from CSS variables
    const styles = getComputedStyle(document.documentElement);
    const bgColor = styles.getPropertyValue("--bg-card").trim() || "#18181b";
    const textColor = styles.getPropertyValue("--text").trim() || "#fafafa";

    const padding = 24;
    const totalH = result.length * lineHeight + padding * 2;
    const scale = window.devicePixelRatio || 2;
    const w = maxWidth + padding * 2;

    canvas.width = w * scale;
    canvas.height = Math.max(totalH, 100) * scale;
    canvas.style.width = `${w}px`;
    canvas.style.height = `${Math.max(totalH, 100)}px`;
    ctx.scale(scale, scale);

    // Background
    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, w, Math.max(totalH, 100));

    // Max width guide
    if (showGuides) {
      ctx.strokeStyle = "rgba(99, 102, 241, 0.3)";
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 4]);
      ctx.strokeRect(padding - 0.5, padding - 0.5, maxWidth + 1, totalH - padding * 2 + lineHeight);
      ctx.setLineDash([]);
    }

    // Render lines
    ctx.font = font;
    for (let i = 0; i < result.length; i++) {
      const line = result[i];
      const y = padding + line.y;

      // Baseline guides
      if (showGuides) {
        ctx.fillStyle = "rgba(99, 102, 241, 0.06)";
        ctx.fillRect(padding, y, maxWidth, lineHeight);
        ctx.strokeStyle = "rgba(99, 102, 241, 0.15)";
        ctx.lineWidth = 0.5;
        ctx.beginPath();
        ctx.moveTo(padding, y + lineHeight);
        ctx.lineTo(padding + maxWidth, y + lineHeight);
        ctx.stroke();
      }

      // Text
      ctx.fillStyle = textColor;
      ctx.fillText(line.text, padding, y + lineHeight * 0.75);

      // Width indicator
      if (showGuides) {
        ctx.fillStyle = "rgba(34, 197, 94, 0.4)";
        ctx.fillRect(padding + line.width, y + lineHeight * 0.75 - 1, 2, 2);
      }
    }
  }, [text, maxWidth, fontSize, lineHeight, whiteSpace, showGuides]);

  useEffect(() => {
    render();
  }, [render]);

  // Re-render canvas when theme changes
  useEffect(() => {
    const observer = new MutationObserver((mutations) => {
      for (const m of mutations) {
        if (m.attributeName === "data-theme") {
          render();
          break;
        }
      }
    });
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    return () => observer.disconnect();
  }, [render]);

  return (
    <DemoShell
      title="Pretext Layout Demo"
      description="Interactive text measurement and line-breaking engine. Adjust parameters to see how PretextLayout handles word-wrap, CJK text, and whitespace modes."
    >
      <Tabs
        tabs={[
          {
            label: "Interactive",
            content: (
              <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                {/* Controls */}
                <div className="space-y-5">
                  {/* Preset text buttons */}
                  <div>
                    <label className="block text-sm font-medium mb-2" style={{ color: "var(--text)" }}>
                      Sample Text
                    </label>
                    <div className="flex flex-wrap gap-2">
                      {Object.entries(sampleTexts).map(([key, val]) => (
                        <button
                          key={key}
                          onClick={() => setText(val)}
                          className="px-3 py-1.5 rounded text-xs font-medium transition-colors"
                          style={{
                            background: text === val ? "var(--accent-soft)" : "var(--bg-card)",
                            color: text === val ? "var(--accent)" : "var(--text-muted)",
                            border: `1px solid ${text === val ? "var(--accent)" : "var(--border)"}`,
                          }}
                        >
                          {key}
                        </button>
                      ))}
                    </div>
                  </div>

                  {/* Text input */}
                  <div>
                    <label className="block text-sm font-medium mb-2" style={{ color: "var(--text)" }}>
                      Text
                    </label>
                    <textarea
                      value={text}
                      onChange={(e) => setText(e.target.value)}
                      rows={4}
                      className="w-full rounded-lg p-3 text-sm resize-y"
                      style={{
                        background: "var(--bg-card)",
                        border: "1px solid var(--border)",
                        color: "var(--text)",
                      }}
                    />
                  </div>

                  {/* Sliders */}
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <label className="block text-sm font-medium mb-1" style={{ color: "var(--text)" }}>
                        Max Width: {maxWidth}px
                      </label>
                      <input
                        type="range"
                        min={100}
                        max={800}
                        value={maxWidth}
                        onChange={(e) => setMaxWidth(Number(e.target.value))}
                        className="w-full"
                      />
                    </div>
                    <div>
                      <label className="block text-sm font-medium mb-1" style={{ color: "var(--text)" }}>
                        Font Size: {fontSize}px
                      </label>
                      <input
                        type="range"
                        min={10}
                        max={36}
                        value={fontSize}
                        onChange={(e) => setFontSize(Number(e.target.value))}
                        className="w-full"
                      />
                    </div>
                    <div>
                      <label className="block text-sm font-medium mb-1" style={{ color: "var(--text)" }}>
                        Line Height: {lineHeight}px
                      </label>
                      <input
                        type="range"
                        min={12}
                        max={48}
                        value={lineHeight}
                        onChange={(e) => setLineHeight(Number(e.target.value))}
                        className="w-full"
                      />
                    </div>
                    <div>
                      <label className="block text-sm font-medium mb-1" style={{ color: "var(--text)" }}>
                        White-Space
                      </label>
                      <select
                        value={whiteSpace}
                        onChange={(e) => setWhiteSpace(e.target.value as typeof whiteSpace)}
                        className="w-full rounded-lg p-2 text-sm"
                        style={{
                          background: "var(--bg-card)",
                          border: "1px solid var(--border)",
                          color: "var(--text)",
                        }}
                      >
                        <option value="normal">normal</option>
                        <option value="pre">pre</option>
                        <option value="pre-wrap">pre-wrap</option>
                        <option value="nowrap">nowrap</option>
                      </select>
                    </div>
                  </div>

                  {/* Show guides */}
                  <label className="flex items-center gap-2 text-sm cursor-pointer" style={{ color: "var(--text)" }}>
                    <input
                      type="checkbox"
                      checked={showGuides}
                      onChange={(e) => setShowGuides(e.target.checked)}
                    />
                    Show layout guides
                  </label>

                  {/* Stats */}
                  <div
                    className="rounded-lg p-4 text-sm space-y-1"
                    style={{ background: "var(--bg-card)", border: "1px solid var(--border)" }}
                  >
                    <div style={{ color: "var(--text-muted)" }}>
                      Lines: <span style={{ color: "var(--text)" }}>{lines.length}</span>
                    </div>
                    <div style={{ color: "var(--text-muted)" }}>
                      Characters: <span style={{ color: "var(--text)" }}>{text.length}</span>
                    </div>
                    <div style={{ color: "var(--text-muted)" }}>
                      Total Height:{" "}
                      <span style={{ color: "var(--text)" }}>{lines.length * lineHeight}px</span>
                    </div>
                    <div style={{ color: "var(--text-muted)" }}>
                      Avg Line Width:{" "}
                      <span style={{ color: "var(--text)" }}>
                        {lines.length > 0
                          ? Math.round(lines.reduce((a, l) => a + l.width, 0) / lines.length)
                          : 0}
                        px
                      </span>
                    </div>
                  </div>
                </div>

                {/* Canvas */}
                <div className="overflow-auto rounded-xl" style={{ border: "1px solid var(--border)" }}>
                  <canvas ref={canvasRef} style={{ display: "block" }} />
                </div>
              </div>
            ),
          },
          {
            label: "Source Code",
            content: (
              <CodeBlock
                filename="pretext-layout.ts"
                code={`import { AgenticPDF, PretextLayout } from 'agenticpdf';

// Enable pretext layout
const pdf = await AgenticPDF.fromFile(file, {
  enablePretextLayout: true
});

// Prepare text with measurements
const prepared = AgenticPDF.prepareText(
  'The quick brown fox jumps over the lazy dog.',
  '16px system-ui',
  { whiteSpace: 'normal' }
);
// prepared.widths: Float64Array of per-character widths

// Layout with line-breaking
const result = AgenticPDF.layoutText(prepared, 400, 24);
console.log(\`Total lines: \${result.lineCount}\`);
console.log(\`Total height: \${result.height}px\`);

// Or use PretextLayout directly for lower-level access
const prep = PretextLayout.prepareWithSegments(text, font);
const layout = PretextLayout.layoutWithLines(prep, maxWidth, lineHeight);

for (const line of layout.lines) {
  console.log(\`Line: "\${line.text}" width=\${line.width}\`);
}

// Walk lines with callback (zero-allocation)
PretextLayout.walkLineRanges(prep, maxWidth, (line) => {
  console.log(\`Range [\${line.start}, \${line.end}] y=\${line.y}\`);
});

// Incremental layout
let cursor = { index: 0 };
let line;
while ((line = PretextLayout.layoutNextLine(prep, cursor, maxWidth))) {
  process.stdout.write(line.text + '\\n');
}

// CJK and locale support
PretextLayout.setLocale('ja');
const cjk = PretextLayout.prepareWithSegments('日本語テキスト', font);

// Cache management
console.log(PretextLayout.isCacheDirty()); // true
PretextLayout.clearCache();`}
              />
            ),
          },
        ]}
      />
    </DemoShell>
  );
}
