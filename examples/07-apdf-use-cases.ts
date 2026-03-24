/**
 * aPDF (Agentic PDF) Real-World Use Cases
 *
 * Practical examples demonstrating how the aPDF format enables
 * AI-native document workflows across research, compliance,
 * knowledge management, and multi-agent systems.
 */

import AgenticPDF from '../agenticpdf';
import type {
  APDFBibEntry,
  APDFArtifact,
} from '../agenticpdf';

// ============================================================================
// Use Case 1: Multi-Agent Research Assistant
//
// An orchestrator agent distributes aPDF chunks to specialist sub-agents
// (summarizer, fact-checker, citation-linker) and merges their outputs.
// ============================================================================

interface AgentMessage {
  role: 'summarizer' | 'fact-checker' | 'citation-linker';
  chunkId: string;
  result: Record<string, unknown>;
}

async function multiAgentResearchAssistant(file: File): Promise<void> {
  console.log('=== Use Case 1: Multi-Agent Research Assistant ===\n');

  const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

  try {
    const apdf = await pdf.generateAPDFMetadata();

    console.log(`Document: "${apdf.metadata.title}"`);
    console.log(`Chunks:   ${apdf.aiContent.chunks.length}`);
    console.log(`Bib refs: ${apdf.structure.bibliography.length}\n`);

    // Build a citation lookup from the bibliography
    const bibByTitle = new Map<string, APDFBibEntry>();
    for (const bib of apdf.structure.bibliography) {
      bibByTitle.set(bib.title.toLowerCase(), bib);
    }

    // Dispatch chunks to simulated specialist agents
    const agentResults: AgentMessage[] = [];

    for (const chunk of apdf.aiContent.chunks) {
      // Summarizer agent — condenses each chunk
      agentResults.push({
        role: 'summarizer',
        chunkId: chunk.id,
        result: {
          summary: chunk.content.substring(0, 120).replace(/\s+/g, ' ').trim() + '…',
          tokenCount: chunk.tokenCount,
          importance: chunk.importance,
        },
      });

      // Citation-linker agent — resolves in-text citations to DOIs
      const citationRefs = chunk.content.match(/\[(\d+(?:,\s*\d+)*)\]/g) || [];
      const resolvedCitations: { ref: string; doi?: string; arxivId?: string }[] = [];
      for (const ref of citationRefs) {
        const nums = ref.replace(/[[\]]/g, '').split(',').map(n => n.trim());
        for (const num of nums) {
          const bibEntry = apdf.structure.bibliography[parseInt(num) - 1];
          if (bibEntry) {
            resolvedCitations.push({
              ref: `[${num}]`,
              doi: bibEntry.doi,
              arxivId: bibEntry.arxivId,
            });
          }
        }
      }

      if (resolvedCitations.length > 0) {
        agentResults.push({
          role: 'citation-linker',
          chunkId: chunk.id,
          result: { citations: resolvedCitations },
        });
      }

      // Fact-checker agent — flags claims that reference specific numbers
      const numericClaims = chunk.content.match(/\d+(?:\.\d+)?%|\d+(?:\.\d+)?x\b/g);
      if (numericClaims && numericClaims.length > 0) {
        agentResults.push({
          role: 'fact-checker',
          chunkId: chunk.id,
          result: {
            flagged: true,
            claims: numericClaims.slice(0, 5),
            pageNumbers: chunk.pageNumbers,
          },
        });
      }
    }

    // Merge agent outputs
    const summaries = agentResults.filter(r => r.role === 'summarizer');
    const citations = agentResults.filter(r => r.role === 'citation-linker');
    const factChecks = agentResults.filter(r => r.role === 'fact-checker');

    console.log('Agent Results:');
    console.log(`  Summaries:      ${summaries.length}`);
    console.log(`  Citation links: ${citations.length} chunks with resolved references`);
    console.log(`  Fact-checks:    ${factChecks.length} chunks flagged for verification`);

    if (factChecks.length > 0) {
      const first = factChecks[0].result as { claims: string[]; pageNumbers: number[] };
      console.log(`\n  Sample flagged claims (chunk ${factChecks[0].chunkId}):`);
      console.log(`    Claims: ${(first.claims as string[]).join(', ')}`);
      console.log(`    Pages:  ${(first.pageNumbers as number[]).join(', ')}`);
    }

  } finally {
    pdf.close();
  }
}

