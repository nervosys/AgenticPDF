/**
 * Batch Processing Example
 * 
 * This example demonstrates efficient batch processing of multiple PDFs:
 * - Memory-efficient concurrent processing
 * - Progress tracking and error handling
 * - Result aggregation and reporting
 * - Different processing strategies
 */

import AgenticPDF, { SemanticChunk, PDFMetadata, DocumentType } from '../neopdf';

interface ProcessingResult {
    fileName: string;
    success: boolean;
    metadata?: PDFMetadata;
    chunks?: SemanticChunk[];
    summary?: DocumentSummary;
    error?: string;
    processingTime: number;
}

interface DocumentSummary {
    documentType: DocumentType;
    pageCount: number;
    textLength: number;
    chunkCount: number;
    keyTopics: string[];
    hasImages: boolean;
    hasTables: boolean;
    complexity: 'Low' | 'Medium' | 'High';
}

interface BatchProcessingOptions {
    maxConcurrent?: number;
    chunkSize?: number;
    enableAI?: boolean;
    includeImages?: boolean;
    outputFormat?: 'summary' | 'detailed' | 'chunks';
    onProgress?: (processed: number, total: number, fileName: string) => void;
    onFileComplete?: (result: ProcessingResult) => void;
    onError?: (fileName: string, error: Error) => void;
}

export class BatchProcessor {
    private options: Required<BatchProcessingOptions>;

    constructor(options: BatchProcessingOptions = {}) {
        this.options = {
            maxConcurrent: options.maxConcurrent || 3,
            chunkSize: options.chunkSize || 1000,
            enableAI: options.enableAI ?? true,
            includeImages: options.includeImages ?? false,
            outputFormat: options.outputFormat || 'summary',
            onProgress: options.onProgress || (() => { }),
            onFileComplete: options.onFileComplete || (() => { }),
            onError: options.onError || (() => { })
        };
    }

    async processBatch(files: File[]): Promise<ProcessingResult[]> {
        console.log(`🚀 Starting batch processing of ${files.length} files`);
        console.log(`⚙️  Configuration:`);
        console.log(`   Max concurrent: ${this.options.maxConcurrent}`);
        console.log(`   Chunk size: ${this.options.chunkSize}`);
        console.log(`   AI enabled: ${this.options.enableAI}`);
        console.log(`   Output format: ${this.options.outputFormat}\n`);

        const results: ProcessingResult[] = [];
        const startTime = Date.now();

        // Process files in batches to control memory usage
        for (let i = 0; i < files.length; i += this.options.maxConcurrent) {
            const batch = files.slice(i, i + this.options.maxConcurrent);
            console.log(`📦 Processing batch ${Math.floor(i / this.options.maxConcurrent) + 1}/${Math.ceil(files.length / this.options.maxConcurrent)} (${batch.length} files)`);

            const batchResults = await Promise.allSettled(
                batch.map(async (file, batchIndex) => {
                    const overallIndex = i + batchIndex;
                    return this.processFile(file, overallIndex, files.length);
                })
            );

            // Collect results
            batchResults.forEach((result, batchIndex) => {
                const overallIndex = i + batchIndex;
                const fileName = batch[batchIndex].name;

                if (result.status === 'fulfilled') {
                    results.push(result.value);
                    this.options.onFileComplete(result.value);
                } else {
                    const errorResult: ProcessingResult = {
                        fileName,
                        success: false,
                        error: result.reason.message,
                        processingTime: 0
                    };
                    results.push(errorResult);
                    this.options.onError(fileName, result.reason);
                }

                this.options.onProgress(overallIndex + 1, files.length, fileName);
            });

            // Small delay between batches for memory cleanup
            if (i + this.options.maxConcurrent < files.length) {
                await new Promise(resolve => setTimeout(resolve, 500));

                // Force garbage collection if available
                if (typeof globalThis !== 'undefined' && (globalThis as any).gc) {
                    (globalThis as any).gc();
                }
            }
        }

        const totalTime = Date.now() - startTime;
        this.printBatchSummary(results, totalTime);

        return results;
    }

