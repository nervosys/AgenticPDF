/**
 * Test fixtures for AgenticPDF testing
 * Sample data and PDFs for consistent testing
 */

export const SAMPLE_TEXTS = {
    SHORT: 'Hello World',
    MEDIUM: 'This is a medium length text sample that contains multiple sentences. It is useful for testing text extraction and semantic chunking. The text includes various punctuation marks and structures.',
    LONG: `Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab illo inventore veritatis et quasi architecto beatae vitae dicta sunt explicabo. Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut odit aut fugit, sed quia consequuntur magni dolores eos qui ratione voluptatem sequi nesciunt.

Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam eius modi tempora incidunt ut labore et dolore magnam aliquam quaerat voluptatem. Ut enim ad minima veniam, quis nostrum exercitationem ullam corporis suscipit laboriosam, nisi ut aliquid ex ea commodi consequatur?`,

    MULTILINE: `Title: Document Processing
Author: Test User
Date: 2024

Chapter 1: Introduction
This document demonstrates various text structures that might appear in a PDF.

Chapter 2: Lists and Formatting
• Bullet point one
• Bullet point two
• Bullet point three

1. Numbered item one
2. Numbered item two
3. Numbered item three

Chapter 3: Tables and Data
Name        Age    City
John        25     New York
Jane        30     San Francisco
Bob         35     Chicago

Chapter 4: Conclusion
This concludes our sample document with various formatting elements.`,

    TECHNICAL: `Technical Specification Document

Abstract:
This document describes the implementation of a PDF processing library with support for modern web standards and AI integration.

1. Architecture Overview
The AgenticPDF library follows a streaming-first architecture that prioritizes memory efficiency and real-time processing capabilities.

1.1 Core Components
- PDFParser: Handles low-level PDF structure parsing
- TextExtractor: Extracts textual content with formatting preservation
- AIAnalyzer: Provides semantic analysis and chunking capabilities

1.2 Streaming Architecture
The library implements a pull-based streaming model that allows for processing of large PDF files without loading them entirely into memory.

2. API Reference
The main entry points for the library are the factory methods:
- AgenticPDF.fromFile(file)
- AgenticPDF.fromUrl(url)
- AgenticPDF.fromStream(stream)

3. Performance Considerations
Memory usage scales with document complexity rather than size when using streaming operations.`,

    WITH_UNICODE: `Unicode Text Sample 🌍

English: Hello World
Spanish: Hola Mundo
French: Bonjour le Monde
German: Hallo Welt
Chinese: 你好世界
Japanese: こんにちは世界
Arabic: مرحبا بالعالم
Russian: Привет мир
Greek: Γεια σας κόσμε

Mathematical Symbols: ∑ ∫ ∞ √ π ∆ ∇ ∂

Special Characters: "curly quotes" 'apostrophe' — em dash – en dash … ellipsis

Emoji: 📄 📝 🔍 ⚡ 🤖 💡 🎯 🚀`,
};

export const SAMPLE_EMBEDDINGS = {
    // Mock embeddings for testing - deterministic values based on content
    SHORT: new Float32Array([0.1, 0.2, 0.3, 0.4, 0.5]),
    MEDIUM: new Float32Array([0.2, 0.4, 0.6, 0.8, 1.0]),
    LONG: new Float32Array([0.3, 0.6, 0.9, 1.2, 1.5]),
    MULTILINE: new Float32Array([0.4, 0.8, 1.2, 1.6, 2.0]),
    TECHNICAL: new Float32Array([0.5, 1.0, 1.5, 2.0, 2.5]),
    WITH_UNICODE: new Float32Array([0.6, 1.2, 1.8, 2.4, 3.0]),
};

