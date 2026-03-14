/**
 * AI Integration Example
 * 
 * This example demonstrates AI-powered PDF processing:
 * - Semantic chunking for RAG systems
 * - Custom embedding providers
 * - Document intelligence analysis
 * - Streaming processing for memory efficiency
 */

import AgenticPDF, { EmbeddingProvider, SemanticChunk, AIFeatures, ChunkType } from '../neopdf';

// Example custom embedding provider
class OpenAIEmbeddingProvider implements EmbeddingProvider {
    model = 'text-embedding-3-small';
    dimensions = 1536;
    private apiKey: string;

    constructor(apiKey: string) {
        this.apiKey = apiKey;
    }

    async generate(text: string): Promise<Float32Array> {
        try {
            const response = await fetch('https://api.openai.com/v1/embeddings', {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${this.apiKey}`,
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    model: this.model,
                    input: text
                })
            });

            if (!response.ok) {
                throw new Error(`OpenAI API error: ${response.statusText}`);
            }

            const data = await response.json();
            return new Float32Array(data.data[0].embedding);
        } catch (error) {
            console.warn('OpenAI embedding failed, returning mock embedding:', (error as Error).message);
            // Return mock embedding for demo purposes
            return new Float32Array(this.dimensions).fill(0.1);
        }
    }

    async generateBatch(texts: string[]): Promise<Float32Array[]> {
        try {
            const response = await fetch('https://api.openai.com/v1/embeddings', {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${this.apiKey}`,
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({
                    model: this.model,
                    input: texts
                })
            });

            if (!response.ok) {
                throw new Error(`OpenAI API error: ${response.statusText}`);
            }

            const data = await response.json();
            return data.data.map((item: any) => new Float32Array(item.embedding));
        } catch (error) {
            console.warn('OpenAI batch embedding failed, returning mock embeddings:', (error as Error).message);
            // Return mock embeddings for demo purposes
            return texts.map(() => new Float32Array(this.dimensions).fill(0.1));
        }
    }
}

// Mock embedding provider for demo purposes
class MockEmbeddingProvider implements EmbeddingProvider {
    model = 'mock-embedding-model';
    dimensions = 384;

    async generate(text: string): Promise<Float32Array> {
        // Simulate processing time
        await new Promise(resolve => setTimeout(resolve, 50));

        // Generate mock embedding based on text characteristics
        const embedding = new Float32Array(this.dimensions);
        const hash = this.simpleHash(text);

        for (let i = 0; i < this.dimensions; i++) {
            embedding[i] = Math.sin((hash + i) * 0.1) * 0.5;
        }

        return embedding;
    }

    async generateBatch(texts: string[]): Promise<Float32Array[]> {
        // Process in parallel for demo
        return Promise.all(texts.map(text => this.generate(text)));
    }

    private simpleHash(str: string): number {
        let hash = 0;
        for (let i = 0; i < str.length; i++) {
            const char = str.charCodeAt(i);
            hash = ((hash << 5) - hash) + char;
            hash = hash & hash; // Convert to 32bit integer
        }
        return Math.abs(hash);
    }
}

export async function aiIntegrationExample(file: File, apiKey?: string) {
    console.log('=== AI Integration Example ===\n');

    let pdf: AgenticPDF | null = null;

    try {
        // Load PDF with AI-optimized settings
        pdf = await AgenticPDF.fromFile(file, {
            lazyLoad: true,
            useWebWorkers: true, // Use workers for CPU-intensive AI operations
            maxMemoryUsage: 100 * 1024 * 1024 // 100MB limit
        });

        console.log(`📄 Processing: ${file.name}`);

        // Choose embedding provider
        const embeddingProvider = apiKey
            ? new OpenAIEmbeddingProvider(apiKey)
            : new MockEmbeddingProvider();

        console.log(`🧠 Using embedding provider: ${embeddingProvider.model}`);

        // Demonstrate different AI features
        await demonstrateSemanticChunking(pdf, embeddingProvider);
        await demonstrateDocumentAnalysis(pdf, embeddingProvider);
        await demonstrateStreamingProcessing(pdf);

    } catch (error) {
        console.error('AI integration error:', error);
    } finally {
        pdf?.close();
        console.log('\n✅ AI processing completed and resources cleaned up');
    }
}

