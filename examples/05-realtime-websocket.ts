/**
 * Real-time WebSocket Processing Example
 * 
 * This example demonstrates real-time PDF processing with WebSocket communication:
 * - Live progress updates
 * - Chunk-by-chunk streaming
 * - Real-time analysis results
 * - Client-server communication patterns
 */

import ModernPDF, { SemanticChunk, ProgressInfo } from '../modernpdf';

// WebSocket message types
interface WebSocketMessage {
    type: string;
    id: string;
    data?: any;
    timestamp: number;
}

interface ProcessingStartedMessage extends WebSocketMessage {
    type: 'processing_started';
    data: {
        fileName: string;
        totalPages: number;
        fileSize: number;
    };
}

interface ChunkProcessedMessage extends WebSocketMessage {
    type: 'chunk_processed';
    data: {
        chunk: {
            id: string;
            content: string;
            pageNumbers: number[];
            type: string;
            confidence: number;
            keywords: string[];
            summary: string;
        };
        progress: {
            processed: number;
            total: number;
            percentage: number;
        };
    };
}

interface ProgressUpdateMessage extends WebSocketMessage {
    type: 'progress_update';
    data: {
        stage: string;
        progress: number;
        currentOperation: string;
        timeElapsed: number;
        estimatedTimeRemaining?: number;
    };
}

interface ProcessingCompleteMessage extends WebSocketMessage {
    type: 'processing_complete';
    data: {
        summary: {
            totalChunks: number;
            totalPages: number;
            documentType: string;
            processingTime: number;
            keyInsights: string[];
        };
    };
}

interface ErrorMessage extends WebSocketMessage {
    type: 'error';
    data: {
        message: string;
        stage: string;
    };
}

export class PDFWebSocketProcessor {
    private websocket: WebSocket | null = null;
    private processingId: string | null = null;
    private startTime: number = 0;
    private totalChunks: number = 0;
    private processedChunks: number = 0;

    constructor(private wsUrl: string) { }

    async connect(): Promise<void> {
        return new Promise((resolve, reject) => {
            try {
                this.websocket = new WebSocket(this.wsUrl);

                this.websocket.onopen = () => {
                    console.log('🔌 WebSocket connected');
                    resolve();
                };

                this.websocket.onerror = (error) => {
                    console.error('❌ WebSocket error:', error);
                    reject(new Error('WebSocket connection failed'));
                };

                this.websocket.onclose = (event) => {
                    console.log(`🔌 WebSocket disconnected (${event.code}: ${event.reason})`);
                };

                this.websocket.onmessage = (event) => {
                    this.handleServerMessage(JSON.parse(event.data));
                };

            } catch (error) {
                reject(error);
            }
        });
    }

    async processWithWebSocket(file: File): Promise<void> {
        if (!this.websocket || this.websocket.readyState !== WebSocket.OPEN) {
            throw new Error('WebSocket not connected');
        }

        this.processingId = crypto.randomUUID();
        this.startTime = Date.now();
        this.processedChunks = 0;

        console.log(`🚀 Starting real-time processing: ${file.name}`);

        let pdf: ModernPDF | null = null;

        try {
            // Send processing started message
            this.sendMessage({
                type: 'processing_started',
                id: this.processingId,
                data: {
                    fileName: file.name,
                    totalPages: 0, // Will update once PDF is loaded
                    fileSize: file.size
                },
                timestamp: Date.now()
            });

            // Load PDF with progress tracking
            pdf = await ModernPDF.fromFile(file, {
                lazyLoad: true,
                maxMemoryUsage: 100 * 1024 * 1024, // 100MB
                streamOptions: {
                    chunkSize: 512 * 1024, // 512KB chunks
                    backpressureThreshold: 2 * 1024 * 1024, // 2MB threshold
                    progressCallback: (progress) => this.handleLoadProgress(progress)
                }
            });

            const metadata = pdf.getMetadata();
            if (!metadata) {
                throw new Error('Could not read PDF metadata');
            }

            // Update total pages
            this.sendMessage({
                type: 'processing_started',
                id: this.processingId,
                data: {
                    fileName: file.name,
                    totalPages: metadata.pageCount,
                    fileSize: file.size
                },
                timestamp: Date.now()
            });

            // Estimate total chunks
            this.totalChunks = Math.ceil(metadata.pageCount * 1.5); // Rough estimate

            // Process chunks in real-time
            await this.processChunksRealTime(pdf);

            // Get final document analysis
            const analysis = await this.getFinalAnalysis(pdf);

            // Send completion message
            this.sendMessage({
                type: 'processing_complete',
                id: this.processingId,
                data: {
                    summary: {
                        totalChunks: this.processedChunks,
                        totalPages: metadata.pageCount,
                        documentType: analysis.documentType,
                        processingTime: Date.now() - this.startTime,
                        keyInsights: analysis.keyInsights
                    }
                },
                timestamp: Date.now()
            });

            console.log(`✅ Real-time processing completed: ${file.name}`);

        } catch (error) {
            console.error('❌ Processing failed:', error);

            this.sendMessage({
                type: 'error',
                id: this.processingId || 'unknown',
                data: {
                    message: (error as Error).message,
                    stage: 'processing'
                },
                timestamp: Date.now()
            });

            throw error;
        } finally {
            pdf?.close();
        }
    }

