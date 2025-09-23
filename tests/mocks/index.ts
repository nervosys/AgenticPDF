/**
 * Mock utilities for ModernPDF testing
 * Provides mock implementations for external dependencies
 */

import { EmbeddingProvider, StreamOptions, ProgressInfo } from '../../modernpdf';

/**
 * Mock embedding provider for testing AI features
 */
export class MockEmbeddingProvider implements EmbeddingProvider {
    model = 'mock-embedding-model';
    dimensions = 384;
    private generateError: Error | null = null;
    private generateDelay = 0;

    async generate(text: string): Promise<Float32Array> {
        // Apply delay if set
        if (this.generateDelay > 0) {
            await new Promise(resolve => setTimeout(resolve, this.generateDelay));
        }

        // Throw error if set
        if (this.generateError) {
            throw this.generateError;
        }

        // Validate dimensions
        if (this.dimensions < 1) {
            throw new Error('Invalid embedding dimensions');
        }

        // Return a deterministic mock embedding based on text length
        const values = new Array(this.dimensions).fill(0).map((_, i) =>
            Math.sin(text.length + i) * 0.1
        );
        return new Float32Array(values);
    }

    async generateBatch(texts: string[]): Promise<Float32Array[]> {
        return Promise.all(texts.map(text => this.generate(text)));
    }

    setGenerateError(error: Error | null): void {
        this.generateError = error;
    }

    setGenerateDelay(delayMs: number): void {
        this.generateDelay = delayMs;
    }
}

/**
 * Mock PDF data generator for testing
 */
export class MockPDFGenerator {
    static createSimplePDF(): Uint8Array {
        // Minimal valid PDF structure for testing
        const pdfContent = `%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj

2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj

3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
>>
endobj

4 0 obj
<<
/Length 44
>>
stream
BT
/F1 12 Tf
72 720 Td
(Hello World) Tj
ET
endstream
endobj

xref
0 5
0000000000 65535 f 
0000000010 00000 n 
0000000053 00000 n 
0000000125 00000 n 
0000000185 00000 n 
trailer
<<
/Size 5
/Root 1 0 R
>>
startxref
279
%%EOF`;

        return new TextEncoder().encode(pdfContent);
    }

    static createMultiPagePDF(): Uint8Array {
        // More complex PDF with multiple pages for testing
        const pdfContent = `%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj

2 0 obj
<<
/Type /Pages
/Kids [3 0 R 5 0 R]
/Count 2
>>
endobj

3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
>>
endobj

4 0 obj
<<
/Length 50
>>
stream
BT
/F1 12 Tf
72 720 Td
(Page 1 Content) Tj
ET
endstream
endobj

5 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 6 0 R
>>
endobj

6 0 obj
<<
/Length 50
>>
stream
BT
/F1 12 Tf
72 720 Td
(Page 2 Content) Tj
ET
endstream
endobj

xref
0 7
0000000000 65535 f 
0000000010 00000 n 
0000000053 00000 n 
0000000132 00000 n 
0000000192 00000 n 
0000000292 00000 n 
0000000352 00000 n 
trailer
<<
/Size 7
/Root 1 0 R
>>
startxref
452
%%EOF`;

        return new TextEncoder().encode(pdfContent);
    }

    static createCorruptedPDF(): Uint8Array {
        // Invalid PDF for error testing
        const pdfContent = `%PDF-1.4
This is not a valid PDF structure
Random content that should cause parsing errors
%%EOF`;

        return new TextEncoder().encode(pdfContent);
    }

    static createPDFWithoutXref(): Uint8Array {
        // PDF missing xref table
        const pdfContent = `%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj

2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj

3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
>>
endobj

trailer
<<
/Size 4
/Root 1 0 R
>>
startxref
200
%%EOF`;

        return new TextEncoder().encode(pdfContent);
    }

    static createEncryptedPDF(): Uint8Array {
        // Simulated encrypted PDF
        const pdfContent = `%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj

2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj

3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
>>
endobj

xref
0 4
0000000000 65535 f 
0000000010 00000 n 
0000000053 00000 n 
0000000125 00000 n 
trailer
<<
/Size 4
/Root 1 0 R
/Encrypt 5 0 R
>>
startxref
185
%%EOF`;

        return new TextEncoder().encode(pdfContent);
    }