async function demonstrateSemanticChunking(pdf: AgenticPDF, embeddingProvider: EmbeddingProvider) {
    console.log('\n--- Semantic Chunking Demo ---');

    try {
        // Generate semantic chunks
        const chunks = await pdf.generateSemanticChunks({
            strategy: 'semantic',
            maxChunkSize: 500,
            minChunkSize: 100,
            preserveParagraphs: true,
            includeMetadata: true
        });

        console.log(`📦 Generated ${chunks.length} semantic chunks`);

        // Analyze chunk types
        const chunkTypes = chunks.reduce((acc, chunk) => {
            acc[chunk.type] = (acc[chunk.type] || 0) + 1;
            return acc;
        }, {} as Record<ChunkType, number>);

        console.log('📊 Chunk distribution:');
        Object.entries(chunkTypes).forEach(([type, count]) => {
            console.log(`  ${type}: ${count} chunks`);
        });

        // Show sample chunks
        console.log('\n📝 Sample chunks:');
        chunks.slice(0, 3).forEach((chunk, index) => {
            console.log(`\n  Chunk ${index + 1}:`);
            console.log(`    Type: ${chunk.type}`);
            console.log(`    Pages: ${chunk.pageNumbers.join(', ')}`);
            console.log(`    Content (${chunk.content.length} chars): "${chunk.content.substring(0, 100)}..."`);
            console.log(`    Keywords: ${chunk.metadata.keywords?.slice(0, 3).join(', ') || 'None'}`);
            console.log(`    Confidence: ${(chunk.metadata.confidence * 100).toFixed(1)}%`);
            console.log(`    Importance: ${(chunk.metadata.importance * 100).toFixed(1)}%`);
        });

        // Generate embeddings for first few chunks
        console.log('\n🔢 Generating embeddings...');
        const sampleChunks = chunks.slice(0, 3);
        const embeddings = await embeddingProvider.generateBatch(
            sampleChunks.map(chunk => chunk.content)
        );

        console.log(`✅ Generated ${embeddings.length} embeddings (${embeddingProvider.dimensions}D each)`);

        // Calculate similarity between first two chunks
        if (embeddings.length >= 2) {
            const similarity = cosineSimilarity(embeddings[0], embeddings[1]);
            console.log(`🔗 Similarity between chunk 1 and 2: ${(similarity * 100).toFixed(1)}%`);
        }

    } catch (error) {
        console.error('Semantic chunking failed:', error);
    }
}

async function demonstrateDocumentAnalysis(pdf: AgenticPDF, embeddingProvider: EmbeddingProvider) {
    console.log('\n--- Document Intelligence Analysis ---');

    try {
        // Get comprehensive AI features
        const aiFeatures = await pdf.getAIFeatures({
            embeddingProvider,
            enableStructuralAnalysis: true,
            enableSemanticChunking: true,
            enableNER: true,
            enableSummarization: true,
            chunkSize: 1000
        });

        console.log('🧠 AI Analysis Complete!');

        // Document structure analysis
        const structure = aiFeatures.structuralAnalysis;
        console.log('\n📋 Document Structure:');
        console.log(`  Document Type: ${structure.documentType}`);
        console.log(`  Sections: ${structure.sections.length}`);
        console.log(`  Tables: ${structure.tables.length}`);
        console.log(`  Figures: ${structure.figures.length}`);
        console.log(`  Equations: ${structure.equations.length}`);
        console.log(`  References: ${structure.references.length}`);

        // Show section hierarchy
        if (structure.sections.length > 0) {
            console.log('\n📑 Section Hierarchy:');
            structure.sections.slice(0, 5).forEach((section, index) => {
                const indent = '  '.repeat((section.level || 0) + 1);
                console.log(`${indent}${index + 1}. ${section.type} (Level ${section.level || 0}): "${section.text.substring(0, 50)}..."`);
                console.log(`${indent}   Pages: ${section.pageStart}-${section.pageEnd}`);
            });
        }

        // Show tables
        if (structure.tables.length > 0) {
            console.log('\n📊 Tables Found:');
            structure.tables.slice(0, 3).forEach((table, index) => {
                console.log(`  Table ${index + 1}: ${table.rows}x${table.columns} on page ${table.pageNumber}`);
                if (table.caption) {
                    console.log(`    Caption: "${table.caption}"`);
                }
                if (table.headers && table.headers.length > 0) {
                    console.log(`    Headers: ${table.headers.slice(0, 3).join(', ')}${table.headers.length > 3 ? '...' : ''}`);
                }
            });
        }

        // NLP-ready content
        const nlpContent = aiFeatures.nlpReady;
        console.log('\n📖 Content Analysis:');
        console.log(`  Total Text Length: ${nlpContent.fullText.length.toLocaleString()} characters`);
        console.log(`  Sentences: ${nlpContent.sentences.length.toLocaleString()}`);
        console.log(`  Paragraphs: ${nlpContent.paragraphs.length.toLocaleString()}`);
        console.log(`  Language: ${nlpContent.language}`);
        console.log(`  Reading Level: ${nlpContent.readingLevel || 'Unknown'}`);

        // Show keywords
        if (nlpContent.keywords && nlpContent.keywords.length > 0) {
            console.log(`  Key Terms: ${nlpContent.keywords.slice(0, 10).join(', ')}${nlpContent.keywords.length > 10 ? '...' : ''}`);
        }

        // Show summary
        if (nlpContent.summary) {
            console.log(`\n📝 Document Summary:`);
            console.log(`  "${nlpContent.summary.substring(0, 200)}${nlpContent.summary.length > 200 ? '...' : ''}"`);
        }

    } catch (error) {
        console.error('Document analysis failed:', error);
    }
}

