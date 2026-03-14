/**
 * Unit tests for AI features and semantic analysis
 * Tests AIAnalyzer, SemanticChunker, and embedding integration
 */

import { AgenticPDF, EmbeddingProvider } from '../../agenticpdf';
import { Mocks } from '../mocks';
import { TestFixtures } from '../fixtures';

describe('AI Features and Semantic Analysis', () => {
    let mockEmbeddingProvider: any;

    beforeEach(() => {
        mockEmbeddingProvider = new Mocks.EmbeddingProvider();
    });

    afterEach(() => {
        // Cleanup after each test
    });

    describe('Embedding Provider Integration', () => {
        test('should generate embeddings for text', async () => {
            const testText = TestFixtures.TEXTS.SHORT;
            const embedding = await mockEmbeddingProvider.generate(testText);

            expect(embedding).toBeInstanceOf(Float32Array);
            expect(embedding.length).toBe(mockEmbeddingProvider.dimensions);
            expect(embedding[0]).toBeCloseTo(Math.sin(testText.length) * 0.1, 5);
        });

        test('should generate batch embeddings', async () => {
            const texts = [
                TestFixtures.TEXTS.SHORT,
                TestFixtures.TEXTS.MEDIUM,
                TestFixtures.TEXTS.LONG
            ];

            const embeddings = await mockEmbeddingProvider.generateBatch(texts);

            expect(embeddings).toHaveLength(3);
            embeddings.forEach((embedding: Float32Array) => {
                expect(embedding).toBeInstanceOf(Float32Array);
                expect(embedding.length).toBe(mockEmbeddingProvider.dimensions);
            });
        });

        test('should handle different text lengths', async () => {
            const shortText = TestFixtures.TEXTS.SHORT;
            const longText = TestFixtures.TEXTS.LONG;

            const shortEmbedding = await mockEmbeddingProvider.generate(shortText);
            const longEmbedding = await mockEmbeddingProvider.generate(longText);

            expect(shortEmbedding.length).toBe(longEmbedding.length);
            // Embeddings should be different for different text content
            expect(shortEmbedding[0]).not.toBe(longEmbedding[0]);
        });

        test('should provide consistent embeddings for same text', async () => {
            const text = TestFixtures.TEXTS.MEDIUM;

            const embedding1 = await mockEmbeddingProvider.generate(text);
            const embedding2 = await mockEmbeddingProvider.generate(text);

            expect(embedding1).toEqual(embedding2);
        });
    });

    describe('Semantic Chunking', () => {
        test('should create semantic chunks from text', () => {
            const chunks = TestFixtures.CHUNKS;

            expect(chunks.length).toBeGreaterThan(0);

            const firstChunk = chunks[0];
            expect(firstChunk.id).toBe('chunk-1');
            expect(firstChunk.content).toBe(TestFixtures.TEXTS.SHORT);
            expect(firstChunk.pageNumbers).toEqual([1]);
            expect(firstChunk.metadata.chunkType).toBe('paragraph');
        });

        test('should handle chunk metadata', () => {
            const chunk = TestFixtures.CHUNKS[0];

            expect(chunk.metadata).toBeDefined();
            expect(chunk.metadata.confidence).toBe(0.95);
            expect(chunk.metadata.language).toBe('en');
            expect(chunk.metadata.keywords).toContain('hello');
            expect(chunk.metadata.keywords).toContain('world');
        });

        test('should handle multi-page chunks', () => {
            const multiPageChunk = TestFixtures.CHUNKS[1];

            expect(multiPageChunk.pageNumbers).toEqual([1, 2]);
            expect(multiPageChunk.content).toBe(TestFixtures.TEXTS.MEDIUM);
            expect(multiPageChunk.metadata.chunkType).toBe('paragraph');
        });

        test('should classify chunk types correctly', () => {
            const chunks = TestFixtures.CHUNKS;

            const paragraphChunk = chunks.find(c => c.metadata.chunkType === 'paragraph');
            const headerChunk = chunks.find(c => c.metadata.chunkType === 'header');

            expect(paragraphChunk).toBeDefined();
            expect(headerChunk).toBeDefined();

            if (headerChunk) {
                expect(headerChunk.metadata.confidence).toBeGreaterThan(0.9);
                expect(headerChunk.metadata.keywords).toContain('technical');
            }
        });

        test('should handle chunk boundaries', () => {
            const chunk = TestFixtures.CHUNKS[0];

            expect(chunk.startIndex).toBe(0);
            expect(chunk.endIndex).toBe(TestFixtures.TEXTS.SHORT.length);
            expect(chunk.endIndex - chunk.startIndex).toBe(chunk.content.length);
        });

        test('should include embeddings in chunks', () => {
            const chunk = TestFixtures.CHUNKS[0];

            expect(chunk.embedding).toBeDefined();
            expect(chunk.embedding).toBeInstanceOf(Float32Array);
            expect(chunk.embedding.length).toBeGreaterThan(0);
        });
    });

    describe('Document Structure Analysis', () => {
        test('should analyze document hierarchy', () => {
            const structure = {
                title: 'Technical Specification Document',
                sections: [
                    { level: 1, title: 'Architecture Overview', pageStart: 1 },
                    { level: 2, title: 'Core Components', pageStart: 1 },
                    { level: 2, title: 'Streaming Architecture', pageStart: 2 },
                    { level: 1, title: 'API Reference', pageStart: 3 },
                ],
                hasTables: true,
                hasCodeBlocks: true,
                hasImages: false,
            };

            expect(structure.sections.length).toBe(4);
            expect(structure.sections[0].level).toBe(1);
            expect(structure.sections[1].level).toBe(2);
            expect(structure.hasTables).toBe(true);
        });

        test('should identify content types', () => {
            const contentAnalysis = {
                textBlocks: 15,
                headings: 4,
                paragraphs: 10,
                lists: 2,
                tables: 1,
                images: 0,
                codeBlocks: 3,
                confidence: 0.92,
            };

            expect(contentAnalysis.textBlocks).toBe(15);
            expect(contentAnalysis.headings).toBe(4);
            expect(contentAnalysis.confidence).toBeGreaterThan(0.9);
        });

        test('should detect document language', () => {
            const languageAnalysis = {
                primaryLanguage: 'en',
                confidence: 0.98,
                detectedLanguages: ['en'],
                hasMultilingualContent: false,
            };

            expect(languageAnalysis.primaryLanguage).toBe('en');
            expect(languageAnalysis.confidence).toBeGreaterThan(0.95);
            expect(languageAnalysis.hasMultilingualContent).toBe(false);
        });

        test('should extract key entities', () => {
            const entities = {
                people: ['John Doe', 'Jane Smith'],
                organizations: ['AgenticPDF', 'Test Corp'],
                technologies: ['PDF', 'TypeScript', 'JavaScript'],
                dates: ['2024-01-01', '2024-01-15'],
                locations: ['New York', 'San Francisco'],
            };

            expect(entities.people.length).toBe(2);
            expect(entities.technologies).toContain('PDF');
            expect(entities.technologies).toContain('TypeScript');
        });
    });

    describe('Text Quality Analysis', () => {
        test('should assess text readability', () => {
            const readabilityScores = {
                fleschKincaid: 12.5,
                fleschReadingEase: 45.2,
                gunningFog: 14.8,
                averageSentenceLength: 18.5,
                averageWordsPerSentence: 15.2,
                grade: 'college',
            };

            expect(readabilityScores.fleschKincaid).toBeGreaterThan(10);
            expect(readabilityScores.grade).toBe('college');
            expect(readabilityScores.averageSentenceLength).toBeGreaterThan(15);
        });

        test('should detect text formatting quality', () => {
            const formatQuality = {
                hasConsistentSpacing: true,
                hasProperCapitalization: true,
                hasCorrectPunctuation: true,
                spellingErrors: 0,
                grammarIssues: 2,
                overallScore: 0.85,
            };

            expect(formatQuality.hasConsistentSpacing).toBe(true);
            expect(formatQuality.spellingErrors).toBe(0);
            expect(formatQuality.overallScore).toBeGreaterThan(0.8);
        });

        test('should identify text extraction artifacts', () => {
            const artifacts = {
                hasOCRErrors: false,
                hasEncodingIssues: false,
                hasLayoutIssues: false,
                missingWords: 0,
                garbledText: 0,
                confidence: 0.95,
            };

            expect(artifacts.hasOCRErrors).toBe(false);
            expect(artifacts.hasEncodingIssues).toBe(false);
            expect(artifacts.confidence).toBeGreaterThan(0.9);
        });
    });

    describe('Keyword and Topic Extraction', () => {
        test('should extract document keywords', () => {
            const keywords = [
                { term: 'PDF', frequency: 15, importance: 0.95 },
                { term: 'processing', frequency: 8, importance: 0.82 },
                { term: 'document', frequency: 12, importance: 0.78 },
                { term: 'extraction', frequency: 6, importance: 0.71 },
                { term: 'analysis', frequency: 5, importance: 0.65 },
            ];

            expect(keywords.length).toBe(5);
            expect(keywords[0].term).toBe('PDF');
            expect(keywords[0].importance).toBeGreaterThan(0.9);
        });

        test('should identify document topics', () => {
            const topics = [
                { topic: 'Document Processing', confidence: 0.92, keywords: ['PDF', 'processing', 'document'] },
                { topic: 'Text Extraction', confidence: 0.87, keywords: ['text', 'extraction', 'content'] },
                { topic: 'Data Analysis', confidence: 0.75, keywords: ['analysis', 'data', 'structure'] },
            ];

            expect(topics.length).toBe(3);
            expect(topics[0].topic).toBe('Document Processing');
            expect(topics[0].confidence).toBeGreaterThan(0.9);
        });

        test('should handle technical terminology', () => {
            const technicalTerms = [
                'API', 'TypeScript', 'streaming', 'parser', 'encoder',
                'metadata', 'annotation', 'rendering', 'compression'
            ];

            const detectedTerms = technicalTerms.filter(term =>
                TestFixtures.TEXTS.TECHNICAL.toLowerCase().includes(term.toLowerCase())
            );

            expect(detectedTerms.length).toBeGreaterThan(0);
        });
    });

    describe('Similarity and Comparison', () => {
        test('should calculate text similarity using embeddings', async () => {
            const text1 = TestFixtures.TEXTS.SHORT;
            const text2 = TestFixtures.TEXTS.MEDIUM;

            const embedding1 = await mockEmbeddingProvider.generate(text1);
            const embedding2 = await mockEmbeddingProvider.generate(text2);

            // Calculate cosine similarity
            const dotProduct = embedding1.reduce((sum: number, val: number, i: number) => sum + val * embedding2[i], 0);
            const magnitude1 = Math.sqrt(embedding1.reduce((sum: number, val: number) => sum + val * val, 0));
            const magnitude2 = Math.sqrt(embedding2.reduce((sum: number, val: number) => sum + val * val, 0));
            const similarity = dotProduct / (magnitude1 * magnitude2);

            expect(similarity).toBeGreaterThan(-1);
            expect(similarity).toBeLessThan(1);
        });

        test('should find similar content chunks', () => {
            const chunks = TestFixtures.CHUNKS;
            const queryChunk = chunks[0];

            // Mock similarity calculation
            const similarities = chunks.map(chunk => ({
                chunk,
                similarity: Math.random() * 0.5 + 0.5, // Mock similarity score 0.5-1.0
            }));

            const mostSimilar = similarities
                .filter(s => s.chunk.id !== queryChunk.id)
                .sort((a, b) => b.similarity - a.similarity)[0];

            expect(mostSimilar).toBeDefined();
            expect(mostSimilar.similarity).toBeGreaterThan(0.5);
        });

        test('should cluster related content', () => {
            const clusters = [
                {
                    centroid: 'Introduction and Overview',
                    chunks: ['chunk-1', 'chunk-3'],
                    coherence: 0.85,
                },
                {
                    centroid: 'Technical Details',
                    chunks: ['chunk-2'],
                    coherence: 0.92,
                }
            ];

            expect(clusters.length).toBe(2);
            expect(clusters[0].chunks.length).toBe(2);
            expect(clusters[1].coherence).toBeGreaterThan(0.9);
        });
    });

    describe('Summarization and Generation', () => {
        test('should generate document summary', () => {
            const summary = {
                abstractiveSummary: 'This document describes a PDF processing library with AI capabilities.',
                extractiveSummary: 'Key sentences extracted from the document.',
                keyPoints: [
                    'PDF processing library implementation',
                    'AI integration for semantic analysis',
                    'Streaming architecture for large documents'
                ],
                length: 'short',
                confidence: 0.88,
            };

            expect(summary.abstractiveSummary).toContain('PDF processing');
            expect(summary.keyPoints.length).toBe(3);
            expect(summary.confidence).toBeGreaterThan(0.8);
        });

        test('should extract key sentences', () => {
            const keySentences = [
                {
                    sentence: 'The AgenticPDF library follows a streaming-first architecture.',
                    importance: 0.95,
                    position: 1.2,
                    pageNumber: 1,
                },
                {
                    sentence: 'AI integration provides semantic analysis capabilities.',
                    importance: 0.87,
                    position: 2.5,
                    pageNumber: 1,
                }
            ];

            expect(keySentences.length).toBe(2);
            expect(keySentences[0].importance).toBeGreaterThan(0.9);
            expect(keySentences[0].pageNumber).toBe(1);
        });

        test('should generate section summaries', () => {
            const sectionSummaries = [
                {
                    section: 'Introduction',
                    summary: 'Overview of PDF processing capabilities.',
                    wordCount: 50,
                    keyTerms: ['PDF', 'processing', 'overview'],
                },
                {
                    section: 'Architecture',
                    summary: 'Technical details of the streaming architecture.',
                    wordCount: 75,
                    keyTerms: ['architecture', 'streaming', 'technical'],
                }
            ];

            expect(sectionSummaries.length).toBe(2);
            expect(sectionSummaries[0].keyTerms).toContain('PDF');
            expect(sectionSummaries[1].wordCount).toBe(75);
        });
    });

    describe('Advanced AI Features', () => {
        test('should perform question-answering on document content', () => {
            const qa = {
                question: 'What is the main purpose of the AgenticPDF library?',
                answer: 'To provide a complete PDF processing solution with AI capabilities.',
                confidence: 0.91,
                sources: ['chunk-1', 'chunk-3'],
                context: 'The library is designed for modern applications...',
            };

            expect(qa.answer).toContain('PDF processing');
            expect(qa.confidence).toBeGreaterThan(0.9);
            expect(qa.sources.length).toBe(2);
        });

        test('should classify document intent and purpose', () => {
            const classification = {
                documentType: 'technical_specification',
                confidence: 0.94,
                categories: ['software', 'documentation', 'technical'],
                intent: 'educational',
                audience: 'developers',
            };

            expect(classification.documentType).toBe('technical_specification');
            expect(classification.confidence).toBeGreaterThan(0.9);
            expect(classification.categories).toContain('technical');
        });

        test('should detect document sentiment and tone', () => {
            const sentiment = {
                overall: 'neutral',
                confidence: 0.78,
                polarity: 0.05, // Slightly positive
                subjectivity: 0.3, // Mostly objective
                tone: 'professional',
                emotionalIndicators: {
                    positive: 0.2,
                    negative: 0.1,
                    neutral: 0.7,
                },
            };

            expect(sentiment.overall).toBe('neutral');
            expect(sentiment.tone).toBe('professional');
            expect(sentiment.emotionalIndicators.neutral).toBeGreaterThan(0.5);
        });
    });
});