    static createPDFWithInvalidObjects(): Uint8Array {
        // PDF with malformed objects
        const pdfContent = `%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages INVALID_REFERENCE
>>
endobj

2 0 obj
MALFORMED OBJECT CONTENT
endobj

xref
0 3
0000000000 65535 f 
0000000010 00000 n 
0000000070 00000 n 
trailer
<<
/Size 3
/Root 1 0 R
>>
startxref
120
%%EOF`;

        return new TextEncoder().encode(pdfContent);
    }

    static createLargePDF(): Uint8Array {
        // Create a larger PDF for memory testing
        const pageContent = 'Lorem ipsum dolor sit amet, consectetur adipiscing elit. '.repeat(1000);
        const pages = [];

        for (let i = 0; i < 100; i++) {
            pages.push(`
${3 + i * 2} 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents ${4 + i * 2} 0 R
>>
endobj

${4 + i * 2} 0 obj
<<
/Length ${pageContent.length}
>>
stream
BT
/F1 12 Tf
72 720 Td
(${pageContent}) Tj
ET
endstream
endobj`);
        }

        const kidRefs = Array.from({ length: 100 }, (_, i) => `${3 + i * 2} 0 R`).join(' ');
        const xrefEntries = '0000000000 65535 f \n' + Array.from({ length: 202 }, (_, i) => {
            const offset = (i + 1) * 100;
            return offset.toString().padStart(10, '0') + ' 00000 n \n';
        }).join('');

        const pdfContent = `%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj

2 0 obj
<<
/Type /Pages
/Kids [${kidRefs}]
/Count 100
>>
endobj
${pages.join('')}

xref
0 203
${xrefEntries}
trailer
<<
/Size 203
/Root 1 0 R
>>
startxref
20000
%%EOF`;

        return new TextEncoder().encode(pdfContent);
    }

    static createPDFWithBadEncoding(): Uint8Array {
        // PDF with invalid text encoding
        const pdfContent = `%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj

2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj

3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
>>
endobj

4 0 obj
<<
/Length 30
>>
stream
BT
/F1 12 Tf
72 720 Td
(\xFF\xFE\x00Invalid) Tj
ET
endstream
endobj

xref
0 5
0000000000 65535 f 
0000000010 00000 n 
0000000053 00000 n 
0000000125 00000 n 
0000000185 00000 n 
trailer
<<
/Size 5
/Root 1 0 R
>>
startxref
260
%%EOF`;

        return new TextEncoder().encode(pdfContent);
    }

    static createPDFWithMissingFonts(): Uint8Array {
        // PDF referencing non-existent fonts
        const pdfContent = `%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj

2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj

3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
/Resources <<
  /Font <<
    /F1 999 0 R
  >>
>>
>>
endobj

4 0 obj
<<
/Length 40
>>
stream
BT
/F1 12 Tf
72 720 Td
(Missing Font Text) Tj
ET
endstream
endobj

xref
0 5
0000000000 65535 f 
0000000010 00000 n 
0000000053 00000 n 
0000000125 00000 n 
0000000270 00000 n 
trailer
<<
/Size 5
/Root 1 0 R
>>
startxref
355
%%EOF`;

        return new TextEncoder().encode(pdfContent);
    }

    static createPDFWithCorruptedImages(): Uint8Array {
        // PDF with invalid image data
        const pdfContent = `%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj

2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj

3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 4 0 R
/Resources <<
  /XObject <<
    /Im1 5 0 R
  >>
>>
>>
endobj

4 0 obj
<<
/Length 25
>>
stream
q
100 0 0 100 50 700 cm
/Im1 Do
Q
endstream
endobj

5 0 obj
<<
/Type /XObject
/Subtype /Image
/Width 10
/Height 10
/BitsPerComponent 8
/ColorSpace /DeviceRGB
/Length 20
>>
stream
CORRUPTED IMAGE DATA
endstream
endobj

xref
0 6
0000000000 65535 f 
0000000010 00000 n 
0000000053 00000 n 
0000000125 00000 n 
0000000280 00000 n 
0000000350 00000 n 
trailer
<<
/Size 6
/Root 1 0 R
>>
startxref
500
%%EOF`;

        return new TextEncoder().encode(pdfContent);
    }
}

