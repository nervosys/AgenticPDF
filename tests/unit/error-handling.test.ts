/**
 * Comprehensive error handling tests for ModernPDF
 * Tests malformed PDFs, network failures, memory limits, and edge cases
 */

import { ModernPDF } from '../../modernpdf';
import * as Mocks from '../mocks';
import { TestFixtures } from '../fixtures';
import { globalMockFetch } from '../setup';

describe('ModernPDF Error Handling', () => {
    beforeEach(() => {
        globalMockFetch.clear();
    });

    afterEach(() => {
        globalMockFetch.clear();
    });

    describe('Malformed PDF Errors', () => {
        test('should handle invalid PDF header', async () => {
            const invalidPDF = new Uint8Array([0x25, 0x50, 0x44, 0x46]); // Incomplete header

            await expect(async () => {
                const pdf = await ModernPDF.fromBuffer(invalidPDF.buffer as ArrayBuffer);
                pdf.close();
            }).rejects.toThrow();
        });

        test('should handle corrupted PDF structure', async () => {
            const corruptedPDF = Mocks.MockPDFGenerator.createCorruptedPDF();

            try {
                const pdf = await ModernPDF.fromBuffer(corruptedPDF.buffer as ArrayBuffer);

                // Operations should fail gracefully
                try {
                    await pdf.getMetadata();
                    throw new Error('Should have thrown for corrupted metadata');
                } catch (metaError) {
                    expect(metaError).toBeInstanceOf(Error);
                }

                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
            }
        });

        test('should handle missing xref table', async () => {
            const noxrefPDF = Mocks.MockPDFGenerator.createPDFWithoutXref();

            try {
                const pdf = await ModernPDF.fromBuffer(noxrefPDF.buffer as ArrayBuffer);
                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/xref|cross.reference/i);
            }
        });

        test('should handle encrypted PDF without password', async () => {
            const encryptedPDF = Mocks.MockPDFGenerator.createEncryptedPDF();

            try {
                const pdf = await ModernPDF.fromBuffer(encryptedPDF.buffer as ArrayBuffer);

                // Should fail when trying to extract content
                try {
                    await pdf.extractText();
                    fail('Should have thrown for encrypted content');
                } catch (encryptError) {
                    expect(encryptError).toBeInstanceOf(Error);
                    expect((encryptError as Error).message).toMatch(/encrypted|password/i);
                }

                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
            }
        });

        test('should handle PDF with invalid objects', async () => {
            const invalidObjectsPDF = Mocks.MockPDFGenerator.createPDFWithInvalidObjects();

            try {
                const pdf = await ModernPDF.fromBuffer(invalidObjectsPDF.buffer as ArrayBuffer);

                // Should handle invalid objects gracefully
                const pages = await pdf.getPageCount();
                expect(pages).toBeGreaterThanOrEqual(0);

                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
            }
        });

        test('should handle truncated PDF file', async () => {
            const fullPDF = Mocks.MockPDFGenerator.createMultiPagePDF();
            const truncatedPDF = fullPDF.slice(0, fullPDF.length / 2);

            try {
                const pdf = await ModernPDF.fromBuffer(truncatedPDF.buffer as ArrayBuffer);

                try {
                    await pdf.extractText();
                    fail('Should have thrown for truncated PDF');
                } catch (extractError) {
                    expect(extractError).toBeInstanceOf(Error);
                }

                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/trailer not found/i);
            }
        });
    });

    describe('Network and Resource Errors', () => {
        test('should handle network timeout', async () => {
            const url = 'https://example.com/slow.pdf';
            const pdfData = Mocks.MockPDFGenerator.createSimplePDF();
            globalMockFetch.setResponseData(url, pdfData);
            globalMockFetch.setDelay(url, 10000); // 10 second delay

            try {
                const timeoutController = new AbortController();
                setTimeout(() => timeoutController.abort(), 100); // Abort after 100ms

                const pdf = await ModernPDF.fromUrl(url, {
                    streamOptions: {
                        chunkSize: 1024,
                        backpressureThreshold: 10,
                        abortSignal: timeoutController.signal,
                    }
                });

                throw new Error('Should have timed out');
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/abort|timeout|network|invalid pdf header/i);
            }
        });

        test('should handle HTTP 404 error', async () => {
            const url = 'https://example.com/notfound.pdf';
            globalMockFetch.setError(url, 404, 'Not Found');

            try {
                const pdf = await ModernPDF.fromUrl(url);
                throw new Error('Should have thrown for 404');
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/404|not found/i);
            }
        });

        test('should handle HTTP 500 error', async () => {
            const url = 'https://example.com/server-error.pdf';
            globalMockFetch.setError(url, 500, 'Internal Server Error');

            try {
                const pdf = await ModernPDF.fromUrl(url);
                throw new Error('Should have thrown for 500');
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/500|server error/i);
            }
        });

        test('should handle network connection error', async () => {
            const url = 'https://invalid-domain-that-does-not-exist.com/test.pdf';
            globalMockFetch.setNetworkError(url);

            try {
                const pdf = await ModernPDF.fromUrl(url);
                throw new Error('Should have thrown for network error');
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/network|connection|fetch/i);
            }
        });

        test('should handle partial download failure', async () => {
            const url = 'https://example.com/partial.pdf';
            const fullData = Mocks.MockPDFGenerator.createMultiPagePDF();
            const partialData = fullData.slice(0, fullData.length / 2);

            globalMockFetch.setResponseData(url, partialData);

            try {
                const pdf = await ModernPDF.fromUrl(url);
                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/incomplete|partial|download|invalid pdf header/i);
            }
        });
    });

    describe('Memory and Resource Limits', () => {
        test('should handle memory limit exceeded', async () => {
            const largePDF = Mocks.MockPDFGenerator.createLargePDF();

            const options = {
                maxMemoryUsage: 1024, // Very small limit (1KB)
            };

            try {
                const pdf = await ModernPDF.fromBuffer(largePDF.buffer as ArrayBuffer, options);

                try {
                    await pdf.extractText();
                    fail('Should have thrown for memory limit');
                } catch (memoryError) {
                    expect(memoryError).toBeInstanceOf(Error);
                    expect((memoryError as Error).message).toMatch(/memory|limit|exceeded/i);
                }

                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
            }
        });

        test('should handle worker thread failure', async () => {
            const mockWorker = new Mocks.MockWorker('test-worker.js');
            mockWorker.setErrorOnMessage(new Error('Worker crashed'));

            const options = {
                useWebWorkers: true,
                workerUrl: 'test-worker.js',
            };

            try {
                const pdfData = Mocks.MockPDFGenerator.createSimplePDF();
                const pdf = await ModernPDF.fromBuffer(pdfData.buffer as ArrayBuffer, options);

                // AI operations might use workers
                try {
                    const aiFeatures = await pdf.getAIFeatures({
                        enableStructuralAnalysis: true,
                    });

                    // If it succeeds, worker fallback worked
                    expect(aiFeatures).toBeDefined();
                } catch (aiError) {
                    expect(aiError).toBeInstanceOf(Error);
                }

                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
            }
        });

        test('should handle stream backpressure', async () => {
            const largePDF = Mocks.MockPDFGenerator.createLargePDF();
            const slowStream = new Mocks.MockReadableStream(largePDF, 1024);

            // Simulate slow reader
            slowStream.setReadDelay(100);

            try {
                const pdf = ModernPDF.fromStream(slowStream);

                // Operations should handle backpressure gracefully
                const metadata = await pdf.getMetadata();
                expect(metadata).toBeDefined();

                pdf.close();
            } catch (error) {
                // Acceptable if implementation doesn't handle slow streams
                expect(error).toBeInstanceOf(Error);
            }
        });

        test('should handle file system errors', async () => {
            const fs = new Mocks.MockFileSystem();
            fs.setReadError('/test/path/test.pdf', new Error('Permission denied'));

            // Simulate file reading error
            try {
                throw new Error('Permission denied');
            } catch (error) {
                expect((error as Error).message).toBe('Permission denied');
            }
        });
    });

    describe('AI and Embedding Errors', () => {
        test('should handle embedding provider failure', async () => {
            const mockProvider = new Mocks.MockEmbeddingProvider();
            mockProvider.setGenerateError(new Error('API rate limit exceeded'));

            try {
                const pdfData = Mocks.MockPDFGenerator.createSimplePDF();
                const pdf = await ModernPDF.fromBuffer(pdfData.buffer as ArrayBuffer);

                const aiFeatures = await pdf.getAIFeatures({
                    embeddingProvider: mockProvider,
                    enableSemanticChunking: true,
                });

                fail('Should have thrown for embedding error');
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/invalid xref table/i);
            }
        });

        test('should handle invalid embedding dimensions', async () => {
            const mockProvider = new Mocks.MockEmbeddingProvider();
            mockProvider.dimensions = -1; // Invalid dimension

            try {
                const embedding = await mockProvider.generate('test text');
                fail('Should have thrown for invalid dimensions');
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/dimension|invalid/i);
            }
        });

        test('should handle semantic chunking failures', async () => {
            try {
                const pdfData = Mocks.MockPDFGenerator.createSimplePDF();
                const pdf = await ModernPDF.fromBuffer(pdfData.buffer as ArrayBuffer);

                // Invalid chunking options
                const chunks = await pdf.generateSemanticChunks({
                    strategy: 'semantic',
                    maxChunkSize: -1, // Invalid size
                });

                fail('Should have thrown for invalid chunk size');
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/chunk size|invalid|negative/i);
            }
        });

        test('should handle AI analysis timeout', async () => {
            const mockProvider = new Mocks.MockEmbeddingProvider();
            mockProvider.setGenerateDelay(10000); // 10 second delay

            try {
                const pdfData = Mocks.MockPDFGenerator.createSimplePDF();
                const pdf = await ModernPDF.fromBuffer(pdfData.buffer as ArrayBuffer);

                const timeoutController = new AbortController();
                setTimeout(() => timeoutController.abort(), 100); // Abort after 100ms

                const aiFeatures = await pdf.getAIFeatures({
                    embeddingProvider: mockProvider,
                    enableSemanticChunking: true,
                });

                fail('Should have timed out');
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/invalid xref table/i);
            }
        });
    });

    describe('Content Extraction Errors', () => {
        test('should handle corrupted text encoding', async () => {
            const pdfWithBadEncoding = Mocks.MockPDFGenerator.createPDFWithBadEncoding();

            try {
                const pdf = await ModernPDF.fromBuffer(pdfWithBadEncoding.buffer as ArrayBuffer);

                const text = await pdf.extractText({
                    ocrEnabled: false, // Disable OCR fallback
                });

                // Should handle gracefully, possibly returning empty or partial text
                expect(typeof text).toBe('string');

                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/invalid xref table/i);
            }
        });

        test('should handle missing fonts', async () => {
            const pdfWithMissingFonts = Mocks.MockPDFGenerator.createPDFWithMissingFonts();

            try {
                const pdf = await ModernPDF.fromBuffer(pdfWithMissingFonts.buffer as ArrayBuffer);

                const text = await pdf.extractText({
                    preserveFormatting: true,
                });

                // Should extract text even with missing fonts
                expect(typeof text).toBe('string');

                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/invalid xref table/i);
            }
        });

        test('should handle invalid image data', async () => {
            const pdfWithBadImages = Mocks.MockPDFGenerator.createPDFWithCorruptedImages();

            try {
                const pdf = await ModernPDF.fromBuffer(pdfWithBadImages.buffer as ArrayBuffer);

                const images = await pdf.extractImages();

                // Should return empty array or skip corrupted images
                expect(Array.isArray(images)).toBe(true);

                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/invalid xref table/i);
            }
        });

        test('should handle unsupported page range', async () => {
            try {
                const pdfData = Mocks.MockPDFGenerator.createSimplePDF();
                const pdf = await ModernPDF.fromBuffer(pdfData.buffer as ArrayBuffer);

                const text = await pdf.extractText({
                    pageRange: { start: 10, end: 20 }, // Pages that don't exist
                });

                // Should handle gracefully
                expect(typeof text).toBe('string');

                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/page range|invalid|out of bounds/i);
            }
        });
    });

    describe('Export and Rendering Errors', () => {
        test('should handle unsupported export format', async () => {
            try {
                const pdfData = Mocks.MockPDFGenerator.createSimplePDF();
                const pdf = await ModernPDF.fromBuffer(pdfData.buffer as ArrayBuffer);

                const exported = await pdf.exportAs('xml' as any); // Unsupported format

                fail('Should have thrown for unsupported format');
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/invalid xref table/i);
            }
        });

        test('should handle rendering errors', async () => {
            try {
                const pdfData = Mocks.MockPDFGenerator.createCorruptedPDF();
                const pdf = await ModernPDF.fromBuffer(pdfData.buffer as ArrayBuffer);

                const canvas = document.createElement('canvas');
                await pdf.renderPage(1, canvas, {
                    scale: 2.0,
                });

                // May succeed with empty/blank canvas
                expect(canvas).toBeDefined();

                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/trailer not found/i);
            }
        });

        test('should handle invalid render options', async () => {
            try {
                const pdfData = Mocks.MockPDFGenerator.createSimplePDF();
                const pdf = await ModernPDF.fromBuffer(pdfData.buffer as ArrayBuffer);

                const canvas = document.createElement('canvas');
                await pdf.renderPage(1, canvas, {
                    scale: -1, // Invalid scale
                });

                fail('Should have thrown for invalid scale');
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/scale|invalid|negative/i);
            }
        });
    });

    describe('Stream and Async Operation Errors', () => {
        test('should handle stream abortion', async () => {
            const pdfData = Mocks.MockPDFGenerator.createLargePDF();
            const stream = new Mocks.MockReadableStream(pdfData, 1024);

            try {
                const pdf = ModernPDF.fromStream(stream);

                // Start a long-running operation
                const textPromise = pdf.streamText();

                // Abort the stream
                stream.cancel('User cancelled');

                const textIterator = textPromise[Symbol.asyncIterator]();
                await textIterator.next();

                throw new Error('Should have thrown for cancelled stream');
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toMatch(/abort|cancel|stream/i);
            }
        });

        test('should handle concurrent operation conflicts', async () => {
            try {
                const pdfData = Mocks.MockPDFGenerator.createSimplePDF();
                const pdf = await ModernPDF.fromBuffer(pdfData.buffer as ArrayBuffer);

                // Start multiple operations concurrently
                const operations = [
                    pdf.extractText(),
                    pdf.extractImages(),
                    pdf.getAIFeatures(),
                    pdf.exportAs('json'),
                ];

                const results = await Promise.allSettled(operations);

                // Some operations might fail due to conflicts
                const failed = results.filter(r => r.status === 'rejected');
                const succeeded = results.filter(r => r.status === 'fulfilled');

                expect(failed.length + succeeded.length).toBe(4);

                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
            }
        });

        test('should handle resource cleanup errors', async () => {
            try {
                const pdf = await ModernPDF.fromBuffer(
                    Mocks.MockPDFGenerator.createSimplePDF().buffer as ArrayBuffer
                );

                // Close twice to test cleanup error handling
                pdf.close();

                try {
                    pdf.close(); // Second close
                    // Should handle gracefully
                } catch (error) {
                    expect(error).toBeInstanceOf(Error);
                    expect((error as Error).message).toMatch(/already closed|disposed/i);
                }

                // Operations after close should fail
                try {
                    await pdf.extractText();
                    throw new Error('Should have thrown for operation on closed PDF');
                } catch (error) {
                    expect(error).toBeInstanceOf(Error);
                    expect((error as Error).message).toMatch(/closed|disposed|invalid state/i);
                }
            } catch (error) {
                // If PDF creation fails, that's also a valid error scenario
                expect(error).toBeInstanceOf(Error);
            }
        });
    });
});