    private async processChunksRealTime(pdf: ModernPDF): Promise<void> {
        console.log('🌊 Starting real-time chunk processing...');

        this.sendProgressUpdate('Generating semantic chunks', 0);

        let chunkIndex = 0;
        const startTime = Date.now();

        for await (const chunk of pdf.streamSemanticChunks({
            strategy: 'semantic',
            maxChunkSize: 1000,
            preserveParagraphs: true,
            includeMetadata: true
        })) {
            chunkIndex++;
            this.processedChunks = chunkIndex;

            // Generate chunk summary
            const summary = this.generateChunkSummary(chunk);

            // Send chunk data
            this.sendMessage({
                type: 'chunk_processed',
                id: this.processingId!,
                data: {
                    chunk: {
                        id: chunk.id,
                        content: chunk.content.length > 500 ? chunk.content.substring(0, 500) + '...' : chunk.content,
                        pageNumbers: chunk.pageNumbers,
                        type: chunk.type,
                        confidence: chunk.metadata.confidence,
                        keywords: chunk.metadata.keywords || [],
                        summary
                    },
                    progress: {
                        processed: chunkIndex,
                        total: this.totalChunks,
                        percentage: Math.round((chunkIndex / this.totalChunks) * 100)
                    }
                },
                timestamp: Date.now()
            });

            // Update progress every 5 chunks
            if (chunkIndex % 5 === 0) {
                const elapsed = Date.now() - startTime;
                const rate = chunkIndex / (elapsed / 1000);
                const remaining = this.totalChunks - chunkIndex;
                const estimatedTimeRemaining = remaining / rate * 1000;

                this.sendProgressUpdate(
                    'Processing semantic chunks',
                    (chunkIndex / this.totalChunks) * 100,
                    estimatedTimeRemaining
                );
            }

            // Throttle to prevent overwhelming the client
            await new Promise(resolve => setTimeout(resolve, 50));

            // Update total estimate based on actual progress
            if (chunkIndex === 10) {
                const metadata = pdf.getMetadata();
                if (metadata) {
                    // Better estimate after processing some chunks
                    this.totalChunks = Math.ceil((chunkIndex / 10) * metadata.pageCount * 1.2);
                }
            }
        }

        console.log(`📊 Processed ${chunkIndex} chunks in real-time`);
    }

    private generateChunkSummary(chunk: SemanticChunk): string {
        const content = chunk.content;
        const words = content.split(/\s+/).length;

        // Generate a simple summary based on chunk characteristics
        if (chunk.type === 'Title' || chunk.type === 'Header') {
            return `Section header: "${content.substring(0, 50)}..."`;
        } else if (chunk.type === 'Table') {
            return `Table with ${words} words of data`;
        } else if (chunk.type === 'List') {
            const items = content.split(/[•\-\*\n]/).filter(item => item.trim().length > 5);
            return `List with ${items.length} items`;
        } else if (words > 100) {
            const sentences = content.split(/[.!?]+/).filter(s => s.trim().length > 10);
            return `${sentences.length} sentences covering ${chunk.metadata.keywords?.slice(0, 3).join(', ') || 'various topics'}`;
        } else {
            return `Brief content about ${chunk.metadata.keywords?.[0] || 'general topics'}`;
        }
    }

    private async getFinalAnalysis(pdf: ModernPDF): Promise<{ documentType: string; keyInsights: string[] }> {
        try {
            this.sendProgressUpdate('Generating final analysis', 90);

            const aiFeatures = await pdf.getAIFeatures({
                enableStructuralAnalysis: true,
                enableSummarization: true
            });

            const keyInsights = [
                `Document contains ${aiFeatures.structuralAnalysis.sections.length} sections`,
                `Found ${aiFeatures.structuralAnalysis.tables.length} tables and ${aiFeatures.structuralAnalysis.figures.length} figures`,
                `${aiFeatures.semanticChunks.length} semantic chunks generated`,
                aiFeatures.nlpReady.summary ? `Summary: ${aiFeatures.nlpReady.summary.substring(0, 100)}...` : 'No summary available'
            ].filter(insight => !insight.includes('0 '));

            return {
                documentType: aiFeatures.structuralAnalysis.documentType,
                keyInsights
            };
        } catch (error) {
            return {
                documentType: 'Unknown',
                keyInsights: ['Analysis completed with basic processing']
            };
        }
    }

