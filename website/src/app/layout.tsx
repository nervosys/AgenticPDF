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
      <body className={`${inter.variable} ${jetbrainsMono.variable} font-sans min-h-screen flex flex-col antialiased`}>
        <Nav />
        <main className="flex-1">{children}</main>
        <Footer />
      </body>
    </html>
  );
}
