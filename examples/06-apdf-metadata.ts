/**
 * aPDF (Agentic PDF) Metadata Examples
 *
 * This example demonstrates the aPDF format — a rich, machine-readable
 * metadata envelope for PDF documents optimized for:
 * - Agentic AI workflows (RAG, embedding pipelines, document agents)
 * - Research paper linking (HuggingFace, arXiv, DOI, ORCID)
 * - Typesetting and web display hints
 * - Structured export for downstream systems
 */

import AgenticPDF from '../agenticpdf';

// ============================================================================
// Example 1: Generate aPDF metadata from a research paper
// ============================================================================

async function generateAPDFFromPaper(file: File): Promise<void> {
  console.log('=== Example 1: aPDF Metadata from Research Paper ===\n');

  const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

  try {
    const apdf = await pdf.generateAPDFMetadata();

    // Core metadata
    console.log(`Title: ${apdf.metadata.title}`);
    console.log(`Type:  ${apdf['@type']}`);
    console.log(`Pages: ${apdf.metadata.pageCount}`);
    console.log(`Lang:  ${apdf.metadata.language}`);

    // Identifiers
    if (apdf.metadata.identifiers.doi) {
      console.log(`DOI:   ${apdf.metadata.identifiers.doi}`);
    }
    if (apdf.metadata.identifiers.arxivId) {
      console.log(`arXiv: ${apdf.metadata.identifiers.arxivId}`);
    }
    if (apdf.metadata.identifiers.huggingFaceId) {
      console.log(`HF:    https://huggingface.co/papers/${apdf.metadata.identifiers.huggingFaceId}`);
    }

    // Authors
    console.log(`\nAuthors (${apdf.authors.length}):`);
    for (const author of apdf.authors) {
      const orcid = author.orcid ? ` (ORCID: ${author.orcid})` : '';
      console.log(`  - ${author.name}${orcid}`);
    }

    // Linked artifacts
    if (apdf.artifacts.length > 0) {
      console.log(`\nLinked Artifacts (${apdf.artifacts.length}):`);
      for (const artifact of apdf.artifacts) {
        console.log(`  [${artifact.type}] ${artifact.name} → ${artifact.url}`);
      }
    }

    // AI content stats
    console.log('\nAI Content:');
    console.log(`  Chunks:    ${apdf.aiContent.chunks.length}`);
    console.log(`  Tokens:    ${apdf.aiContent.stats.tokenCount}`);
    console.log(`  Keywords:  ${apdf.aiContent.keywords.slice(0, 5).join(', ')}`);
    if (apdf.aiContent.summary) {
      console.log(`  Summary:   ${apdf.aiContent.summary.substring(0, 120)}...`);
    }

    // Structure
    console.log('\nDocument Structure:');
    console.log(`  Sections:  ${apdf.structure.sections.length}`);
    console.log(`  Tables:    ${apdf.structure.tables.length}`);
    console.log(`  Figures:   ${apdf.structure.figures.length}`);
    console.log(`  Equations: ${apdf.structure.equations.length}`);
    console.log(`  Bib refs:  ${apdf.structure.bibliography.length}`);

    // Display hints
    console.log('\nDisplay Hints:');
    console.log(`  Layout:    ${apdf.display.readingOrder}`);
    console.log(`  Has math:  ${apdf.display.hasMath}`);
    console.log(`  Theme:     ${apdf.display.suggestedTheme}`);

  } finally {
    pdf.close();
  }
}

// ============================================================================
// Example 2: Export as aPDF JSON
// ============================================================================

async function exportAsAPDF(file: File): Promise<string> {
  console.log('\n=== Example 2: Export as aPDF JSON ===\n');

  const pdf = await AgenticPDF.fromFile(file);

  try {
    // Export via the standard exportAs() API
    const apdfJson = await pdf.exportAs('apdf');
    const jsonStr = typeof apdfJson === 'string' ? apdfJson : await apdfJson.text();

    console.log(`aPDF JSON size: ${(jsonStr.length / 1024).toFixed(1)} KB`);
    console.log('Preview (first 500 chars):');
    console.log(jsonStr.substring(0, 500));
    console.log('...\n');

    return jsonStr;
  } finally {
    pdf.close();
  }
}

// ============================================================================
// Example 3: Build a HuggingFace paper-page linking index
// ============================================================================