async function demonstrateStreamingProcessing(pdf: AgenticPDF) {
    console.log('\n--- Streaming Processing Demo ---');

    try {
        let chunkCount = 0;
        let totalChars = 0;
        const startTime = Date.now();

        console.log('🌊 Starting streaming semantic chunking...');

        // Process chunks as they arrive
        for await (const chunk of pdf.streamSemanticChunks({
            strategy: 'semantic',
            maxChunkSize: 800,
            includeMetadata: true
        })) {
            chunkCount++;
            totalChars += chunk.content.length;

            // Log progress every 5 chunks
            if (chunkCount % 5 === 0) {
                const elapsed = Date.now() - startTime;
                const rate = chunkCount / (elapsed / 1000);
                console.log(`  Processed ${chunkCount} chunks (${rate.toFixed(1)} chunks/sec)`);
            }

            // Process chunk (simulate work)
            await processChunkForRAG(chunk);

            // Stop after 20 chunks for demo
            if (chunkCount >= 20) {
                console.log('  (Stopping demo after 20 chunks)');
                break;
            }
        }

        const elapsed = Date.now() - startTime;
        console.log(`\n✅ Streaming completed:`);
        console.log(`  Processed: ${chunkCount} chunks`);
        console.log(`  Total content: ${totalChars.toLocaleString()} characters`);
        console.log(`  Time elapsed: ${(elapsed / 1000).toFixed(1)}s`);
        console.log(`  Average rate: ${(chunkCount / (elapsed / 1000)).toFixed(1)} chunks/sec`);

    } catch (error) {
        console.error('Streaming processing failed:', error);
    }
}

async function processChunkForRAG(chunk: SemanticChunk) {
    // Simulate RAG processing (database storage, vector indexing, etc.)
    await new Promise(resolve => setTimeout(resolve, 10));

    // In a real application, you might:
    // 1. Generate embeddings
    // 2. Store in vector database
    // 3. Index for search
    // 4. Extract entities
    // 5. Update knowledge graph
}

function cosineSimilarity(a: Float32Array, b: Float32Array): number {
    if (a.length !== b.length) return 0;

    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
        dotProduct += a[i] * b[i];
        normA += a[i] * a[i];
        normB += b[i] * b[i];
    }

    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
}

// Browser usage
if (typeof window !== 'undefined') {
    // Create a simple UI for the example
    document.addEventListener('DOMContentLoaded', () => {
        const container = document.createElement('div');
        container.innerHTML = `
      <h2>AI Integration Example</h2>
      <div>
        <input type="file" id="pdfFile" accept=".pdf" />
        <br><br>
        <label>
          OpenAI API Key (optional):
          <input type="password" id="apiKey" placeholder="sk-..." />
        </label>
        <br><br>
        <button id="processBtn" disabled>Process PDF with AI</button>
      </div>
      <div id="output"></div>
    `;

        document.body.appendChild(container);

        const fileInput = document.getElementById('pdfFile') as HTMLInputElement;
        const apiKeyInput = document.getElementById('apiKey') as HTMLInputElement;
        const processBtn = document.getElementById('processBtn') as HTMLButtonElement;
        const output = document.getElementById('output') as HTMLDivElement;

        fileInput.onchange = () => {
            processBtn.disabled = !fileInput.files?.[0];
        };

        processBtn.onclick = async () => {
            const file = fileInput.files?.[0];
            if (!file) return;

            const apiKey = apiKeyInput.value.trim() || undefined;

            // Redirect console.log to output div
            const originalLog = console.log;
            console.log = (...args) => {
                output.innerHTML += args.join(' ') + '<br>';
                originalLog(...args);
            };

            try {
                processBtn.disabled = true;
                output.innerHTML = '';
                await aiIntegrationExample(file, apiKey);
            } finally {
                processBtn.disabled = false;
                console.log = originalLog;
            }
        };
    });
}