// ============================================================================
// Use Case 2: Compliance & Policy Audit
//
// Given a set of required policy topics, check whether a document covers them
// using aPDF keyword and section analysis. Produces a compliance report.
// ============================================================================

interface ComplianceResult {
  topic: string;
  covered: boolean;
  evidence: { sectionTitle?: string; keywords: string[]; pages: number[] }[];
}

async function compliancePolicyAudit(
  file: File,
  requiredTopics: string[] = [
    'data privacy',
    'retention policy',
    'access control',
    'encryption',
    'incident response',
    'audit logging',
  ],
): Promise<ComplianceResult[]> {
  console.log('\n=== Use Case 2: Compliance & Policy Audit ===\n');

  const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

  try {
    const apdf = await pdf.generateAPDFMetadata();

    console.log(`Document: "${apdf.metadata.title}"`);
    console.log(`Sections: ${apdf.structure.sections.length}`);
    console.log(`Keywords: ${apdf.aiContent.keywords.length}`);
    console.log(`Required topics: ${requiredTopics.length}\n`);

    const results: ComplianceResult[] = requiredTopics.map(topic => {
      const topicTerms = topic.toLowerCase().split(/\s+/);
      const evidence: ComplianceResult['evidence'] = [];

      // Search sections for topic coverage
      for (const section of apdf.structure.sections) {
        if (!section.title) continue;
        const titleLower = section.title.toLowerCase();
        if (topicTerms.some(term => titleLower.includes(term))) {
          evidence.push({
            sectionTitle: section.title,
            keywords: topicTerms.filter(t => titleLower.includes(t)),
            pages: [section.pageStart],
          });
        }
      }

      // Search chunks for keyword matches
      for (const chunk of apdf.aiContent.chunks) {
        const contentLower = chunk.content.toLowerCase();
        const matchedTerms = topicTerms.filter(t => contentLower.includes(t));
        if (matchedTerms.length >= Math.ceil(topicTerms.length / 2)) {
          evidence.push({
            keywords: matchedTerms,
            pages: chunk.pageNumbers,
          });
        }
      }

      return {
        topic,
        covered: evidence.length > 0,
        evidence: evidence.slice(0, 3), // Top 3 evidence items
      };
    });

    // Report
    const covered = results.filter(r => r.covered).length;
    console.log(`Compliance Score: ${covered}/${results.length}\n`);

    for (const result of results) {
      const status = result.covered ? '✅' : '❌';
      console.log(`  ${status} ${result.topic}`);
      if (result.evidence.length > 0) {
        const ev = result.evidence[0];
        const loc = ev.sectionTitle
          ? `Section: "${ev.sectionTitle}" (p${ev.pages[0]})`
          : `Pages: ${ev.pages.join(', ')}`;
        console.log(`     ↳ ${loc}`);
      }
    }

    return results;
  } finally {
    pdf.close();
  }
}

// ============================================================================
// Use Case 3: Knowledge Graph Builder
//
// Constructs a directed graph of entities, citations, and artifact
// relationships from the aPDF envelope — ready for Neo4j / NetworkX.
// ============================================================================

interface KnowledgeNode {
  id: string;
  type: 'paper' | 'author' | 'model' | 'dataset' | 'entity' | 'topic';
  label: string;
  properties: Record<string, unknown>;
}

interface KnowledgeEdge {
  source: string;
  target: string;
  relation: string;
}

interface KnowledgeGraph {
  nodes: KnowledgeNode[];
  edges: KnowledgeEdge[];
}