interface PaperIndex {
  arxivId: string;
  title: string;
  authors: string[];
  models: string[];
  datasets: string[];
  spaces: string[];
  codeRepos: string[];
}

async function buildHuggingFacePaperIndex(files: File[]): Promise<PaperIndex[]> {
  console.log('\n=== Example 3: HuggingFace Paper-Page Index ===\n');

  const index: PaperIndex[] = [];

  for (const file of files) {
    const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

    try {
      const apdf = await pdf.generateAPDFMetadata();

      if (!apdf.metadata.identifiers.arxivId) {
        console.log(`  Skipping "${apdf.metadata.title}" — no arXiv ID found`);
        continue;
      }

      const entry: PaperIndex = {
        arxivId: apdf.metadata.identifiers.arxivId,
        title: apdf.metadata.title,
        authors: apdf.authors.map(a => a.name),
        models: apdf.artifacts.filter(a => a.type === 'model').map(a => a.huggingFaceRepo!).filter(Boolean),
        datasets: apdf.artifacts.filter(a => a.type === 'dataset').map(a => a.huggingFaceRepo!).filter(Boolean),
        spaces: apdf.artifacts.filter(a => a.type === 'space').map(a => a.huggingFaceRepo!).filter(Boolean),
        codeRepos: apdf.artifacts.filter(a => a.type === 'code').map(a => a.url).filter(Boolean),
      };

      index.push(entry);

      console.log(`  [${entry.arxivId}] ${entry.title}`);
      console.log(`    Models:   ${entry.models.length ? entry.models.join(', ') : '(none)'}`);
      console.log(`    Datasets: ${entry.datasets.length ? entry.datasets.join(', ') : '(none)'}`);
      console.log(`    Spaces:   ${entry.spaces.length ? entry.spaces.join(', ') : '(none)'}`);
      console.log(`    Code:     ${entry.codeRepos.length ? entry.codeRepos.join(', ') : '(none)'}`);
    } finally {
      pdf.close();
    }
  }

  console.log(`\nIndexed ${index.length} papers with artifact links.`);
  return index;
}

// ============================================================================
// Example 4: aPDF-powered RAG ingestion pipeline
// ============================================================================

interface RAGDocument {
  id: string;
  content: string;
  metadata: Record<string, unknown>;
}

async function apdfRAGPipeline(file: File): Promise<RAGDocument[]> {
  console.log('\n=== Example 4: aPDF-Powered RAG Pipeline ===\n');

  const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

  try {
    const apdf = await pdf.generateAPDFMetadata();

    // Use aPDF's pre-computed semantic chunks as RAG documents
    const ragDocs: RAGDocument[] = apdf.aiContent.chunks.map(chunk => ({
      id: chunk.id,
      content: chunk.content,
      metadata: {
        // Carry forward rich aPDF metadata into every chunk
        documentTitle: apdf.metadata.title,
        doi: apdf.metadata.identifiers.doi,
        arxivId: apdf.metadata.identifiers.arxivId,
        authors: apdf.authors.map(a => a.name),
        pageNumbers: chunk.pageNumbers,
        chunkType: chunk.chunkType,
        importance: chunk.importance,
        keywords: chunk.keywords,
        // Artifact links available for citation grounding
        artifacts: apdf.artifacts.map(a => ({ type: a.type, url: a.url })),
      },
    }));

    console.log(`Generated ${ragDocs.length} RAG documents with enriched metadata.`);
    console.log('\nSample document:');
    console.log(JSON.stringify(ragDocs[0], null, 2).substring(0, 600));

    return ragDocs;
  } finally {
    pdf.close();
  }
}

// ============================================================================
// Example 5: aPDF for document comparison / deduplication
// ============================================================================