    private async processFile(file: File, index: number, total: number): Promise<ProcessingResult> {
        const startTime = Date.now();
        console.log(`   📄 [${index + 1}/${total}] Processing: ${file.name}`);

        let pdf: AgenticPDF | null = null;

        try {
            // Load PDF with memory constraints
            pdf = await AgenticPDF.fromFile(file, {
                lazyLoad: true,
                maxMemoryUsage: 50 * 1024 * 1024, // 50MB per file
                cachePages: false
            });

            const metadata = pdf.getMetadata();
            if (!metadata) {
                throw new Error('Could not read PDF metadata');
            }

            let chunks: SemanticChunk[] = [];
            let summary: DocumentSummary | undefined;

            if (this.options.enableAI) {
                // Generate semantic chunks
                chunks = await pdf.generateSemanticChunks({
                    strategy: 'semantic',
                    maxChunkSize: this.options.chunkSize,
                    preserveParagraphs: true,
                    includeMetadata: true
                });

                // Generate document summary
                summary = await this.generateDocumentSummary(pdf, chunks, metadata);
            }

            const processingTime = Date.now() - startTime;

            const result: ProcessingResult = {
                fileName: file.name,
                success: true,
                metadata,
                chunks: this.options.outputFormat === 'chunks' ? chunks : undefined,
                summary,
                processingTime
            };

            console.log(`   ✅ [${index + 1}/${total}] Completed: ${file.name} (${processingTime}ms)`);

            return result;

        } catch (error) {
            const processingTime = Date.now() - startTime;
            console.log(`   ❌ [${index + 1}/${total}] Failed: ${file.name} - ${(error as Error).message}`);

            throw error;
        } finally {
            pdf?.close();
        }
    }

    private async generateDocumentSummary(
        pdf: AgenticPDF,
        chunks: SemanticChunk[],
        metadata: PDFMetadata
    ): Promise<DocumentSummary> {
        try {
            const aiFeatures = await pdf.getAIFeatures({
                enableStructuralAnalysis: true,
                chunkSize: this.options.chunkSize
            });

            // Extract key topics from chunks
            const keyTopics = this.extractKeyTopics(chunks);

            // Calculate complexity based on various factors
            const complexity = this.calculateComplexity(chunks, aiFeatures.structuralAnalysis);

            // Get total text length
            const textLength = chunks.reduce((total, chunk) => total + chunk.content.length, 0);

            return {
                documentType: aiFeatures.structuralAnalysis.documentType,
                pageCount: metadata.pageCount,
                textLength,
                chunkCount: chunks.length,
                keyTopics,
                hasImages: aiFeatures.structuralAnalysis.figures.length > 0,
                hasTables: aiFeatures.structuralAnalysis.tables.length > 0,
                complexity
            };
        } catch (error) {
            // Fallback summary without AI features
            const textLength = chunks.reduce((total, chunk) => total + chunk.content.length, 0);
            const keyTopics = this.extractKeyTopics(chunks);

            return {
                documentType: DocumentType.Other,
                pageCount: metadata.pageCount,
                textLength,
                chunkCount: chunks.length,
                keyTopics,
                hasImages: false,
                hasTables: false,
                complexity: textLength > 50000 ? 'High' : textLength > 20000 ? 'Medium' : 'Low'
            };
        }
    }

    private extractKeyTopics(chunks: SemanticChunk[]): string[] {
        const topicCounts = new Map<string, number>();

        chunks.forEach(chunk => {
            chunk.metadata.keywords?.forEach(keyword => {
                const normalizedKeyword = keyword.toLowerCase().trim();
                if (normalizedKeyword.length > 3) {
                    topicCounts.set(normalizedKeyword, (topicCounts.get(normalizedKeyword) || 0) + 1);
                }
            });
        });

        return Array.from(topicCounts.entries())
            .sort((a, b) => b[1] - a[1])
            .slice(0, 5)
            .map(([topic]) => topic);
    }

    private calculateComplexity(chunks: SemanticChunk[], structuralAnalysis: any): 'Low' | 'Medium' | 'High' {
        let complexityScore = 0;

        // Factor in document length
        const totalChars = chunks.reduce((total, chunk) => total + chunk.content.length, 0);
        if (totalChars > 100000) complexityScore += 3;
        else if (totalChars > 50000) complexityScore += 2;
        else if (totalChars > 20000) complexityScore += 1;

        // Factor in structural elements
        complexityScore += Math.min(structuralAnalysis.tables?.length || 0, 3);
        complexityScore += Math.min(structuralAnalysis.figures?.length || 0, 2);
        complexityScore += Math.min(structuralAnalysis.equations?.length || 0, 2);

        // Factor in chunk diversity
        const chunkTypes = new Set(chunks.map(chunk => chunk.type));
        complexityScore += Math.min(chunkTypes.size, 3);

        if (complexityScore >= 8) return 'High';
        if (complexityScore >= 4) return 'Medium';
        return 'Low';
    }

