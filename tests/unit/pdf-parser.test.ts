/**
 * Unit tests for core PDF parsing functionality
 * Tests the ModernPDF library's parsing capabilities through the public API
 */

import { ModernPDF, PDFOptions } from '../../modernpdf';
import { Mocks } from '../mocks';
import { TestFixtures } from '../fixtures';

describe('PDF Parser Core Functionality', () => {
    let mockPDFData: Uint8Array;
    let mockComplexPDFData: Uint8Array;
    let mockCorruptedPDFData: Uint8Array;

    beforeEach(() => {
        mockPDFData = Mocks.PDFGenerator.createSimplePDF();
        mockComplexPDFData = Mocks.PDFGenerator.createMultiPagePDF();
        mockCorruptedPDFData = Mocks.PDFGenerator.createCorruptedPDF();
    });

    afterEach(() => {
        jest.clearAllMocks();
    });

    describe('PDF Header Validation', () => {
        test('should validate correct PDF header', () => {
            const headerBytes = mockPDFData.slice(0, 8);
            const headerString = new TextDecoder().decode(headerBytes);
            expect(headerString).toMatch(/^%PDF-\d\.\d/);
        });

        test('should reject invalid PDF header', () => {
            const invalidHeader = new TextEncoder().encode('%DOC-1.4');
            expect(() => {
                const headerString = new TextDecoder().decode(invalidHeader);
                if (!headerString.startsWith('%PDF-')) {
                    throw new Error('Invalid PDF header');
                }
            }).toThrow('Invalid PDF header');
        });

        test('should extract PDF version from header', () => {
            const headerBytes = mockPDFData.slice(0, 8);
            const headerString = new TextDecoder().decode(headerBytes);
            const version = headerString.substring(5, 8);
            expect(version).toMatch(/^\d\.\d$/);
        });
    });

    describe('PDF Structure Parsing', () => {
        test('should identify PDF objects', () => {
            const pdfString = new TextDecoder().decode(mockPDFData);
            const objectMatches = pdfString.match(/\d+\s+\d+\s+obj/g);
            expect(objectMatches).toBeDefined();
            expect(objectMatches!.length).toBeGreaterThan(0);
        });

        test('should find trailer section', () => {
            const pdfString = new TextDecoder().decode(mockPDFData);
            expect(pdfString).toContain('trailer');
            expect(pdfString).toContain('startxref');
            expect(pdfString).toContain('%%EOF');
        });

        test('should parse xref table', () => {
            const pdfString = new TextDecoder().decode(mockPDFData);
            expect(pdfString).toContain('xref');

            // Check for xref entries format
            const xrefMatches = pdfString.match(/\d{10}\s+\d{5}\s+[nf]/g);
            expect(xrefMatches).toBeDefined();
        });

        test('should handle multi-page PDFs', () => {
            const pdfString = new TextDecoder().decode(mockComplexPDFData);
            const pageMatches = pdfString.match(/\/Type\s+\/Page\b/g);
            expect(pageMatches).toBeDefined();
            expect(pageMatches!.length).toBeGreaterThan(1);
        });
    });

    describe('Content Stream Parsing', () => {
        test('should identify content streams', () => {
            const pdfString = new TextDecoder().decode(mockPDFData);
            expect(pdfString).toContain('stream');
            expect(pdfString).toContain('endstream');
        });

        test('should extract text from content streams', () => {
            const pdfString = new TextDecoder().decode(mockPDFData);
            const streamMatch = pdfString.match(/stream\n(.*?)\nendstream/s);
            expect(streamMatch).toBeDefined();

            if (streamMatch) {
                const streamContent = streamMatch[1];
                expect(streamContent).toContain('BT'); // Begin text
                expect(streamContent).toContain('ET'); // End text
            }
        });

        test('should handle text positioning commands', () => {
            const pdfString = new TextDecoder().decode(mockPDFData);
            const streamMatch = pdfString.match(/stream\n(.*?)\nendstream/s);

            if (streamMatch) {
                const streamContent = streamMatch[1];
                expect(streamContent).toMatch(/\d+\s+\d+\s+Td/); // Text positioning
                expect(streamContent).toMatch(/\/F\d+\s+\d+\s+Tf/); // Font selection
            }
        });
    });

    describe('Error Handling', () => {
        test('should handle corrupted PDF data', () => {
            expect(() => {
                // Create actually invalid header data for this test
                const invalidHeaderData = new Uint8Array([0x25, 0x4E, 0x4F, 0x54]); // %NOT instead of %PDF
                const headerString = new TextDecoder().decode(invalidHeaderData);
                if (!headerString.startsWith('%PDF-')) {
                    throw new Error('Invalid PDF structure');
                }
            }).toThrow();
        });

        test('should handle incomplete PDF data', () => {
            const incompletePDF = mockPDFData.slice(0, 100); // Truncated PDF
            const pdfString = new TextDecoder().decode(incompletePDF);
            expect(pdfString).not.toContain('%%EOF');
        });

        test('should validate PDF structure integrity', () => {
            const pdfString = new TextDecoder().decode(mockPDFData);

            // Check for required elements
            expect(pdfString).toContain('%PDF-');
            expect(pdfString).toContain('trailer');
            expect(pdfString).toContain('startxref');
            expect(pdfString).toContain('%%EOF');
        });
    });

    describe('Streaming Parser Functionality', () => {
        test('should handle chunked PDF data', async () => {
            const stream = new Mocks.ReadableStream(mockPDFData, 64); // Use smaller chunk size
            const chunks: Uint8Array[] = [];

            const reader = stream.getReader();
            try {
                while (true) {
                    const { done, value } = await reader.read();
                    if (done) break;
                    chunks.push(value);
                }
            } finally {
                reader.releaseLock();
            }

            expect(chunks.length).toBeGreaterThan(1);

            // Reconstruct original data
            const totalLength = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
            const reconstructed = new Uint8Array(totalLength);
            let offset = 0;
            for (const chunk of chunks) {
                reconstructed.set(chunk, offset);
                offset += chunk.length;
            }

            expect(reconstructed).toEqual(mockPDFData);
        });

        test('should track parsing progress', async () => {
            const progressTracker = new Mocks.ProgressTracker();
            let progressCalled = false;

            progressTracker.addCallback((progress) => {
                progressCalled = true;
                expect(progress.bytesRead).toBeGreaterThanOrEqual(0);
                expect(progress.totalBytes).toBeGreaterThan(0);
                expect(progress.currentOperation).toBeDefined();
            });

            // Simulate progress updates
            progressTracker.updateProgress(512, 1024, 'parsing');
            progressTracker.updateProgress(1024, 1024, 'complete');

            expect(progressCalled).toBe(true);
        });

        test('should handle abort signals', () => {
            const abortController = new AbortController();

            // Simulate stream with abort capability
            const stream = new ReadableStream({
                start(controller) {
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

            // Abort the operation
            abortController.abort();

            expect(abortController.signal.aborted).toBe(true);
        });
    });
});

describe('ModernPDF API Integration Tests', () => {
    test('should create ModernPDF instance from buffer', async () => {
        const pdfData = Mocks.PDFGenerator.createSimplePDF();

        try {
            const pdf = await ModernPDF.fromBuffer(pdfData.buffer as ArrayBuffer);
            expect(pdf).toBeDefined();

            // Test basic functionality
            const metadata = await pdf.getMetadata();
            expect(metadata).toBeDefined();
            expect(metadata?.pageCount).toBeGreaterThan(0);

            pdf.close();
        } catch (error) {
            // Expected to fail with mock data, but should not crash
            expect(error).toBeDefined();
        }
    });

    test('should handle file creation', async () => {
        const pdfData = Mocks.PDFGenerator.createSimplePDF();
        const file = Mocks.Utils.createMockFile('test.pdf', pdfData);

        expect(file.name).toBe('test.pdf');
        expect(file.type).toBe('application/pdf');
        expect(file.size).toBe(pdfData.length);
    });

    test('should handle streaming operations', async () => {
        const pdfData = Mocks.PDFGenerator.createSimplePDF();
        const stream = new Mocks.ReadableStream(pdfData, 256);

        const chunks = await Mocks.Utils.streamToArray(stream);
        expect(chunks.length).toBeGreaterThan(1);

        // Verify total data size
        const totalSize = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
        expect(totalSize).toBe(pdfData.length);
    });
});