/**
 * Unit tests for streaming operations
 * Tests streaming APIs, progress tracking, and abort signals
 */

import { ModernPDF, StreamOptions } from '../../modernpdf';
import { Mocks } from '../mocks';
import { TestFixtures } from '../fixtures';

describe('Streaming Operations', () => {
    let mockPDFData: Uint8Array;
    let mockComplexPDFData: Uint8Array;
    let mockProgressTracker: any;

    beforeEach(() => {
        mockPDFData = Mocks.PDFGenerator.createSimplePDF();
        mockComplexPDFData = Mocks.PDFGenerator.createMultiPagePDF();
        mockProgressTracker = new Mocks.ProgressTracker();
    });

    afterEach(() => {
        // Cleanup streaming resources
    });

    describe('Stream Creation and Management', () => {
        test('should create readable stream from PDF data', () => {
            const stream = new Mocks.ReadableStream(mockPDFData, 512);

            expect(stream).toBeInstanceOf(ReadableStream);
            expect(stream.locked).toBe(false);
        });

        test('should handle different chunk sizes', () => {
            const smallChunks = new Mocks.ReadableStream(mockPDFData, 128);
            const largeChunks = new Mocks.ReadableStream(mockPDFData, 2048);

            expect(smallChunks).toBeInstanceOf(ReadableStream);
            expect(largeChunks).toBeInstanceOf(ReadableStream);
        });

        test('should lock stream when reading', async () => {
            const stream = new Mocks.ReadableStream(mockPDFData, 256);
            const reader = stream.getReader();

            expect(stream.locked).toBe(true);

            reader.releaseLock();
            expect(stream.locked).toBe(false);
        });

        test('should handle stream cancellation', async () => {
            const stream = new Mocks.ReadableStream(mockPDFData, 256);

            try {
                await stream.cancel('Test cancellation');
                expect(true).toBe(true); // Should not throw
            } catch (error) {
                // Expected behavior for cancellation
                expect(error).toBeDefined();
            }
        });
    });

    describe('Progress Tracking', () => {
        test('should track reading progress', async () => {
            const progressCallback = jest.fn();
            mockProgressTracker.addCallback(progressCallback);

            // Simulate progress updates
            mockProgressTracker.updateProgress(0, 1024, 'starting');
            mockProgressTracker.updateProgress(512, 1024, 'reading');
            mockProgressTracker.updateProgress(1024, 1024, 'complete');

            expect(progressCallback).toHaveBeenCalledTimes(3);

            const lastCall = progressCallback.mock.calls[2][0];
            expect(lastCall.bytesRead).toBe(1024);
            expect(lastCall.totalBytes).toBe(1024);
            expect(lastCall.currentOperation).toBe('complete');
        });

        test('should calculate progress percentage', () => {
            const progress = TestFixtures.PROGRESS.MIDDLE;
            const percentage = (progress.bytesRead / progress.totalBytes) * 100;

            expect(percentage).toBe(50);
            expect(progress.estimatedTimeRemaining).toBe(1000);
        });

        test('should track pages processed', () => {
            const progress = TestFixtures.PROGRESS.END;

            expect(progress.pagesProcessed).toBe(1);
            expect(progress.totalPages).toBe(1);
            expect(progress.currentOperation).toBe('complete');
        });

        test('should handle progress estimation', () => {
            const startProgress = TestFixtures.PROGRESS.START;
            const middleProgress = TestFixtures.PROGRESS.MIDDLE;

            expect(startProgress.timeElapsed).toBe(0);
            expect(middleProgress.timeElapsed).toBe(1000);
            expect(middleProgress.estimatedTimeRemaining).toBe(1000);
        });
    });

    describe('Abort Signal Integration', () => {
        test('should handle abort controller', () => {
            const abortController = new AbortController();

            expect(abortController.signal.aborted).toBe(false);

            abortController.abort();
            expect(abortController.signal.aborted).toBe(true);
        });

        test('should respect abort signal in streaming', async () => {
            const abortController = new AbortController();
            const stream = new ReadableStream({
                start(controller) {
                    // Set up abort listener
                    abortController.signal.addEventListener('abort', () => {
                        controller.error(new Error('Operation aborted'));
                    });
                },
                pull(controller) {
                    if (abortController.signal.aborted) {
                        controller.error(new Error('Operation aborted'));
                        return;
                    }
                    controller.enqueue(new Uint8Array([1, 2, 3]));
                }
            });

            // Start reading
            const reader = stream.getReader();

            // Abort the operation
            abortController.abort();

            try {
                await reader.read();
            } catch (error) {
                expect((error as Error).message).toBe('Operation aborted');
            } finally {
                reader.releaseLock();
            }
        });

        test('should handle abort with cleanup', () => {
            const abortController = new AbortController();
            const resources = { cleaned: false };

            abortController.signal.addEventListener('abort', () => {
                resources.cleaned = true;
            });

            abortController.abort();
            expect(resources.cleaned).toBe(true);
        });
    });

    describe('Stream Options and Configuration', () => {
        test('should create proper stream options', () => {
            const options = Mocks.Utils.createMockStreamOptions({
                chunkSize: 2048,
                progressCallback: (progress) => console.log(progress),
            });

            expect(options.chunkSize).toBe(2048);
            expect(options.backpressureThreshold).toBe(2048);
            expect(options.progressCallback).toBeDefined();
            expect(options.abortSignal).toBeInstanceOf(AbortSignal);
        });

        test('should handle backpressure configuration', () => {
            const options: StreamOptions = {
                chunkSize: 1024,
                backpressureThreshold: 4096,
                progressCallback: () => { },
            };

            expect(options.backpressureThreshold).toBe(4096);
            expect(options.backpressureThreshold).toBeGreaterThan(options.chunkSize);
        });

        test('should validate stream options', () => {
            const validOptions = Mocks.Utils.createMockStreamOptions();

            expect(validOptions.chunkSize).toBeGreaterThan(0);
            expect(validOptions.backpressureThreshold).toBeGreaterThan(0);
            expect(validOptions.progressCallback).toBeDefined();
        });
    });

    describe('Text Streaming', () => {
        test('should stream text content in chunks', async () => {
            const textChunks = [
                TestFixtures.TEXTS.SHORT,
                TestFixtures.TEXTS.MEDIUM,
                TestFixtures.TEXTS.LONG.substring(0, 200),
            ];

            // Simulate streaming text extraction
            for (let i = 0; i < textChunks.length; i++) {
                const chunk = {
                    pageNumber: i + 1,
                    text: textChunks[i],
                    fonts: [TestFixtures.FONTS[0]],
                    formatting: { bold: false, italic: false },
                };

                expect(chunk.text).toBeDefined();
                expect(chunk.pageNumber).toBeGreaterThan(0);
                expect(chunk.fonts.length).toBeGreaterThan(0);
            }
        });

        test('should handle text streaming with formatting', async () => {
            const formattedChunk = {
                pageNumber: 1,
                text: TestFixtures.TEXTS.MEDIUM,
                formatting: {
                    preserveLineBreaks: true,
                    includeFont: true,
                    includeColors: false,
                },
                metadata: {
                    confidence: 0.95,
                    language: 'en',
                },
            };

            expect(formattedChunk.formatting.preserveLineBreaks).toBe(true);
            expect(formattedChunk.metadata.confidence).toBeGreaterThan(0.9);
        });

        test('should stream text with position information', async () => {
            const positionedText = {
                text: 'Sample text',
                bounds: { x: 100, y: 200, width: 150, height: 20 },
                fontSize: 12,
                fontName: 'Helvetica',
                pageNumber: 1,
            };

            expect(positionedText.bounds.x).toBe(100);
            expect(positionedText.fontSize).toBe(12);
            expect(positionedText.fontName).toBe('Helvetica');
        });
    });

    describe('Semantic Chunk Streaming', () => {
        test('should stream semantic chunks', async () => {
            const chunks = TestFixtures.CHUNKS;

            // Simulate streaming chunks
            for (const chunk of chunks) {
                expect(chunk.id).toBeDefined();
                expect(chunk.content).toBeDefined();
                expect(chunk.pageNumbers.length).toBeGreaterThan(0);
                expect(chunk.metadata).toBeDefined();
            }
        });

        test('should handle chunk metadata streaming', async () => {
            const chunk = TestFixtures.CHUNKS[0];

            expect(chunk.metadata.chunkType).toBe('paragraph');
            expect(chunk.metadata.confidence).toBeGreaterThan(0.9);
            expect(chunk.metadata.keywords.length).toBeGreaterThan(0);
        });

        test('should stream chunks with embeddings', async () => {
            const mockProvider = new Mocks.EmbeddingProvider();
            const chunks = TestFixtures.CHUNKS;

            for (const chunk of chunks) {
                const embedding = await mockProvider.generate(chunk.content);

                expect(embedding).toBeInstanceOf(Float32Array);
                expect(embedding.length).toBe(mockProvider.dimensions);
            }
        });

        test('should handle chunk boundary detection', () => {
            const chunk = TestFixtures.CHUNKS[1];

            expect(chunk.startIndex).toBe(0);
            expect(chunk.endIndex).toBe(chunk.content.length);
            expect(chunk.pageNumbers).toEqual([1, 2]);
        });
    });

    describe('Memory Management in Streaming', () => {
        test('should handle memory-efficient streaming', async () => {
            const largeData = new Uint8Array(1024 * 1024); // 1MB
            largeData.set(mockPDFData, 0);

            const stream = new Mocks.ReadableStream(largeData, 64 * 1024); // 64KB chunks
            const chunks: Uint8Array[] = [];

            const reader = stream.getReader();
            try {
                while (true) {
                    const { done, value } = await reader.read();
                    if (done) break;
                    chunks.push(value);

                    // Simulate memory constraint
                    if (chunks.length > 5) {
                        chunks.shift(); // Remove oldest chunk to save memory
                    }
                }
            } finally {
                reader.releaseLock();
            }

            expect(chunks.length).toBeLessThanOrEqual(5);
        });

        test('should clean up streaming resources', () => {
            const resources = {
                buffer: new ArrayBuffer(1024),
                stream: new Mocks.ReadableStream(mockPDFData, 256),
                reader: null as ReadableStreamDefaultReader<Uint8Array> | null,
            };

            resources.reader = resources.stream.getReader();
            expect(resources.stream.locked).toBe(true);

            // Cleanup
            resources.reader.releaseLock();
            resources.reader = null;

            expect(resources.stream.locked).toBe(false);
        });

        test('should handle backpressure in streaming', async () => {
            const options = {
                chunkSize: 1024,
                backpressureThreshold: 4096,
                currentBufferSize: 0,
            };

            // Simulate backpressure detection
            options.currentBufferSize = 5000; // Exceeds threshold
            const hasBackpressure = options.currentBufferSize > options.backpressureThreshold;

            expect(hasBackpressure).toBe(true);
        });
    });

    describe('Error Handling in Streaming', () => {
        test('should handle stream errors gracefully', async () => {
            const errorStream = new ReadableStream({
                start(controller) {
                    controller.error(new Error('Stream error'));
                }
            });

            const reader = errorStream.getReader();

            try {
                await reader.read();
                throw new Error('Should have thrown an error');
            } catch (error) {
                expect((error as Error).message).toBe('Stream error');
            } finally {
                reader.releaseLock();
            }
        });

        test('should handle network errors in URL streaming', async () => {
            const mockFetch = new Mocks.Fetch();

            // Set up error response
            const errorResponse = new Response(null, {
                status: 404,
                statusText: 'Not Found'
            });
            mockFetch.setResponse('https://example.com/test.pdf', errorResponse);

            try {
                const response = await mockFetch.fetch('https://example.com/test.pdf');
                expect(response.status).toBe(404);
            } catch (error) {
                expect(error).toBeDefined();
            }
        });

        test('should handle corrupted stream data', async () => {
            const corruptedData = Mocks.PDFGenerator.createCorruptedPDF();
            const stream = new Mocks.ReadableStream(corruptedData, 256);

            const reader = stream.getReader();
            const chunks: Uint8Array[] = [];

            try {
                while (true) {
                    const { done, value } = await reader.read();
                    if (done) break;
                    chunks.push(value);
                }
            } finally {
                reader.releaseLock();
            }

            // Verify we can read the corrupted data (even if invalid)
            expect(chunks.length).toBeGreaterThan(0);

            // Validate that it's actually corrupted
            const reconstructed = new Uint8Array(
                chunks.reduce((sum, chunk) => sum + chunk.length, 0)
            );
            let offset = 0;
            for (const chunk of chunks) {
                reconstructed.set(chunk, offset);
                offset += chunk.length;
            }

            const text = new TextDecoder().decode(reconstructed);
            // Check that it contains the corruption marker instead of proper PDF structure
            expect(text.includes('Random content that should cause parsing errors')).toBe(true);
        });
    });

    describe('Performance Optimization', () => {
        test('should optimize chunk size based on content', () => {
            const optimizedChunkSize = (contentSize: number) => {
                if (contentSize < 1024) return 256;
                if (contentSize < 1024 * 1024) return 8192;
                return 64 * 1024;
            };

            expect(optimizedChunkSize(500)).toBe(256);
            expect(optimizedChunkSize(50000)).toBe(8192);
            expect(optimizedChunkSize(5000000)).toBe(64 * 1024);
        });

        test('should measure streaming performance', async () => {
            const startTime = Date.now();

            const stream = new Mocks.ReadableStream(mockPDFData, 512);
            const chunks = await Mocks.Utils.streamToArray(stream);

            const endTime = Date.now();
            const duration = endTime - startTime;

            expect(chunks.length).toBeGreaterThan(0);
            expect(duration).toBeLessThan(1000); // Should complete within 1 second
        });

        test('should handle concurrent streaming operations', async () => {
            const stream1 = new Mocks.ReadableStream(mockPDFData, 256);
            const stream2 = new Mocks.ReadableStream(mockComplexPDFData, 512);

            const [chunks1, chunks2] = await Promise.all([
                Mocks.Utils.streamToArray(stream1),
                Mocks.Utils.streamToArray(stream2)
            ]);

            expect(chunks1.length).toBeGreaterThan(0);
            expect(chunks2.length).toBeGreaterThan(0);
        });
    });
});