    private printBatchSummary(results: ProcessingResult[], totalTime: number) {
        const successful = results.filter(r => r.success);
        const failed = results.filter(r => !r.success);

        console.log(`\n📊 Batch Processing Summary:`);
        console.log(`   Total files: ${results.length}`);
        console.log(`   Successful: ${successful.length}`);
        console.log(`   Failed: ${failed.length}`);
        console.log(`   Total time: ${(totalTime / 1000).toFixed(1)}s`);
        console.log(`   Average time per file: ${(totalTime / results.length / 1000).toFixed(1)}s`);

        if (successful.length > 0) {
            const totalPages = successful.reduce((sum, r) => sum + (r.metadata?.pageCount || 0), 0);
            const totalChunks = successful.reduce((sum, r) => sum + (r.chunks?.length || r.summary?.chunkCount || 0), 0);
            const avgProcessingTime = successful.reduce((sum, r) => sum + r.processingTime, 0) / successful.length;

            console.log(`\n📈 Processing Statistics:`);
            console.log(`   Total pages processed: ${totalPages.toLocaleString()}`);
            console.log(`   Total chunks generated: ${totalChunks.toLocaleString()}`);
            console.log(`   Average processing time: ${avgProcessingTime.toFixed(0)}ms`);
            console.log(`   Pages per second: ${(totalPages / (totalTime / 1000)).toFixed(1)}`);

            // Document type distribution
            if (this.options.enableAI) {
                const typeDistribution = successful.reduce((acc, r) => {
                    const type = r.summary?.documentType || 'Unknown';
                    acc[type] = (acc[type] || 0) + 1;
                    return acc;
                }, {} as Record<string, number>);

                console.log(`\n📋 Document Types:`);
                Object.entries(typeDistribution).forEach(([type, count]) => {
                    console.log(`   ${type}: ${count} files`);
                });

                // Complexity distribution
                const complexityDistribution = successful.reduce((acc, r) => {
                    const complexity = r.summary?.complexity || 'Unknown';
                    acc[complexity] = (acc[complexity] || 0) + 1;
                    return acc;
                }, {} as Record<string, number>);

                console.log(`\n🧠 Complexity Distribution:`);
                Object.entries(complexityDistribution).forEach(([complexity, count]) => {
                    console.log(`   ${complexity}: ${count} files`);
                });
            }
        }

        if (failed.length > 0) {
            console.log(`\n❌ Failed Files:`);
            failed.forEach(result => {
                console.log(`   ${result.fileName}: ${result.error}`);
            });
        }
    }
}

// Example usage functions
export async function batchProcessingExample(files: File[]) {
    console.log('=== Batch Processing Example ===\n');

    const processor = new BatchProcessor({
        maxConcurrent: 2,
        chunkSize: 1200,
        enableAI: true,
        outputFormat: 'summary',
        onProgress: (processed, total, fileName) => {
            const percentage = Math.round((processed / total) * 100);
            console.log(`📊 Progress: ${percentage}% (${processed}/${total}) - Last: ${fileName}`);
        },
        onFileComplete: (result) => {
            if (result.success && result.summary) {
                console.log(`   📝 ${result.fileName}: ${result.summary.documentType}, ${result.summary.pageCount} pages, ${result.summary.complexity} complexity`);
            }
        },
        onError: (fileName, error) => {
            console.log(`   ⚠️  ${fileName}: ${error.message}`);
        }
    });

    const results = await processor.processBatch(files);

    // Demonstrate result analysis
    console.log('\n--- Detailed Results Analysis ---');

    const successful = results.filter(r => r.success);
    if (successful.length > 0) {
        // Find most complex document
        const mostComplex = successful
            .filter(r => r.summary)
            .sort((a, b) => {
                const complexityOrder = { 'Low': 1, 'Medium': 2, 'High': 3 };
                return complexityOrder[b.summary!.complexity] - complexityOrder[a.summary!.complexity];
            })[0];

        if (mostComplex?.summary) {
            console.log(`🏆 Most complex document: ${mostComplex.fileName}`);
            console.log(`   Type: ${mostComplex.summary.documentType}`);
            console.log(`   Pages: ${mostComplex.summary.pageCount}`);
            console.log(`   Text length: ${mostComplex.summary.textLength.toLocaleString()} chars`);
            console.log(`   Key topics: ${mostComplex.summary.keyTopics.join(', ')}`);
        }

        // Find documents with similar content
        console.log('\n🔗 Content similarity analysis:');
        await analyzeContentSimilarity(successful.slice(0, 5)); // Limit for demo
    }

    return results;
}