/**
 * Mock stream implementation for testing streaming operations
 */
export class MockReadableStream extends ReadableStream<Uint8Array> {
    private readDelay = 0;

    constructor(data: Uint8Array, chunkSize = 1024) {
        let position = 0;
        let readDelay = 0;

        super({
            start(controller) {
                // Start streaming immediately
            },
            async pull(controller) {
                if (position >= data.length) {
                    controller.close();
                    return;
                }

                // Apply read delay if set
                if (readDelay > 0) {
                    await new Promise(resolve => setTimeout(resolve, readDelay));
                }

                const chunk = data.slice(position, position + chunkSize);
                position += chunkSize;
                controller.enqueue(chunk);
            },
            cancel(reason) {
                // Handle cancellation
                console.log('Stream cancelled:', reason);
            }
        });

        // Store reference to access delay later
        (this as any)._setReadDelay = (delayMs: number) => {
            readDelay = delayMs;
        };
    }

    setReadDelay(delayMs: number): void {
        (this as any)._setReadDelay(delayMs);
    }
}

/**
 * Mock progress callback for testing progress tracking
 */
export class MockProgressTracker {
    private callbacks: ((progress: ProgressInfo) => void)[] = [];
    private currentProgress = {
        bytesRead: 0,
        totalBytes: 100,
        pagesProcessed: 0,
        currentOperation: 'test',
        timeElapsed: 0
    };

    addCallback(callback: (progress: ProgressInfo) => void): void {
        this.callbacks.push(callback);
    }

    updateProgress(bytesRead: number, totalBytes: number, operation: string): void {
        this.currentProgress = {
            bytesRead,
            totalBytes,
            pagesProcessed: Math.floor(bytesRead / totalBytes * 10),
            currentOperation: operation,
            timeElapsed: Date.now()
        };
        this.callbacks.forEach(callback => callback(this.currentProgress));
    }

    getProgress() {
        return this.currentProgress;
    }
}

/**
 * Mock file system operations for testing
 */
export class MockFileSystem {
    private files = new Map<string, Uint8Array>();
    private readErrors = new Map<string, Error>();

    setFile(path: string, content: Uint8Array): void {
        this.files.set(path, content);
    }

    getFile(path: string): Uint8Array | undefined {
        return this.files.get(path);
    }

    hasFile(path: string): boolean {
        return this.files.has(path);
    }

    setReadError(path: string, error: Error): void {
        this.readErrors.set(path, error);
    }

    async readFile(path: string): Promise<Uint8Array> {
        const error = this.readErrors.get(path);
        if (error) {
            throw error;
        }
        return this.files.get(path) || new TextEncoder().encode('mock file content');
    }

    clear(): void {
        this.files.clear();
        this.readErrors.clear();
    }

    removeFile(path: string): boolean {
        return this.files.delete(path);
    }

    listFiles(): string[] {
        return Array.from(this.files.keys());
    }
}

/**
 * Mock fetch implementation for URL testing
 */
export class MockFetch {
    private responses = new Map<string, Response>();
    private delays = new Map<string, number>();
    private errors = new Map<string, { status: number; statusText: string }>();
    private networkErrors = new Set<string>();

    setResponse(url: string, response: Response): void {
        this.responses.set(url, response);
    }

    setResponseData(url: string, data: Uint8Array, options: ResponseInit = {}): void {
        const blob = new Blob([new ArrayBuffer(data.byteLength)]);
        const response = new Response(blob, {
            status: 200,
            statusText: 'OK',
            headers: { 'Content-Type': 'application/pdf' },
            ...options
        });
        this.responses.set(url, response);
    }

    setDelay(url: string, delayMs: number): void {
        this.delays.set(url, delayMs);
    }