export const SAMPLE_METADATA = {
    SIMPLE: {
        title: 'Test Document',
        author: 'Test Author',
        subject: 'Testing',
        keywords: 'test, pdf, processing',
        creator: 'Test Creator',
        producer: 'AgenticPDF Test Suite',
        creationDate: new Date(2024, 0, 1),
        modificationDate: new Date(2024, 0, 2),
        version: '1.4',
        pageCount: 1,
        isEncrypted: false,
        isLinearized: false,
        fileSize: 1024,
    },

    COMPLEX: {
        title: 'Complex Multi-Page Document',
        author: 'Multiple Authors',
        subject: 'Advanced PDF Features',
        keywords: 'complex, multipage, forms, annotations',
        creator: 'Advanced PDF Creator',
        producer: 'AgenticPDF Advanced Test Suite',
        creationDate: new Date('2024-01-01'),
        modificationDate: new Date('2024-01-15'),
        version: '1.7',
        pageCount: 10,
        isEncrypted: false,
        isLinearized: true,
        fileSize: 102400,
    },

    ENCRYPTED: {
        title: 'Encrypted Document',
        author: 'Security Team',
        subject: 'Encrypted Content',
        keywords: 'security, encryption, protected',
        creator: 'Security Creator',
        producer: 'AgenticPDF Security Test Suite',
        creationDate: new Date('2024-01-01'),
        modificationDate: new Date('2024-01-10'),
        version: '1.6',
        pageCount: 5,
        isEncrypted: true,
        isLinearized: false,
        fileSize: 51200,
    },
};

export const SAMPLE_CHUNKS = [
    {
        id: 'chunk-1',
        content: SAMPLE_TEXTS.SHORT,
        pageNumbers: [1],
        startIndex: 0,
        endIndex: SAMPLE_TEXTS.SHORT.length,
        metadata: {
            chunkType: 'paragraph' as const,
            confidence: 0.95,
            language: 'en',
            keywords: ['hello', 'world'],
        },
        embedding: SAMPLE_EMBEDDINGS.SHORT,
    },

    {
        id: 'chunk-2',
        content: SAMPLE_TEXTS.MEDIUM,
        pageNumbers: [1, 2],
        startIndex: 0,
        endIndex: SAMPLE_TEXTS.MEDIUM.length,
        metadata: {
            chunkType: 'paragraph' as const,
            confidence: 0.90,
            language: 'en',
            keywords: ['text', 'sample', 'extraction'],
        },
        embedding: SAMPLE_EMBEDDINGS.MEDIUM,
    },

    {
        id: 'chunk-3',
        content: SAMPLE_TEXTS.TECHNICAL.substring(0, 200),
        pageNumbers: [1],
        startIndex: 0,
        endIndex: 200,
        metadata: {
            chunkType: 'header' as const,
            confidence: 0.98,
            language: 'en',
            keywords: ['technical', 'specification', 'document'],
        },
        embedding: SAMPLE_EMBEDDINGS.TECHNICAL,
    },
];

export const SAMPLE_PROGRESS = {
    START: {
        bytesRead: 0,
        totalBytes: 1024,
        pagesProcessed: 0,
        totalPages: 1,
        currentOperation: 'parsing',
        timeElapsed: 0,
    },

    MIDDLE: {
        bytesRead: 512,
        totalBytes: 1024,
        pagesProcessed: 0,
        totalPages: 1,
        currentOperation: 'extracting',
        timeElapsed: 1000,
        estimatedTimeRemaining: 1000,
    },

    END: {
        bytesRead: 1024,
        totalBytes: 1024,
        pagesProcessed: 1,
        totalPages: 1,
        currentOperation: 'complete',
        timeElapsed: 2000,
        estimatedTimeRemaining: 0,
    },
};

export const SAMPLE_ERRORS = {
    INVALID_PDF: new Error('Invalid PDF structure: Missing PDF header'),
    NETWORK_ERROR: new Error('Network request failed: Unable to fetch PDF'),
    MEMORY_ERROR: new Error('Insufficient memory: Document too large for available memory'),
    PARSING_ERROR: new Error('Parsing error: Corrupted PDF content stream'),
    ENCRYPTION_ERROR: new Error('Encryption error: Document is password protected'),
    ABORT_ERROR: new Error('Operation aborted: User cancelled the operation'),
};