async function analyzeContentSimilarity(results: ProcessingResult[]) {
    const documentsWithTopics = results.filter(r => r.summary?.keyTopics.length);

    for (let i = 0; i < documentsWithTopics.length - 1; i++) {
        for (let j = i + 1; j < documentsWithTopics.length; j++) {
            const doc1 = documentsWithTopics[i];
            const doc2 = documentsWithTopics[j];

            const similarity = calculateTopicSimilarity(
                doc1.summary!.keyTopics,
                doc2.summary!.keyTopics
            );

            if (similarity > 0.3) { // 30% similarity threshold
                console.log(`   📎 ${doc1.fileName} ↔ ${doc2.fileName}: ${(similarity * 100).toFixed(1)}% similar`);
                const commonTopics = doc1.summary!.keyTopics.filter(topic =>
                    doc2.summary!.keyTopics.includes(topic)
                );
                if (commonTopics.length > 0) {
                    console.log(`      Common topics: ${commonTopics.join(', ')}`);
                }
            }
        }
    }
}

function calculateTopicSimilarity(topics1: string[], topics2: string[]): number {
    const set1 = new Set(topics1);
    const set2 = new Set(topics2);

    const intersection = new Set([...set1].filter(x => set2.has(x)));
    const union = new Set([...set1, ...set2]);

    return union.size > 0 ? intersection.size / union.size : 0;
}

// Browser integration
if (typeof window !== 'undefined') {
    document.addEventListener('DOMContentLoaded', () => {
        const container = document.createElement('div');
        container.innerHTML = `
      <h2>Batch Processing Example</h2>
      <div>
        <input type="file" id="pdfFiles" accept=".pdf" multiple />
        <br><br>
        <label>
          <input type="checkbox" id="enableAI" checked /> Enable AI Analysis
        </label>
        <br>
        <label>
          Max Concurrent:
          <select id="maxConcurrent">
            <option value="1">1</option>
            <option value="2" selected>2</option>
            <option value="3">3</option>
            <option value="4">4</option>
          </select>
        </label>
        <br><br>
        <button id="processBtn" disabled>Process Batch</button>
      </div>
      <div id="progress" style="margin: 10px 0;"></div>
      <div id="output" style="white-space: pre-wrap; font-family: monospace; background: #f5f5f5; padding: 10px; margin-top: 10px; max-height: 400px; overflow-y: auto;"></div>
    `;

        document.body.appendChild(container);

        const fileInput = document.getElementById('pdfFiles') as HTMLInputElement;
        const enableAICheckbox = document.getElementById('enableAI') as HTMLInputElement;
        const maxConcurrentSelect = document.getElementById('maxConcurrent') as HTMLSelectElement;
        const processBtn = document.getElementById('processBtn') as HTMLButtonElement;
        const progressDiv = document.getElementById('progress') as HTMLDivElement;
        const output = document.getElementById('output') as HTMLDivElement;

        fileInput.onchange = () => {
            const files = fileInput.files;
            processBtn.disabled = !files || files.length === 0;
            if (files && files.length > 0) {
                progressDiv.textContent = `${files.length} files selected`;
            }
        };

        processBtn.onclick = async () => {
            const files = Array.from(fileInput.files || []);
            if (files.length === 0) return;

            const enableAI = enableAICheckbox.checked;
            const maxConcurrent = parseInt(maxConcurrentSelect.value);

            // Redirect console.log to output
            const originalLog = console.log;
            console.log = (...args) => {
                output.textContent += args.join(' ') + '\n';
                output.scrollTop = output.scrollHeight;
                originalLog(...args);
            };

            try {
                processBtn.disabled = true;
                output.textContent = '';
                progressDiv.textContent = 'Processing...';

                const processor = new BatchProcessor({
                    maxConcurrent,
                    enableAI,
                    onProgress: (processed, total, fileName) => {
                        const percentage = Math.round((processed / total) * 100);
                        progressDiv.textContent = `Progress: ${percentage}% (${processed}/${total}) - ${fileName}`;
                    }
                });

                await processor.processBatch(files);
                progressDiv.textContent = 'Completed!';

            } finally {
                processBtn.disabled = false;
                console.log = originalLog;
            }
        };
    });
}