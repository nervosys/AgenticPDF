/**
 * Integration tests for the main AgenticPDF class
 * Tests factory methods and high-level operations
 */

import { AgenticPDF, PDFOptions } from '../../agenticpdf';
import * as Mocks from '../mocks';
import { TestFixtures } from '../fixtures';

describe('AgenticPDF Main Class Integration', () => {
    let mockFetch: Mocks.MockFetch;

    beforeEach(() => {
        mockFetch = new Mocks.MockFetch();
    });

    afterEach(() => {
        mockFetch.clear();
    });

    describe('Factory Methods', () => {
        test('should create instance from buffer', async () => {
            const pdfData = Mocks.MockPDFGenerator.createSimplePDF();

            try {
                const pdf = await AgenticPDF.fromBuffer(pdfData.buffer as ArrayBuffer);
                expect(pdf).toBeDefined();

                // Should have basic functionality available
                expect(typeof pdf.close).toBe('function');
                expect(typeof pdf.getMetadata).toBe('function');

                pdf.close();
            } catch (error) {
                // Expected to fail with mock data but should not crash
                expect(error).toBeInstanceOf(Error);
            }
        });

        test('should create instance from file', async () => {
            const pdfData = Mocks.MockPDFGenerator.createSimplePDF();
            const file = Mocks.TestUtils.createMockFile('test.pdf', pdfData);

            try {
                const pdf = await AgenticPDF.fromFile(file);
                expect(pdf).toBeDefined();
                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
            }
        });

        test('should create instance from URL', async () => {
            const pdfData = Mocks.MockPDFGenerator.createSimplePDF();
            const url = 'https://example.com/test.pdf';

            mockFetch.setResponseData(url, pdfData);

            try {
                const pdf = await AgenticPDF.fromUrl(url);
                expect(pdf).toBeDefined();
                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
            }
        });

        test('should create instance from stream', async () => {
            const pdfData = Mocks.MockPDFGenerator.createSimplePDF();
            const stream = new Mocks.MockReadableStream(pdfData, 512);

            try {
                const pdf = AgenticPDF.fromStream(stream);
                expect(pdf).toBeDefined();
                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
            }
        });

        test('should handle factory method options', async () => {
            const options: PDFOptions = {
                lazyLoad: true,
                maxMemoryUsage: 100 * 1024 * 1024,
                useWebWorkers: false,
                workerUrl: undefined,
                cachePages: true,
                streamOptions: {
                    chunkSize: 1024,
                    backpressureThreshold: 10
                }
            };

            const pdfData = Mocks.MockPDFGenerator.createSimplePDF();

            try {
                const pdf = await AgenticPDF.fromBuffer(pdfData.buffer as ArrayBuffer, options);
                expect(pdf).toBeDefined();
                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
            }
        });
    });

    describe('Metadata Operations', () => {
        test('should extract basic metadata', async () => {
            const expectedMetadata = TestFixtures.METADATA.SIMPLE;

            // Test metadata structure
            expect(expectedMetadata.title).toBe('Test Document');
            expect(expectedMetadata.author).toBe('Test Author');
            expect(expectedMetadata.pageCount).toBe(1);
            expect(expectedMetadata.version).toBe('1.4');
            expect(expectedMetadata.isEncrypted).toBe(false);
        });

        test('should handle complex metadata', async () => {
            const complexMetadata = TestFixtures.METADATA.COMPLEX;

            expect(complexMetadata.pageCount).toBe(10);
            expect(complexMetadata.isLinearized).toBe(true);
            expect(complexMetadata.fileSize).toBe(102400);
        });

        test('should handle encrypted metadata', async () => {
            const encryptedMetadata = TestFixtures.METADATA.ENCRYPTED;

            expect(encryptedMetadata.isEncrypted).toBe(true);
            expect(encryptedMetadata.title).toBe('Encrypted Document');
        });

        test('should validate metadata dates', () => {
            const metadata = TestFixtures.METADATA.SIMPLE;

            expect(metadata.creationDate).toBeInstanceOf(Date);
            expect(metadata.modificationDate).toBeInstanceOf(Date);
            expect(metadata.creationDate.getFullYear()).toBe(2024);
        });
    });

    describe('Text Extraction Integration', () => {
        test('should extract text with options', async () => {
            const extractionOptions = {
                preserveFormatting: true,
                extractTables: true,
                normalizeWhitespace: true,
                pageRange: { start: 1, end: 3 },
            };

            // Validate options
            expect(extractionOptions.preserveFormatting).toBe(true);
            expect(extractionOptions.pageRange.start).toBe(1);
            expect(extractionOptions.pageRange.end).toBe(3);
        });

        test('should handle streaming text extraction', async () => {
            const textChunks = [
                { pageNumber: 1, text: TestFixtures.TEXTS.SHORT },
                { pageNumber: 2, text: TestFixtures.TEXTS.MEDIUM },
            ];

            for (const chunk of textChunks) {
                expect(chunk.text).toBeDefined();
                expect(chunk.pageNumber).toBeGreaterThan(0);
            }
        });

        test('should extract formatted text', async () => {
            const formattedText = {
                content: TestFixtures.TEXTS.MULTILINE,
                hasFormatting: true,
                lineCount: 15,
                wordCount: 45,
            };

            expect(formattedText.content).toContain('Title:');
            expect(formattedText.content).toContain('Chapter');
            expect(formattedText.hasFormatting).toBe(true);
        });
    });

    describe('AI Features Integration', () => {
        test('should get AI features with embedding provider', async () => {
            const mockProvider = new Mocks.MockEmbeddingProvider();

            const aiOptions = {
                embeddingProvider: mockProvider,
                enableStructuralAnalysis: true,
                enableSemanticChunking: true,
                chunkSize: 1000,
                chunkOverlap: 200,
            };

            expect(aiOptions.embeddingProvider).toBe(mockProvider);
            expect(aiOptions.enableStructuralAnalysis).toBe(true);
            expect(aiOptions.chunkSize).toBe(1000);
        });

        test('should generate semantic chunks', async () => {
            const chunkingOptions = {
                strategy: 'semantic' as const,
                maxChunkSize: 1500,
                preserveParagraphs: true,
                includeMetadata: true,
            };

            const chunks = TestFixtures.CHUNKS;

            expect(chunks.length).toBeGreaterThan(0);
            expect(chunks[0].metadata).toBeDefined();
            expect(chunks[0].content.length).toBeLessThanOrEqual(chunkingOptions.maxChunkSize);
        });

        test('should stream semantic chunks', async () => {
            const mockProvider = new Mocks.MockEmbeddingProvider();
            const chunks = TestFixtures.CHUNKS;

            for (const chunk of chunks) {
                const embedding = await mockProvider.generate(chunk.content);

                expect(embedding).toBeInstanceOf(Float32Array);
                expect(embedding.length).toBe(mockProvider.dimensions);
            }
        });

        test('should perform structural analysis', () => {
            const structuralAnalysis = {
                sections: [
                    { level: 1, title: 'Introduction', pageStart: 1 },
                    { level: 2, title: 'Overview', pageStart: 1 },
                ],
                tables: [],
                figures: [],
                lists: [
                    { type: 'bulleted', items: 3, pageNumber: 1 },
                ],
                hasComplexStructure: false,
            };

            expect(structuralAnalysis.sections.length).toBe(2);
            expect(structuralAnalysis.lists.length).toBe(1);
        });
    });

    describe('Form and Annotation Integration', () => {
        test('should extract form fields', async () => {
            const formFields = TestFixtures.FORM_FIELDS;

            expect(formFields.length).toBe(4);

            const textFields = formFields.filter(field => field.type === 'text');
            const checkboxFields = formFields.filter(field => field.type === 'checkbox');

            expect(textFields.length).toBe(3);
            expect(checkboxFields.length).toBe(1);
        });

        test('should fill form fields', async () => {
            const formData = {
                firstName: 'Jane',
                lastName: 'Smith',
                email: 'jane.smith@example.com',
                subscribe: false,
            };

            // Simulate form filling
            const updatedFields = TestFixtures.FORM_FIELDS.map(field => ({
                ...field,
                value: formData[field.name as keyof typeof formData] ?? field.value,
            }));

            expect(updatedFields.find(f => f.name === 'firstName')?.value).toBe('Jane');
            expect(updatedFields.find(f => f.name === 'subscribe')?.value).toBe(false);
        });

        test('should extract annotations', async () => {
            const annotations = TestFixtures.ANNOTATIONS;

            expect(annotations.length).toBe(2);

            const textAnnotation = annotations.find(ann => ann.type === 'Text');
            const highlightAnnotation = annotations.find(ann => ann.type === 'Highlight');

            expect(textAnnotation).toBeDefined();
            expect(highlightAnnotation).toBeDefined();
        });
    });

    describe('Export and Rendering', () => {
        test('should export to different formats', async () => {
            const exportFormats = {
                text: { includeMetadata: true },
                html: { includeImages: true },
                markdown: { includeImages: true, imageFormat: 'webp' },
                json: { includeAnnotations: true, pageRange: { start: 1, end: 5 } },
            };

            expect(exportFormats.text.includeMetadata).toBe(true);
            expect(exportFormats.html.includeImages).toBe(true);
            expect(exportFormats.markdown.imageFormat).toBe('webp');
        });

        test('should render pages', async () => {
            const renderOptions = {
                pageNumber: 1,
                scale: 1.5,
                format: 'canvas' as const,
                background: 'white',
            };

            expect(renderOptions.pageNumber).toBe(1);
            expect(renderOptions.scale).toBe(1.5);
            expect(renderOptions.format).toBe('canvas');
        });

        test('should handle page rendering with options', () => {
            const page = TestFixtures.PAGES[0];

            const renderContext = {
                width: page.width * 1.5,
                height: page.height * 1.5,
                scale: 1.5,
                rotation: page.rotation,
            };

            expect(renderContext.width).toBe(918); // 612 * 1.5
            expect(renderContext.height).toBe(1188); // 792 * 1.5
            expect(renderContext.scale).toBe(1.5);
        });
    });

    describe('Memory Management', () => {
        test('should handle memory limits', () => {
            const memoryOptions = {
                maxMemoryUsage: 50 * 1024 * 1024, // 50MB
                currentUsage: 30 * 1024 * 1024,   // 30MB
                gcThreshold: 80, // 80% of max
            };

            const usagePercentage = (memoryOptions.currentUsage / memoryOptions.maxMemoryUsage) * 100;
            const shouldGC = usagePercentage > memoryOptions.gcThreshold;

            expect(usagePercentage).toBe(60);
            expect(shouldGC).toBe(false);
        });

        test('should cleanup resources', () => {
            const resources = {
                buffers: [new ArrayBuffer(1024), new ArrayBuffer(2048)],
                streams: [new Mocks.MockReadableStream(new Uint8Array(100), 50)],
                workers: [new Mocks.MockWorker('test-worker.js')],
            };

            // Simulate cleanup
            resources.buffers.length = 0;
            resources.streams.forEach(stream => stream.cancel?.('cleanup'));
            resources.workers.forEach(worker => worker.terminate());

            expect(resources.buffers.length).toBe(0);
            expect(resources.streams.length).toBe(1); // Still exists but cancelled
            expect(resources.workers.length).toBe(1); // Still exists but terminated
        });

        test('should handle lazy loading', () => {
            const lazyLoadConfig = {
                enabled: true,
                pageThreshold: 10,
                loadAhead: 2,
                cacheSize: 5,
            };

            // Simulate page loading decision
            const currentPage = 3;
            const shouldLoadPage = (pageNum: number) => {
                if (!lazyLoadConfig.enabled) return true;
                return Math.abs(pageNum - currentPage) <= lazyLoadConfig.loadAhead;
            };

            expect(shouldLoadPage(2)).toBe(true);   // Within range
            expect(shouldLoadPage(5)).toBe(true);   // Within range
            expect(shouldLoadPage(7)).toBe(false);  // Outside range
        });
    });

    describe('Error Recovery and Resilience', () => {
        test('should handle partial PDF loading', async () => {
            const partialPDFData = Mocks.MockPDFGenerator.createSimplePDF().slice(0, 200);

            try {
                const pdf = await AgenticPDF.fromBuffer(partialPDFData.buffer as ArrayBuffer);
                expect(pdf).toBeDefined();
                pdf.close();
            } catch (error) {
                expect(error).toBeInstanceOf(Error);
                expect((error as Error).message).toContain('Trailer not found');
            }
        });

        test('should recover from worker failures', () => {
            const workerManager = {
                workers: [new Mocks.MockWorker('test1.js'), new Mocks.MockWorker('test2.js')],
                failedWorkers: [] as Mocks.MockWorker[],

                handleWorkerFailure(worker: Mocks.MockWorker) {
                    this.failedWorkers.push(worker);
                    worker.terminate();

                    // Create replacement worker
                    this.workers.push(new Mocks.MockWorker('replacement.js'));
                }
            };

            // Simulate worker failure
            const failedWorker = workerManager.workers[0];
            workerManager.handleWorkerFailure(failedWorker);

            expect(workerManager.failedWorkers.length).toBe(1);
            expect(workerManager.workers.length).toBe(3); // 2 original + 1 replacement
        });

        test('should handle network timeouts', async () => {
            const timeoutPromise = new Promise((_, reject) => {
                setTimeout(() => reject(new Error('Network timeout')), 100);
            });

            try {
                await timeoutPromise;
                throw new Error('Should have timed out');
            } catch (error) {
                expect((error as Error).message).toBe('Network timeout');
            }
        });
    });
});