export const SAMPLE_PAGES = [
    {
        pageNumber: 1,
        width: 612,
        height: 792,
        rotation: 0,
        userUnit: 1.0,
        mediaBox: { x: 0, y: 0, width: 612, height: 792 },
        cropBox: { x: 0, y: 0, width: 612, height: 792 },
        contents: new TextEncoder().encode('Basic page content'),
    },

    {
        pageNumber: 2,
        width: 612,
        height: 792,
        rotation: 90,
        userUnit: 1.0,
        mediaBox: { x: 0, y: 0, width: 612, height: 792 },
        cropBox: { x: 36, y: 36, width: 540, height: 720 },
        bleedBox: { x: 18, y: 18, width: 576, height: 756 },
        trimBox: { x: 36, y: 36, width: 540, height: 720 },
        artBox: { x: 54, y: 54, width: 504, height: 684 },
        contents: new TextEncoder().encode('Rotated page with different boxes'),
    },
];

export const SAMPLE_FONTS = [
    {
        name: 'Helvetica',
        type: 'Type1',
        subtype: 'Type1',
        baseFont: 'Helvetica',
        encoding: 'WinAnsiEncoding',
    },

    {
        name: 'Times-Roman',
        type: 'Type1',
        subtype: 'Type1',
        baseFont: 'Times-Roman',
        encoding: 'StandardEncoding',
    },

    {
        name: 'Arial',
        type: 'TrueType',
        subtype: 'TrueType',
        baseFont: 'Arial',
        encoding: 'WinAnsiEncoding',
    },
];

export const SAMPLE_IMAGES = [
    {
        width: 100,
        height: 100,
        bitsPerComponent: 8,
        colorSpace: 'DeviceRGB',
        filter: ['DCTDecode'],
        data: new Uint8Array(1000).fill(128), // Gray image data
    },

    {
        width: 200,
        height: 150,
        bitsPerComponent: 8,
        colorSpace: 'DeviceGray',
        filter: ['FlateDecode'],
        data: new Uint8Array(30000).fill(64), // Larger grayscale image
    },
];

export const SAMPLE_ANNOTATIONS = [
    {
        type: 'Text',
        rect: { x: 100, y: 700, width: 20, height: 20 },
        contents: 'This is a text annotation',
        author: 'Test User',
        creationDate: new Date(2024, 0, 1), // Year, Month (0-based), Day
        modificationDate: new Date(2024, 0, 1),
    },

    {
        type: 'Highlight',
        rect: { x: 150, y: 650, width: 200, height: 15 },
        contents: 'Highlighted text',
        author: 'Reviewer',
        creationDate: new Date(2024, 0, 2),
        modificationDate: new Date(2024, 0, 2),
    },
];

export const SAMPLE_FORM_FIELDS = [
    {
        name: 'firstName',
        type: 'text',
        value: 'John',
        rect: { x: 100, y: 600, width: 150, height: 20 },
        required: true,
        readonly: false,
    },

    {
        name: 'lastName',
        type: 'text',
        value: 'Doe',
        rect: { x: 300, y: 600, width: 150, height: 20 },
        required: true,
        readonly: false,
    },

    {
        name: 'email',
        type: 'text',
        value: 'john.doe@example.com',
        rect: { x: 100, y: 570, width: 350, height: 20 },
        required: false,
        readonly: false,
    },

    {
        name: 'subscribe',
        type: 'checkbox',
        value: true,
        rect: { x: 100, y: 540, width: 15, height: 15 },
        required: false,
        readonly: false,
    },
];

// Export all fixtures as a single object for easy access
export const TestFixtures = {
    TEXTS: SAMPLE_TEXTS,
    EMBEDDINGS: SAMPLE_EMBEDDINGS,
    METADATA: SAMPLE_METADATA,
    CHUNKS: SAMPLE_CHUNKS,
    PROGRESS: SAMPLE_PROGRESS,
    ERRORS: SAMPLE_ERRORS,
    PAGES: SAMPLE_PAGES,
    FONTS: SAMPLE_FONTS,
    IMAGES: SAMPLE_IMAGES,
    ANNOTATIONS: SAMPLE_ANNOTATIONS,
    FORM_FIELDS: SAMPLE_FORM_FIELDS,
};
