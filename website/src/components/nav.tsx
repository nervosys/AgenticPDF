"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useState } from "react";

const links = [
  { href: "/docs/", label: "Docs" },
  { href: "/docs/api/", label: "API" },
  { href: "/docs/cli/", label: "CLI" },
  { href: "/demos/", label: "Demos" },
  { href: "/demos/ai/", label: "AI Demo" },
  { href: "/demos/streaming/", label: "Streaming" },
  { href: "/demos/viewer/", label: "Viewer" },
  { href: "/demos/pretext/", label: "Pretext" },
];

export function Nav() {
  const pathname = usePathname();
  const [open, setOpen] = useState(false);

  return (
    <header
      className="sticky top-0 z-50 border-b backdrop-blur-xl header-bg"
      style={{
        borderColor: "var(--border)",
      }}
    >
      <div className="mx-auto flex h-16 max-w-7xl items-center justify-between px-4 sm:px-6">
        <Link
          href="/"
          className="flex items-center gap-2.5 font-mono font-bold text-base tracking-tight"
          style={{ color: "var(--text)" }}
        >
          <span
            className="flex h-8 w-8 items-center justify-center rounded-md text-sm font-black"
            style={{ background: "var(--accent)", color: "#0a0e17" }}
          >
            A
          </span>
          AgenticPDF
        </Link>

        {/* Desktop nav */}
        <nav className="hidden md:flex items-center gap-1">
          {links.map(({ href, label }) => {
            const active = pathname === href || pathname === href.slice(0, -1);
            return (
              <Link
                key={href}
                href={href}
                className="px-3 py-1.5 rounded-md text-sm font-mono font-medium transition-colors"
                style={{
                  color: active ? "var(--accent)" : "var(--text-muted)",
                  background: active ? "var(--accent-soft)" : "transparent",
                }}
              >
                {label}
              </Link>
            );
          })}
        </nav>

        <div className="hidden md:flex items-center gap-3">
          <ThemeToggle />
          <a
            href="https://github.com/nervosys/AgenticPDF"
            target="_blank"
            rel="noopener noreferrer"
            className="px-4 py-2 rounded-lg text-sm font-medium transition-colors flex items-center gap-2"
            style={{
              border: "1px solid var(--border)",
              color: "var(--text-muted)",
            }}
          >
            <GitHubIcon />
            GitHub
          </a>
        </div>

        {/* Mobile toggle */}
        <button
          className="md:hidden p-2"
          onClick={() => setOpen(!open)}
          aria-label="Toggle menu"
        >
          <svg
            width="20"
            height="20"
            viewBox="0 0 20 20"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          >
            {open ? (
              <path d="M5 5l10 10M15 5l-10 10" />
            ) : (
              <path d="M3 6h14M3 10h14M3 14h14" />
            )}
          </svg>
        </button>
      </div>

      {/* Mobile menu */}
      {open && (
        <nav className="md:hidden border-t px-4 pb-4 pt-2" style={{ borderColor: "var(--border)" }}>
          {links.map(({ href, label }) => (
            <Link
              key={href}
              href={href}
              className="block py-2 text-sm"
              style={{ color: "var(--text-muted)" }}
              onClick={() => setOpen(false)}
            >
              {label}
            </Link>
          ))}
          <div className="pt-2 flex items-center gap-3">
            <ThemeToggle />
          </div>
        </nav>
      )}
    </header>
  );
}

function ThemeToggle() {
  const toggle = () => {
    const html = document.documentElement;
    const next = html.getAttribute("data-theme") === "dark" ? "light" : "dark";
    html.setAttribute("data-theme", next);
    localStorage.setItem("agenticpdf-theme", next);
  };

  return (
    <button
      onClick={toggle}
      className="p-2 rounded-md transition-colors"
      style={{ color: "var(--text-muted)" }}
      aria-label="Toggle theme"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <circle cx="12" cy="12" r="5" />
        <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
      </svg>
    </button>
  );
}

function GitHubIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
    </svg>
  );
}