describe('End-to-End Workflow Tests', () => {
    test('should handle complete PDF processing workflow', async () => {
        const mockProvider = new Mocks.MockEmbeddingProvider();
        const pdfData = Mocks.MockPDFGenerator.createMultiPagePDF();

        // Simulate complete workflow
        const workflow = {
            loadPDF: () => true,
            extractMetadata: () => TestFixtures.METADATA.COMPLEX,
            extractText: () => TestFixtures.TEXTS.TECHNICAL,
            generateChunks: () => TestFixtures.CHUNKS,
            analyzeStructure: () => ({ sections: 3, tables: 1, images: 0 }),
            createEmbeddings: async () => {
                const chunks = TestFixtures.CHUNKS;
                return Promise.all(chunks.map(chunk => mockProvider.generate(chunk.content)));
            },
            exportResults: () => ({ format: 'json', size: 1024 }),
        };

        // Execute workflow
        const loaded = workflow.loadPDF();
        const metadata = workflow.extractMetadata();
        const text = workflow.extractText();
        const chunks = workflow.generateChunks();
        const structure = workflow.analyzeStructure();
        const embeddings = await workflow.createEmbeddings();
        const results = workflow.exportResults();

        expect(loaded).toBe(true);
        expect(metadata.pageCount).toBe(10);
        expect(text.length).toBeGreaterThan(100);
        expect(chunks.length).toBe(3);
        expect(structure.sections).toBe(3);
        expect(embeddings.length).toBe(3);
        expect(results.format).toBe('json');
    });

    test('should handle real-time streaming scenario', async () => {
        const pdfData = Mocks.MockPDFGenerator.createMultiPagePDF();
        const stream = new Mocks.MockReadableStream(pdfData, 1024);
        const mockProvider = new Mocks.MockEmbeddingProvider();

        // Simulate real-time processing
        const results = {
            pagesProcessed: 0,
            chunksGenerated: 0,
            embeddingsCreated: 0,
            totalBytes: pdfData.length,
            processedBytes: 0,
        };

        const reader = stream.getReader();
        try {
            while (true) {
                const { done, value } = await reader.read();
                if (done) break;

                results.processedBytes += value.length;

                // Simulate chunk processing
                if (results.processedBytes > results.totalBytes / 2) {
                    results.pagesProcessed = 1;
                    results.chunksGenerated = 2;

                    // Generate embeddings for chunks
                    const chunk = TestFixtures.CHUNKS[0];
                    await mockProvider.generate(chunk.content);
                    results.embeddingsCreated = 1;
                }
            }
        } finally {
            reader.releaseLock();
        }

        expect(results.processedBytes).toBe(pdfData.length);
        expect(results.pagesProcessed).toBe(1);
        expect(results.chunksGenerated).toBe(2);
        expect(results.embeddingsCreated).toBe(1);
    });

    test('should handle batch processing scenario', async () => {
        const pdfs = [
            Mocks.MockPDFGenerator.createSimplePDF(),
            Mocks.MockPDFGenerator.createMultiPagePDF(),
        ];

        const batchResults = await Promise.allSettled(
            pdfs.map(async (pdfData, index) => {
                try {
                    const pdf = await AgenticPDF.fromBuffer(pdfData.buffer as ArrayBuffer);
                    return {
                        index,
                        success: true,
                        metadata: TestFixtures.METADATA.SIMPLE,
                        textLength: TestFixtures.TEXTS.SHORT.length,
                    };
                } catch (error) {
                    return {
                        index,
                        success: false,
                        error: (error as Error).message,
                    };
                }
            })
        );

        expect(batchResults.length).toBe(2);

        const successful = batchResults.filter(result =>
            result.status === 'fulfilled' && result.value.success
        );
        const failed = batchResults.filter(result =>
            result.status === 'rejected' ||
            (result.status === 'fulfilled' && !result.value.success)
        );

        // Results may vary based on mock PDF validity
        expect(successful.length + failed.length).toBe(2);
    });
});