    private handleLoadProgress(progress: ProgressInfo): void {
        const percentage = Math.round((progress.bytesRead / progress.totalBytes) * 100);
        this.sendProgressUpdate(
            progress.currentOperation,
            percentage * 0.3, // Loading is 30% of total progress
            progress.estimatedTimeRemaining
        );
    }

    private sendProgressUpdate(
        operation: string,
        percentage: number,
        estimatedTimeRemaining?: number
    ): void {
        this.sendMessage({
            type: 'progress_update',
            id: this.processingId!,
            data: {
                stage: operation,
                progress: Math.min(percentage, 100),
                currentOperation: operation,
                timeElapsed: Date.now() - this.startTime,
                estimatedTimeRemaining
            },
            timestamp: Date.now()
        });
    }

    protected sendMessage(message: WebSocketMessage): void {
        if (this.websocket && this.websocket.readyState === WebSocket.OPEN) {
            this.websocket.send(JSON.stringify(message));
        }
    }

    private handleServerMessage(message: WebSocketMessage): void {
        console.log('📨 Server message:', message.type, message.data);

        // Handle server responses if needed
        switch (message.type) {
            case 'ack':
                console.log('✅ Server acknowledged:', message.id);
                break;
            case 'request_chunk':
                // Server requesting specific chunk data
                console.log('📤 Server requesting chunk:', message.data);
                break;
            default:
                console.log('📨 Unknown server message type:', message.type);
        }
    }

    disconnect(): void {
        if (this.websocket) {
            this.websocket.close();
            this.websocket = null;
        }
    }
}

// Mock WebSocket server for demonstration
export class MockWebSocketServer {
    private clients: Set<WebSocket> = new Set();
    private server: any = null;

    start(port: number = 8080): Promise<void> {
        return new Promise((resolve) => {
            console.log(`🖥️  Mock WebSocket server starting on port ${port}...`);

            // In a real implementation, you'd use a proper WebSocket server library
            // This is just for demonstration purposes
            console.log(`✅ Mock server ready - connect to ws://localhost:${port}`);
            resolve();
        });
    }

    stop(): void {
        console.log('🛑 Mock WebSocket server stopped');
    }

    // Simulate server message handling
    handleClientMessage(message: WebSocketMessage): void {
        console.log(`🖥️  Server received: ${message.type} from ${message.id}`);

        switch (message.type) {
            case 'processing_started':
                console.log(`   📄 Processing started for: ${message.data.fileName}`);
                break;
            case 'chunk_processed':
                console.log(`   📦 Chunk processed: ${message.data.progress.percentage}% complete`);
                break;
            case 'processing_complete':
                console.log(`   ✅ Processing completed: ${message.data.summary.totalChunks} chunks`);
                break;
            case 'error':
                console.log(`   ❌ Error: ${message.data.message}`);
                break;
        }
    }
}

// Example usage
export async function realTimeProcessingExample(file: File, wsUrl: string = 'ws://localhost:8080') {
    console.log('=== Real-time WebSocket Processing Example ===\n');

    // For demo, we'll simulate WebSocket behavior
    const processor = new PDFWebSocketProcessor(wsUrl);

    try {
        // In a real scenario, connect to actual WebSocket server
        console.log('🔌 Connecting to WebSocket server...');
        // await processor.connect();

        // Simulate connection for demo
        console.log('✅ Connected to WebSocket server (simulated)');

        // Process PDF with real-time updates
        await processor.processWithWebSocket(file);

    } catch (error) {
        console.error('❌ Real-time processing failed:', error);
    } finally {
        processor.disconnect();
    }
}

