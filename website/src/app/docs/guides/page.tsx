import { HighlightedCode } from "@/components/highlighted-code";

const guides = [
  {
    id: "rag-pipeline",
    title: "Building a RAG Pipeline",
    desc: "Ingest PDFs into a vector database for retrieval-augmented generation.",
    code: `import { AgenticPDF } from 'agenticpdf';

async function ingestPDF(file: File, vectorStore: VectorStore) {
  const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

  try {
    for await (const chunk of pdf.streamSemanticChunks({
      strategy: 'semantic',
      maxChunkSize: 1000,
      preserveParagraphs: true,
      includeMetadata: true
    })) {
      const embedding = await embedModel.generate(chunk.content);

      await vectorStore.upsert({
        id: chunk.id,
        vector: Array.from(embedding),
        metadata: {
          source: file.name,
          pages: chunk.pageNumbers,
          type: chunk.type,
          confidence: chunk.metadata.confidence,
          keywords: chunk.metadata.keywords
        },
        content: chunk.content
      });
    }
  } finally {
    pdf.close();
  }
}`,
  },
  {
    id: "custom-embeddings",
    title: "Custom Embedding Provider",
    desc: "Integrate your own embedding model with AgenticPDF's AI features.",
    code: `import { AgenticPDF, EmbeddingProvider } from 'agenticpdf';

class OpenAIEmbeddings implements EmbeddingProvider {
  model = 'text-embedding-3-small';
  dimensions = 1536;

  async generate(text: string): Promise<Float32Array> {
    const res = await fetch('https://api.openai.com/v1/embeddings', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': \`Bearer \${process.env.OPENAI_API_KEY}\`
      },
      body: JSON.stringify({ model: this.model, input: text })
    });
    const json = await res.json();
    return new Float32Array(json.data[0].embedding);
  }

  async generateBatch(texts: string[]): Promise<Float32Array[]> {
    const res = await fetch('https://api.openai.com/v1/embeddings', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': \`Bearer \${process.env.OPENAI_API_KEY}\`
      },
      body: JSON.stringify({ model: this.model, input: texts })
    });
    const json = await res.json();
    return json.data.map((d: any) => new Float32Array(d.embedding));
  }
}

const ai = await pdf.getAIFeatures({
  embeddingProvider: new OpenAIEmbeddings(),
  enableStructuralAnalysis: true,
  enableSemanticChunking: true
});`,
  },
  {
    id: "streaming-large",
    title: "Streaming Large Documents",
    desc: "Process PDFs of any size with constant memory using async generators.",
    code: `import { AgenticPDF } from 'agenticpdf';

async function processLargeDocument(url: string) {
  const pdf = await AgenticPDF.fromUrl(url, {
    lazyLoad: true,
    maxMemoryUsage: 100 * 1024 * 1024,
    streamOptions: {
      chunkSize: 1024 * 1024, // 1MB stream chunks
      progressCallback: (p) => {
        console.log(\`\${p.currentOperation}: \${Math.round(
          (p.bytesRead / p.totalBytes) * 100
        )}%\`);
      }
    }
  });

  // Stream text — never loads full doc into memory
  let totalChars = 0;
  for await (const page of pdf.streamText()) {
    totalChars += page.text.length;
    console.log(\`Page \${page.pageNumber}: \${page.text.length} chars\`);
  }

  console.log(\`Total characters: \${totalChars}\`);
  pdf.close();
}`,
  },
  {
    id: "form-processing",
    title: "Form Processing",
    desc: "Extract form fields, fill them programmatically, and save the result.",
    code: `import { AgenticPDF } from 'agenticpdf';

async function processForm(file: File) {
  const pdf = await AgenticPDF.fromFile(file);

  // Read existing form data
  const fields = await pdf.getFormFields();
  for (const field of fields) {
    console.log(\`\${field.name} (\${field.type}): \${field.value}\`);
  }

  // Fill form fields
  await pdf.fillForm({
    'firstName': 'John',
    'lastName': 'Doe',
    'email': 'john@example.com',
    'agreesToTerms': true
  });

  // Save the filled PDF
  const blob = await pdf.save();
  downloadBlob(blob, 'filled-form.pdf');

  pdf.close();
}`,
  },
  {
    id: "multi-format-export",
    title: "Multi-Format Export",
    desc: "Export a single PDF to multiple formats for different consumers.",
    code: `import { AgenticPDF } from 'agenticpdf';

async function exportAll(file: File) {
  const pdf = await AgenticPDF.fromFile(file);

  // Plain text for search indexing
  const text = await pdf.exportAs('text', {
    includeMetadata: true
  });

  // Markdown for documentation
  const md = await pdf.exportAs('markdown', {
    includeImages: true,
    imageFormat: 'webp'
  });

  // HTML for web display
  const html = await pdf.exportAs('html', {
    includeImages: true
  });

  // JSON for programmatic access
  const json = await pdf.exportAs('json', {
    includeAnnotations: true,
    pageRange: { start: 1, end: 100 }
  });

  pdf.close();
  return { text, md, html, json };
}`,
  },
  {
    id: "agentic-ingestion",
    title: "Agentic Ingestion",
    desc: "Use the unified ingestion API to extract metadata, structure, and semantic chunks in a single call — ideal for LLM pipelines and function-calling agents.",
    code: `import { AgenticPDF } from 'agenticpdf';

// Single-call ingestion — everything in one object
const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });
const result = await pdf.ingest({
  chunkSize: 1000,
  includeText: true,
  includeStructure: true
});

console.log(result.metadata.title);
console.log(result.stats.totalChunks);

for (const chunk of result.chunks) {
  await vectorStore.add({
    content: chunk.content,
    metadata: { pages: chunk.pageNumbers, type: chunk.type }
  });
}
pdf.close();

// Or stream as NDJSON for real-time processing
const pdf2 = await AgenticPDF.fromFile(file);
for await (const line of pdf2.streamIngest({ chunkSize: 1000 })) {
  const record = JSON.parse(line);
  if (record.type === 'chunk') {
    process.stdout.write('.');
  }
}
pdf2.close();`,
  },
  {
    id: "agent-discovery",
    title: "AI Agent Discovery",
    desc: "Let AI agents programmatically discover AgenticPDF's capabilities.",
    code: `import { AgenticPDF } from 'agenticpdf';

// Full ontology for agent systems
const ontology = AgenticPDF.describe();
console.log(ontology['@context']); // JSON-LD context

// Capability map
const caps = AgenticPDF.getCapabilities();
for (const cap of caps) {
  console.log(\`[\${cap.category}] \${cap.name}: \${cap.description}\`);
}

// Method signatures for code generation
const methods = AgenticPDF.getMethodSignatures();

// Pre-built workflow templates
const workflows = AgenticPDF.getWorkflows();
for (const wf of workflows) {
  console.log(\`Workflow: \${wf.name}\`);
  for (const step of wf.steps) {
    console.log(\`  \${step.order}. \${step.description}\`);
  }
}

// Tool schemas for function-calling LLMs
const tools = AgenticPDF.getToolSchemas('openai');
// Pass directly to OpenAI chat completions API`,
  },
];

