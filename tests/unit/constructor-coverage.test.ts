/**
 * Constructor and Direct Instantiation Coverage Tests
 * Tests direct constructor usage and private method paths not covered by factory methods
 */

import { AgenticPDF, PDFOptions } from '../../agenticpdf';
import { MockPDFGenerator, MockReadableStream } from '../mocks';

describe('Constructor and Direct Instantiation Coverage', () => {
    describe('Direct Constructor Usage', () => {
        it('should create instance with no options', () => {
            const pdf = new (AgenticPDF as any)(); // Access constructor directly
            expect(pdf).toBeInstanceOf(AgenticPDF);
            expect((pdf as any).options).toBeDefined();
            expect((pdf as any).options.renderOptions).toBeDefined();
        });

        it('should create instance with empty options object', () => {
            const options: PDFOptions = {};
            const pdf = new (AgenticPDF as any)(options);
            expect(pdf).toBeInstanceOf(AgenticPDF);
            expect((pdf as any).options).toBe(options);
            expect((pdf as any).options.renderOptions).toBeDefined();
        });

        it('should create instance with render options specified', () => {
            const options: PDFOptions = {
                renderOptions: {
                    scale: 2.0,
                    imageQuality: 0.95
                }
            };
            const pdf = new (AgenticPDF as any)(options);
            expect(pdf).toBeInstanceOf(AgenticPDF);
            expect((pdf as any).options.renderOptions.scale).toBe(2.0);
        });

        it('should create instance with all options specified', () => {
            const options: PDFOptions = {
                lazyLoad: true,
                streamOptions: {
                    chunkSize: 4096,
                    backpressureThreshold: 8192
                },
                renderOptions: {
                    scale: 1.5,
                    imageQuality: 0.85
                },
                cachePages: false,
                maxMemoryUsage: 50 * 1024 * 1024,
                useWebWorkers: true,
                workerUrl: '/custom-worker.js'
            };
            const pdf = new (AgenticPDF as any)(options);
            expect(pdf).toBeInstanceOf(AgenticPDF);
            expect((pdf as any).options).toEqual(options);
            expect((pdf as any).options.renderOptions).toBeDefined();
        });
    });

    describe('Private Method Coverage', () => {
        let pdf: any;

        beforeEach(() => {
            pdf = new (AgenticPDF as any)();
        });

        it('should call loadFromStream method directly', () => {
            const stream = new MockReadableStream(MockPDFGenerator.createSimplePDF());
            pdf.loadFromStream(stream);
            expect(pdf.stream).toBe(stream);
        });

        it('should handle loadFromFile with valid file', async () => {
            const pdfData = MockPDFGenerator.createSimplePDF();
            const mockFile = {
                arrayBuffer: jest.fn().mockResolvedValue(pdfData.buffer)
            } as any;

            const parseSpy = jest.spyOn(pdf, 'parse').mockResolvedValue(undefined);

            await pdf.loadFromFile(mockFile);

            expect(mockFile.arrayBuffer).toHaveBeenCalled();
            expect(pdf.buffer).toBeDefined();
            expect(parseSpy).toHaveBeenCalled();
        });

        it('should handle loadFromBuffer with valid buffer', async () => {
            const pdfData = MockPDFGenerator.createSimplePDF();
            const buffer = pdfData.buffer;
            const parseSpy = jest.spyOn(pdf, 'parse').mockResolvedValue(undefined);

            await pdf.loadFromBuffer(buffer);

            expect(pdf.buffer).toBe(buffer);
            expect(parseSpy).toHaveBeenCalled();
        });

        it('should handle loadFromUrl with successful response (buffer mode)', async () => {
            const pdfData = MockPDFGenerator.createSimplePDF();
            const mockResponse = {
                ok: true,
                statusText: 'OK',
                body: null,
                arrayBuffer: jest.fn().mockResolvedValue(pdfData.buffer)
            };

            global.fetch = jest.fn().mockResolvedValue(mockResponse);
            const parseSpy = jest.spyOn(pdf, 'parse').mockResolvedValue(undefined);

            await pdf.loadFromUrl('https://example.com/test.pdf');

            expect(fetch).toHaveBeenCalledWith('https://example.com/test.pdf');
            expect(mockResponse.arrayBuffer).toHaveBeenCalled();
            expect(pdf.buffer).toBeDefined();
            expect(parseSpy).toHaveBeenCalled();
        });

        it('should handle loadFromUrl with successful response (stream mode)', async () => {
            const pdfData = MockPDFGenerator.createSimplePDF();
            const stream = new MockReadableStream(pdfData);
            const mockResponse = {
                ok: true,
                statusText: 'OK',
                body: stream,
                arrayBuffer: jest.fn().mockResolvedValue(pdfData.buffer)
            };

            pdf.options.streamOptions = { chunkSize: 1024, backpressureThreshold: 2048 };
            global.fetch = jest.fn().mockResolvedValue(mockResponse);
            const parseStreamSpy = jest.spyOn(pdf, 'parseStream').mockResolvedValue(undefined);

            await pdf.loadFromUrl('https://example.com/test.pdf');

            expect(fetch).toHaveBeenCalledWith('https://example.com/test.pdf');
            expect(pdf.stream).toBe(stream);
            expect(parseStreamSpy).toHaveBeenCalled();
        });

        it('should handle loadFromUrl with failed response', async () => {
            const mockResponse = {
                ok: false,
                statusText: 'Not Found'
            };

            global.fetch = jest.fn().mockResolvedValue(mockResponse);

            await expect(pdf.loadFromUrl('https://example.com/missing.pdf'))
                .rejects.toThrow('Failed to fetch PDF: Not Found');
        });
    });

    describe('Internal State Management', () => {
        it('should initialize all private properties correctly', () => {
            const pdf = new (AgenticPDF as any)();

            expect(pdf.buffer).toBeUndefined();
            expect(pdf.stream).toBeUndefined();
            expect(pdf.metadata).toBeUndefined();
            expect(pdf.pages).toBeInstanceOf(Map);
            expect(pdf.pages.size).toBe(0);
            expect(pdf.aiFeatures).toBeUndefined();
            expect(pdf.xrefTable).toBeUndefined();
            expect(pdf.catalog).toBeUndefined();
            expect(pdf.pageTree).toBeUndefined();
            expect(pdf.objects).toBeInstanceOf(Map);
            expect(pdf.objects.size).toBe(0);
        });
    });

    describe('Options Validation and Edge Cases', () => {
        it('should handle null options with error', () => {
            expect(() => new (AgenticPDF as any)(null)).toThrow();
        });

        it('should handle undefined options gracefully', () => {
            const pdf = new (AgenticPDF as any)(undefined);
            expect(pdf).toBeInstanceOf(AgenticPDF);
            expect((pdf as any).options).toBeDefined();
            expect((pdf as any).options.renderOptions).toBeDefined();
        });

        it('should preserve existing renderOptions when provided', () => {
            const customRenderOptions = {
                scale: 3.0,
                imageQuality: 1.0,
                maintainAspectRatio: false
            };

            const options: PDFOptions = {
                renderOptions: customRenderOptions
            };

            const pdf = new (AgenticPDF as any)(options);
            expect((pdf as any).options.renderOptions).toBe(customRenderOptions);
        });
    });
});