// Browser integration with simulated WebSocket
if (typeof window !== 'undefined') {
    document.addEventListener('DOMContentLoaded', () => {
        const container = document.createElement('div');
        container.innerHTML = `
      <h2>Real-time WebSocket Processing Example</h2>
      <div>
        <input type="file" id="pdfFile" accept=".pdf" />
        <br><br>
        <button id="processBtn" disabled>Start Real-time Processing</button>
        <button id="stopBtn" disabled>Stop</button>
      </div>
      <div id="status" style="margin: 10px 0; font-weight: bold;"></div>
      <div id="progress" style="margin: 10px 0;">
        <div style="width: 100%; background-color: #f0f0f0; border-radius: 5px;">
          <div id="progressBar" style="width: 0%; height: 20px; background-color: #4CAF50; border-radius: 5px; transition: width 0.3s;"></div>
        </div>
        <div id="progressText" style="margin-top: 5px; font-size: 12px;"></div>
      </div>
      <div id="chunks" style="max-height: 300px; overflow-y: auto; border: 1px solid #ddd; padding: 10px; margin: 10px 0;"></div>
      <div id="output" style="white-space: pre-wrap; font-family: monospace; background: #f5f5f5; padding: 10px; margin-top: 10px; max-height: 200px; overflow-y: auto;"></div>
    `;

        document.body.appendChild(container);

        const fileInput = document.getElementById('pdfFile') as HTMLInputElement;
        const processBtn = document.getElementById('processBtn') as HTMLButtonElement;
        const stopBtn = document.getElementById('stopBtn') as HTMLButtonElement;
        const status = document.getElementById('status') as HTMLDivElement;
        const progressBar = document.getElementById('progressBar') as HTMLDivElement;
        const progressText = document.getElementById('progressText') as HTMLDivElement;
        const chunks = document.getElementById('chunks') as HTMLDivElement;
        const output = document.getElementById('output') as HTMLDivElement;

        let currentProcessor: PDFWebSocketProcessor | null = null;

        fileInput.onchange = () => {
            processBtn.disabled = !fileInput.files?.[0];
        };

        processBtn.onclick = async () => {
            const file = fileInput.files?.[0];
            if (!file) return;

            // Create simulated processor with UI updates
            currentProcessor = new (class extends PDFWebSocketProcessor {
                constructor() {
                    super('ws://localhost:8080');
                }

                protected sendMessage(message: WebSocketMessage): void {
                    // Simulate real-time updates in UI
                    switch (message.type) {
                        case 'processing_started':
                            status.textContent = `Processing: ${message.data.fileName}`;
                            chunks.innerHTML = '';
                            break;

                        case 'progress_update':
                            progressBar.style.width = `${message.data.progress}%`;
                            progressText.textContent = `${message.data.currentOperation}: ${message.data.progress.toFixed(1)}%`;
                            break;

                        case 'chunk_processed':
                            const chunkDiv = document.createElement('div');
                            chunkDiv.style.cssText = 'border: 1px solid #ccc; margin: 5px 0; padding: 8px; border-radius: 3px; background: #fff;';
                            chunkDiv.innerHTML = `
                <strong>Chunk ${message.data.chunk.id.substring(0, 8)}...</strong> (Pages: ${message.data.chunk.pageNumbers.join(', ')})
                <br><small>${message.data.chunk.type} - ${message.data.chunk.confidence * 100}% confidence</small>
                <br>${message.data.chunk.summary}
                <br><em>Keywords: ${message.data.chunk.keywords.slice(0, 3).join(', ')}</em>
              `;
                            chunks.appendChild(chunkDiv);
                            chunks.scrollTop = chunks.scrollHeight;

                            progressBar.style.width = `${message.data.progress.percentage}%`;
                            progressText.textContent = `Processing chunks: ${message.data.progress.processed}/${message.data.progress.total} (${message.data.progress.percentage}%)`;
                            break;

                        case 'processing_complete':
                            status.textContent = 'Processing Complete!';
                            progressBar.style.width = '100%';
                            progressText.textContent = `Completed: ${message.data.summary.totalChunks} chunks in ${(message.data.summary.processingTime / 1000).toFixed(1)}s`;

                            const summaryDiv = document.createElement('div');
                            summaryDiv.style.cssText = 'background: #e8f5e8; border: 1px solid #4CAF50; padding: 10px; margin: 10px 0; border-radius: 5px;';
                            summaryDiv.innerHTML = `
                <strong>📊 Final Summary:</strong><br>
                Document Type: ${message.data.summary.documentType}<br>
                Total Pages: ${message.data.summary.totalPages}<br>
                Processing Time: ${(message.data.summary.processingTime / 1000).toFixed(1)}s<br>
                Key Insights: ${message.data.summary.keyInsights.join('; ')}
              `;
                            chunks.appendChild(summaryDiv);
                            break;

                        case 'error':
                            status.textContent = `Error: ${message.data.message}`;
                            status.style.color = 'red';
                            break;
                    }
                }
            })();

            // Redirect console.log to output
            const originalLog = console.log;
            console.log = (...args) => {
                output.textContent += args.join(' ') + '\n';
                output.scrollTop = output.scrollHeight;
                originalLog(...args);
            };

            try {
                processBtn.disabled = true;
                stopBtn.disabled = false;
                status.textContent = 'Starting...';
                status.style.color = 'black';
                output.textContent = '';

                if (currentProcessor) {
                    await currentProcessor.processWithWebSocket(file);
                }

            } catch (error) {
                status.textContent = `Error: ${(error as Error).message}`;
                status.style.color = 'red';
            } finally {
                processBtn.disabled = false;
                stopBtn.disabled = true;
                console.log = originalLog;
            }
        };

        stopBtn.onclick = () => {
            if (currentProcessor) {
                currentProcessor.disconnect();
                status.textContent = 'Stopped';
                processBtn.disabled = false;
                stopBtn.disabled = true;
            }
        };
    });
}