async function buildKnowledgeGraph(files: File[]): Promise<KnowledgeGraph> {
  console.log('\n=== Use Case 3: Knowledge Graph Builder ===\n');

  const nodes = new Map<string, KnowledgeNode>();
  const edges: KnowledgeEdge[] = [];

  for (const file of files) {
    const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

    try {
      const apdf = await pdf.generateAPDFMetadata();
      const paperId = apdf.metadata.identifiers.doi || `apdf:${apdf.id}`;

      // Paper node
      nodes.set(paperId, {
        id: paperId,
        type: 'paper',
        label: apdf.metadata.title,
        properties: {
          doi: apdf.metadata.identifiers.doi,
          arxivId: apdf.metadata.identifiers.arxivId,
          year: apdf.metadata.datePublished?.substring(0, 4),
          venue: apdf.metadata.venue,
          pageCount: apdf.metadata.pageCount,
        },
      });

      // Author nodes and edges
      for (const author of apdf.authors) {
        const authorId = author.orcid || `author:${author.name.toLowerCase().replace(/\s+/g, '-')}`;
        if (!nodes.has(authorId)) {
          nodes.set(authorId, {
            id: authorId,
            type: 'author',
            label: author.name,
            properties: {
              orcid: author.orcid,
              affiliation: author.affiliations?.[0]?.name,
              isCorresponding: author.isCorresponding,
            },
          });
        }
        edges.push({ source: authorId, target: paperId, relation: 'authored' });
      }

      // Artifact nodes and edges
      for (const artifact of apdf.artifacts) {
        const artifactId = artifact.huggingFaceRepo || artifact.githubRepo || artifact.url;
        const nodeType = artifact.type === 'model' ? 'model'
          : artifact.type === 'dataset' ? 'dataset' : 'entity';

        if (!nodes.has(artifactId)) {
          nodes.set(artifactId, {
            id: artifactId,
            type: nodeType,
            label: artifact.name,
            properties: {
              url: artifact.url,
              framework: artifact.framework,
              task: artifact.task,
            },
          });
        }
        edges.push({ source: paperId, target: artifactId, relation: artifact.relation });
      }

      // Citation edges from bibliography
      for (const bib of apdf.structure.bibliography) {
        const citedId = bib.doi || bib.arxivId || `bib:${bib.id}`;
        if (!nodes.has(citedId)) {
          nodes.set(citedId, {
            id: citedId,
            type: 'paper',
            label: bib.title,
            properties: { doi: bib.doi, arxivId: bib.arxivId, year: bib.year },
          });
        }
        edges.push({ source: paperId, target: citedId, relation: 'cites' });
      }

      // Topic nodes from subjects
      for (const subject of apdf.metadata.subjects) {
        const topicId = `topic:${subject.scheme}:${subject.term}`;
        if (!nodes.has(topicId)) {
          nodes.set(topicId, {
            id: topicId,
            type: 'topic',
            label: subject.label || subject.term,
            properties: { scheme: subject.scheme },
          });
        }
        edges.push({ source: paperId, target: topicId, relation: 'about' });
      }

      // Entity nodes from NER
      if (apdf.aiContent.entities) {
        for (const entity of apdf.aiContent.entities) {
          if (entity.confidence < 0.7) continue;
          const entityId = `entity:${entity.type}:${entity.text.toLowerCase().replace(/\s+/g, '-')}`;
          if (!nodes.has(entityId)) {
            nodes.set(entityId, {
              id: entityId,
              type: 'entity',
              label: entity.text,
              properties: { entityType: entity.type, confidence: entity.confidence },
            });
          }
          edges.push({ source: paperId, target: entityId, relation: 'mentions' });
        }
      }

    } finally {
      pdf.close();
    }
  }

  const graph: KnowledgeGraph = {
    nodes: [...nodes.values()],
    edges,
  };

  console.log(`Graph constructed:`);
  console.log(`  Nodes: ${graph.nodes.length}`);
  console.log(`  Edges: ${graph.edges.length}`);
  console.log(`  By type:`);

  const typeCounts = new Map<string, number>();
  for (const node of graph.nodes) {
    typeCounts.set(node.type, (typeCounts.get(node.type) || 0) + 1);
  }
  for (const [type, count] of typeCounts) {
    console.log(`    ${type}: ${count}`);
  }

  return graph;
}

// ============================================================================
// Use Case 4: Smart Document Router
//
// Classifies incoming PDFs by type, complexity, and topic using aPDF metadata,
// then routes them to the appropriate processing pipeline or human reviewer.
// ============================================================================

interface RoutingDecision {
  filename: string;
  documentType: string;
  complexity: 'simple' | 'moderate' | 'complex';
  route: string;
  reason: string;
  priority: number;
}