    setError(url: string, status: number, statusText: string): void {
        this.errors.set(url, { status, statusText });
    }

    setNetworkError(url: string): void {
        this.networkErrors.add(url);
    }

    fetch = async (url: string | URL): Promise<Response> => {
        const urlString = typeof url === 'string' ? url : url.toString();

        // Check for network error
        if (this.networkErrors.has(urlString)) {
            throw new Error(`Network error: Failed to fetch ${urlString}`);
        }

        // Check for HTTP error
        const errorConfig = this.errors.get(urlString);
        if (errorConfig) {
            const response = new Response(null, {
                status: errorConfig.status,
                statusText: errorConfig.statusText
            });
            if (!response.ok) {
                throw new Error(`${errorConfig.status} ${errorConfig.statusText}`);
            }
            return response;
        }

        // Apply delay if configured
        const delay = this.delays.get(urlString);
        if (delay) {
            await new Promise(resolve => setTimeout(resolve, delay));
        }

        const response = this.responses.get(urlString);

        if (response) {
            return Promise.resolve(response.clone());
        }

        return Promise.reject(new Error(`No mock response for URL: ${urlString}`));
    };

    clear(): void {
        this.responses.clear();
        this.delays.clear();
        this.errors.clear();
        this.networkErrors.clear();
    }
}

/**
 * Test utilities for creating common test scenarios
 */
export class TestUtils {
    static createMockStreamOptions(overrides: Partial<StreamOptions> = {}): StreamOptions {
        return {
            chunkSize: 1024,
            backpressureThreshold: 2048,
            progressCallback: () => { },
            abortSignal: new AbortController().signal,
            ...overrides
        };
    }

    static createMockFile(name: string, content: Uint8Array, type = 'application/pdf'): File {
        const buffer = new ArrayBuffer(content.byteLength);
        const view = new Uint8Array(buffer);
        view.set(content);
        return new File([buffer], name, { type });
    }

    static async streamToArray<T>(stream: ReadableStream<T>): Promise<T[]> {
        const reader = stream.getReader();
        const chunks: T[] = [];

        try {
            while (true) {
                const { done, value } = await reader.read();
                if (done) break;
                chunks.push(value);
            }
        } finally {
            reader.releaseLock();
        }

        return chunks;
    }

    static sleep(ms: number): Promise<void> {
        return new Promise(resolve => setTimeout(resolve, ms));
    }

    static createMockAbortController(): AbortController {
        const controller = new AbortController();
        return controller;
    }
}

/**
 * Mock Web Worker for testing worker-based operations
 */
export class MockWorker {
    private listeners = new Map<string, Function[]>();
    private messageError: Error | null = null;

    constructor(public scriptURL: string) {
        // Simulate worker initialization
    }

    postMessage(message: any): void {
        // Simulate message handling
        setTimeout(() => {
            if (this.messageError) {
                this.triggerEvent('error', this.messageError);
            } else {
                this.triggerEvent('message', { data: { type: 'response', result: 'mock-result' } });
            }
        }, 10);
    }

    terminate(): void {
        this.listeners.clear();
    }

    addEventListener(type: string, listener: Function): void {
        if (!this.listeners.has(type)) {
            this.listeners.set(type, []);
        }
        this.listeners.get(type)!.push(listener);
    }

    removeEventListener(type: string, listener: Function): void {
        const listeners = this.listeners.get(type);
        if (listeners) {
            const index = listeners.indexOf(listener);
            if (index > -1) {
                listeners.splice(index, 1);
            }
        }
    }

    setErrorOnMessage(error: Error): void {
        this.messageError = error;
    }

    private triggerEvent(type: string, event: any): void {
        const listeners = this.listeners.get(type) || [];
        listeners.forEach(listener => listener(event));
    }
}

// Export all mocks as default collection
export const Mocks = {
    EmbeddingProvider: MockEmbeddingProvider,
    PDFGenerator: MockPDFGenerator,
    ReadableStream: MockReadableStream,
    ProgressTracker: MockProgressTracker,
    FileSystem: MockFileSystem,
    Fetch: MockFetch,
    Worker: MockWorker,
    Utils: TestUtils
};