export default function GuidesPage() {
  return (
    <div className="mx-auto max-w-4xl px-4 sm:px-6 py-12">
      <h1 className="text-4xl font-bold tracking-tight mb-2" style={{ color: "var(--text)" }}>
        Guides
      </h1>
      <p className="mb-10" style={{ color: "var(--text-muted)" }}>
        Step-by-step guides for common AgenticPDF use cases.
      </p>

      <nav className="mb-12">
        <h2 className="text-sm font-semibold uppercase tracking-wider mb-3" style={{ color: "var(--text-muted)" }}>
          On this page
        </h2>
        <ul className="space-y-1">
          {guides.map(g => (
            <li key={g.id}>
              <a href={`#${g.id}`} className="text-sm hover:underline" style={{ color: "var(--accent)" }}>
                {g.title}
              </a>
            </li>
          ))}
        </ul>
      </nav>

      {guides.map(g => (
        <section key={g.id} id={g.id} className="mb-16">
          <h2 className="text-2xl font-bold mb-2" style={{ color: "var(--text)" }}>{g.title}</h2>
          <p className="mb-4" style={{ color: "var(--text-muted)" }}>{g.desc}</p>
          <HighlightedCode code={g.code} filename={`${g.id}.ts`} />
        </section>
      ))}
    </div>
  );
}
