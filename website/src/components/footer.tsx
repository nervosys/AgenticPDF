export function Footer() {
  return (
    <footer
      className="border-t py-8 mt-20"
      style={{ borderColor: "var(--border)" }}
    >
      <div className="mx-auto max-w-7xl px-4 sm:px-6 flex flex-col sm:flex-row items-center justify-between gap-4">
        <p className="font-mono text-xs" style={{ color: "var(--text-muted)" }}>
          &copy; {new Date().getFullYear()} NERVOSYS. AgenticPDF is
          AGPL-3.0-or-later licensed.
        </p>
        <div className="flex gap-6 font-mono text-xs" style={{ color: "var(--text-muted)" }}>
          <a
            href="https://github.com/nervosys/AgenticPDF"
            target="_blank"
            rel="noopener noreferrer"
            className="hover:underline"
          >
            GitHub
          </a>
          <a
            href="https://www.npmjs.com/package/agenticpdf"
            target="_blank"
            rel="noopener noreferrer"
            className="hover:underline"
          >
            npm
          </a>
          <a href="/docs/" className="hover:underline">
            Documentation
          </a>
        </div>
      </div>
    </footer>
  );
}