async function smartDocumentRouter(files: File[]): Promise<RoutingDecision[]> {
  console.log('\n=== Use Case 4: Smart Document Router ===\n');

  const decisions: RoutingDecision[] = [];

  for (const file of files) {
    const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

    try {
      const apdf = await pdf.generateAPDFMetadata();

      // Assess complexity
      const chunkCount = apdf.aiContent.chunks.length;
      const hasTables = apdf.structure.tables.length > 0;
      const hasEquations = apdf.structure.equations.length > 0;
      const hasBibliography = apdf.structure.bibliography.length > 5;
      const isMultiColumn = apdf.display.readingOrder === 'multi-column';

      const complexityScore =
        (chunkCount > 30 ? 2 : chunkCount > 10 ? 1 : 0) +
        (hasTables ? 1 : 0) +
        (hasEquations ? 1 : 0) +
        (hasBibliography ? 1 : 0) +
        (isMultiColumn ? 1 : 0);

      const complexity: RoutingDecision['complexity'] =
        complexityScore >= 4 ? 'complex' : complexityScore >= 2 ? 'moderate' : 'simple';

      // Determine topic from keywords and subjects
      const allTerms = [
        ...apdf.aiContent.keywords,
        ...apdf.metadata.subjects.map(s => s.label || s.term),
      ].map(t => t.toLowerCase());

      const isLegal = allTerms.some(t => /\b(legal|compliance|regulation|gdpr|hipaa|contract)\b/.test(t));
      const isMedical = allTerms.some(t => /\b(clinical|patient|medical|diagnosis|treatment)\b/.test(t));
      const isFinancial = allTerms.some(t => /\b(financial|revenue|earnings|fiscal|audit)\b/.test(t));
      const isResearch = hasBibliography || apdf.metadata.identifiers.doi !== undefined;

      // Route decision
      let route: string;
      let reason: string;
      let priority: number;

      if (isMedical && complexity === 'complex') {
        route = 'medical-specialist-review';
        reason = 'Complex medical document requires specialist review';
        priority = 1;
      } else if (isLegal) {
        route = 'legal-compliance-pipeline';
        reason = 'Legal/compliance content detected';
        priority = 2;
      } else if (isFinancial) {
        route = 'financial-analysis-pipeline';
        reason = 'Financial content detected';
        priority = 2;
      } else if (isResearch && complexity !== 'simple') {
        route = 'research-ai-analysis';
        reason = `Research paper (${apdf.structure.bibliography.length} refs, ${apdf.structure.tables.length} tables)`;
        priority = 3;
      } else if (complexity === 'simple') {
        route = 'auto-extract-and-index';
        reason = 'Simple document — fully automated processing';
        priority = 5;
      } else {
        route = 'general-ai-analysis';
        reason = `${apdf['@type']} with ${complexity} complexity`;
        priority = 4;
      }

      decisions.push({
        filename: file.name,
        documentType: apdf['@type'],
        complexity,
        route,
        reason,
        priority,
      });

    } finally {
      pdf.close();
    }
  }

  // Sort by priority
  decisions.sort((a, b) => a.priority - b.priority);

  console.log(`Routed ${decisions.length} documents:\n`);
  for (const d of decisions) {
    console.log(`  📄 ${d.filename}`);
    console.log(`     Type: ${d.documentType} | Complexity: ${d.complexity}`);
    console.log(`     → Route: ${d.route}`);
    console.log(`     Reason: ${d.reason}\n`);
  }

  return decisions;
}

// ============================================================================
// Use Case 5: Changelog / Version Diff Between Document Revisions
//
// Compares two revisions of the same document using aPDF structural metadata
// to identify added, removed, and modified sections.
// ============================================================================

interface DocumentDiff {
  titleChanged: boolean;
  addedSections: string[];
  removedSections: string[];
  modifiedSections: string[];
  addedReferences: APDFBibEntry[];
  removedReferences: APDFBibEntry[];
  chunkDelta: number;
  keywordsDelta: { added: string[]; removed: string[] };
}

