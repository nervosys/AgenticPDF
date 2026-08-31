import type { Metadata } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import "./globals.css";
import { Nav } from "@/components/nav";
import { Footer } from "@/components/footer";

const inter = Inter({ subsets: ["latin"], display: "swap", variable: "--font-inter" });
const jetbrainsMono = JetBrains_Mono({ subsets: ["latin"], display: "swap", variable: "--font-jetbrains-mono" });

export const metadata: Metadata = {
  title: "AgenticPDF — AI-Native PDF Processing for TypeScript",
  description:
    "Streaming-first, AI-native PDF library with semantic chunking, canvas rendering, and zero runtime dependencies. Built for modern applications.",
  keywords: [
    "PDF",
    "TypeScript",
    "AI",
    "RAG",
    "streaming",
    "semantic chunking",
    "PDF viewer",
    "text extraction",
  ],
  openGraph: {
    title: "AgenticPDF",
    description: "AI-Native PDF Processing for TypeScript",
    type: "website",
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" data-theme="dark">
      <head>
        {/*
          The site is a static export (`output: 'export'`), so there is no
          server to send headers and Next's `headers()` config does not apply.
          A meta policy is what a static host will honour without being
          configured, and it travels with the files wherever they are served.

          Two honest limitations. `frame-ancestors`, `report-uri` and `sandbox`
          are ignored in meta form, so clickjacking protection still has to
          come from the host's own `X-Frame-Options` or CSP header. And
          `script-src` carries 'unsafe-inline' because a static export
          hydrates through inline bootstrap scripts and cannot use a nonce --
          there is no request to generate one per. What the policy still buys
          is real: no third-party script or style origin, `object-src 'none'`,
          `base-uri 'none'` against base-tag injection, and `connect-src`
          limited to this origin.
        */}
        <meta
          httpEquiv="Content-Security-Policy"
          content={[
            "default-src 'self'",
            "script-src 'self' 'unsafe-inline'",
            "style-src 'self' 'unsafe-inline'",
            "img-src 'self' data: blob:",
            "font-src 'self' data:",
            "connect-src 'self'",
            "object-src 'none'",
            "base-uri 'none'",
            "form-action 'self'",
          ].join('; ')}
        />
        <meta name="referrer" content="strict-origin-when-cross-origin" />
      </head>
      <body className={`${inter.variable} ${jetbrainsMono.variable} font-sans min-h-screen flex flex-col antialiased`}>
        <Nav />
        <main className="flex-1">{children}</main>
        <Footer />
      </body>
    </html>
  );
}