async function compareDocuments(fileA: File, fileB: File): Promise<void> {
  console.log('\n=== Example 5: Document Comparison via aPDF ===\n');

  const pdfA = await AgenticPDF.fromFile(fileA, { lazyLoad: true });
  const pdfB = await AgenticPDF.fromFile(fileB, { lazyLoad: true });

  try {
    const [apdfA, apdfB] = await Promise.all([
      pdfA.generateAPDFMetadata(),
      pdfB.generateAPDFMetadata(),
    ]);

    // Identity comparison
    const sameDoc = apdfA.metadata.identifiers.doi === apdfB.metadata.identifiers.doi
      && apdfA.metadata.identifiers.doi !== undefined;

    console.log(`Document A: "${apdfA.metadata.title}"`);
    console.log(`Document B: "${apdfB.metadata.title}"`);
    console.log(`Same DOI:   ${sameDoc ? 'YES — likely same document' : 'No'}`);

    // Structural similarity
    const sectionsA = new Set(apdfA.structure.sections.filter(s => s.title).map(s => s.title!.toLowerCase()));
    const sectionsB = new Set(apdfB.structure.sections.filter(s => s.title).map(s => s.title!.toLowerCase()));
    const shared = [...sectionsA].filter(s => sectionsB.has(s));
    const union = new Set([...sectionsA, ...sectionsB]);
    const jaccard = union.size > 0 ? shared.length / union.size : 0;
    console.log(`\nSection overlap (Jaccard): ${(jaccard * 100).toFixed(1)}%`);
    console.log(`  Shared sections: ${shared.join(', ') || '(none)'}`);

    // Keyword overlap
    const kwA = new Set(apdfA.aiContent.keywords.map(k => k.toLowerCase()));
    const kwB = new Set(apdfB.aiContent.keywords.map(k => k.toLowerCase()));
    const sharedKw = [...kwA].filter(k => kwB.has(k));
    console.log(`\nKeyword overlap: ${sharedKw.length} shared — ${sharedKw.slice(0, 10).join(', ')}`);

    // Shared artifact references
    const urlsA = new Set(apdfA.artifacts.map(a => a.url));
    const urlsB = new Set(apdfB.artifacts.map(a => a.url));
    const sharedUrls = [...urlsA].filter(u => urlsB.has(u));
    if (sharedUrls.length > 0) {
      console.log(`\nShared artifact links: ${sharedUrls.join(', ')}`);
    }

  } finally {
    pdfA.close();
    pdfB.close();
  }
}

// ============================================================================
// Example 6: Serialize aPDF as a JSON-LD document for the semantic web
// ============================================================================

async function generateJsonLD(file: File): Promise<object> {
  console.log('\n=== Example 6: JSON-LD Linked Data Export ===\n');

  const pdf = await AgenticPDF.fromFile(file);

  try {
    const apdf = await pdf.generateAPDFMetadata();

    // The aPDF envelope is already JSON-LD compatible
    const jsonLd = {
      '@context': apdf['@context'],
      '@type': apdf['@type'],
      '@id': apdf.metadata.identifiers.doi
        ? `https://doi.org/${apdf.metadata.identifiers.doi}`
        : `urn:apdf:${apdf.id}`,
      name: apdf.metadata.title,
      abstract: apdf.metadata.abstract,
      datePublished: apdf.metadata.datePublished,
      inLanguage: apdf.metadata.language,
      author: apdf.authors.map(a => ({
        '@type': 'Person',
        name: a.name,
        givenName: a.givenName,
        familyName: a.familyName,
        ...(a.orcid ? { '@id': `https://orcid.org/${a.orcid}` } : {}),
      })),
      identifier: [
        ...(apdf.metadata.identifiers.doi
          ? [{ '@type': 'PropertyValue', propertyID: 'DOI', value: apdf.metadata.identifiers.doi }]
          : []),
        ...(apdf.metadata.identifiers.arxivId
          ? [{ '@type': 'PropertyValue', propertyID: 'arXiv', value: apdf.metadata.identifiers.arxivId }]
          : []),
      ],
      about: apdf.metadata.subjects.map(s => ({
        '@type': 'DefinedTerm',
        termCode: s.term,
        inDefinedTermSet: s.scheme,
      })),
      hasPart: apdf.artifacts.map(a => ({
        '@type': a.type === 'code' ? 'SoftwareSourceCode' : 'CreativeWork',
        name: a.name,
        url: a.url,
      })),
    };

    console.log(JSON.stringify(jsonLd, null, 2).substring(0, 800));
    console.log('...\n');

    return jsonLd;
  } finally {
    pdf.close();
  }
}

// ============================================================================
// Runner
// ============================================================================

export async function apdfExamples(file: File, fileB?: File): Promise<void> {
  await generateAPDFFromPaper(file);
  await exportAsAPDF(file);
  await buildHuggingFacePaperIndex([file]);
  await apdfRAGPipeline(file);
  if (fileB) {
    await compareDocuments(file, fileB);
  }
  await generateJsonLD(file);

  console.log('\n=== All aPDF examples completed ===');
}