async function diffDocumentRevisions(oldFile: File, newFile: File): Promise<DocumentDiff> {
  console.log('\n=== Use Case 5: Document Revision Diff ===\n');

  const pdfOld = await AgenticPDF.fromFile(oldFile, { lazyLoad: true });
  const pdfNew = await AgenticPDF.fromFile(newFile, { lazyLoad: true });

  try {
    const [apdfOld, apdfNew] = await Promise.all([
      pdfOld.generateAPDFMetadata(),
      pdfNew.generateAPDFMetadata(),
    ]);

    console.log(`Old: "${apdfOld.metadata.title}" (${apdfOld.metadata.pageCount} pages)`);
    console.log(`New: "${apdfNew.metadata.title}" (${apdfNew.metadata.pageCount} pages)\n`);

    // Section titles
    const oldSections = new Set(
      apdfOld.structure.sections.filter(s => s.title).map(s => s.title!),
    );
    const newSections = new Set(
      apdfNew.structure.sections.filter(s => s.title).map(s => s.title!),
    );

    const addedSections = [...newSections].filter(s => !oldSections.has(s));
    const removedSections = [...oldSections].filter(s => !newSections.has(s));
    const modifiedSections = [...newSections].filter(s => oldSections.has(s));

    // Bibliography diff
    const oldBibTitles = new Set(apdfOld.structure.bibliography.map(b => b.title.toLowerCase()));
    const newBibTitles = new Set(apdfNew.structure.bibliography.map(b => b.title.toLowerCase()));

    const addedReferences = apdfNew.structure.bibliography
      .filter(b => !oldBibTitles.has(b.title.toLowerCase()));
    const removedReferences = apdfOld.structure.bibliography
      .filter(b => !newBibTitles.has(b.title.toLowerCase()));

    // Keywords diff
    const oldKeywords = new Set(apdfOld.aiContent.keywords.map(k => k.toLowerCase()));
    const newKeywords = new Set(apdfNew.aiContent.keywords.map(k => k.toLowerCase()));

    const diff: DocumentDiff = {
      titleChanged: apdfOld.metadata.title !== apdfNew.metadata.title,
      addedSections,
      removedSections,
      modifiedSections,
      addedReferences,
      removedReferences,
      chunkDelta: apdfNew.aiContent.chunks.length - apdfOld.aiContent.chunks.length,
      keywordsDelta: {
        added: [...newKeywords].filter(k => !oldKeywords.has(k)),
        removed: [...oldKeywords].filter(k => !newKeywords.has(k)),
      },
    };

    // Report
    if (diff.titleChanged) {
      console.log(`⚠️  Title changed: "${apdfOld.metadata.title}" → "${apdfNew.metadata.title}"`);
    }

    console.log(`\nSections:`);
    console.log(`  Added:     ${diff.addedSections.length}${diff.addedSections.length > 0 ? ' — ' + diff.addedSections.join(', ') : ''}`);
    console.log(`  Removed:   ${diff.removedSections.length}${diff.removedSections.length > 0 ? ' — ' + diff.removedSections.join(', ') : ''}`);
    console.log(`  Unchanged: ${diff.modifiedSections.length}`);

    console.log(`\nReferences:`);
    console.log(`  Added:   ${diff.addedReferences.length}`);
    console.log(`  Removed: ${diff.removedReferences.length}`);

    console.log(`\nContent delta: ${diff.chunkDelta > 0 ? '+' : ''}${diff.chunkDelta} chunks`);

    if (diff.keywordsDelta.added.length > 0) {
      console.log(`  New keywords: ${diff.keywordsDelta.added.slice(0, 10).join(', ')}`);
    }
    if (diff.keywordsDelta.removed.length > 0) {
      console.log(`  Dropped keywords: ${diff.keywordsDelta.removed.slice(0, 10).join(', ')}`);
    }

    return diff;
  } finally {
    pdfOld.close();
    pdfNew.close();
  }
}

// ============================================================================
// Use Case 6: Automated Literature Survey
//
// Processes a collection of papers, groups them by topic cluster, and
// generates a structured survey outline with per-cluster summaries.
// ============================================================================

interface SurveyCluster {
  topic: string;
  papers: { title: string; doi?: string; year?: string; keyContribution: string }[];
  sharedKeywords: string[];
}

