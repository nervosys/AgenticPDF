/**
 * Utility Classes and Methods Coverage Tests  
 * Tests for internal classes, enums, and utility methods not covered by integration tests
 */

import { ModernPDF, AnnotationType, FormFieldType, DocumentType, ChunkType } from '../../modernpdf';
import { MockEmbeddingProvider } from '../mocks';

describe('Utility Classes and Methods Coverage', () => {
    describe('Enums Coverage', () => {
        it('should have all AnnotationType values accessible', () => {
            const annotationTypes = Object.values(AnnotationType);
            expect(annotationTypes).toContain('Text');
            expect(annotationTypes).toContain('Link');
            expect(annotationTypes).toContain('FreeText');
            expect(annotationTypes).toContain('Line');
            expect(annotationTypes).toContain('Square');
            expect(annotationTypes).toContain('Circle');
            expect(annotationTypes).toContain('Highlight');
            expect(annotationTypes).toContain('Underline');
            expect(annotationTypes).toContain('Widget');
            expect(annotationTypes).toContain('FileAttachment');
            expect(annotationTypes.length).toBeGreaterThan(15);
        });

        it('should have all FormFieldType values accessible', () => {
            const formFieldTypes = Object.values(FormFieldType);
            expect(formFieldTypes).toContain('Text');
            expect(formFieldTypes).toContain('Button');
            expect(formFieldTypes).toContain('Choice');
            expect(formFieldTypes).toContain('Signature');
            expect(formFieldTypes.length).toBe(4);
        });

        it('should have all DocumentType values accessible', () => {
            const documentTypes = Object.values(DocumentType);
            expect(documentTypes).toContain('Article');
            expect(documentTypes).toContain('Book');
            expect(documentTypes).toContain('Report');
            expect(documentTypes).toContain('Form');
            expect(documentTypes).toContain('Invoice');
            expect(documentTypes).toContain('Manual');
            expect(documentTypes).toContain('Resume');
            expect(documentTypes).toContain('Presentation');
            expect(documentTypes).toContain('Other');
            expect(documentTypes.length).toBe(9);
        });

        it('should have all ChunkType values accessible', () => {
            const chunkTypes = Object.values(ChunkType);
            expect(chunkTypes).toContain('Title');
            expect(chunkTypes).toContain('Header');
            expect(chunkTypes).toContain('Paragraph');
            expect(chunkTypes).toContain('List');
            expect(chunkTypes).toContain('Table');
            expect(chunkTypes).toContain('Figure');
            expect(chunkTypes).toContain('Code');
            expect(chunkTypes).toContain('Quote');
            expect(chunkTypes).toContain('Footnote');
            expect(chunkTypes.length).toBe(9);
        });
    });

    describe('Static Methods Coverage', () => {
        it('should handle theme manager access', () => {
            const themeManager = ModernPDF.getThemeManager();
            expect(themeManager).toBeDefined();
            expect(typeof themeManager.toggleTheme).toBe('function');
            expect(typeof themeManager.getCurrentTheme).toBe('function');
        });
    });

    describe('Error Path Coverage', () => {
        it('should handle corrupted PDF data gracefully', async () => {
            const corruptedData = new Uint8Array([1, 2, 3, 4, 5]); // Invalid PDF

            await expect(ModernPDF.fromBuffer(corruptedData.buffer as ArrayBuffer))
                .rejects.toThrow();
        });

        it('should handle empty buffer', async () => {
            const emptyData = new Uint8Array(0);

            await expect(ModernPDF.fromBuffer(emptyData.buffer as ArrayBuffer))
                .rejects.toThrow();
        });

        it('should handle invalid PDF header', async () => {
            const invalidHeader = new TextEncoder().encode('Not a PDF file');

            await expect(ModernPDF.fromBuffer(invalidHeader.buffer as ArrayBuffer))
                .rejects.toThrow();
        });

        it('should handle network errors in URL loading', async () => {
            global.fetch = jest.fn().mockRejectedValue(new Error('Network error'));

            await expect(ModernPDF.fromUrl('https://invalid.url/test.pdf'))
                .rejects.toThrow('Network error');
        });
    });

    describe('Embedding Provider Coverage', () => {
        it('should test embedding provider interface methods', async () => {
            const provider = new MockEmbeddingProvider();

            // Test single embedding generation
            const embedding = await provider.generate('test text');
            expect(embedding).toBeInstanceOf(Float32Array);
            expect(embedding.length).toBe(provider.dimensions);

            // Test batch embedding generation
            const embeddings = await provider.generateBatch(['text1', 'text2', 'text3']);
            expect(embeddings).toHaveLength(3);
            expect(embeddings[0]).toBeInstanceOf(Float32Array);

            // Test error handling
            provider.setGenerateError(new Error('Mock error'));
            await expect(provider.generate('test')).rejects.toThrow('Mock error');

            // Test delay functionality
            provider.setGenerateError(null);
            provider.setGenerateDelay(10);
            const startTime = Date.now();
            await provider.generate('test');
            const endTime = Date.now();
            expect(endTime - startTime).toBeGreaterThanOrEqual(9); // Allow for some timing variance
        });

        it('should handle invalid dimensions', async () => {
            const provider = new MockEmbeddingProvider();
            provider.dimensions = 0;

            await expect(provider.generate('test')).rejects.toThrow('Invalid embedding dimensions');
        });

        it('should generate consistent embeddings for same text', async () => {
            const provider = new MockEmbeddingProvider();

            const embedding1 = await provider.generate('identical text');
            const embedding2 = await provider.generate('identical text');

            // Should be identical for same input
            expect(embedding1).toEqual(embedding2);
        });

        it('should generate different embeddings for different text', async () => {
            const provider = new MockEmbeddingProvider();

            const embedding1 = await provider.generate('short');
            const embedding2 = await provider.generate('this is a much longer text with different content');

            // Should be different for different inputs (different lengths should produce different embeddings)
            let areEqual = true;
            for (let i = 0; i < embedding1.length; i++) {
                if (Math.abs(embedding1[i] - embedding2[i]) > 0.001) {
                    areEqual = false;
                    break;
                }
            }
            expect(areEqual).toBe(false);
        });
    });
});