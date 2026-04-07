"use client";

import { useState, useRef, ReactNode } from "react";

/* ── Reusable code block with copy button ── */
export function CodeBlock({
  code,
  language = "typescript",
  filename,
  html,
}: {
  code: string;
  language?: string;
  filename?: string;
  html?: string;
}) {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  return (
    <div className="relative group rounded-lg overflow-hidden" style={{ border: "1px solid var(--border)" }}>
      {filename && (
        <div
          className="px-4 py-2 text-xs font-mono border-b"
          style={{ background: "var(--bg)", borderColor: "var(--border)", color: "var(--text-muted)" }}
        >
          {filename}
        </div>
      )}
      {html ? (
        <div
          className="shiki-wrapper [&_pre]:!rounded-none [&_pre]:!border-0 [&_pre]:!m-0 [&_pre]:!bg-transparent [&_code]:!font-[var(--font-mono)]"
          dangerouslySetInnerHTML={{ __html: html }}
        />
      ) : (
        <pre className="!rounded-none !border-0 !m-0">
          <code className={`language-${language}`}>{code}</code>
        </pre>
      )}
      <button
        onClick={copy}
        className="absolute top-2 right-2 px-2.5 py-1 rounded-md text-xs font-medium opacity-0 group-hover:opacity-100 transition-opacity"
        style={{
          background: "var(--bg-hover)",
          color: "var(--text-muted)",
          border: "1px solid var(--border)",
        }}
      >
        {copied ? "Copied!" : "Copy"}
      </button>
    </div>
  );
}

/* ── Feature card ── */
export function FeatureCard({
  icon,
  title,
  description,
}: {
  icon: ReactNode;
  title: string;
  description: string;
}) {
  return (
    <div
      className="card-glow rounded-lg p-6 transition-colors animate-in"
      style={{
        background: "var(--bg-card)",
        border: "1px solid var(--border)",
      }}
    >
      <div
        className="mb-3 flex h-10 w-10 items-center justify-center rounded-md"
        style={{ background: "var(--accent-soft)", color: "var(--accent)" }}
      >
        {icon}
      </div>
      <h3 className="font-mono text-sm font-semibold mb-1" style={{ color: "var(--text)" }}>
        {title}
      </h3>
      <p className="text-sm leading-relaxed" style={{ color: "var(--text-muted)" }}>
        {description}
      </p>
    </div>
  );
}

/* ── Tabs ── */
export function Tabs({
  tabs,
}: {
  tabs: { label: string; content: ReactNode }[];
}) {
  const [active, setActive] = useState(0);
  return (
    <div>
      <div className="flex gap-1 mb-4 border-b" style={{ borderColor: "var(--border)" }}>
        {tabs.map((t, i) => (
          <button
            key={t.label}
            onClick={() => setActive(i)}
            className="px-4 py-2 text-sm font-mono font-medium transition-colors -mb-px"
            style={{
              color: i === active ? "var(--accent)" : "var(--text-muted)",
              borderBottom: i === active ? "2px solid var(--accent)" : "2px solid transparent",
            }}
          >
            {t.label}
          </button>
        ))}
      </div>
      <div>{tabs[active].content}</div>
    </div>
  );
}

/* ── Interactive demo shell ── */
export function DemoShell({
  title,
  description,
  children,
}: {
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <section className="mx-auto max-w-5xl px-4 sm:px-6 py-12">
      <h1
        className="font-tactical text-3xl mb-2"
        style={{ color: "var(--text)" }}
      >
        {title}
      </h1>
      <p className="mb-8 max-w-2xl text-sm" style={{ color: "var(--text-muted)" }}>
        {description}
      </p>
      {children}
    </section>
  );
}

/* ── Sidebar for docs ── */
export function DocsSidebar({
  sections,
}: {
  sections: { title: string; links: { href: string; label: string }[] }[];
}) {
  return (
    <aside className="hidden lg:block w-56 shrink-0 pr-8">
      <nav className="sticky top-20 space-y-6">
        {sections.map((s) => (
          <div key={s.title}>
            <h4
              className="font-mono text-xs font-semibold uppercase tracking-[0.15em] mb-2"
              style={{ color: "var(--text-muted)" }}
            >
              {s.title}
            </h4>
            <ul className="space-y-1">
              {s.links.map((l) => (
                <li key={l.href}>
                  <a
                    href={l.href}
                    className="block px-2 py-1 text-sm rounded-md transition-colors hover:bg-[var(--bg-hover)]"
                    style={{ color: "var(--text-muted)" }}
                  >
                    {l.label}
                  </a>
                </li>
              ))}
            </ul>
          </div>
        ))}
      </nav>
    </aside>
  );
}

/* ── Console output panel ── */
export function ConsolePanel({ lines }: { lines: string[] }) {
  const ref = useRef<HTMLDivElement>(null);
  return (
    <div
      ref={ref}
      className="rounded-lg font-mono text-sm p-4 max-h-64 overflow-y-auto space-y-1"
      style={{
        background: "var(--bg-card)",
        border: "1px solid var(--border)",
        color: "var(--success)",
      }}
    >
      {lines.length === 0 && (
        <span style={{ color: "var(--text-muted)" }}>
          Output will appear here...
        </span>
      )}
      {lines.map((l, i) => (
        <div key={i}>{l}</div>
      ))}
    </div>
  );
}