async function automatedLiteratureSurvey(files: File[]): Promise<SurveyCluster[]> {
  console.log('\n=== Use Case 6: Automated Literature Survey ===\n');

  // Phase 1: Extract aPDF metadata from all papers
  interface PaperInfo {
    title: string;
    doi?: string;
    year?: string;
    keywords: Set<string>;
    subjects: string[];
    summary?: string;
    artifacts: APDFArtifact[];
  }

  const papers: PaperInfo[] = [];

  for (const file of files) {
    const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });
    try {
      const apdf = await pdf.generateAPDFMetadata();
      papers.push({
        title: apdf.metadata.title,
        doi: apdf.metadata.identifiers.doi,
        year: apdf.metadata.datePublished?.substring(0, 4),
        keywords: new Set(apdf.aiContent.keywords.map(k => k.toLowerCase())),
        subjects: apdf.metadata.subjects.map(s => s.term),
        summary: apdf.aiContent.summary,
        artifacts: apdf.artifacts,
      });
    } finally {
      pdf.close();
    }
  }

  console.log(`Processed ${papers.length} papers.\n`);

  // Phase 2: Cluster papers by keyword overlap (simple greedy clustering)
  const assigned = new Set<number>();
  const clusters: SurveyCluster[] = [];

  for (let i = 0; i < papers.length; i++) {
    if (assigned.has(i)) continue;
    assigned.add(i);

    const cluster: number[] = [i];
    const clusterKeywords = new Set(papers[i].keywords);

    // Find papers with significant keyword overlap
    for (let j = i + 1; j < papers.length; j++) {
      if (assigned.has(j)) continue;
      const overlap = [...papers[j].keywords].filter(k => clusterKeywords.has(k));
      if (overlap.length >= 2) {
        cluster.push(j);
        assigned.add(j);
        for (const k of papers[j].keywords) clusterKeywords.add(k);
      }
    }

    // Determine shared keywords across cluster members
    const keywordCounts = new Map<string, number>();
    for (const idx of cluster) {
      for (const kw of papers[idx].keywords) {
        keywordCounts.set(kw, (keywordCounts.get(kw) || 0) + 1);
      }
    }
    const sharedKeywords = [...keywordCounts.entries()]
      .filter(([, count]) => count > 1)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 8)
      .map(([kw]) => kw);

    // Topic label from most frequent shared keyword or first paper's subject
    const topic = sharedKeywords[0] || papers[cluster[0]].subjects[0] || 'General';

    clusters.push({
      topic,
      papers: cluster.map(idx => ({
        title: papers[idx].title,
        doi: papers[idx].doi,
        year: papers[idx].year,
        keyContribution: papers[idx].summary?.substring(0, 150) || '(no summary)',
      })),
      sharedKeywords,
    });
  }

  // Report
  console.log(`Found ${clusters.length} topic cluster(s):\n`);
  for (const cluster of clusters) {
    console.log(`📚 ${cluster.topic.toUpperCase()}`);
    console.log(`   Shared keywords: ${cluster.sharedKeywords.join(', ') || '(none)'}`);
    for (const paper of cluster.papers) {
      const doi = paper.doi ? ` [${paper.doi}]` : '';
      const year = paper.year ? ` (${paper.year})` : '';
      console.log(`   • ${paper.title}${year}${doi}`);
    }
    console.log();
  }

  return clusters;
}

// ============================================================================
// Use Case 7: aPDF-to-Markdown Publishing Pipeline
//
// Converts an aPDF document into a publish-ready Markdown file with
// structured frontmatter, linked citations, and figure placeholders.
// ============================================================================