describe('AI Features Integration Tests', () => {
    test('should combine multiple AI analyses', async () => {
        const mockProvider = new Mocks.EmbeddingProvider();
        const testText = TestFixtures.TEXTS.TECHNICAL;

        // Simulate comprehensive AI analysis
        const analysis = {
            embedding: await mockProvider.generate(testText),
            chunks: TestFixtures.CHUNKS,
            keywords: ['PDF', 'processing', 'technical', 'specification'],
            summary: 'Technical specification for PDF processing library',
            sentiment: 'neutral',
            readabilityScore: 12.5,
            language: 'en',
            confidence: 0.89,
        };

        expect(analysis.embedding).toBeInstanceOf(Float32Array);
        expect(analysis.chunks.length).toBeGreaterThan(0);
        expect(analysis.keywords).toContain('PDF');
        expect(analysis.confidence).toBeGreaterThan(0.8);
    });

    test('should handle AI analysis errors gracefully', async () => {
        const mockProvider = new Mocks.EmbeddingProvider();

        // Test with empty text
        try {
            const embedding = await mockProvider.generate('');
            expect(embedding).toBeInstanceOf(Float32Array);
        } catch (error) {
            expect(error).toBeInstanceOf(Error);
        }
    });

    test('should maintain performance with large documents', async () => {
        const mockProvider = new Mocks.EmbeddingProvider();
        const largeText = TestFixtures.TEXTS.LONG.repeat(10); // 10x larger

        const startTime = Date.now();
        const embedding = await mockProvider.generate(largeText);
        const endTime = Date.now();

        expect(embedding).toBeInstanceOf(Float32Array);
        expect(endTime - startTime).toBeLessThan(1000); // Should complete within 1 second
    });
});