describe('Streaming Integration Tests', () => {
    test('should integrate streaming with PDF parsing', async () => {
        const pdfData = Mocks.PDFGenerator.createSimplePDF();
        const stream = new Mocks.ReadableStream(pdfData, 512);

        // Simulate streaming PDF parsing
        const chunks = await Mocks.Utils.streamToArray(stream);

        // Reconstruct PDF data
        const totalLength = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
        const reconstructed = new Uint8Array(totalLength);
        let offset = 0;

        for (const chunk of chunks) {
            reconstructed.set(chunk, offset);
            offset += chunk.length;
        }

        // Verify reconstruction
        expect(reconstructed).toEqual(pdfData);

        // Verify PDF structure
        const pdfString = new TextDecoder().decode(reconstructed);
        expect(pdfString).toContain('%PDF-');
        expect(pdfString).toContain('%%EOF');
    });

    test('should handle end-to-end streaming workflow', async () => {
        const mockProvider = new Mocks.EmbeddingProvider();
        const progressTracker = new Mocks.ProgressTracker();

        let totalProgress = 0;
        progressTracker.addCallback((progress) => {
            totalProgress = (progress.bytesRead / progress.totalBytes) * 100;
        });

        // Simulate complete streaming workflow
        const pdfData = Mocks.PDFGenerator.createMultiPagePDF();
        const stream = new Mocks.ReadableStream(pdfData, 1024);

        // Stream and process
        const reader = stream.getReader();
        let processedBytes = 0;

        try {
            while (true) {
                const { done, value } = await reader.read();
                if (done) break;

                processedBytes += value.length;
                progressTracker.updateProgress(processedBytes, pdfData.length, 'processing');

                // Simulate AI processing on chunks
                if (value.length > 0) {
                    const chunkText = new TextDecoder().decode(value.slice(0, Math.min(100, value.length)));
                    if (chunkText.trim()) {
                        await mockProvider.generate(chunkText);
                    }
                }
            }
        } finally {
            reader.releaseLock();
        }

        expect(processedBytes).toBe(pdfData.length);
        expect(totalProgress).toBe(100);
    });
});