async function apdfToMarkdownPublisher(file: File): Promise<string> {
  console.log('\n=== Use Case 7: aPDF → Markdown Publishing Pipeline ===\n');

  const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

  try {
    const apdf = await pdf.generateAPDFMetadata();

    const lines: string[] = [];

    // YAML frontmatter
    lines.push('---');
    lines.push(`title: "${apdf.metadata.title.replace(/"/g, '\\"')}"`);
    if (apdf.metadata.datePublished) {
      lines.push(`date: ${apdf.metadata.datePublished.substring(0, 10)}`);
    }
    lines.push(`authors:`);
    for (const author of apdf.authors) {
      lines.push(`  - name: "${author.name}"`);
      if (author.orcid) lines.push(`    orcid: "${author.orcid}"`);
      if (author.affiliations?.[0]) lines.push(`    affiliation: "${author.affiliations[0].name}"`);
    }
    if (apdf.metadata.identifiers.doi) {
      lines.push(`doi: "${apdf.metadata.identifiers.doi}"`);
    }
    if (apdf.metadata.identifiers.arxivId) {
      lines.push(`arxiv: "${apdf.metadata.identifiers.arxivId}"`);
    }
    lines.push(`tags: [${apdf.aiContent.keywords.slice(0, 10).map(k => `"${k}"`).join(', ')}]`);
    lines.push(`apdf_version: "${apdf.apdfVersion}"`);
    lines.push('---');
    lines.push('');

    // Title
    lines.push(`# ${apdf.metadata.title}`);
    lines.push('');

    // Authors line
    const authorLine = apdf.authors.map(a => {
      const orcidLink = a.orcid ? ` [![ORCID](https://orcid.org/sites/default/files/images/orcid_16x16.png)](https://orcid.org/${a.orcid})` : '';
      return `**${a.name}**${orcidLink}`;
    }).join(', ');
    lines.push(authorLine);
    lines.push('');

    // Abstract
    if (apdf.metadata.abstract) {
      lines.push('## Abstract');
      lines.push('');
      lines.push(`> ${apdf.metadata.abstract}`);
      lines.push('');
    }

    // Table of Contents from aPDF structure
    if (apdf.structure.tableOfContents.length > 0) {
      lines.push('## Contents');
      lines.push('');
      for (const toc of apdf.structure.tableOfContents) {
        const indent = '  '.repeat(toc.level - 1);
        const anchor = toc.title.toLowerCase().replace(/[^a-z0-9]+/g, '-');
        lines.push(`${indent}- [${toc.title}](#${anchor})`);
      }
      lines.push('');
    }

    // Body from semantic chunks, grouped by page
    let currentPage = 0;
    for (const chunk of apdf.aiContent.chunks) {
      const page = chunk.pageNumbers[0];
      if (page !== currentPage) {
        currentPage = page;
        lines.push(`<!-- Page ${page} -->`);
      }
      lines.push(chunk.content.trim());
      lines.push('');
    }

    // Figures
    if (apdf.structure.figures.length > 0) {
      lines.push('## Figures');
      lines.push('');
      for (const fig of apdf.structure.figures) {
        lines.push(`**${fig.id}** (p. ${fig.pageNumber}): ${fig.caption || '(no caption)'}`);
        lines.push('');
      }
    }

    // Tables
    if (apdf.structure.tables.length > 0) {
      lines.push('## Tables');
      lines.push('');
      for (const table of apdf.structure.tables) {
        lines.push(`**${table.id}** (p. ${table.pageNumber}, ${table.rows}×${table.columns}): ${table.caption || '(no caption)'}`);
        lines.push('');
      }
    }

    // Bibliography
    if (apdf.structure.bibliography.length > 0) {
      lines.push('## References');
      lines.push('');
      for (let i = 0; i < apdf.structure.bibliography.length; i++) {
        const bib = apdf.structure.bibliography[i];
        const authors = bib.authors?.join(', ') || 'Unknown';
        const year = bib.year ? ` (${bib.year})` : '';
        const venue = bib.venue ? `. *${bib.venue}*` : '';
        const doi = bib.doi ? ` [doi:${bib.doi}](https://doi.org/${bib.doi})` : '';
        lines.push(`${i + 1}. ${authors}${year}. "${bib.title}"${venue}.${doi}`);
      }
      lines.push('');
    }

    // Linked artifacts
    if (apdf.artifacts.length > 0) {
      lines.push('## Linked Artifacts');
      lines.push('');
      for (const artifact of apdf.artifacts) {
        lines.push(`- **[${artifact.name}](${artifact.url})** (${artifact.type}) — ${artifact.relation}`);
      }
      lines.push('');
    }

    const markdown = lines.join('\n');

    console.log(`Generated ${markdown.length} chars of Markdown`);
    console.log(`  Sections:   ${apdf.structure.tableOfContents.length}`);
    console.log(`  Figures:    ${apdf.structure.figures.length}`);
    console.log(`  Tables:     ${apdf.structure.tables.length}`);
    console.log(`  References: ${apdf.structure.bibliography.length}`);
    console.log(`  Artifacts:  ${apdf.artifacts.length}`);
    console.log(`\nPreview (first 600 chars):\n`);
    console.log(markdown.substring(0, 600));
    console.log('...');

    return markdown;
  } finally {
    pdf.close();
  }
}

