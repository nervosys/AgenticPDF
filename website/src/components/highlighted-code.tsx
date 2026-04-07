import { highlight } from "@/lib/highlight";
import { CodeBlock } from "@/components/ui";

export async function HighlightedCode({
  code,
  language = "typescript",
  filename,
}: {
  code: string;
  language?: string;
  filename?: string;
}) {
  const html = await highlight(code, language);
  return <CodeBlock code={code} language={language} filename={filename} html={html} />;
}