// ============================================================================
// Use Case 8: Embedding Cache with aPDF Provenance
//
// Generates embeddings for aPDF chunks and stores them with full provenance
// metadata so downstream systems can verify freshness and trace origins.
// ============================================================================

interface EmbeddingRecord {
  chunkId: string;
  documentId: string;
  documentTitle: string;
  pageNumbers: number[];
  chunkType: string;
  importance: number;
  content: string;
  embedding: number[];
  provenance: {
    generator: string;
    generatorVersion: string;
    generatedAt: string;
    sourceHash?: string;
    embeddingModel: string;
    embeddingDimensions: number;
  };
}

async function buildEmbeddingCacheWithProvenance(
  file: File,
  mockEmbedder: (text: string) => number[] = (text) => {
    // Mock: deterministic hash-based pseudo-embedding
    let hash = 0;
    for (let i = 0; i < text.length; i++) {
      hash = ((hash << 5) - hash + text.charCodeAt(i)) | 0;
    }
    return Array.from({ length: 384 }, (_, i) => Math.sin(hash + i) * 0.5);
  },
): Promise<EmbeddingRecord[]> {
  console.log('\n=== Use Case 8: Embedding Cache with Provenance ===\n');

  const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });

  try {
    const apdf = await pdf.generateAPDFMetadata();

    console.log(`Document: "${apdf.metadata.title}"`);
    console.log(`Chunks:   ${apdf.aiContent.chunks.length}`);
    console.log(`Generator: ${apdf.provenance.generator} v${apdf.provenance.generatorVersion}\n`);

    const records: EmbeddingRecord[] = [];
    const startTime = Date.now();

    for (const chunk of apdf.aiContent.chunks) {
      const embedding = mockEmbedder(chunk.content);

      records.push({
        chunkId: chunk.id,
        documentId: apdf.id,
        documentTitle: apdf.metadata.title,
        pageNumbers: chunk.pageNumbers,
        chunkType: chunk.chunkType,
        importance: chunk.importance,
        content: chunk.content,
        embedding,
        provenance: {
          generator: apdf.provenance.generator,
          generatorVersion: apdf.provenance.generatorVersion,
          generatedAt: apdf.provenance.generatedAt,
          sourceHash: apdf.provenance.sourceHash,
          embeddingModel: 'mock-384d',
          embeddingDimensions: embedding.length,
        },
      });
    }

    const elapsed = Date.now() - startTime;

    console.log(`Embedded ${records.length} chunks in ${elapsed}ms`);
    console.log(`  Dimensions: ${records[0]?.embedding.length || 0}`);
    console.log(`  Total vectors: ${records.length}`);
    console.log(`  Provenance chain: ${apdf.provenance.pipeline.join(' → ')}`);
    console.log(`\nSample record (without embedding):`);

    if (records.length > 0) {
      const { embedding: _, ...sample } = records[0];
      console.log(JSON.stringify(sample, null, 2).substring(0, 500));
    }

    return records;
  } finally {
    pdf.close();
  }
}

// ============================================================================
// Runner
// ============================================================================

export async function apdfUseCases(
  file: File,
  additionalFiles?: File[],
): Promise<void> {
  await multiAgentResearchAssistant(file);
  await compliancePolicyAudit(file);
  await buildKnowledgeGraph(additionalFiles ? [file, ...additionalFiles] : [file]);
  await smartDocumentRouter(additionalFiles ? [file, ...additionalFiles] : [file]);

  if (additionalFiles && additionalFiles.length > 0) {
    await diffDocumentRevisions(file, additionalFiles[0]);
    await automatedLiteratureSurvey([file, ...additionalFiles]);
  }

  await apdfToMarkdownPublisher(file);
  await buildEmbeddingCacheWithProvenance(file);

  console.log('\n=== All aPDF use case examples completed ===');
}
