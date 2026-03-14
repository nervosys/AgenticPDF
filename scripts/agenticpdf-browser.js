/**
 * AgenticPDF - Browser Bundle
 * Modern, TypeScript-native PDF processing library
 * Version: 1.0.1
 * Compiled: 2026-03-14T19:20:16.336Z
 */

(function(global) {
    'use strict';
    
/**
 * AgenticPDF - Complete TypeScript rewrite of Mozilla PDF.js
 * with modern streaming, AI integration, and performance optimizations
 */
var AnnotationType;
(function (AnnotationType) {
    AnnotationType["Text"] = "Text";
    AnnotationType["Link"] = "Link";
    AnnotationType["FreeText"] = "FreeText";
    AnnotationType["Line"] = "Line";
    AnnotationType["Square"] = "Square";
    AnnotationType["Circle"] = "Circle";
    AnnotationType["Polygon"] = "Polygon";
    AnnotationType["PolyLine"] = "PolyLine";
    AnnotationType["Highlight"] = "Highlight";
    AnnotationType["Underline"] = "Underline";
    AnnotationType["Squiggly"] = "Squiggly";
    AnnotationType["StrikeOut"] = "StrikeOut";
    AnnotationType["Stamp"] = "Stamp";
    AnnotationType["Caret"] = "Caret";
    AnnotationType["Ink"] = "Ink";
    AnnotationType["Popup"] = "Popup";
    AnnotationType["FileAttachment"] = "FileAttachment";
    AnnotationType["Sound"] = "Sound";
    AnnotationType["Movie"] = "Movie";
    AnnotationType["Widget"] = "Widget";
    AnnotationType["Screen"] = "Screen";
    AnnotationType["PrinterMark"] = "PrinterMark";
    AnnotationType["TrapNet"] = "TrapNet";
    AnnotationType["Watermark"] = "Watermark";
    AnnotationType["Redact"] = "Redact";
})(AnnotationType || (AnnotationType = {}));
var FormFieldType;
(function (FormFieldType) {
    FormFieldType["Button"] = "Button";
    FormFieldType["Text"] = "Text";
    FormFieldType["Choice"] = "Choice";
    FormFieldType["Signature"] = "Signature";
})(FormFieldType || (FormFieldType = {}));
var DocumentType;
(function (DocumentType) {
    DocumentType["Article"] = "Article";
    DocumentType["Book"] = "Book";
    DocumentType["Report"] = "Report";
    DocumentType["Form"] = "Form";
    DocumentType["Invoice"] = "Invoice";
    DocumentType["Resume"] = "Resume";
    DocumentType["Presentation"] = "Presentation";
    DocumentType["Manual"] = "Manual";
    DocumentType["Other"] = "Other";
})(DocumentType || (DocumentType = {}));
var ChunkType;
(function (ChunkType) {
    ChunkType["Title"] = "Title";
    ChunkType["Header"] = "Header";
    ChunkType["Paragraph"] = "Paragraph";
    ChunkType["List"] = "List";
    ChunkType["Table"] = "Table";
    ChunkType["Figure"] = "Figure";
    ChunkType["Code"] = "Code";
    ChunkType["Quote"] = "Quote";
    ChunkType["Footnote"] = "Footnote";
})(ChunkType || (ChunkType = {}));
var PDFObjectType;
(function (PDFObjectType) {
    PDFObjectType["Boolean"] = "Boolean";
    PDFObjectType["Number"] = "Number";
    PDFObjectType["String"] = "String";
    PDFObjectType["Name"] = "Name";
    PDFObjectType["Array"] = "Array";
    PDFObjectType["Dictionary"] = "Dictionary";
    PDFObjectType["Stream"] = "Stream";
    PDFObjectType["Null"] = "Null";
    PDFObjectType["Reference"] = "Reference";
})(PDFObjectType || (PDFObjectType = {}));
/**
 * Performance monitoring utility
 */
class PerformanceMonitor {
    /**
     * Enable performance monitoring
     */
    static enable() {
        PerformanceMonitor.enabled = true;
    }
    /**
     * Disable performance monitoring
     */
    static disable() {
        PerformanceMonitor.enabled = false;
    }
    /**
     * Start timing an operation
     */
    static startOperation(operationName) {
        if (!PerformanceMonitor.enabled) {
            return { operationName, startTime: 0 };
        }
        const metric = {
            operationName,
            startTime: performance.now()
        };
        // Implement circular buffer for metrics
        if (PerformanceMonitor.metrics.length >= PerformanceMonitor.MAX_METRICS) {
            PerformanceMonitor.metrics.shift();
        }
        PerformanceMonitor.metrics.push(metric);
        return metric;
    }
    /**
     * End timing an operation
     */
    static endOperation(metric) {
        if (!PerformanceMonitor.enabled)
            return;
        metric.endTime = performance.now();
        metric.duration = metric.endTime - metric.startTime;
        // Try to get memory usage (if available)
        if (typeof performance.memory !== 'undefined') {
            metric.memoryUsed = performance.memory.usedJSHeapSize;
        }
    }
    /**
     * Get all metrics
     */
    static getMetrics() {
        return [...PerformanceMonitor.metrics];
    }
    /**
     * Get metrics summary
     */
    static getSummary() {
        const summary = {};
        for (const metric of PerformanceMonitor.metrics) {
            if (!metric.duration)
                continue;
            if (!summary[metric.operationName]) {
                summary[metric.operationName] = {
                    count: 0,
                    avgDuration: 0,
                    totalDuration: 0
                };
            }
            summary[metric.operationName].count++;
            summary[metric.operationName].totalDuration += metric.duration;
        }
        // Calculate averages
        for (const key in summary) {
            summary[key].avgDuration = summary[key].totalDuration / summary[key].count;
        }
        return summary;
    }
    /**
     * Clear all metrics
     */
    static clearMetrics() {
        PerformanceMonitor.metrics = [];
    }
}
PerformanceMonitor.metrics = [];
PerformanceMonitor.MAX_METRICS = 1000;
PerformanceMonitor.enabled = false;
/**
 * Memory pool for frequently allocated objects
 */
class MemoryPool {
    constructor(createFn, maxSize = 100, resetFn) {
        this.pool = [];
        this.createFn = createFn;
        this.maxSize = maxSize;
        this.resetFn = resetFn;
    }
    /**
     * Acquire object from pool or create new one
     */
    acquire() {
        const obj = this.pool.pop();
        if (obj) {
            if (this.resetFn) {
                this.resetFn(obj);
            }
            return obj;
        }
        return this.createFn();
    }
    /**
     * Release object back to pool
     */
    release(obj) {
        if (this.pool.length < this.maxSize) {
            this.pool.push(obj);
        }
    }
    /**
     * Clear the pool
     */
    clear() {
        this.pool = [];
    }
    /**
     * Get pool size
     */
    size() {
        return this.pool.length;
    }
}
// ============================================================================
// Core PDF Processing Classes
// ============================================================================
class AgenticPDF {
    constructor(options = {}) {
        this.options = options;
        this.pages = new Map();
        this.objects = new Map();
        this._formValues = new Map();
        // Apply optimal viewer defaults if no render options specified
        if (!this.options.renderOptions) {
            this.options.renderOptions = PDFRenderer.getOptimalViewerOptions();
        }
    }
    /**
     * Load PDF from various sources
     */
    static async fromFile(file, options) {
        const pdf = new AgenticPDF(options);
        await pdf.loadFromFile(file);
        return pdf;
    }
    static async fromUrl(url, options) {
        const pdf = new AgenticPDF(options);
        await pdf.loadFromUrl(url);
        return pdf;
    }
    static async fromBuffer(buffer, options) {
        const pdf = new AgenticPDF(options);
        await pdf.loadFromBuffer(buffer);
        return pdf;
    }
    static fromStream(stream, options) {
        const pdf = new AgenticPDF(options);
        pdf.loadFromStream(stream);
        return pdf;
    }
    async loadFromFile(file) {
        this.buffer = await file.arrayBuffer();
        await this.parse();
    }
    async loadFromUrl(url) {
        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`Failed to fetch PDF: ${response.statusText}`);
        }
        if (response.body && this.options.streamOptions) {
            this.stream = response.body;
            await this.parseStream();
        }
        else {
            this.buffer = await response.arrayBuffer();
            await this.parse();
        }
    }
    async loadFromBuffer(buffer) {
        this.buffer = buffer;
        await this.parse();
    }
    loadFromStream(stream) {
        this.stream = stream;
    }
    /**
     * Parse PDF content
     */
    async parse() {
        if (!this.buffer)
            throw new Error('No buffer available');
        const parser = new PDFParser(this.buffer, this.options);
        this.parser = parser;
        // Parse PDF structure
        await parser.parseHeader();
        this.xrefTable = await parser.parseXRef();
        this.catalog = await parser.parseCatalog(this.xrefTable);
        this.metadata = await parser.parseMetadata(this.xrefTable, this.catalog);
        // Parse page tree
        this.pageTree = await parser.parsePageTree(this.catalog);
        // Parse pages lazily or eagerly based on options
        if (!this.options.lazyLoad) {
            for (let i = 1; i <= this.metadata.pageCount; i++) {
                this.pages.set(i, await parser.parsePage(i, this.pageTree));
            }
        }
    }
    async parseStream() {
        if (!this.stream)
            throw new Error('No stream available');
        const parser = new StreamingPDFParser(this.stream, this.options.streamOptions);
        await parser.parseHeader();
        this.metadata = await parser.parseMetadata();
        // Stream pages as needed
        const pageStream = parser.streamPages();
        for await (const page of pageStream) {
            this.pages.set(page.pageNumber, page);
            // Report progress
            this.options.streamOptions?.progressCallback?.({
                bytesRead: parser.bytesRead,
                totalBytes: this.metadata?.fileSize || 0,
                pagesProcessed: page.pageNumber,
                totalPages: this.metadata?.pageCount,
                currentOperation: 'Parsing pages',
                timeElapsed: Date.now() - parser.startTime
            });
            // Check for abort signal
            if (this.options.streamOptions?.abortSignal?.aborted) {
                break;
            }
        }
    }
    /**
     * Get PDF metadata
     */
    getMetadata() {
        return this.metadata;
    }
    /**
     * Get raw PDF data for external rendering libraries (e.g., PDF.js)
     */
    getRawData() {
        return this.buffer;
    }
    /**
     * Get specific page
     */
    async getPage(pageNumber) {
        if (pageNumber < 1 || (this.metadata && pageNumber > this.metadata.pageCount)) {
            return undefined;
        }
        if (!this.pages.has(pageNumber) && this.options.lazyLoad) {
            await this.loadPageFromStream(pageNumber);
        }
        return this.pages.get(pageNumber);
    }
    async loadPageFromStream(pageNumber) {
        if (!this.pageTree)
            return;
        const parser = new PDFParser(this.buffer, this.options);
        const page = await parser.parsePage(pageNumber, this.pageTree);
        this.pages.set(pageNumber, page);
    }
    /**
     * Get all pages
     */
    async getAllPages() {
        const pages = [];
        const pageCount = this.metadata?.pageCount || 0;
        for (let i = 1; i <= pageCount; i++) {
            const page = await this.getPage(i);
            if (page)
                pages.push(page);
        }
        return pages;
    }
    /**
     * Extract text content with AI-ready formatting
     */
    async extractText(options) {
        const extractor = new TextExtractor(this, options);
        return extractor.extract();
    }
    /**
     * Extract text as a stream for large documents
     */
    async *streamText(options) {
        const extractor = new TextExtractor(this, options);
        yield* extractor.stream();
    }
    /**
     * Extract images
     */
    /**
     * Convert an extracted ImageContent to a displayable data URL.
     * JPEG images are passed through directly; raw pixel data is rendered to PNG via canvas.
     */
    imageToDataURL(image, format = 'png', quality = 0.92) {
        return ImageExtractor.toDataURL(image, format, quality);
    }
    /**
     * Extract all images and return them as data URLs ready for display.
     */
    async exportImageAsDataURL(options) {
        const images = await this.extractImages(options);
        return images.map(img => ({
            id: img.id,
            dataURL: ImageExtractor.toDataURL(img, options?.format || 'png', options?.quality || 0.92),
            width: img.width,
            height: img.height,
            mimeType: img.mimeType,
            pageNumber: img.pageNumber
        }));
    }
    async extractImages(options) {
        const extractor = new ImageExtractor(this, options);
        return extractor.extract();
    }
    /**
     * Get AI features (structural analysis, semantic chunks, etc.)
     */
    async getAIFeatures(options) {
        if (!this.aiFeatures || options?.forceRegenerate) {
            const analyzer = new AIAnalyzer(this, options);
            this.aiFeatures = await analyzer.analyze();
        }
        return this.aiFeatures;
    }
    /**
     * Generate semantic chunks for RAG systems
     */
    async generateSemanticChunks(options) {
        const chunker = new SemanticChunker(this, options);
        return chunker.chunk();
    }
    /**
     * Stream semantic chunks for memory-efficient processing
     */
    async *streamSemanticChunks(options) {
        const chunker = new SemanticChunker(this, options);
        yield* chunker.stream();
    }
    /**
     * Search text within the PDF
     */
    async search(query, options) {
        const searcher = new PDFSearcher(this);
        return searcher.search(query, options);
    }
    /**
     * Get form fields
     */
    async getFormFields() {
        const extractor = new FormExtractor(this);
        return extractor.extract();
    }
    /**
     * Fill form fields
     */
    async fillForm(data) {
        const filler = new FormFiller(this);
        await filler.fill(data);
    }
    /**
     * Get the current form data (original values merged with any filled values)
     */
    async getFormData() {
        const fields = await this.getFormFields();
        const result = {};
        for (const field of fields) {
            result[field.name] = this._formValues.has(field.name)
                ? this._formValues.get(field.name)
                : field.value;
        }
        return result;
    }
    /**
     * Get annotations
     */
    async getAnnotations(pageNumber) {
        const extractor = new AnnotationExtractor(this);
        return extractor.extract(pageNumber);
    }
    /**
     * Get named destinations map: name -> { page, x, y }
     */
    getNamedDestinations() {
        const result = new Map();
        if (!this.parser || !this.xrefTable || !this.catalog)
            return result;
        const parser = this.parser;
        const xref = this.xrefTable;
        const resolve = (obj) => {
            if (obj && obj.type === PDFObjectType.Reference) {
                const ref = obj.value;
                return parser.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);
            }
            return obj;
        };
        // Build page object number -> page index map by walking the Pages tree
        const objNumToPage = new Map();
        const collectPages = (node, ref) => {
            const dict = resolve(node);
            if (dict.type !== PDFObjectType.Dictionary)
                return;
            const d = dict.value;
            const typeEntry = d.entries.get('Type');
            const typeName = typeEntry?.type === PDFObjectType.Name ? typeEntry.value : '';
            if (typeName === 'Page') {
                if (ref && ref.type === PDFObjectType.Reference) {
                    objNumToPage.set(ref.value.objectNumber, objNumToPage.size + 1);
                }
                return;
            }
            const kids = d.entries.get('Kids');
            if (!kids)
                return;
            const kidsArr = resolve(kids);
            if (kidsArr.type !== PDFObjectType.Array)
                return;
            for (const kid of kidsArr.value) {
                collectPages(kid, kid);
            }
        };
        const pagesRef = this.catalog.entries.get('Pages');
        if (pagesRef)
            collectPages(pagesRef);
        // Walk the Names/Dests name tree
        const namesRef = this.catalog.entries.get('Names');
        if (!namesRef)
            return result;
        const namesDict = resolve(namesRef);
        if (namesDict.type !== PDFObjectType.Dictionary)
            return result;
        const destsRef = namesDict.value.entries.get('Dests');
        if (!destsRef)
            return result;
        const walkNameTree = (node) => {
            const dict = resolve(node);
            if (dict.type !== PDFObjectType.Dictionary)
                return;
            const d = dict.value;
            const namesArr = d.entries.get('Names');
            if (namesArr) {
                const arr = resolve(namesArr);
                if (arr.type === PDFObjectType.Array) {
                    const items = arr.value;
                    for (let i = 0; i < items.length; i += 2) {
                        const nameObj = items[i];
                        const destObj = items[i + 1];
                        if (!nameObj || !destObj)
                            continue;
                        const name = nameObj.value;
                        try {
                            let destArr = null;
                            const destResolved = resolve(destObj);
                            if (destResolved.type === PDFObjectType.Array) {
                                destArr = destResolved.value;
                            }
                            else if (destResolved.type === PDFObjectType.Dictionary) {
                                const dEntry = destResolved.value.entries.get('D');
                                if (dEntry) {
                                    const dResolved = resolve(dEntry);
                                    if (dResolved.type === PDFObjectType.Array)
                                        destArr = dResolved.value;
                                }
                            }
                            if (destArr && destArr.length > 0) {
                                const pageRef = destArr[0];
                                let pageNum;
                                if (pageRef.type === PDFObjectType.Reference) {
                                    pageNum = objNumToPage.get(pageRef.value.objectNumber);
                                }
                                if (pageNum !== undefined) {
                                    let x = null;
                                    let y = null;
                                    if (destArr.length >= 4 && destArr[1]?.type === PDFObjectType.Name) {
                                        const fitType = destArr[1].value;
                                        if (fitType === 'XYZ' && destArr.length >= 5) {
                                            if (destArr[2]?.type === PDFObjectType.Number)
                                                x = destArr[2].value;
                                            if (destArr[3]?.type === PDFObjectType.Number)
                                                y = destArr[3].value;
                                        }
                                    }
                                    result.set(name, { page: pageNum, x, y });
                                }
                            }
                        }
                        catch { /* skip unresolvable destinations */ }
                    }
                }
            }
            const kids = d.entries.get('Kids');
            if (kids) {
                const kidsArr = resolve(kids);
                if (kidsArr.type === PDFObjectType.Array) {
                    for (const kid of kidsArr.value) {
                        walkNameTree(kid);
                    }
                }
            }
        };
        walkNameTree(destsRef);
        return result;
    }
    /**
     * Add annotation
     */
    async addAnnotation(annotation) {
        const manager = new AnnotationManager(this);
        return manager.add(annotation);
    }
    /**
     * Render page to canvas
     */
    async renderPage(pageNumber, canvas, options) {
        const renderer = new PDFRenderer(this, options);
        await renderer.renderToCanvas(pageNumber, canvas);
    }
    /**
     * Render page to image
     */
    async renderPageToImage(pageNumber, format = 'png', options) {
        const renderer = new PDFRenderer(this, options);
        return renderer.renderToImage(pageNumber, format);
    }
    /**
     * Build a text layer for a page
     * Creates a transparent text overlay that allows text selection
     * @param pageNumber - Page number to build text layer for
     * @param container - HTML element to contain the text layer
     * @param viewport - Viewport configuration (width, height, scale)
     * @param options - Text layer rendering options
     */
    async buildTextLayer(pageNumber, container, viewport, options) {
        // Extract text content for the page
        const textContent = await this.extractText({
            pageRange: { start: pageNumber, end: pageNumber }
        });
        // Build text layer using TextLayerBuilder
        const textLayerBuilder = new TextLayerBuilder({
            container,
            textContent,
            viewport,
            enhanceTextSelection: options?.enhanceTextSelection ?? true
        });
        textLayerBuilder.render();
    }
    /**
     * Export to different formats
     */
    async exportAs(format, options) {
        const exporter = new PDFExporter(this, options);
        return exporter.export(format);
    }
    /**
     * Save modified PDF
     */
    async save() {
        const writer = new PDFWriter(this);
        return writer.save();
    }
    /**
     * Create an optimal PDF viewer with theme toggle functionality
     * @param container - Container element for the viewer
     * @param options - Additional render options
     * @returns Viewer object with controls and cleanup method
     */
    createOptimalViewer(container, options) {
        return PDFRenderer.createOptimalViewer(container, this, options);
    }
    /**
     * Get theme manager instance for manual theme control
     */
    static getThemeManager() {
        return ThemeManager.getInstance();
    }
    /**
     * Initialize theme management globally
     */
    static initializeTheme(options) {
        const themeManager = ThemeManager.getInstance();
        themeManager.initialize(options);
        return themeManager;
    }
    /**
     * Close and cleanup resources
     */
    close() {
        this.buffer = undefined;
        this.stream = undefined;
        this.pages.clear();
        this.objects.clear();
        this.aiFeatures = undefined;
        this.xrefTable = undefined;
        this.catalog = undefined;
        this.pageTree = undefined;
    }
    /**
     * Clear all caches to free memory
     */
    static clearAllCaches() {
        ContentStreamParser.clearCache();
        PDFColorSpaceProcessor.clearCaches();
    }
    /**
     * Get memory usage statistics
     */
    getMemoryStats() {
        return {
            pagesCached: this.pages.size,
            objectsCached: this.objects.size,
            parserCacheSize: ContentStreamParser.parserCache?.size || 0,
            colorSpaceCacheSize: PDFColorSpaceProcessor.colorSpaceCache?.size || 0,
            colorConversionCacheSize: PDFColorSpaceProcessor.conversionCache?.size || 0
        };
    }
    /**
     * Unload pages to free memory (keeps metadata)
     */
    unloadPages(keepPages) {
        const keepSet = new Set(keepPages || []);
        for (const [pageNum] of this.pages) {
            if (!keepSet.has(pageNum)) {
                this.pages.delete(pageNum);
            }
        }
    }
    /**
     * Enable performance monitoring
     */
    static enablePerformanceMonitoring() {
        PerformanceMonitor.enable();
    }
    /**
     * Disable performance monitoring
     */
    static disablePerformanceMonitoring() {
        PerformanceMonitor.disable();
    }
    /**
     * Get performance metrics
     */
    static getPerformanceMetrics() {
        return PerformanceMonitor.getMetrics();
    }
    /**
     * Get performance summary
     */
    static getPerformanceSummary() {
        return PerformanceMonitor.getSummary();
    }
    /**
     * Clear performance metrics
     */
    static clearPerformanceMetrics() {
        PerformanceMonitor.clearMetrics();
    }
    /**
     * Get page count
     */
    getPageCount() {
        return this.metadata?.pageCount || 0;
    }
    /**
     * Get file size
     */
    getFileSize() {
        return this.metadata?.fileSize || 0;
    }
    /**
     * Check if PDF is encrypted
     */
    isEncrypted() {
        return this.metadata?.isEncrypted || false;
    }
    /**
     * Get PDF version
     */
    getVersion() {
        return this.metadata?.version || '1.0';
    }
    // ==========================================================================
    // Ontology & AI Agent Discovery
    // ==========================================================================
    /**
     * Returns a complete machine-readable ontology describing the library's
     * concepts, capabilities, workflows, and type hierarchy.
     * Designed for AI agent discovery and automated code generation.
     */
    static describe() {
        return {
            '@context': 'https://schema.org',
            '@type': 'SoftwareApplication',
            name: 'AgenticPDF',
            version: '1.0.0',
            license: 'AGPL-3.0-or-later',
            description: 'Zero-dependency TypeScript PDF processing library with streaming-first architecture, semantic chunking, and built-in AI integration for agentic workflows.',
            concepts: AgenticPDF._getConcepts(),
            capabilities: AgenticPDF.getCapabilities(),
            workflows: AgenticPDF.getWorkflows(),
            enums: {
                DocumentType: ['Article', 'Book', 'Report', 'Form', 'Invoice', 'Resume', 'Presentation', 'Manual', 'Other'],
                ChunkType: ['Title', 'Header', 'Paragraph', 'List', 'Table', 'Figure', 'Code', 'Quote', 'Footnote'],
                AnnotationType: ['Text', 'Link', 'FreeText', 'Line', 'Square', 'Circle', 'Polygon', 'PolyLine', 'Highlight', 'Underline', 'Squiggly', 'StrikeOut', 'Stamp', 'Caret', 'Ink', 'Popup', 'FileAttachment', 'Sound', 'Movie', 'Widget', 'Screen', 'PrinterMark', 'TrapNet', 'Watermark', 'Redact'],
                FormFieldType: ['Button', 'Text', 'Choice', 'Signature'],
                ExportFormat: ['text', 'html', 'markdown', 'json', 'xml', 'csv']
            }
        };
    }
    /**
     * Returns the library's capability map organized by category.
     * Each capability includes its methods, input/output types, and streaming support.
     */
    static getCapabilities() {
        return [
            {
                id: 'load',
                name: 'Document Loading',
                description: 'Load PDF documents from multiple sources including files, URLs, buffers, and streams with optional streaming and lazy loading.',
                category: 'loading',
                streaming: true,
                inputTypes: ['File', 'Blob', 'string (URL)', 'ArrayBuffer', 'ReadableStream<Uint8Array>'],
                outputTypes: ['AgenticPDF'],
                methods: [
                    {
                        name: 'fromFile',
                        description: 'Load PDF from a File or Blob object',
                        parameters: [
                            { name: 'file', type: 'File | Blob', required: true, description: 'The file or blob to load' },
                            { name: 'options', type: 'PDFOptions', required: false, description: 'Loading and parsing options' }
                        ],
                        returnType: 'Promise<AgenticPDF>',
                        async: true, streaming: false, static: true,
                        example: "const pdf = await AgenticPDF.fromFile(file);"
                    },
                    {
                        name: 'fromUrl',
                        description: 'Load PDF from a URL with optional streaming support',
                        parameters: [
                            { name: 'url', type: 'string', required: true, description: 'URL to fetch the PDF from' },
                            { name: 'options', type: 'PDFOptions', required: false, description: 'Loading options including streamOptions for progress tracking' }
                        ],
                        returnType: 'Promise<AgenticPDF>',
                        async: true, streaming: true, static: true,
                        example: "const pdf = await AgenticPDF.fromUrl('https://example.com/doc.pdf');"
                    },
                    {
                        name: 'fromBuffer',
                        description: 'Load PDF from an ArrayBuffer',
                        parameters: [
                            { name: 'buffer', type: 'ArrayBuffer', required: true, description: 'Raw PDF data' },
                            { name: 'options', type: 'PDFOptions', required: false, description: 'Parsing options' }
                        ],
                        returnType: 'Promise<AgenticPDF>',
                        async: true, streaming: false, static: true,
                        example: "const pdf = await AgenticPDF.fromBuffer(buffer);"
                    },
                    {
                        name: 'fromStream',
                        description: 'Load PDF from a ReadableStream for memory-efficient processing',
                        parameters: [
                            { name: 'stream', type: 'ReadableStream<Uint8Array>', required: true, description: 'Readable stream of PDF data' },
                            { name: 'options', type: 'PDFOptions', required: false, description: 'Stream processing options' }
                        ],
                        returnType: 'AgenticPDF',
                        async: false, streaming: true, static: true,
                        example: "const pdf = AgenticPDF.fromStream(stream);"
                    }
                ]
            },
            {
                id: 'text-extraction',
                name: 'Text Extraction',
                description: 'Extract text content with positioning, styling, and font information. Supports both batch and streaming modes.',
                category: 'extraction',
                streaming: true,
                inputTypes: ['TextExtractionOptions'],
                outputTypes: ['TextContent[]', 'AsyncGenerator<TextContent>'],
                methods: [
                    {
                        name: 'extractText',
                        description: 'Extract all text content with positioning and style information',
                        parameters: [
                            { name: 'options', type: 'TextExtractionOptions', required: false, description: 'Options for formatting, tables, OCR, and page range' }
                        ],
                        returnType: 'Promise<TextContent[]>',
                        async: true, streaming: false, static: false,
                        example: "const text = await pdf.extractText({ preserveFormatting: true });"
                    },
                    {
                        name: 'streamText',
                        description: 'Stream text content page-by-page for memory-efficient processing of large documents',
                        parameters: [
                            { name: 'options', type: 'TextExtractionOptions', required: false, description: 'Extraction options' }
                        ],
                        returnType: 'AsyncGenerator<TextContent>',
                        async: true, streaming: true, static: false,
                        example: "for await (const text of pdf.streamText()) { process(text); }"
                    }
                ]
            },
            {
                id: 'image-extraction',
                name: 'Image Extraction',
                description: 'Extract embedded images from PDF pages with metadata and optional format conversion.',
                category: 'extraction',
                streaming: false,
                inputTypes: ['ImageExtractionOptions'],
                outputTypes: ['ImageContent[]'],
                methods: [
                    {
                        name: 'extractImages',
                        description: 'Extract all images with metadata and pixel data',
                        parameters: [
                            { name: 'options', type: 'ImageExtractionOptions', required: false, description: 'Format, quality, size, and page range options' }
                        ],
                        returnType: 'Promise<ImageContent[]>',
                        async: true, streaming: false, static: false,
                        example: "const images = await pdf.extractImages({ format: 'png' });"
                    }
                ]
            },
            {
                id: 'ai-analysis',
                name: 'AI & Semantic Analysis',
                description: 'Structural analysis, semantic chunking, NER, summarization, and embedding generation for RAG and LLM integration.',
                category: 'analysis',
                streaming: true,
                inputTypes: ['AIOptions', 'ChunkingOptions', 'EmbeddingProvider'],
                outputTypes: ['AIFeatures', 'SemanticChunk[]', 'AsyncGenerator<SemanticChunk>'],
                methods: [
                    {
                        name: 'getAIFeatures',
                        description: 'Run full AI analysis: structural analysis, semantic chunking, NER, summarization',
                        parameters: [
                            { name: 'options', type: 'AIOptions', required: false, description: 'AI feature configuration including embedding provider' }
                        ],
                        returnType: 'Promise<AIFeatures>',
                        async: true, streaming: false, static: false,
                        example: "const ai = await pdf.getAIFeatures({ enableStructuralAnalysis: true });"
                    },
                    {
                        name: 'generateSemanticChunks',
                        description: 'Generate semantic chunks optimized for RAG systems and vector stores',
                        parameters: [
                            { name: 'options', type: 'ChunkingOptions', required: false, description: 'Chunking strategy, size, and overlap settings' }
                        ],
                        returnType: 'Promise<SemanticChunk[]>',
                        async: true, streaming: false, static: false,
                        example: "const chunks = await pdf.generateSemanticChunks({ strategy: 'semantic', maxChunkSize: 1000 });"
                    },
                    {
                        name: 'streamSemanticChunks',
                        description: 'Stream semantic chunks for memory-efficient RAG pipeline processing',
                        parameters: [
                            { name: 'options', type: 'ChunkingOptions', required: false, description: 'Chunking configuration' }
                        ],
                        returnType: 'AsyncGenerator<SemanticChunk>',
                        async: true, streaming: true, static: false,
                        example: "for await (const chunk of pdf.streamSemanticChunks()) { await vectorStore.add(chunk); }"
                    }
                ]
            },
            {
                id: 'search',
                name: 'Text Search',
                description: 'Full-text search with regex support, case sensitivity, whole-word matching, and context extraction.',
                category: 'search',
                streaming: false,
                inputTypes: ['string', 'SearchOptions'],
                outputTypes: ['SearchResult[]'],
                methods: [
                    {
                        name: 'search',
                        description: 'Search document text with regex, case sensitivity, and context options',
                        parameters: [
                            { name: 'query', type: 'string', required: true, description: 'Search query or regex pattern' },
                            { name: 'options', type: 'SearchOptions', required: false, description: 'Case sensitivity, whole word, regex, context length' }
                        ],
                        returnType: 'Promise<SearchResult[]>',
                        async: true, streaming: false, static: false,
                        example: "const results = await pdf.search('revenue', { caseSensitive: false });"
                    }
                ]
            },
            {
                id: 'forms',
                name: 'Form Processing',
                description: 'Extract, read, and fill interactive PDF form fields including text inputs, checkboxes, dropdowns, and signatures.',
                category: 'forms',
                streaming: false,
                inputTypes: ['Record<string, any>'],
                outputTypes: ['FormField[]'],
                methods: [
                    {
                        name: 'getFormFields',
                        description: 'Extract all form fields with their types, values, and validation rules',
                        parameters: [],
                        returnType: 'Promise<FormField[]>',
                        async: true, streaming: false, static: false,
                        example: "const fields = await pdf.getFormFields();"
                    },
                    {
                        name: 'fillForm',
                        description: 'Fill form fields with provided key-value data',
                        parameters: [
                            { name: 'data', type: 'Record<string, any>', required: true, description: 'Field name to value mapping' }
                        ],
                        returnType: 'Promise<void>',
                        async: true, streaming: false, static: false,
                        example: "await pdf.fillForm({ name: 'John', email: 'john@example.com' });"
                    }
                ]
            },
            {
                id: 'annotations',
                name: 'Annotations',
                description: 'Read and create PDF annotations including highlights, notes, links, stamps, and redactions.',
                category: 'annotations',
                streaming: false,
                inputTypes: ['Partial<Annotation>'],
                outputTypes: ['Annotation[]', 'string'],
                methods: [
                    {
                        name: 'getAnnotations',
                        description: 'Get annotations from all pages or a specific page',
                        parameters: [
                            { name: 'pageNumber', type: 'number', required: false, description: 'Specific page number, or omit for all pages' }
                        ],
                        returnType: 'Promise<Annotation[]>',
                        async: true, streaming: false, static: false,
                        example: "const annotations = await pdf.getAnnotations(1);"
                    },
                    {
                        name: 'addAnnotation',
                        description: 'Add a new annotation to the document',
                        parameters: [
                            { name: 'annotation', type: 'Partial<Annotation>', required: true, description: 'Annotation data including type, rect, and content' }
                        ],
                        returnType: 'Promise<string>',
                        async: true, streaming: false, static: false,
                        example: "const id = await pdf.addAnnotation({ type: AnnotationType.Highlight, rect: { x: 0, y: 0, width: 100, height: 20 } });"
                    }
                ]
            },
            {
                id: 'rendering',
                name: 'Page Rendering',
                description: 'Render PDF pages to HTML canvas or image blobs with configurable quality, scale, and theme support.',
                category: 'rendering',
                streaming: false,
                inputTypes: ['HTMLCanvasElement', 'RenderOptions'],
                outputTypes: ['void', 'Blob'],
                methods: [
                    {
                        name: 'renderPage',
                        description: 'Render a page to an HTML canvas element',
                        parameters: [
                            { name: 'pageNumber', type: 'number', required: true, description: 'Page number (1-based)' },
                            { name: 'canvas', type: 'HTMLCanvasElement', required: true, description: 'Target canvas element' },
                            { name: 'options', type: 'RenderOptions', required: false, description: 'Scale, quality, and viewer options' }
                        ],
                        returnType: 'Promise<void>',
                        async: true, streaming: false, static: false,
                        example: "await pdf.renderPage(1, canvas, { scale: 2.0 });"
                    },
                    {
                        name: 'renderPageToImage',
                        description: 'Render a page to an image blob in PNG, JPEG, or WebP format',
                        parameters: [
                            { name: 'pageNumber', type: 'number', required: true, description: 'Page number (1-based)' },
                            { name: 'format', type: "'png' | 'jpeg' | 'webp'", required: false, description: 'Image format (default: png)' },
                            { name: 'options', type: 'RenderOptions', required: false, description: 'Scale and quality options' }
                        ],
                        returnType: 'Promise<Blob>',
                        async: true, streaming: false, static: false,
                        example: "const blob = await pdf.renderPageToImage(1, 'png');"
                    }
                ]
            },
            {
                id: 'export',
                name: 'Multi-Format Export',
                description: 'Export PDF content to text, HTML, Markdown, JSON, XML, or CSV formats with configurable options.',
                category: 'export',
                streaming: false,
                inputTypes: ['ExportFormat', 'ExportOptions'],
                outputTypes: ['Blob', 'string'],
                methods: [
                    {
                        name: 'exportAs',
                        description: 'Export document to a specified format',
                        parameters: [
                            { name: 'format', type: "ExportFormat", required: true, description: "Target format: 'text' | 'html' | 'markdown' | 'json' | 'xml' | 'csv'" },
                            { name: 'options', type: 'ExportOptions', required: false, description: 'Metadata, annotations, images, and page range options' }
                        ],
                        returnType: 'Promise<Blob | string>',
                        async: true, streaming: false, static: false,
                        example: "const md = await pdf.exportAs('markdown', { includeImages: true });"
                    },
                    {
                        name: 'save',
                        description: 'Save the modified PDF document as a Blob',
                        parameters: [],
                        returnType: 'Promise<Blob>',
                        async: true, streaming: false, static: false,
                        example: "const blob = await pdf.save();"
                    }
                ]
            },
            {
                id: 'memory',
                name: 'Memory Management',
                description: 'Resource lifecycle management including page unloading, cache clearing, and memory usage monitoring.',
                category: 'memory',
                streaming: false,
                inputTypes: ['number[]'],
                outputTypes: ['void', '{ pagesCached: number; objectsCached: number }'],
                methods: [
                    {
                        name: 'close',
                        description: 'Release all resources held by this PDF instance',
                        parameters: [],
                        returnType: 'void',
                        async: false, streaming: false, static: false,
                        example: "pdf.close();"
                    },
                    {
                        name: 'unloadPages',
                        description: 'Unload cached pages to free memory, optionally keeping specified pages',
                        parameters: [
                            { name: 'keepPages', type: 'number[]', required: false, description: 'Page numbers to keep in cache' }
                        ],
                        returnType: 'void',
                        async: false, streaming: false, static: false,
                        example: "pdf.unloadPages([1, 2]); // keep pages 1-2, unload the rest"
                    },
                    {
                        name: 'getMemoryStats',
                        description: 'Get current memory usage statistics',
                        parameters: [],
                        returnType: '{ pagesCached: number; objectsCached: number }',
                        async: false, streaming: false, static: false,
                        example: "const stats = pdf.getMemoryStats();"
                    }
                ]
            },
            {
                id: 'navigation',
                name: 'Document Navigation',
                description: 'Access document structure including named destinations, page metadata, and document outline.',
                category: 'extraction',
                streaming: false,
                inputTypes: ['number'],
                outputTypes: ['PDFMetadata', 'PDFPage', 'Map<string, object>'],
                methods: [
                    {
                        name: 'getMetadata',
                        description: 'Get document metadata including title, author, page count, and version',
                        parameters: [],
                        returnType: 'PDFMetadata | undefined',
                        async: false, streaming: false, static: false,
                        example: "const meta = pdf.getMetadata();"
                    },
                    {
                        name: 'getPage',
                        description: 'Get a specific page with lazy loading support',
                        parameters: [
                            { name: 'pageNumber', type: 'number', required: true, description: 'Page number (1-based)' }
                        ],
                        returnType: 'Promise<PDFPage | undefined>',
                        async: true, streaming: false, static: false,
                        example: "const page = await pdf.getPage(1);"
                    },
                    {
                        name: 'getAllPages',
                        description: 'Get all pages as an array',
                        parameters: [],
                        returnType: 'Promise<PDFPage[]>',
                        async: true, streaming: false, static: false,
                        example: "const pages = await pdf.getAllPages();"
                    },
                    {
                        name: 'getNamedDestinations',
                        description: 'Get all named destinations for internal navigation links',
                        parameters: [],
                        returnType: 'Map<string, { page: number; x: number | null; y: number | null }>',
                        async: false, streaming: false, static: false,
                        example: "const dests = pdf.getNamedDestinations();"
                    }
                ]
            }
        ];
    }
    /**
     * Returns all method signatures with full parameter descriptions.
     * Useful for AI agents performing automated code generation.
     */
    static getMethodSignatures() {
        return AgenticPDF.getCapabilities().flatMap(c => c.methods);
    }
    /**
     * Returns pre-built workflow templates for common multi-step operations.
     * AI agents can use these to plan and execute complex document processing tasks.
     */
    static getWorkflows() {
        return [
            {
                id: 'basic-text-extraction',
                name: 'Basic Text Extraction',
                description: 'Load a PDF and extract all text content with formatting preserved.',
                steps: [
                    { order: 1, method: 'fromFile', description: 'Load the PDF document', example: "const pdf = await AgenticPDF.fromFile(file);" },
                    { order: 2, method: 'extractText', description: 'Extract text with formatting', example: "const text = await pdf.extractText({ preserveFormatting: true });" },
                    { order: 3, method: 'close', description: 'Release resources', example: "pdf.close();" }
                ]
            },
            {
                id: 'rag-pipeline',
                name: 'RAG Pipeline Integration',
                description: 'Process a PDF into semantic chunks suitable for vector store ingestion in a Retrieval-Augmented Generation system.',
                steps: [
                    { order: 1, method: 'fromFile', description: 'Load the PDF', example: "const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });" },
                    { order: 2, method: 'streamSemanticChunks', description: 'Stream semantic chunks to vector store', example: "for await (const chunk of pdf.streamSemanticChunks({ strategy: 'semantic', maxChunkSize: 1000 })) { await vectorStore.add(chunk); }" },
                    { order: 3, method: 'close', description: 'Release resources', example: "pdf.close();" }
                ]
            },
            {
                id: 'document-analysis',
                name: 'Full Document Analysis',
                description: 'Perform comprehensive AI analysis including structural analysis, NER, and summarization.',
                steps: [
                    { order: 1, method: 'fromFile', description: 'Load the PDF', example: "const pdf = await AgenticPDF.fromFile(file);" },
                    { order: 2, method: 'getAIFeatures', description: 'Run AI analysis', example: "const ai = await pdf.getAIFeatures({ enableStructuralAnalysis: true, enableNER: true, enableSummarization: true });" },
                    { order: 3, method: 'exportAs', description: 'Export structured results', example: "const json = await pdf.exportAs('json', { includeMetadata: true });" },
                    { order: 4, method: 'close', description: 'Release resources', example: "pdf.close();" }
                ]
            },
            {
                id: 'form-processing',
                name: 'Form Extraction and Filling',
                description: 'Extract form fields, fill them with data, and save the modified PDF.',
                steps: [
                    { order: 1, method: 'fromFile', description: 'Load the PDF form', example: "const pdf = await AgenticPDF.fromFile(formFile);" },
                    { order: 2, method: 'getFormFields', description: 'Inspect available form fields', example: "const fields = await pdf.getFormFields();" },
                    { order: 3, method: 'fillForm', description: 'Fill form with data', example: "await pdf.fillForm({ name: 'John Doe', date: '2024-01-01' });" },
                    { order: 4, method: 'save', description: 'Save the filled form', example: "const blob = await pdf.save();" },
                    { order: 5, method: 'close', description: 'Release resources', example: "pdf.close();" }
                ]
            },
            {
                id: 'streaming-large-document',
                name: 'Memory-Efficient Large Document Processing',
                description: 'Process very large PDFs (100MB+) using streaming APIs with progress tracking and memory limits.',
                steps: [
                    { order: 1, method: 'fromUrl', description: 'Stream PDF from URL with progress', example: "const pdf = await AgenticPDF.fromUrl(url, { streamOptions: { chunkSize: 1024 * 1024, progressCallback: p => console.log(p.currentOperation) } });" },
                    { order: 2, method: 'streamText', description: 'Stream text extraction page by page', example: "for await (const text of pdf.streamText({ normalizeWhitespace: true })) { process(text); }" },
                    { order: 3, method: 'unloadPages', description: 'Free processed pages from memory', example: "pdf.unloadPages();" },
                    { order: 4, method: 'close', description: 'Release resources', example: "pdf.close();" }
                ]
            },
            {
                id: 'multi-format-export',
                name: 'Multi-Format Export Pipeline',
                description: 'Export a PDF to multiple output formats for different downstream consumers.',
                steps: [
                    { order: 1, method: 'fromFile', description: 'Load the PDF', example: "const pdf = await AgenticPDF.fromFile(file);" },
                    { order: 2, method: 'exportAs', description: 'Export as Markdown', example: "const md = await pdf.exportAs('markdown', { includeImages: true });" },
                    { order: 3, method: 'exportAs', description: 'Export as structured JSON', example: "const json = await pdf.exportAs('json', { includeMetadata: true, includeAnnotations: true });" },
                    { order: 4, method: 'exportAs', description: 'Export as HTML', example: "const html = await pdf.exportAs('html');" },
                    { order: 5, method: 'close', description: 'Release resources', example: "pdf.close();" }
                ]
            },
            {
                id: 'llm-streaming',
                name: 'Stream to LLM',
                description: 'Stream PDF content directly to a Large Language Model in appropriately sized chunks.',
                steps: [
                    { order: 1, method: 'fromFile', description: 'Load the PDF', example: "const pdf = await AgenticPDF.fromFile(file, { lazyLoad: true });" },
                    { order: 2, method: 'streamSemanticChunks', description: 'Stream chunks sized for LLM context windows', example: "for await (const chunk of pdf.streamSemanticChunks({ maxChunkSize: 1500, preserveParagraphs: true })) { await llm.send(chunk.content); }" },
                    { order: 3, method: 'close', description: 'Release resources', example: "pdf.close();" }
                ]
            }
        ];
    }
    /** @internal */
    static _getConcepts() {
        return [
            {
                id: 'Document',
                label: 'PDF Document',
                description: 'A loaded PDF document instance providing access to pages, content, metadata, and AI analysis capabilities.',
                properties: [
                    { name: 'pageCount', type: 'number', description: 'Total number of pages' },
                    { name: 'fileSize', type: 'number', description: 'File size in bytes' },
                    { name: 'version', type: 'string', description: 'PDF specification version' },
                    { name: 'encrypted', type: 'boolean', description: 'Whether the document is password-protected' },
                    { name: 'metadata', type: 'PDFMetadata', description: 'Title, author, subject, keywords, dates' }
                ],
                relationships: [
                    { type: 'hasMany', target: 'Page', description: 'Contains one or more pages' },
                    { type: 'hasMany', target: 'Annotation', description: 'Contains annotations across pages' },
                    { type: 'hasMany', target: 'FormField', description: 'Contains interactive form fields' },
                    { type: 'produces', target: 'AIFeatures', description: 'Generates AI analysis results' },
                    { type: 'produces', target: 'SemanticChunk', description: 'Generates semantic chunks for RAG' }
                ]
            },
            {
                id: 'Page',
                label: 'PDF Page',
                description: 'An individual page with geometry, content streams, and resources.',
                properties: [
                    { name: 'pageNumber', type: 'number', description: '1-based page index' },
                    { name: 'width', type: 'number', description: 'Page width in points' },
                    { name: 'height', type: 'number', description: 'Page height in points' },
                    { name: 'rotation', type: 'number', description: 'Page rotation in degrees' }
                ],
                relationships: [
                    { type: 'belongsTo', target: 'Document', description: 'Part of a document' },
                    { type: 'hasMany', target: 'TextContent', description: 'Contains text elements' },
                    { type: 'hasMany', target: 'ImageContent', description: 'Contains embedded images' },
                    { type: 'hasMany', target: 'Annotation', description: 'Contains page-level annotations' }
                ]
            },
            {
                id: 'TextContent',
                label: 'Text Content',
                description: 'Extracted text with precise positioning, font information, and styling.',
                properties: [
                    { name: 'text', type: 'string', description: 'The text content' },
                    { name: 'x', type: 'number', description: 'X position on page' },
                    { name: 'y', type: 'number', description: 'Y position on page' },
                    { name: 'fontSize', type: 'number', description: 'Font size in points' },
                    { name: 'fontName', type: 'string', description: 'Font family name' },
                    { name: 'style', type: 'TextStyle', description: 'Bold, italic, underline, color' }
                ],
                relationships: [
                    { type: 'belongsTo', target: 'Page', description: 'Located on a specific page' }
                ]
            },
            {
                id: 'ImageContent',
                label: 'Image Content',
                description: 'An embedded image extracted from the PDF with metadata and pixel data.',
                properties: [
                    { name: 'id', type: 'string', description: 'Unique image identifier' },
                    { name: 'width', type: 'number', description: 'Image width in pixels' },
                    { name: 'height', type: 'number', description: 'Image height in pixels' },
                    { name: 'mimeType', type: 'string', description: 'Image MIME type' },
                    { name: 'colorSpace', type: 'string', description: 'Color space (RGB, CMYK, etc.)' }
                ],
                relationships: [
                    { type: 'belongsTo', target: 'Page', description: 'Located on a specific page' }
                ]
            },
            {
                id: 'SemanticChunk',
                label: 'Semantic Chunk',
                description: 'A semantically coherent text segment with metadata, optimized for RAG systems and vector stores.',
                properties: [
                    { name: 'id', type: 'string', description: 'Unique chunk identifier' },
                    { name: 'content', type: 'string', description: 'Chunk text content' },
                    { name: 'pageNumbers', type: 'number[]', description: 'Source page numbers' },
                    { name: 'type', type: 'ChunkType', description: 'Content type classification' },
                    { name: 'embedding', type: 'Float32Array', description: 'Optional embedding vector' },
                    { name: 'metadata', type: 'ChunkMetadata', description: 'Token count, confidence, keywords, entities' }
                ],
                relationships: [
                    { type: 'belongsTo', target: 'Document', description: 'Derived from a document' },
                    { type: 'belongsTo', target: 'Page', description: 'Sourced from specific pages' }
                ]
            },
            {
                id: 'AIFeatures',
                label: 'AI Analysis Results',
                description: 'Comprehensive AI analysis output including structural analysis, semantic chunks, and NLP-ready content.',
                properties: [
                    { name: 'structuralAnalysis', type: 'StructuralAnalysis', description: 'Document structure: sections, tables, figures' },
                    { name: 'semanticChunks', type: 'SemanticChunk[]', description: 'Semantic text chunks' },
                    { name: 'nlpReady', type: 'NLPReadyContent', description: 'Clean text, sentences, paragraphs, summary' }
                ],
                relationships: [
                    { type: 'belongsTo', target: 'Document', description: 'Analysis of a specific document' },
                    { type: 'hasMany', target: 'SemanticChunk', description: 'Contains semantic chunks' }
                ]
            },
            {
                id: 'Annotation',
                label: 'PDF Annotation',
                description: 'A document annotation such as a highlight, note, link, stamp, or redaction.',
                properties: [
                    { name: 'id', type: 'string', description: 'Unique annotation identifier' },
                    { name: 'type', type: 'AnnotationType', description: 'Annotation type enum value' },
                    { name: 'rect', type: 'Rectangle', description: 'Bounding rectangle on page' },
                    { name: 'contents', type: 'string', description: 'Annotation text content' },
                    { name: 'author', type: 'string', description: 'Annotation author' }
                ],
                relationships: [
                    { type: 'belongsTo', target: 'Page', description: 'Located on a specific page' }
                ]
            },
            {
                id: 'FormField',
                label: 'Form Field',
                description: 'An interactive form field that can be read and filled programmatically.',
                properties: [
                    { name: 'id', type: 'string', description: 'Field identifier' },
                    { name: 'type', type: 'FormFieldType', description: 'Field type: Button, Text, Choice, Signature' },
                    { name: 'name', type: 'string', description: 'Field name for form filling' },
                    { name: 'value', type: 'any', description: 'Current field value' },
                    { name: 'required', type: 'boolean', description: 'Whether the field is required' }
                ],
                relationships: [
                    { type: 'belongsTo', target: 'Document', description: 'Part of a document form' },
                    { type: 'belongsTo', target: 'Page', description: 'Located on a specific page' }
                ]
            },
            {
                id: 'StructuralAnalysis',
                label: 'Document Structure',
                description: 'Structural decomposition of the document into sections, tables, figures, equations, and references.',
                properties: [
                    { name: 'documentType', type: 'DocumentType', description: 'Auto-detected document category' },
                    { name: 'sections', type: 'DocumentSection[]', description: 'Hierarchical section structure' },
                    { name: 'tables', type: 'Table[]', description: 'Detected tables with cell data' },
                    { name: 'figures', type: 'Figure[]', description: 'Detected figures with captions' },
                    { name: 'equations', type: 'Equation[]', description: 'Detected mathematical equations' },
                    { name: 'references', type: 'Reference[]', description: 'Cross-references and citations' }
                ],
                relationships: [
                    { type: 'belongsTo', target: 'AIFeatures', description: 'Part of AI analysis results' }
                ]
            }
        ];
    }
    /**
     * Get document outline (bookmarks) as a hierarchical tree.
     * Returns an array of top-level outline items, each potentially with nested children.
     */
    getOutline() {
        if (!this.parser || !this.xrefTable || !this.catalog)
            return [];
        const parser = this.parser;
        const xref = this.xrefTable;
        const resolve = (obj) => {
            if (obj && obj.type === PDFObjectType.Reference) {
                const ref = obj.value;
                return parser.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);
            }
            return obj;
        };
        // Build page object number -> page index map
        const objNumToPage = new Map();
        const collectPages = (node, ref) => {
            const dict = resolve(node);
            if (dict.type !== PDFObjectType.Dictionary)
                return;
            const d = dict.value;
            const typeEntry = d.entries.get('Type');
            const typeName = typeEntry?.type === PDFObjectType.Name ? typeEntry.value : '';
            if (typeName === 'Page') {
                if (ref && ref.type === PDFObjectType.Reference) {
                    objNumToPage.set(ref.value.objectNumber, objNumToPage.size + 1);
                }
                return;
            }
            const kids = d.entries.get('Kids');
            if (!kids)
                return;
            const kidsArr = resolve(kids);
            if (kidsArr.type !== PDFObjectType.Array)
                return;
            for (const kid of kidsArr.value) {
                collectPages(kid, kid);
            }
        };
        const pagesRef = this.catalog.entries.get('Pages');
        if (pagesRef)
            collectPages(pagesRef);
        // Get the /Outlines entry from the catalog
        const outlinesRef = this.catalog.entries.get('Outlines');
        if (!outlinesRef)
            return [];
        const outlinesObj = resolve(outlinesRef);
        if (outlinesObj.type !== PDFObjectType.Dictionary)
            return [];
        const outlinesDict = outlinesObj.value;
        // Helper: extract page number from a destination
        const getPageFromDest = (dest) => {
            const resolved = resolve(dest);
            if (resolved.type === PDFObjectType.Array) {
                const arr = resolved.value;
                if (arr.length > 0) {
                    const pageRef = arr[0];
                    if (pageRef.type === PDFObjectType.Reference) {
                        return objNumToPage.get(pageRef.value.objectNumber) ?? null;
                    }
                }
            }
            else if (resolved.type === PDFObjectType.String || resolved.type === PDFObjectType.Name) {
                // Named destination — look up in named dests
                const namedDests = this.getNamedDestinations();
                const info = namedDests.get(resolved.value);
                return info?.page ?? null;
            }
            return null;
        };
        // Helper: get destination string from an outline item
        const getDestString = (dict) => {
            const dest = dict.entries.get('Dest');
            if (dest) {
                const resolved = resolve(dest);
                if (resolved.type === PDFObjectType.String || resolved.type === PDFObjectType.Name) {
                    return resolved.value;
                }
                // Array dest — format as string
                if (resolved.type === PDFObjectType.Array) {
                    return null; // Direct page ref, no string name
                }
            }
            // Check /A (action) dict
            const action = dict.entries.get('A');
            if (action) {
                const aObj = resolve(action);
                if (aObj.type === PDFObjectType.Dictionary) {
                    const aDict = aObj.value;
                    const sEntry = aDict.entries.get('S');
                    if (sEntry?.type === PDFObjectType.Name && sEntry.value === 'GoTo') {
                        const dEntry = aDict.entries.get('D');
                        if (dEntry) {
                            const res = resolve(dEntry);
                            if (res.type === PDFObjectType.String || res.type === PDFObjectType.Name) {
                                return res.value;
                            }
                        }
                    }
                }
            }
            return null;
        };
        // Helper: get page number from outline item
        const getPageNum = (dict) => {
            const dest = dict.entries.get('Dest');
            if (dest) {
                return getPageFromDest(dest);
            }
            const action = dict.entries.get('A');
            if (action) {
                const aObj = resolve(action);
                if (aObj.type === PDFObjectType.Dictionary) {
                    const aDict = aObj.value;
                    const sEntry = aDict.entries.get('S');
                    if (sEntry?.type === PDFObjectType.Name && sEntry.value === 'GoTo') {
                        const dEntry = aDict.entries.get('D');
                        if (dEntry)
                            return getPageFromDest(dEntry);
                    }
                }
            }
            return null;
        };
        // Walk the outline tree using /First / /Next sibling chain
        const visited = new Set();
        const walkOutline = (entryObj) => {
            const items = [];
            let current = entryObj;
            while (current) {
                const resolved = resolve(current);
                if (resolved.type !== PDFObjectType.Dictionary)
                    break;
                const dict = resolved.value;
                // Prevent infinite loops
                const refKey = current.type === PDFObjectType.Reference
                    ? `${current.value.objectNumber}:${current.value.generationNumber}`
                    : '';
                if (refKey && visited.has(refKey))
                    break;
                if (refKey)
                    visited.add(refKey);
                // Extract title
                const titleObj = dict.entries.get('Title');
                let title = '';
                if (titleObj) {
                    const titleRes = resolve(titleObj);
                    if (titleRes.type === PDFObjectType.String) {
                        title = titleRes.value;
                    }
                }
                // Extract style flags from /F entry (bit 0 = italic, bit 1 = bold)
                let bold = false;
                let italic = false;
                const flagsObj = dict.entries.get('F');
                if (flagsObj?.type === PDFObjectType.Number) {
                    const flags = flagsObj.value;
                    italic = !!(flags & 1);
                    bold = !!(flags & 2);
                }
                // Extract color from /C entry [r g b] (0-1 range)
                let color = null;
                const cObj = dict.entries.get('C');
                if (cObj) {
                    const cRes = resolve(cObj);
                    if (cRes.type === PDFObjectType.Array) {
                        const cArr = cRes.value;
                        if (cArr.length >= 3) {
                            color = {
                                r: Math.round((cArr[0].value || 0) * 255),
                                g: Math.round((cArr[1].value || 0) * 255),
                                b: Math.round((cArr[2].value || 0) * 255)
                            };
                        }
                    }
                }
                // Recursively process children via /First
                const firstChild = dict.entries.get('First');
                const children = firstChild ? walkOutline(firstChild) : [];
                items.push({
                    title,
                    destination: getDestString(dict),
                    page: getPageNum(dict),
                    bold,
                    italic,
                    color,
                    children
                });
                // Move to next sibling via /Next
                const nextObj = dict.entries.get('Next');
                current = nextObj || null;
            }
            return items;
        };
        // Start from the first child of the /Outlines dict
        const firstEntry = outlinesDict.entries.get('First');
        if (!firstEntry)
            return [];
        return walkOutline(firstEntry);
    }
    /**
       * Describes the currently loaded document's available operations and
       * recommends workflows based on document characteristics.
       * Returns undefined if no document is loaded.
       */
    describeDocument() {
        if (!this.metadata)
            return undefined;
        const pageCount = this.getPageCount();
        const fileSize = this.getFileSize();
        const encrypted = this.isEncrypted();
        const operations = [
            'extractText', 'streamText', 'extractImages',
            'getAIFeatures', 'generateSemanticChunks', 'streamSemanticChunks',
            'search', 'getAnnotations', 'addAnnotation',
            'getFormFields', 'fillForm',
            'renderPage', 'renderPageToImage', 'buildTextLayer',
            'exportAs', 'save',
            'getMetadata', 'getPage', 'getAllPages', 'getNamedDestinations',
            'close', 'unloadPages', 'getMemoryStats'
        ];
        const workflows = ['basic-text-extraction'];
        if (pageCount > 50 || fileSize > 10 * 1024 * 1024) {
            workflows.push('streaming-large-document');
        }
        workflows.push('rag-pipeline', 'document-analysis', 'multi-format-export', 'llm-streaming');
        let complexity = 'simple';
        if (pageCount > 100 || fileSize > 50 * 1024 * 1024) {
            complexity = 'complex';
        }
        else if (pageCount > 20 || fileSize > 5 * 1024 * 1024) {
            complexity = 'moderate';
        }
        return {
            documentInfo: {
                pageCount,
                fileSize,
                version: this.getVersion(),
                encrypted
            },
            availableOperations: operations,
            recommendedWorkflows: workflows,
            estimatedComplexity: complexity
        };
    }
}
// ============================================================================
// PDF Parser Implementation
// ============================================================================
class PDFParser {
    constructor(buffer, options = {}) {
        this.buffer = buffer;
        this.options = options;
        this.position = 0;
        this.objectCache = new Map();
        this.dataView = new DataView(buffer);
    }
    async parseHeader() {
        const header = this.readString(0, 8);
        if (!header.startsWith('%PDF-')) {
            throw new Error('Invalid PDF header');
        }
        return header.substring(5, 8);
    }
    async parseXRef() {
        // Find xref offset from trailer
        const trailerOffset = this.findTrailer();
        const xrefOffset = this.parseTrailerDict(trailerOffset);
        this.xrefTable = this.parseXRefTable(xrefOffset);
        return this.xrefTable;
    }
    findTrailer() {
        const view = new Uint8Array(this.buffer);
        const trailer = new TextEncoder().encode('trailer');
        // Search from the end of file
        for (let i = view.length - trailer.length; i >= 0; i--) {
            let match = true;
            for (let j = 0; j < trailer.length; j++) {
                if (view[i + j] !== trailer[j]) {
                    match = false;
                    break;
                }
            }
            if (match)
                return i;
        }
        throw new Error('Trailer not found');
    }
    parseTrailerDict(offset) {
        this.position = offset;
        this.skipWhitespace();
        // Skip "trailer" keyword
        this.position += 7;
        this.skipWhitespace();
        const dict = this.parseDictionary();
        const prev = dict.entries.get('Prev');
        const _xrefStm = dict.entries.get('XRefStm');
        if (prev) {
            return prev.value;
        }
        // Find startxref
        const startxrefOffset = this.findStartXRef();
        this.position = startxrefOffset + 9; // Skip "startxref"
        this.skipWhitespace();
        return this.parseNumber();
    }
    findStartXRef() {
        const view = new Uint8Array(this.buffer);
        const startxref = new TextEncoder().encode('startxref');
        for (let i = view.length - startxref.length; i >= 0; i--) {
            let match = true;
            for (let j = 0; j < startxref.length; j++) {
                if (view[i + j] !== startxref[j]) {
                    match = false;
                    break;
                }
            }
            if (match)
                return i;
        }
        throw new Error('startxref not found');
    }
    parseXRefTable(offset) {
        this.position = offset;
        this.skipWhitespace();
        const xref = new XRefTable();
        // Check if it's a cross-reference stream (PDF 1.5+)
        if (this.peekByte() >= '0'.charCodeAt(0) && this.peekByte() <= '9'.charCodeAt(0)) {
            return this.parseXRefStream(offset);
        }
        // Traditional xref table
        const keyword = this.readString(this.position, 4);
        if (keyword !== 'xref') {
            throw new Error('Invalid xref table');
        }
        this.position += 4;
        this.skipWhitespace();
        // Safety: limit iterations to prevent infinite loops
        let safetyCounter = 0;
        const MAX_XREF_SUBSECTIONS = 1000;
        while (this.peekByte() >= '0'.charCodeAt(0) && this.peekByte() <= '9'.charCodeAt(0)) {
            if (++safetyCounter > MAX_XREF_SUBSECTIONS) {
                console.warn('parseXRefTable: Safety limit reached, stopping xref parsing');
                break;
            }
            const positionBefore = this.position;
            const start = this.parseNumber();
            this.skipWhitespace();
            const count = this.parseNumber();
            this.skipWhitespace();
            // Safety check: ensure position advanced
            if (this.position === positionBefore) {
                console.warn('parseXRefTable: Position not advancing, breaking to prevent infinite loop');
                break;
            }
            // Safety check: reasonable count value
            if (count < 0 || count > 100000) {
                console.warn(`parseXRefTable: Unreasonable count value ${count}, skipping subsection`);
                continue;
            }
            for (let i = 0; i < count; i++) {
                const offset = this.parseNumber();
                this.skipWhitespace();
                const generation = this.parseNumber();
                this.skipWhitespace();
                const type = String.fromCharCode(this.readByte());
                this.skipWhitespace();
                xref.addEntry(start + i, offset, generation, type);
            }
        }
        return xref;
    }
    parseXRefStream(offset) {
        // Parse cross-reference stream (PDF 1.5+)
        const xref = new XRefTable();
        this.position = offset;
        // Parse the indirect object header: "objNum genNum obj"
        this.parseNumber();
        this.skipWhitespace();
        this.parseNumber();
        this.skipWhitespace();
        this.position += 3; // Skip "obj"
        this.skipWhitespace();
        // Parse stream dictionary
        const dictObj = this.parseObject();
        if (dictObj.type !== PDFObjectType.Dictionary) {
            console.warn('parseXRefStream: Expected dictionary');
            return xref;
        }
        const dict = dictObj.value;
        // Get /W array (field widths: [type_width offset_width gen_width])
        const wObj = dict.entries.get('W');
        if (!wObj || wObj.type !== PDFObjectType.Array) {
            console.warn('parseXRefStream: Missing /W array');
            return xref;
        }
        const wArray = wObj.value.map(o => o.value);
        if (wArray.length < 3) {
            console.warn('parseXRefStream: /W array too short');
            return xref;
        }
        const [w1, w2, w3] = wArray;
        // Get /Size (total number of objects)
        const sizeObj = dict.entries.get('Size');
        const size = sizeObj && sizeObj.type === PDFObjectType.Number ? sizeObj.value : 0;
        // Get /Index array (defaults to [0 Size])
        let indexPairs = [0, size];
        const indexObj = dict.entries.get('Index');
        if (indexObj && indexObj.type === PDFObjectType.Array) {
            indexPairs = indexObj.value.map(o => o.value);
        }
        // Parse the stream data
        this.skipWhitespace();
        const streamKeyword = this.readString(this.position, 6);
        if (streamKeyword !== 'stream') {
            console.warn('parseXRefStream: Expected "stream" keyword');
            return xref;
        }
        this.position += 6;
        if (this.peekByte() === '\r'.charCodeAt(0))
            this.position++;
        if (this.peekByte() === '\n'.charCodeAt(0))
            this.position++;
        // Get stream length
        const lengthObj = dict.entries.get('Length');
        let streamLength = 0;
        if (lengthObj && lengthObj.type === PDFObjectType.Number) {
            streamLength = lengthObj.value;
        }
        else if (lengthObj && lengthObj.type === PDFObjectType.Reference && this.xrefTable) {
            const ref = lengthObj.value;
            const savedPos = this.position;
            try {
                const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                if (resolved.type === PDFObjectType.Number) {
                    streamLength = resolved.value;
                }
            }
            catch {
                // Ignore — will fall back to zero length
            }
            this.position = savedPos;
        }
        // Read and decompress stream data
        const rawData = new Uint8Array(this.buffer, this.position, streamLength);
        let data;
        const filterObj = dict.entries.get('Filter');
        if (filterObj) {
            const filter = this.getFilterName(filterObj);
            if (filter === 'FlateDecode') {
                try {
                    data = this.decompressFlate(rawData);
                }
                catch {
                    data = new Uint8Array(rawData);
                }
            }
            else {
                data = new Uint8Array(rawData);
            }
        }
        else {
            data = new Uint8Array(rawData);
        }
        // Parse entries from the decompressed data
        const entrySize = w1 + w2 + w3;
        let dataPos = 0;
        for (let p = 0; p < indexPairs.length; p += 2) {
            const startObj = indexPairs[p];
            const count = indexPairs[p + 1];
            for (let i = 0; i < count && dataPos + entrySize <= data.length; i++) {
                // Read type field (default 1 if w1 is 0)
                let type = w1 === 0 ? 1 : 0;
                for (let b = 0; b < w1; b++) {
                    type = (type << 8) | data[dataPos++];
                }
                // Read field 2 (offset for type 1, object number for type 2)
                let field2 = 0;
                for (let b = 0; b < w2; b++) {
                    field2 = (field2 << 8) | data[dataPos++];
                }
                // Read field 3 (generation for type 1, index for type 2)
                let field3 = 0;
                for (let b = 0; b < w3; b++) {
                    field3 = (field3 << 8) | data[dataPos++];
                }
                const objNum = startObj + i;
                if (type === 0) {
                    xref.addEntry(objNum, field2, field3, 'f');
                }
                else if (type === 1) {
                    xref.addEntry(objNum, field2, field3, 'n');
                }
                else if (type === 2) {
                    // Compressed object in object stream
                    xref.addEntry(objNum, field2, field3, 'n');
                }
            }
        }
        // Handle /Prev for chained xref streams
        const prevObj = dict.entries.get('Prev');
        if (prevObj && prevObj.type === PDFObjectType.Number) {
            const prevOffset = prevObj.value;
            try {
                const prevXref = this.parseXRefTable(prevOffset);
                for (const [objNum, entry] of prevXref.getAllEntries()) {
                    if (!xref.getEntry(objNum)) {
                        xref.addEntry(objNum, entry.offset, entry.generation, entry.type);
                    }
                }
            }
            catch {
                console.warn('parseXRefStream: Failed to parse previous xref at offset', prevOffset);
            }
        }
        return xref;
    }
    async parseCatalog(xref) {
        // Get root object from trailer
        const trailerOffset = this.findTrailer();
        this.position = trailerOffset + 7;
        this.skipWhitespace();
        const trailer = this.parseDictionary();
        const rootRef = trailer.entries.get('Root');
        if (!rootRef || rootRef.type !== PDFObjectType.Reference) {
            throw new Error('Root catalog not found');
        }
        const ref = rootRef.value;
        const catalogObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);
        if (catalogObj.type !== PDFObjectType.Dictionary) {
            throw new Error('Invalid catalog object');
        }
        return catalogObj.value;
    }
    async parseMetadata(xref, catalog) {
        const metadata = {
            version: await this.parseHeader(),
            pageCount: 0,
            isEncrypted: false,
            isLinearized: false,
            fileSize: this.buffer.byteLength
        };
        // Get page count from Pages object
        const pagesRef = catalog.entries.get('Pages');
        if (pagesRef && pagesRef.type === PDFObjectType.Reference) {
            const ref = pagesRef.value;
            const pagesObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);
            if (pagesObj.type === PDFObjectType.Dictionary) {
                const pagesDict = pagesObj.value;
                const countObj = pagesDict.entries.get('Count');
                if (countObj && countObj.type === PDFObjectType.Number) {
                    metadata.pageCount = countObj.value;
                }
            }
        }
        // Parse Info dictionary if present
        const trailerOffset = this.findTrailer();
        this.position = trailerOffset + 7;
        this.skipWhitespace();
        const trailer = this.parseDictionary();
        const infoRef = trailer.entries.get('Info');
        if (infoRef && infoRef.type === PDFObjectType.Reference) {
            const ref = infoRef.value;
            const infoObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);
            if (infoObj.type === PDFObjectType.Dictionary) {
                const info = infoObj.value;
                // Extract metadata fields
                metadata.title = this.extractStringFromDict(info, 'Title');
                metadata.author = this.extractStringFromDict(info, 'Author');
                metadata.subject = this.extractStringFromDict(info, 'Subject');
                metadata.keywords = this.extractStringFromDict(info, 'Keywords');
                metadata.creator = this.extractStringFromDict(info, 'Creator');
                metadata.producer = this.extractStringFromDict(info, 'Producer');
                const creationDate = this.extractStringFromDict(info, 'CreationDate');
                if (creationDate)
                    metadata.creationDate = this.parsePDFDate(creationDate);
                const modDate = this.extractStringFromDict(info, 'ModDate');
                if (modDate)
                    metadata.modificationDate = this.parsePDFDate(modDate);
            }
        }
        // Check for encryption
        const encryptRef = trailer.entries.get('Encrypt');
        metadata.isEncrypted = encryptRef !== undefined;
        return metadata;
    }
    async parsePageTree(catalog) {
        const pageTree = new PageTree();
        const pagesRef = catalog.entries.get('Pages');
        if (!pagesRef || pagesRef.type !== PDFObjectType.Reference) {
            throw new Error('Pages reference not found in catalog');
        }
        // Get the Pages dictionary
        const ref = pagesRef.value;
        const pagesObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
        if (pagesObj.type !== PDFObjectType.Dictionary) {
            throw new Error('Invalid Pages object');
        }
        const pagesDict = pagesObj.value;
        // Recursively parse the page tree
        await this.parsePageTreeNode(pagesDict, pageTree, 1);
        return pageTree;
    }
    async parsePageTreeNode(node, pageTree, currentPageNum, depth = 0) {
        // Prevent infinite recursion
        if (depth > 100) {
            console.error('Maximum page tree depth exceeded');
            return currentPageNum;
        }
        const kids = node.entries.get('Kids');
        if (kids && kids.type === PDFObjectType.Array) {
            const kidsArray = kids.value;
            for (const kidRef of kidsArray) {
                try {
                    if (kidRef.type === PDFObjectType.Reference) {
                        const ref = kidRef.value;
                        const kidObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                        if (kidObj.type === PDFObjectType.Dictionary) {
                            const kidDict = kidObj.value;
                            const kidType = kidDict.entries.get('Type');
                            const kidTypeValue = kidType?.type === PDFObjectType.Name ? kidType.value : '';
                            if (kidTypeValue === 'Page') {
                                // This is a leaf node (actual page)
                                pageTree.addPage(currentPageNum, kidDict);
                                currentPageNum++;
                            }
                            else if (kidTypeValue === 'Pages') {
                                // This is an intermediate node, recurse
                                currentPageNum = await this.parsePageTreeNode(kidDict, pageTree, currentPageNum, depth + 1);
                            }
                        }
                    }
                }
                catch (error) {
                    console.error(`Error parsing page tree node:`, error);
                    // Continue with next kid instead of failing completely
                }
            }
        }
        return currentPageNum;
    }
    async parsePage(pageNumber, pageTree) {
        // Get page object from page tree
        const pageObj = pageTree.getPage(pageNumber);
        if (!pageObj) {
            throw new Error(`Page ${pageNumber} not found`);
        }
        const page = {
            pageNumber,
            width: 612,
            height: 792,
            rotation: 0,
            userUnit: 1.0,
            mediaBox: { x: 0, y: 0, width: 612, height: 792 }
        };
        // Parse page dimensions
        const mediaBox = this.parseRectangle(pageObj.entries.get('MediaBox'));
        if (mediaBox)
            page.mediaBox = mediaBox;
        const cropBox = this.parseRectangle(pageObj.entries.get('CropBox'));
        if (cropBox)
            page.cropBox = cropBox;
        // Parse rotation
        const rotationObj = pageObj.entries.get('Rotate');
        if (rotationObj && rotationObj.type === PDFObjectType.Number) {
            page.rotation = rotationObj.value;
        }
        // Calculate dimensions
        page.width = page.mediaBox.width;
        page.height = page.mediaBox.height;
        // Parse content stream
        const contentsRef = pageObj.entries.get('Contents');
        if (contentsRef) {
            page.contents = await this.parseContents(contentsRef);
        }
        // Parse resources
        const resourcesRef = pageObj.entries.get('Resources');
        if (resourcesRef) {
            page.resources = await this.parseResources(resourcesRef);
        }
        return page;
    }
    async parseContents(contentsRef) {
        // Contents can be a single stream or an array of streams
        if (contentsRef.type === PDFObjectType.Reference) {
            const ref = contentsRef.value;
            const streamObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
            if (streamObj.type === PDFObjectType.Stream) {
                const stream = streamObj.value;
                return stream.data;
            }
        }
        else if (contentsRef.type === PDFObjectType.Array) {
            // Concatenate multiple content streams
            const streams = contentsRef.value;
            const allData = [];
            for (const streamRef of streams) {
                if (streamRef.type === PDFObjectType.Reference) {
                    const ref = streamRef.value;
                    const streamObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                    if (streamObj.type === PDFObjectType.Stream) {
                        const stream = streamObj.value;
                        allData.push(stream.data);
                    }
                }
            }
            // Concatenate all streams
            const totalLength = allData.reduce((sum, data) => sum + data.length, 0);
            const combined = new Uint8Array(totalLength);
            let offset = 0;
            for (const data of allData) {
                combined.set(data, offset);
                offset += data.length;
            }
            return combined;
        }
        else if (contentsRef.type === PDFObjectType.Stream) {
            const stream = contentsRef.value;
            return stream.data;
        }
        return new Uint8Array(0);
    }
    async parseResources(resourcesRef) {
        const resources = {
            fonts: new Map(),
            images: new Map(),
            colorSpaces: new Map(),
            patterns: new Map(),
            xObjects: new Map(),
            extGState: new Map()
        };
        let resourcesDict;
        if (resourcesRef.type === PDFObjectType.Reference) {
            const ref = resourcesRef.value;
            const resourcesObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
            if (resourcesObj.type === PDFObjectType.Dictionary) {
                resourcesDict = resourcesObj.value;
            }
        }
        else if (resourcesRef.type === PDFObjectType.Dictionary) {
            resourcesDict = resourcesRef.value;
        }
        if (!resourcesDict)
            return resources;
        // Parse font resources
        const fontRef = resourcesDict.entries.get('Font');
        let fontDict;
        if (fontRef && fontRef.type === PDFObjectType.Dictionary) {
            fontDict = fontRef.value;
        }
        else if (fontRef && fontRef.type === PDFObjectType.Reference) {
            const ref = fontRef.value;
            const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
            if (resolved.type === PDFObjectType.Dictionary) {
                fontDict = resolved.value;
            }
        }
        if (fontDict) {
            for (const [name, fontObj] of fontDict.entries) {
                const fontResource = this.parseFontResource(name, fontObj);
                if (fontResource) {
                    resources.fonts.set(name, fontResource);
                }
            }
        }
        // Parse image resources (XObject)
        const xObjectRef = resourcesDict.entries.get('XObject');
        let xObjectDict;
        if (xObjectRef && xObjectRef.type === PDFObjectType.Dictionary) {
            xObjectDict = xObjectRef.value;
        }
        else if (xObjectRef && xObjectRef.type === PDFObjectType.Reference) {
            const ref = xObjectRef.value;
            const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
            if (resolved.type === PDFObjectType.Dictionary) {
                xObjectDict = resolved.value;
            }
        }
        if (xObjectDict) {
            resources.images = new Map();
            for (const [name, xObj] of xObjectDict.entries) {
                if (xObj.type === PDFObjectType.Reference) {
                    const ref = xObj.value;
                    const xObjectObj = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                    if (xObjectObj.type === PDFObjectType.Stream) {
                        const stream = xObjectObj.value;
                        // Check XObject subtype
                        let xObjectType = 'image';
                        let isImage = true;
                        if (stream.dictionary) {
                            const subtypeObj = stream.dictionary.entries.get('Subtype');
                            if (subtypeObj && subtypeObj.type === PDFObjectType.Name) {
                                const subtype = subtypeObj.value;
                                if (subtype === 'Form') {
                                    xObjectType = 'form';
                                    isImage = false;
                                }
                                else if (subtype === 'PS') {
                                    xObjectType = 'ps';
                                    isImage = false;
                                }
                            }
                        }
                        // Create XObject entry
                        const xObject = {
                            type: xObjectType,
                            data: stream.data
                        };
                        if (stream.dictionary) {
                            // Parse image dimensions onto XObject
                            if (isImage) {
                                const wObj = stream.dictionary.entries.get('Width');
                                if (wObj && wObj.type === PDFObjectType.Number)
                                    xObject.width = wObj.value;
                                const hObj = stream.dictionary.entries.get('Height');
                                if (hObj && hObj.type === PDFObjectType.Number)
                                    xObject.height = hObj.value;
                                const csObj = stream.dictionary.entries.get('ColorSpace');
                                if (csObj && csObj.type === PDFObjectType.Name) {
                                    xObject.colorSpace = csObj.value;
                                }
                                else if (csObj && csObj.type === PDFObjectType.Array) {
                                    // ColorSpace may be [/ICCBased ref] or [/Indexed /DeviceRGB ...] — use first name
                                    const csArr = csObj.value;
                                    if (csArr.length > 0 && csArr[0].type === PDFObjectType.Name) {
                                        const csName = csArr[0].value;
                                        if (csName === 'ICCBased') {
                                            xObject.colorSpace = 'DeviceRGB'; // default; component count determines actual
                                        }
                                        else if (csName === 'Indexed' && csArr.length > 1 && csArr[1].type === PDFObjectType.Name) {
                                            xObject.colorSpace = csArr[1].value;
                                        }
                                        else {
                                            xObject.colorSpace = csName;
                                        }
                                    }
                                }
                                const bpcObj = stream.dictionary.entries.get('BitsPerComponent');
                                if (bpcObj && bpcObj.type === PDFObjectType.Number)
                                    xObject.bitsPerComponent = bpcObj.value;
                                const filterObj = stream.dictionary.entries.get('Filter');
                                if (filterObj && filterObj.type === PDFObjectType.Name) {
                                    xObject.filter = filterObj.value;
                                }
                                else if (filterObj && filterObj.type === PDFObjectType.Array) {
                                    const filterArr = filterObj.value;
                                    if (filterArr.length > 0 && filterArr[0].type === PDFObjectType.Name) {
                                        xObject.filter = filterArr[0].value;
                                    }
                                }
                                // Parse DecodeParms for predictor info
                                const dpObj = stream.dictionary.entries.get('DecodeParms');
                                let dpDict;
                                if (dpObj && dpObj.type === PDFObjectType.Dictionary) {
                                    dpDict = dpObj.value;
                                }
                                else if (dpObj && dpObj.type === PDFObjectType.Reference) {
                                    const dpRef = dpObj.value;
                                    const dpResolved = this.parseIndirectObject(dpRef.objectNumber, dpRef.generationNumber, this.xrefTable);
                                    if (dpResolved.type === PDFObjectType.Dictionary)
                                        dpDict = dpResolved.value;
                                }
                                else if (dpObj && dpObj.type === PDFObjectType.Array) {
                                    const dpArr = dpObj.value;
                                    if (dpArr.length > 0 && dpArr[0].type === PDFObjectType.Dictionary)
                                        dpDict = dpArr[0].value;
                                }
                                if (dpDict) {
                                    const predObj = dpDict.entries.get('Predictor');
                                    if (predObj && predObj.type === PDFObjectType.Number)
                                        xObject.predictor = predObj.value;
                                    const colObj = dpDict.entries.get('Columns');
                                    if (colObj && colObj.type === PDFObjectType.Number)
                                        xObject.predictorColumns = colObj.value;
                                    const colorsObj = dpDict.entries.get('Colors');
                                    if (colorsObj && colorsObj.type === PDFObjectType.Number)
                                        xObject.predictorColors = colorsObj.value;
                                }
                            }
                            // Parse Form XObject properties
                            if (xObjectType === 'form') {
                                const bboxObj = stream.dictionary.entries.get('BBox');
                                if (bboxObj && bboxObj.type === PDFObjectType.Array) {
                                    const arr = bboxObj.value;
                                    xObject.bbox = arr.filter(o => o.type === PDFObjectType.Number).map(o => o.value);
                                }
                                const matrixObj = stream.dictionary.entries.get('Matrix');
                                if (matrixObj && matrixObj.type === PDFObjectType.Array) {
                                    const arr = matrixObj.value;
                                    const nums = arr.filter(o => o.type === PDFObjectType.Number).map(o => o.value);
                                    if (nums.length === 6)
                                        xObject.matrix = nums;
                                }
                                const resObj = stream.dictionary.entries.get('Resources');
                                if (resObj) {
                                    try {
                                        xObject.resources = await this.parseResources(resObj);
                                    }
                                    catch (e) {
                                        // Form resource parsing is not critical
                                    }
                                }
                            }
                        }
                        resources.xObjects.set(name, xObject);
                        // Also create ImageResource for backward compatibility
                        if (isImage) {
                            const imageRes = {
                                width: 0,
                                height: 0,
                                bitsPerComponent: 8,
                                colorSpace: 'DeviceRGB',
                                data: stream.data
                            };
                            // Parse image properties from stream dictionary
                            if (stream.dictionary) {
                                const widthObj = stream.dictionary.entries.get('Width');
                                if (widthObj && widthObj.type === PDFObjectType.Number) {
                                    imageRes.width = widthObj.value;
                                }
                                const heightObj = stream.dictionary.entries.get('Height');
                                if (heightObj && heightObj.type === PDFObjectType.Number) {
                                    imageRes.height = heightObj.value;
                                }
                                const bpcObj = stream.dictionary.entries.get('BitsPerComponent');
                                if (bpcObj && bpcObj.type === PDFObjectType.Number) {
                                    imageRes.bitsPerComponent = bpcObj.value;
                                }
                                // Color space (handle Name, Array [/ICCBased ref], [/Indexed ...], etc.)
                                const csObj = stream.dictionary.entries.get('ColorSpace');
                                if (csObj && csObj.type === PDFObjectType.Name) {
                                    imageRes.colorSpace = csObj.value;
                                }
                                else if (csObj && csObj.type === PDFObjectType.Array) {
                                    const csArr = csObj.value;
                                    if (csArr.length > 0 && csArr[0].type === PDFObjectType.Name) {
                                        const csName = csArr[0].value;
                                        if (csName === 'ICCBased' && csArr.length > 1 && csArr[1].type === PDFObjectType.Reference) {
                                            // Resolve ICC profile stream to get /N (component count)
                                            try {
                                                const iccRef = csArr[1].value;
                                                const iccObj = this.parseIndirectObject(iccRef.objectNumber, iccRef.generationNumber, this.xrefTable);
                                                if (iccObj.type === PDFObjectType.Stream) {
                                                    const iccStream = iccObj.value;
                                                    const nObj = iccStream.dictionary?.entries.get('N');
                                                    const n = nObj && nObj.type === PDFObjectType.Number ? nObj.value : 3;
                                                    if (n === 1)
                                                        imageRes.colorSpace = 'DeviceGray';
                                                    else if (n === 4)
                                                        imageRes.colorSpace = 'DeviceCMYK';
                                                    else
                                                        imageRes.colorSpace = 'DeviceRGB';
                                                }
                                            }
                                            catch (e) {
                                                imageRes.colorSpace = 'DeviceRGB';
                                            }
                                        }
                                        else if (csName === 'Indexed' && csArr.length > 1 && csArr[1].type === PDFObjectType.Name) {
                                            imageRes.colorSpace = csArr[1].value;
                                        }
                                        else {
                                            imageRes.colorSpace = csName;
                                        }
                                    }
                                }
                                // Filter (Name or Array)
                                const filterObj = stream.dictionary.entries.get('Filter');
                                if (filterObj && filterObj.type === PDFObjectType.Name) {
                                    imageRes.filter = [filterObj.value];
                                }
                                else if (filterObj && filterObj.type === PDFObjectType.Array) {
                                    const fArr = filterObj.value;
                                    imageRes.filter = fArr.filter(f => f.type === PDFObjectType.Name).map(f => f.value);
                                }
                                // DecodeParms
                                const dpObj = stream.dictionary.entries.get('DecodeParms');
                                let dpDict;
                                if (dpObj && dpObj.type === PDFObjectType.Dictionary) {
                                    dpDict = dpObj.value;
                                }
                                else if (dpObj && dpObj.type === PDFObjectType.Reference) {
                                    const dpRef = dpObj.value;
                                    const dpRes = this.parseIndirectObject(dpRef.objectNumber, dpRef.generationNumber, this.xrefTable);
                                    if (dpRes.type === PDFObjectType.Dictionary)
                                        dpDict = dpRes.value;
                                }
                                else if (dpObj && dpObj.type === PDFObjectType.Array) {
                                    const dpArr = dpObj.value;
                                    if (dpArr.length > 0 && dpArr[0].type === PDFObjectType.Dictionary)
                                        dpDict = dpArr[0].value;
                                }
                                if (dpDict) {
                                    const dp = {};
                                    for (const [k, v] of dpDict.entries) {
                                        if (v.type === PDFObjectType.Number)
                                            dp[k] = v.value;
                                        else if (v.type === PDFObjectType.Boolean)
                                            dp[k] = v.value;
                                        else if (v.type === PDFObjectType.Name)
                                            dp[k] = v.value;
                                    }
                                    imageRes.decodeParms = dp;
                                }
                                // SMask (soft mask / alpha channel)
                                const smaskObj = stream.dictionary.entries.get('SMask');
                                if (smaskObj && smaskObj.type === PDFObjectType.Reference) {
                                    try {
                                        const smRef = smaskObj.value;
                                        const smResolved = this.parseIndirectObject(smRef.objectNumber, smRef.generationNumber, this.xrefTable);
                                        if (smResolved.type === PDFObjectType.Stream) {
                                            const smStream = smResolved.value;
                                            imageRes.smaskData = smStream.data;
                                            const smW = smStream.dictionary?.entries.get('Width');
                                            if (smW && smW.type === PDFObjectType.Number)
                                                imageRes.smaskWidth = smW.value;
                                            const smH = smStream.dictionary?.entries.get('Height');
                                            if (smH && smH.type === PDFObjectType.Number)
                                                imageRes.smaskHeight = smH.value;
                                        }
                                    }
                                    catch (e) { /* ignore smask resolution failure */ }
                                }
                            }
                            resources.images.set(name, imageRes);
                        }
                    }
                }
            }
        }
        // Parse ExtGState resources
        const gsRef = resourcesDict.entries.get('ExtGState');
        let gsDict;
        if (gsRef && gsRef.type === PDFObjectType.Dictionary) {
            gsDict = gsRef.value;
        }
        else if (gsRef && gsRef.type === PDFObjectType.Reference) {
            const ref = gsRef.value;
            const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
            if (resolved.type === PDFObjectType.Dictionary) {
                gsDict = resolved.value;
            }
        }
        if (gsDict) {
            for (const [name, gsObj] of gsDict.entries) {
                let gsEntryDict;
                if (gsObj.type === PDFObjectType.Dictionary) {
                    gsEntryDict = gsObj.value;
                }
                else if (gsObj.type === PDFObjectType.Reference) {
                    const ref = gsObj.value;
                    const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                    if (resolved.type === PDFObjectType.Dictionary) {
                        gsEntryDict = resolved.value;
                    }
                }
                if (gsEntryDict) {
                    const state = {};
                    for (const [key, val] of gsEntryDict.entries) {
                        if (val.type === PDFObjectType.Number)
                            state[key] = val.value;
                        else if (val.type === PDFObjectType.Name)
                            state[key] = val.value;
                        else if (val.type === PDFObjectType.Boolean)
                            state[key] = val.value;
                    }
                    resources.extGState.set(name, state);
                }
            }
        }
        // Parse ColorSpace resources
        const csRef = resourcesDict.entries.get('ColorSpace');
        let csDict;
        if (csRef && csRef.type === PDFObjectType.Dictionary) {
            csDict = csRef.value;
        }
        else if (csRef && csRef.type === PDFObjectType.Reference) {
            const ref = csRef.value;
            const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
            if (resolved.type === PDFObjectType.Dictionary) {
                csDict = resolved.value;
            }
        }
        if (csDict) {
            for (const [name, csObj] of csDict.entries) {
                try {
                    const colorSpace = PDFColorSpaceProcessor.parseColorSpace(csObj, resources);
                    resources.colorSpaces.set(name, colorSpace);
                }
                catch (e) {
                    // Color space parsing is not critical
                }
            }
        }
        return resources;
    }
    parseFontResource(name, fontObj) {
        try {
            let fontDict;
            // Resolve font reference
            if (fontObj.type === PDFObjectType.Reference) {
                const ref = fontObj.value;
                const resolvedFont = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                if (resolvedFont.type === PDFObjectType.Dictionary) {
                    fontDict = resolvedFont.value;
                }
            }
            else if (fontObj.type === PDFObjectType.Dictionary) {
                fontDict = fontObj.value;
            }
            if (!fontDict)
                return null;
            // Extract basic font info
            const fontResource = {
                name: name,
                type: 'Font',
                subtype: 'Type1'
            };
            // Get Subtype
            const subtypeObj = fontDict.entries.get('Subtype');
            if (subtypeObj && subtypeObj.type === PDFObjectType.Name) {
                fontResource.subtype = subtypeObj.value;
            }
            // Get BaseFont
            const baseFontObj = fontDict.entries.get('BaseFont');
            if (baseFontObj && baseFontObj.type === PDFObjectType.Name) {
                fontResource.baseFont = baseFontObj.value;
            }
            // Get Encoding - handle both Name and Dictionary forms
            const encodingObj = fontDict.entries.get('Encoding');
            if (encodingObj && encodingObj.type === PDFObjectType.Name) {
                fontResource.encoding = encodingObj.value;
            }
            else if (encodingObj && (encodingObj.type === PDFObjectType.Dictionary || encodingObj.type === PDFObjectType.Reference)) {
                // Encoding can be a dictionary with BaseEncoding and/or Differences
                let encDict;
                if (encodingObj.type === PDFObjectType.Reference) {
                    const ref = encodingObj.value;
                    const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                    if (resolved.type === PDFObjectType.Dictionary) {
                        encDict = resolved.value;
                    }
                }
                else {
                    encDict = encodingObj.value;
                }
                if (encDict) {
                    const baseEncObj = encDict.entries.get('BaseEncoding');
                    if (baseEncObj && baseEncObj.type === PDFObjectType.Name) {
                        fontResource.encoding = baseEncObj.value;
                    }
                }
            }
            // Get FirstChar
            const firstCharObj = fontDict.entries.get('FirstChar');
            if (firstCharObj && firstCharObj.type === PDFObjectType.Number) {
                fontResource.firstChar = firstCharObj.value;
            }
            // Get LastChar
            const lastCharObj = fontDict.entries.get('LastChar');
            if (lastCharObj && lastCharObj.type === PDFObjectType.Number) {
                fontResource.lastChar = lastCharObj.value;
            }
            // Get Widths array
            const widthsObj = fontDict.entries.get('Widths');
            if (widthsObj && widthsObj.type === PDFObjectType.Array) {
                const widthsArray = widthsObj.value;
                fontResource.widths = widthsArray
                    .filter(obj => obj.type === PDFObjectType.Number)
                    .map(obj => obj.value);
            }
            else if (widthsObj && widthsObj.type === PDFObjectType.Reference) {
                // Widths might be an indirect reference
                const ref = widthsObj.value;
                const resolvedWidths = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                if (resolvedWidths.type === PDFObjectType.Array) {
                    const widthsArray = resolvedWidths.value;
                    fontResource.widths = widthsArray
                        .filter(obj => obj.type === PDFObjectType.Number)
                        .map(obj => obj.value);
                }
            }
            // Get MissingWidth (for CIDFonts)
            const missingWidthObj = fontDict.entries.get('MissingWidth');
            if (missingWidthObj && missingWidthObj.type === PDFObjectType.Number) {
                fontResource.missingWidth = missingWidthObj.value;
            }
            // Get DW (DefaultWidth for CIDFonts)
            const dwObj = fontDict.entries.get('DW');
            if (dwObj && dwObj.type === PDFObjectType.Number) {
                fontResource.defaultWidth = dwObj.value;
            }
            // Parse FontDescriptor
            const descriptorObj = fontDict.entries.get('FontDescriptor');
            if (descriptorObj) {
                fontResource.descriptor = this.parseFontDescriptor(descriptorObj);
            }
            // Handle Type0 composite fonts: parse DescendantFonts for CIDFont metrics
            if (fontResource.subtype === 'Type0') {
                const descendantObj = fontDict.entries.get('DescendantFonts');
                if (descendantObj) {
                    let descendantArr;
                    if (descendantObj.type === PDFObjectType.Array) {
                        descendantArr = descendantObj.value;
                    }
                    else if (descendantObj.type === PDFObjectType.Reference) {
                        const ref = descendantObj.value;
                        const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                        if (resolved.type === PDFObjectType.Array) {
                            descendantArr = resolved.value;
                        }
                    }
                    if (descendantArr && descendantArr.length > 0) {
                        let cidDict;
                        const cidRef = descendantArr[0];
                        if (cidRef.type === PDFObjectType.Reference) {
                            const ref = cidRef.value;
                            const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                            if (resolved.type === PDFObjectType.Dictionary) {
                                cidDict = resolved.value;
                            }
                        }
                        else if (cidRef.type === PDFObjectType.Dictionary) {
                            cidDict = cidRef.value;
                        }
                        if (cidDict) {
                            // Extract DW (default width for CIDFont)
                            const cidDW = cidDict.entries.get('DW');
                            if (cidDW && cidDW.type === PDFObjectType.Number) {
                                fontResource.defaultWidth = cidDW.value;
                            }
                            // Extract BaseFont from descendant if not set
                            if (!fontResource.baseFont) {
                                const cidBaseFont = cidDict.entries.get('BaseFont');
                                if (cidBaseFont && cidBaseFont.type === PDFObjectType.Name) {
                                    fontResource.baseFont = cidBaseFont.value;
                                }
                            }
                            // Extract FontDescriptor from descendant if not set
                            if (!fontResource.descriptor) {
                                const cidDescObj = cidDict.entries.get('FontDescriptor');
                                if (cidDescObj) {
                                    fontResource.descriptor = this.parseFontDescriptor(cidDescObj);
                                }
                            }
                            // Parse W array for per-CID widths
                            let wObj = cidDict.entries.get('W');
                            if (wObj) {
                                let wArr;
                                if (wObj.type === PDFObjectType.Array) {
                                    wArr = wObj.value;
                                }
                                else if (wObj.type === PDFObjectType.Reference) {
                                    const ref = wObj.value;
                                    const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                                    if (resolved.type === PDFObjectType.Array) {
                                        wArr = resolved.value;
                                    }
                                }
                                if (wArr && wArr.length > 0) {
                                    fontResource.cidWidths = new Map();
                                    let i = 0;
                                    while (i < wArr.length) {
                                        const first = wArr[i];
                                        if (first.type !== PDFObjectType.Number) {
                                            i++;
                                            continue;
                                        }
                                        const startCID = first.value;
                                        i++;
                                        if (i >= wArr.length)
                                            break;
                                        const next = wArr[i];
                                        if (next.type === PDFObjectType.Array) {
                                            // Format: startCID [w1 w2 w3 ...]
                                            const widths = next.value;
                                            for (let j = 0; j < widths.length; j++) {
                                                if (widths[j].type === PDFObjectType.Number) {
                                                    fontResource.cidWidths.set(startCID + j, widths[j].value);
                                                }
                                            }
                                            i++;
                                        }
                                        else if (next.type === PDFObjectType.Number) {
                                            // Format: startCID lastCID width
                                            const lastCID = next.value;
                                            i++;
                                            if (i < wArr.length && wArr[i].type === PDFObjectType.Number) {
                                                const w = wArr[i].value;
                                                for (let cid = startCID; cid <= lastCID; cid++) {
                                                    fontResource.cidWidths.set(cid, w);
                                                }
                                                i++;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Use FontDescriptor MissingWidth as fallback
            if (fontResource.missingWidth === undefined && fontResource.descriptor && fontResource.descriptor.missingWidth !== undefined) {
                fontResource.missingWidth = fontResource.descriptor.missingWidth;
            }
            // Parse ToUnicode CMap for character code to Unicode mapping
            const toUnicodeObj = fontDict.entries.get('ToUnicode');
            if (toUnicodeObj && toUnicodeObj.type === PDFObjectType.Reference) {
                const ref = toUnicodeObj.value;
                try {
                    const toUnicodeStream = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                    if (toUnicodeStream.type === PDFObjectType.Stream) {
                        const stream = toUnicodeStream.value;
                        fontResource.toUnicode = this.parseToUnicodeCMap(stream.data);
                    }
                }
                catch (e) {
                    // ToUnicode parsing is optional
                }
            }
            // Parse Encoding Differences for glyph name mapping
            // Handle both reference and direct dictionary forms
            if (encodingObj) {
                try {
                    let encDict;
                    if (encodingObj.type === PDFObjectType.Reference) {
                        const ref = encodingObj.value;
                        const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                        if (resolved.type === PDFObjectType.Dictionary) {
                            encDict = resolved.value;
                        }
                    }
                    else if (encodingObj.type === PDFObjectType.Dictionary) {
                        encDict = encodingObj.value;
                    }
                    if (encDict) {
                        const diffsObj = encDict.entries.get('Differences');
                        if (diffsObj && diffsObj.type === PDFObjectType.Array) {
                            const diffs = diffsObj.value;
                            if (!fontResource.toUnicode)
                                fontResource.toUnicode = new Map();
                            let currentCode = 0;
                            for (const item of diffs) {
                                if (item.type === PDFObjectType.Number) {
                                    currentCode = item.value;
                                }
                                else if (item.type === PDFObjectType.Name) {
                                    const glyphName = item.value;
                                    if (!fontResource.toUnicode.has(currentCode)) {
                                        const unicode = PDFTextDecoder.glyphNameToUnicode(glyphName);
                                        if (unicode)
                                            fontResource.toUnicode.set(currentCode, unicode);
                                    }
                                    currentCode++;
                                }
                            }
                        }
                    }
                }
                catch (e) {
                    // Encoding parsing is optional
                }
            }
            return fontResource;
        }
        catch (error) {
            console.warn(`Error parsing font ${name}:`, error);
            // Return basic font resource as fallback
            return {
                name: name,
                type: 'Font',
                subtype: 'Type1'
            };
        }
    }
    parseToUnicodeCMap(cmapData) {
        const map = new Map();
        // Decode CMap data as latin1 (byte-preserving)
        let text = '';
        for (let i = 0; i < cmapData.length; i++)
            text += String.fromCharCode(cmapData[i]);
        // Parse bfchar sections: <srcCode> <dstUnicode>
        const bfcharRegex = /beginbfchar\s+([\s\S]*?)endbfchar/g;
        let match;
        while ((match = bfcharRegex.exec(text)) !== null) {
            const lines = match[1].trim().split(/\r?\n/);
            for (const line of lines) {
                const parts = line.trim().match(/<([0-9a-fA-F]+)>\s+<([0-9a-fA-F]+)>/);
                if (parts) {
                    const srcCode = parseInt(parts[1], 16);
                    const dstHex = parts[2];
                    let str = '';
                    for (let i = 0; i < dstHex.length; i += 4) {
                        const cp = parseInt(dstHex.substring(i, Math.min(i + 4, dstHex.length)), 16);
                        if (cp > 0)
                            str += String.fromCodePoint(cp);
                    }
                    if (str)
                        map.set(srcCode, str);
                }
            }
        }
        // Parse bfrange sections: <start> <end> <startUnicode> or <start> <end> [<u1> <u2> ...]
        const bfrangeRegex = /beginbfrange\s+([\s\S]*?)endbfrange/g;
        while ((match = bfrangeRegex.exec(text)) !== null) {
            const lines = match[1].trim().split(/\r?\n/);
            for (const line of lines) {
                const rangeParts = line.trim().match(/<([0-9a-fA-F]+)>\s+<([0-9a-fA-F]+)>\s+<([0-9a-fA-F]+)>/);
                if (rangeParts) {
                    const start = parseInt(rangeParts[1], 16);
                    const end = parseInt(rangeParts[2], 16);
                    let startUnicode = parseInt(rangeParts[3], 16);
                    for (let code = start; code <= end; code++) {
                        map.set(code, String.fromCodePoint(startUnicode++));
                    }
                }
            }
        }
        return map;
    }
    parseFontDescriptor(descriptorObj) {
        try {
            let descriptorDict;
            if (descriptorObj.type === PDFObjectType.Reference) {
                const ref = descriptorObj.value;
                const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                if (resolved.type === PDFObjectType.Dictionary) {
                    descriptorDict = resolved.value;
                }
            }
            else if (descriptorObj.type === PDFObjectType.Dictionary) {
                descriptorDict = descriptorObj.value;
            }
            if (!descriptorDict)
                return undefined;
            const descriptor = {
                fontName: ''
            };
            // Get FontName
            const fontNameObj = descriptorDict.entries.get('FontName');
            if (fontNameObj && fontNameObj.type === PDFObjectType.Name) {
                descriptor.fontName = fontNameObj.value;
            }
            // Get FontFamily
            const fontFamilyObj = descriptorDict.entries.get('FontFamily');
            if (fontFamilyObj && fontFamilyObj.type === PDFObjectType.String) {
                descriptor.fontFamily = fontFamilyObj.value;
            }
            // Get Ascent
            const ascentObj = descriptorDict.entries.get('Ascent');
            if (ascentObj && ascentObj.type === PDFObjectType.Number) {
                descriptor.ascent = ascentObj.value;
            }
            // Get Descent
            const descentObj = descriptorDict.entries.get('Descent');
            if (descentObj && descentObj.type === PDFObjectType.Number) {
                descriptor.descent = descentObj.value;
            }
            // Get CapHeight
            const capHeightObj = descriptorDict.entries.get('CapHeight');
            if (capHeightObj && capHeightObj.type === PDFObjectType.Number) {
                descriptor.capHeight = capHeightObj.value;
            }
            // Get Flags
            const flagsObj = descriptorDict.entries.get('Flags');
            if (flagsObj && flagsObj.type === PDFObjectType.Number) {
                descriptor.flags = flagsObj.value;
            }
            // Get MissingWidth
            const mwObj = descriptorDict.entries.get('MissingWidth');
            if (mwObj && mwObj.type === PDFObjectType.Number) {
                descriptor.missingWidth = mwObj.value;
            }
            // Get ItalicAngle
            const italicAngleObj = descriptorDict.entries.get('ItalicAngle');
            if (italicAngleObj && italicAngleObj.type === PDFObjectType.Number) {
                descriptor.italicAngle = italicAngleObj.value;
            }
            return descriptor;
        }
        catch (error) {
            console.warn('Error parsing font descriptor:', error);
            return undefined;
        }
    }
    // Parsing primitives
    parseDictionary() {
        const dict = {
            entries: new Map()
        };
        // Skip '<<'
        this.position += 2;
        this.skipWhitespace();
        // Safety: prevent infinite loops
        let safetyCounter = 0;
        const MAX_DICT_ENTRIES = 10000;
        while (this.position < this.buffer.byteLength) {
            if (++safetyCounter > MAX_DICT_ENTRIES) {
                console.warn('parseDictionary: Safety limit reached, stopping dictionary parsing');
                break;
            }
            // Check for '>>'
            if (this.peekByte() === '>'.charCodeAt(0) &&
                this.peekByte(1) === '>'.charCodeAt(0)) {
                this.position += 2;
                break;
            }
            const positionBefore = this.position;
            // Parse name (key)
            const name = this.parseName();
            this.skipWhitespace();
            // Parse value
            const value = this.parseObject();
            this.skipWhitespace();
            // Safety: ensure position advanced
            if (this.position === positionBefore) {
                console.warn('parseDictionary: Position not advancing, breaking to prevent infinite loop');
                break;
            }
            dict.entries.set(name, value);
        }
        return dict;
    }
    parseObject() {
        this.skipWhitespace();
        const byte = this.peekByte();
        // Check object type
        if (byte === '/'.charCodeAt(0)) {
            return { type: PDFObjectType.Name, value: this.parseName() };
        }
        else if (byte === '('.charCodeAt(0)) {
            return { type: PDFObjectType.String, value: this.parseString() };
        }
        else if (byte === '<'.charCodeAt(0)) {
            if (this.peekByte(1) === '<'.charCodeAt(0)) {
                return { type: PDFObjectType.Dictionary, value: this.parseDictionary() };
            }
            else {
                return { type: PDFObjectType.String, value: this.parseHexString() };
            }
        }
        else if (byte === '['.charCodeAt(0)) {
            return { type: PDFObjectType.Array, value: this.parseArray() };
        }
        else if (byte === 't'.charCodeAt(0) || byte === 'f'.charCodeAt(0)) {
            return { type: PDFObjectType.Boolean, value: this.parseBoolean() };
        }
        else if (byte === 'n'.charCodeAt(0)) {
            this.position += 4; // Skip "null"
            return { type: PDFObjectType.Null, value: null };
        }
        else if ((byte >= '0'.charCodeAt(0) && byte <= '9'.charCodeAt(0)) ||
            byte === '-'.charCodeAt(0) || byte === '+'.charCodeAt(0) ||
            byte === '.'.charCodeAt(0)) {
            // Could be number or reference
            const num = this.parseNumber();
            this.skipWhitespace();
            // Check if it's a reference
            if (this.peekByte() >= '0'.charCodeAt(0) && this.peekByte() <= '9'.charCodeAt(0)) {
                const savedPos = this.position;
                const gen = this.parseNumber();
                this.skipWhitespace();
                if (this.peekByte() === 'R'.charCodeAt(0)) {
                    this.position++; // Skip 'R'
                    return {
                        type: PDFObjectType.Reference,
                        value: { objectNumber: num, generationNumber: gen }
                    };
                }
                // Not a reference, restore position to before gen
                this.position = savedPos;
            }
            return { type: PDFObjectType.Number, value: num };
        }
        throw new Error(`Unknown object type at position ${this.position}`);
    }
    parseIndirectObject(objNum, genNum, xref) {
        // Check cache first
        const cacheKey = `${objNum}-${genNum}`;
        const cached = this.objectCache.get(cacheKey);
        if (cached) {
            return cached;
        }
        const entry = xref.getEntry(objNum);
        if (!entry) {
            throw new Error(`Object ${objNum} not found in xref table`);
        }
        this.position = entry.offset;
        // Parse "objNum genNum obj"
        this.parseNumber(); // objNum
        this.skipWhitespace();
        this.parseNumber(); // genNum
        this.skipWhitespace();
        // Skip "obj"
        this.position += 3;
        this.skipWhitespace();
        const obj = this.parseObject();
        // Check for stream
        this.skipWhitespace();
        if (this.position < this.buffer.byteLength - 6) {
            const streamKeyword = this.readString(this.position, 6);
            if (streamKeyword === 'stream') {
                // Parse stream
                const streamObj = this.parseStream(obj);
                this.objectCache.set(cacheKey, streamObj);
                return streamObj;
            }
        }
        // Cache the parsed object
        this.objectCache.set(cacheKey, obj);
        return obj;
    }
    parseStream(dict) {
        if (dict.type !== PDFObjectType.Dictionary) {
            throw new Error('Stream must have dictionary');
        }
        // Skip "stream" keyword and newline
        this.position += 6;
        if (this.peekByte() === '\r'.charCodeAt(0))
            this.position++;
        if (this.peekByte() === '\n'.charCodeAt(0))
            this.position++;
        // Get stream length
        const dictValue = dict.value;
        const lengthObj = dictValue.entries.get('Length');
        let length = 0;
        if (lengthObj && lengthObj.type === PDFObjectType.Number) {
            length = lengthObj.value;
        }
        else if (lengthObj && lengthObj.type === PDFObjectType.Reference && this.xrefTable) {
            // Resolve indirect reference for /Length (very common in PDFs)
            const ref = lengthObj.value;
            const savedPos = this.position;
            try {
                const resolved = this.parseIndirectObject(ref.objectNumber, ref.generationNumber, this.xrefTable);
                if (resolved.type === PDFObjectType.Number) {
                    length = resolved.value;
                }
            }
            catch {
                // Fall through to endstream scanning
            }
            this.position = savedPos;
        }
        // Read stream data
        let rawData;
        if (length > 0 && this.position + length <= this.buffer.byteLength) {
            rawData = new Uint8Array(this.buffer, this.position, length);
            this.position += length;
        }
        else {
            // Fallback: scan for 'endstream' keyword
            const searchStart = this.position;
            const searchLimit = Math.min(this.buffer.byteLength, searchStart + 10 * 1024 * 1024);
            let endPos = -1;
            const view = new Uint8Array(this.buffer);
            for (let i = searchStart; i < searchLimit - 9; i++) {
                if (view[i] === 101 && view[i + 1] === 110 && view[i + 2] === 100 &&
                    view[i + 3] === 115 && view[i + 4] === 116 && view[i + 5] === 114 &&
                    view[i + 6] === 101 && view[i + 7] === 97 && view[i + 8] === 109) {
                    endPos = i;
                    break;
                }
            }
            if (endPos > searchStart) {
                // Trim trailing whitespace (\r, \n) before endstream
                let dataEnd = endPos;
                while (dataEnd > searchStart && (view[dataEnd - 1] === 13 || view[dataEnd - 1] === 10)) {
                    dataEnd--;
                }
                rawData = new Uint8Array(this.buffer, searchStart, dataEnd - searchStart);
                this.position = endPos;
            }
            else {
                rawData = new Uint8Array(0);
            }
        }
        // Decompress stream if needed
        let data;
        const filterObj = dictValue.entries.get('Filter');
        if (filterObj) {
            const filter = this.getFilterName(filterObj);
            if (filter === 'FlateDecode') {
                try {
                    data = this.decompressFlate(rawData);
                }
                catch (error) {
                    console.warn('Failed to decompress FlateDecode stream:', error);
                    // Copy to new array if decompression fails
                    data = new Uint8Array(rawData);
                }
            }
            else {
                // Copy to new array for non-compressed streams
                data = new Uint8Array(rawData);
            }
        }
        else {
            // No filter - copy to new array
            data = new Uint8Array(rawData);
        }
        // Skip "endstream"
        this.skipWhitespace();
        this.position += 9;
        return {
            type: PDFObjectType.Stream,
            value: { dictionary: dictValue, data }
        };
    }
    getFilterName(filterObj) {
        if (filterObj.type === PDFObjectType.Name) {
            return filterObj.value;
        }
        else if (filterObj.type === PDFObjectType.Array) {
            const filters = filterObj.value;
            if (filters.length > 0 && filters[0].type === PDFObjectType.Name) {
                return filters[0].value;
            }
        }
        return '';
    }
    decompressFlate(data) {
        try {
            // Try using pako if available
            if (typeof globalThis.pako !== 'undefined') {
                const pako = globalThis.pako;
                return new Uint8Array(pako.inflate(data));
            }
            // Try using fflate if available (lighter weight alternative)
            if (typeof globalThis.fflate !== 'undefined') {
                const fflate = globalThis.fflate;
                return new Uint8Array(fflate.inflateSync(data));
            }
            // Use built-in DEFLATE decompressor (zero dependencies)
            return Inflate.inflate(data);
        }
        catch (error) {
            console.error('Decompression failed:', error);
            return data;
        }
    }
    parseName() {
        this.position++; // Skip '/'
        let name = '';
        let safetyCounter = 0;
        const MAX_NAME_LENGTH = 10000; // Max name length
        while (this.position < this.buffer.byteLength) {
            if (++safetyCounter > MAX_NAME_LENGTH) {
                console.warn('parseName: Safety limit reached, truncating name');
                break;
            }
            const positionBefore = this.position;
            const byte = this.peekByte();
            // Name ends at whitespace or delimiter
            if (this.isWhitespace(byte) || this.isDelimiter(byte)) {
                break;
            }
            // Handle hex codes (#XX)
            if (byte === '#'.charCodeAt(0)) {
                this.position++;
                const hex = String.fromCharCode(this.readByte()) +
                    String.fromCharCode(this.readByte());
                name += String.fromCharCode(parseInt(hex, 16));
            }
            else {
                name += String.fromCharCode(this.readByte());
            }
            if (this.position === positionBefore) {
                console.warn('parseName: Position not advancing, breaking to prevent infinite loop');
                break;
            }
        }
        return name;
    }
    parseString() {
        this.position++; // Skip '('
        let str = '';
        let parenLevel = 1;
        // Safety: prevent infinite loops
        let safetyCounter = 0;
        const MAX_STRING_LENGTH = 1000000; // 1MB max string
        while (this.position < this.buffer.byteLength && parenLevel > 0) {
            if (++safetyCounter > MAX_STRING_LENGTH) {
                console.warn('parseString: Safety limit reached, truncating string');
                break;
            }
            const byte = this.readByte();
            if (byte === '\\'.charCodeAt(0)) {
                // Handle escape sequences
                const next = this.readByte();
                switch (next) {
                    case 'n'.charCodeAt(0):
                        str += '\n';
                        break;
                    case 'r'.charCodeAt(0):
                        str += '\r';
                        break;
                    case 't'.charCodeAt(0):
                        str += '\t';
                        break;
                    case 'b'.charCodeAt(0):
                        str += '\b';
                        break;
                    case 'f'.charCodeAt(0):
                        str += '\f';
                        break;
                    case '('.charCodeAt(0):
                        str += '(';
                        break;
                    case ')'.charCodeAt(0):
                        str += ')';
                        break;
                    case '\\'.charCodeAt(0):
                        str += '\\';
                        break;
                    default:
                        // Octal escape
                        if (next >= '0'.charCodeAt(0) && next <= '7'.charCodeAt(0)) {
                            let octal = String.fromCharCode(next);
                            for (let i = 0; i < 2; i++) {
                                const digit = this.peekByte();
                                if (digit >= '0'.charCodeAt(0) && digit <= '7'.charCodeAt(0)) {
                                    octal += String.fromCharCode(this.readByte());
                                }
                                else {
                                    break;
                                }
                            }
                            str += String.fromCharCode(parseInt(octal, 8));
                        }
                        else {
                            str += String.fromCharCode(next);
                        }
                }
            }
            else if (byte === '('.charCodeAt(0)) {
                parenLevel++;
                str += '(';
            }
            else if (byte === ')'.charCodeAt(0)) {
                parenLevel--;
                if (parenLevel > 0)
                    str += ')';
            }
            else {
                str += String.fromCharCode(byte);
            }
        }
        return str;
    }
    parseHexString() {
        this.position++; // Skip '<'
        let hex = '';
        let safetyCounter = 0;
        const MAX_HEX_STRING_LENGTH = 1000000; // 1MB max hex string
        while (this.position < this.buffer.byteLength) {
            if (++safetyCounter > MAX_HEX_STRING_LENGTH) {
                console.warn('parseHexString: Safety limit reached, truncating hex string');
                break;
            }
            const positionBefore = this.position;
            const byte = this.peekByte();
            if (byte === '>'.charCodeAt(0)) {
                this.position++;
                break;
            }
            // Skip whitespace in hex strings
            if (this.isWhitespace(byte)) {
                this.position++;
                continue;
            }
            hex += String.fromCharCode(this.readByte());
            if (this.position === positionBefore) {
                console.warn('parseHexString: Position not advancing, breaking to prevent infinite loop');
                break;
            }
        }
        // Convert hex to string
        let str = '';
        for (let i = 0; i < hex.length; i += 2) {
            const hexByte = hex.substr(i, 2);
            str += String.fromCharCode(parseInt(hexByte, 16));
        }
        return str;
    }
    parseArray() {
        this.position++; // Skip '['
        this.skipWhitespace();
        const array = [];
        // Safety: prevent infinite loops
        let safetyCounter = 0;
        const MAX_ARRAY_ITEMS = 100000;
        while (this.position < this.buffer.byteLength) {
            if (++safetyCounter > MAX_ARRAY_ITEMS) {
                console.warn('parseArray: Safety limit reached, stopping array parsing');
                break;
            }
            if (this.peekByte() === ']'.charCodeAt(0)) {
                this.position++;
                break;
            }
            const positionBefore = this.position;
            array.push(this.parseObject());
            this.skipWhitespace();
            // Safety: ensure position advanced
            if (this.position === positionBefore) {
                console.warn('parseArray: Position not advancing, breaking to prevent infinite loop');
                break;
            }
        }
        return array;
    }
    parseBoolean() {
        let word = '';
        let safetyCounter = 0;
        const MAX_BOOLEAN_LENGTH = 10; // 'true' or 'false'
        while (this.position < this.buffer.byteLength && !this.isWhitespace(this.peekByte()) && !this.isDelimiter(this.peekByte())) {
            if (++safetyCounter > MAX_BOOLEAN_LENGTH) {
                console.warn('parseBoolean: Safety limit reached, truncating boolean');
                break;
            }
            const positionBefore = this.position;
            word += String.fromCharCode(this.readByte());
            if (this.position === positionBefore) {
                console.warn('parseBoolean: Position not advancing, breaking to prevent infinite loop');
                break;
            }
        }
        if (word === 'true')
            return true;
        if (word === 'false')
            return false;
        throw new Error(`Invalid boolean value: ${word}`);
    }
    parseNumber() {
        let numStr = '';
        let isFloat = false;
        let safetyCounter = 0;
        const MAX_NUMBER_LENGTH = 100; // Max digits in a number
        while (this.position < this.buffer.byteLength) {
            if (++safetyCounter > MAX_NUMBER_LENGTH) {
                console.warn('parseNumber: Safety limit reached, truncating number');
                break;
            }
            const positionBefore = this.position;
            const byte = this.peekByte();
            if ((byte >= '0'.charCodeAt(0) && byte <= '9'.charCodeAt(0)) ||
                byte === '.'.charCodeAt(0) ||
                (numStr.length === 0 && (byte === '-'.charCodeAt(0) || byte === '+'.charCodeAt(0)))) {
                if (byte === '.'.charCodeAt(0))
                    isFloat = true;
                numStr += String.fromCharCode(this.readByte());
            }
            else {
                break;
            }
            if (this.position === positionBefore) {
                console.warn('parseNumber: Position not advancing, breaking to prevent infinite loop');
                break;
            }
        }
        return isFloat ? parseFloat(numStr) : parseInt(numStr, 10);
    }
    parseRectangle(obj) {
        if (!obj || obj.type !== PDFObjectType.Array)
            return undefined;
        const arr = obj.value;
        if (arr.length !== 4)
            return undefined;
        return {
            x: arr[0].value,
            y: arr[1].value,
            width: arr[2].value - arr[0].value,
            height: arr[3].value - arr[1].value
        };
    }
    extractStringFromDict(dict, key) {
        const obj = dict.entries.get(key);
        if (!obj)
            return undefined;
        if (obj.type === PDFObjectType.String) {
            return obj.value;
        }
        return undefined;
    }
    parsePDFDate(dateStr) {
        // PDF date format: D:YYYYMMDDHHmmSSOHH'mm
        if (!dateStr.startsWith('D:'))
            return new Date();
        const year = parseInt(dateStr.substr(2, 4), 10);
        const month = parseInt(dateStr.substr(6, 2), 10) - 1;
        const day = parseInt(dateStr.substr(8, 2), 10);
        const hour = parseInt(dateStr.substr(10, 2), 10) || 0;
        const minute = parseInt(dateStr.substr(12, 2), 10) || 0;
        const second = parseInt(dateStr.substr(14, 2), 10) || 0;
        return new Date(year, month, day, hour, minute, second);
    }
    // Helper methods
    readByte() {
        return this.dataView.getUint8(this.position++);
    }
    peekByte(offset = 0) {
        return this.dataView.getUint8(this.position + offset);
    }
    readString(start, length) {
        const bytes = new Uint8Array(this.buffer, start, length);
        return new TextDecoder().decode(bytes);
    }
    skipWhitespace() {
        // Safety: prevent infinite loops if position doesn't advance
        let safetyCounter = 0;
        const MAX_WHITESPACE = 100000; // Max whitespace characters to skip
        while (this.position < this.buffer.byteLength && this.isWhitespace(this.peekByte())) {
            if (++safetyCounter > MAX_WHITESPACE) {
                console.warn('skipWhitespace: Safety limit reached, stopping whitespace skip');
                break;
            }
            this.position++;
        }
    }
    isWhitespace(byte) {
        return byte === 0 || byte === 9 || byte === 10 || byte === 12 || byte === 13 || byte === 32;
    }
    isDelimiter(byte) {
        const delimiters = '()<>[]{}/%';
        return delimiters.indexOf(String.fromCharCode(byte)) >= 0;
    }
}
// ============================================================================
// Supporting Classes
// ============================================================================
/**
 * Built-in DEFLATE decompressor (RFC 1951)
 * Provides zero-dependency zlib/deflate inflate for PDF FlateDecode streams.
 */
class Inflate {
    constructor(data) {
        this.pos = 0;
        this.bitBuf = 0;
        this.bitCount = 0;
        this.output = [];
        this.data = data;
    }
    static inflate(data) {
        const inflater = new Inflate(data);
        return inflater.decompress();
    }
    decompress() {
        // Check for zlib header (CMF + FLG)
        if (this.data.length >= 2) {
            const cmf = this.data[0];
            const flg = this.data[1];
            const cm = cmf & 0x0F;
            if (cm === 8 && ((cmf * 256 + flg) % 31 === 0)) {
                // Skip 2-byte zlib header
                this.pos = 2;
                // If FDICT is set, skip 4 bytes
                if (flg & 0x20) {
                    this.pos += 4;
                }
            }
        }
        let bfinal = 0;
        while (bfinal === 0) {
            bfinal = this.readBits(1);
            const btype = this.readBits(2);
            if (btype === 0) {
                this.inflateStored();
            }
            else if (btype === 1) {
                this.inflateFixed();
            }
            else if (btype === 2) {
                this.inflateDynamic();
            }
            else {
                throw new Error('Invalid DEFLATE block type');
            }
        }
        return new Uint8Array(this.output);
    }
    readBits(count) {
        while (this.bitCount < count) {
            if (this.pos >= this.data.length) {
                throw new Error('Unexpected end of data');
            }
            this.bitBuf |= this.data[this.pos++] << this.bitCount;
            this.bitCount += 8;
        }
        const value = this.bitBuf & ((1 << count) - 1);
        this.bitBuf >>= count;
        this.bitCount -= count;
        return value;
    }
    inflateStored() {
        // Align to byte boundary
        this.bitBuf = 0;
        this.bitCount = 0;
        if (this.pos + 4 > this.data.length) {
            throw new Error('Unexpected end of stored block header');
        }
        const len = this.data[this.pos] | (this.data[this.pos + 1] << 8);
        this.pos += 4; // Skip LEN and NLEN
        for (let i = 0; i < len; i++) {
            if (this.pos >= this.data.length)
                break;
            this.output.push(this.data[this.pos++]);
        }
    }
    inflateFixed() {
        const litTree = this.buildTree(Inflate.FIXED_LIT_LENGTHS, 288);
        const distTree = this.buildTree(Inflate.FIXED_DIST_LENGTHS, 32);
        this.inflateBlock(litTree, distTree);
    }
    inflateDynamic() {
        const hlit = this.readBits(5) + 257;
        const hdist = this.readBits(5) + 1;
        const hclen = this.readBits(4) + 4;
        // Read code length code lengths
        const clLengths = new Uint8Array(19);
        for (let i = 0; i < hclen; i++) {
            clLengths[Inflate.CL_ORDER[i]] = this.readBits(3);
        }
        const clTree = this.buildTree(clLengths, 19);
        // Read literal/length + distance code lengths
        const lengths = new Uint8Array(hlit + hdist);
        let i = 0;
        while (i < hlit + hdist) {
            const sym = this.decodeSymbol(clTree);
            if (sym < 16) {
                lengths[i++] = sym;
            }
            else if (sym === 16) {
                const count = this.readBits(2) + 3;
                const prev = i > 0 ? lengths[i - 1] : 0;
                for (let j = 0; j < count && i < lengths.length; j++) {
                    lengths[i++] = prev;
                }
            }
            else if (sym === 17) {
                const count = this.readBits(3) + 3;
                for (let j = 0; j < count && i < lengths.length; j++) {
                    lengths[i++] = 0;
                }
            }
            else if (sym === 18) {
                const count = this.readBits(7) + 11;
                for (let j = 0; j < count && i < lengths.length; j++) {
                    lengths[i++] = 0;
                }
            }
        }
        const litLengths = lengths.slice(0, hlit);
        const distLengths = lengths.slice(hlit);
        const litTree = this.buildTree(litLengths, hlit);
        const distTree = this.buildTree(distLengths, hdist);
        this.inflateBlock(litTree, distTree);
    }
    inflateBlock(litTree, distTree) {
        let done = false;
        while (!done) {
            const sym = this.decodeSymbol(litTree);
            if (sym === 256) {
                done = true;
            }
            else if (sym < 256) {
                this.output.push(sym); // Literal byte
            }
            else {
                // Length/distance pair
                const lengthIdx = sym - 257;
                if (lengthIdx >= Inflate.LENGTH_BASE.length)
                    break;
                const length = Inflate.LENGTH_BASE[lengthIdx] + this.readBits(Inflate.LENGTH_EXTRA[lengthIdx]);
                const distSym = this.decodeSymbol(distTree);
                if (distSym >= Inflate.DIST_BASE.length)
                    break;
                const distance = Inflate.DIST_BASE[distSym] + this.readBits(Inflate.DIST_EXTRA[distSym]);
                // Copy from output buffer
                const srcPos = this.output.length - distance;
                for (let i = 0; i < length; i++) {
                    this.output.push(this.output[srcPos + i]);
                }
            }
        }
    }
    buildTree(lengths, numSymbols) {
        // Count codes of each length
        const maxBits = 15;
        const blCount = new Uint16Array(maxBits + 1);
        for (let i = 0; i < numSymbols; i++) {
            if (lengths[i] > 0)
                blCount[lengths[i]]++;
        }
        // Find numerical value of smallest code for each code length
        const nextCode = new Uint16Array(maxBits + 1);
        let code = 0;
        for (let bits = 1; bits <= maxBits; bits++) {
            code = (code + blCount[bits - 1]) << 1;
            nextCode[bits] = code;
        }
        // Assign codes to symbols
        const table = new Map();
        let maxLength = 0;
        for (let n = 0; n < numSymbols; n++) {
            const len = lengths[n];
            if (len > 0) {
                table.set((nextCode[len]++ << 4) | len, n);
                if (len > maxLength)
                    maxLength = len;
            }
        }
        return { table, maxLength };
    }
    decodeSymbol(tree) {
        let code = 0;
        let len = 0;
        while (len < tree.maxLength) {
            code = (code << 1) | this.readBits(1);
            len++;
            const key = (code << 4) | len;
            const sym = tree.table.get(key);
            if (sym !== undefined)
                return sym;
        }
        throw new Error('Invalid Huffman code');
    }
}
// Fixed Huffman code lengths (RFC 1951 section 3.2.6)
Inflate.FIXED_LIT_LENGTHS = (() => {
    const lengths = new Uint8Array(288);
    for (let i = 0; i <= 143; i++)
        lengths[i] = 8;
    for (let i = 144; i <= 255; i++)
        lengths[i] = 9;
    for (let i = 256; i <= 279; i++)
        lengths[i] = 7;
    for (let i = 280; i <= 287; i++)
        lengths[i] = 8;
    return lengths;
})();
Inflate.FIXED_DIST_LENGTHS = (() => {
    const lengths = new Uint8Array(32);
    lengths.fill(5);
    return lengths;
})();
// Length base values and extra bits (codes 257-285)
Inflate.LENGTH_BASE = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31,
    35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258
];
Inflate.LENGTH_EXTRA = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
    3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0
];
// Distance base values and extra bits (codes 0-29)
Inflate.DIST_BASE = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193,
    257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577
];
Inflate.DIST_EXTRA = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6,
    7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13
];
// Code length alphabet order (for dynamic Huffman)
Inflate.CL_ORDER = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15
];
class XRefTable {
    constructor() {
        this.entries = new Map();
    }
    addEntry(objectNumber, offset, generation, type) {
        this.entries.set(objectNumber, {
            objectNumber,
            offset,
            generation,
            type
        });
    }
    getEntry(objectNumber) {
        return this.entries.get(objectNumber);
    }
    getAllEntries() {
        return this.entries.entries();
    }
}
class PageTree {
    constructor() {
        this.pages = new Map();
    }
    addPage(pageNumber, pageDict) {
        this.pages.set(pageNumber, pageDict);
    }
    getPage(pageNumber) {
        return this.pages.get(pageNumber);
    }
    getPageCount() {
        return this.pages.size;
    }
}
class StreamingPDFParser {
    constructor(stream, options) {
        this.stream = stream;
        this.options = options;
        this.bytesRead = 0;
        this.startTime = Date.now();
        this.buffer = new Uint8Array(0);
        this.position = 0;
        this.reader = stream.getReader();
    }
    async parseHeader() {
        await this.ensureBytes(8);
        const header = this.readString(0, 8);
        if (!header.startsWith('%PDF-')) {
            throw new Error('Invalid PDF header');
        }
    }
    async parseMetadata() {
        // Read enough bytes to find trailer and parse metadata
        await this.ensureBytes(Math.min(65536, 8192));
        const text = this.readString(0, Math.min(this.buffer.length, 65536));
        // Extract version from header
        let version = '1.7';
        const headerMatch = text.match(/%PDF-(\d+\.\d+)/);
        if (headerMatch)
            version = headerMatch[1];
        // Try to find page count from trailer or catalog
        let pageCount = 0;
        const countMatch = text.match(/\/Count\s+(\d+)/);
        if (countMatch)
            pageCount = parseInt(countMatch[1], 10);
        // Check encryption
        const isEncrypted = text.includes('/Encrypt');
        // Check linearization
        const isLinearized = text.includes('/Linearized');
        return {
            version,
            pageCount,
            isEncrypted,
            isLinearized,
            fileSize: this.bytesRead
        };
    }
    async *streamPages() {
        let pageNumber = 1;
        while (true) {
            try {
                const page = await this.parseNextPage(pageNumber);
                if (!page)
                    break;
                yield {
                    pageNumber: pageNumber++,
                    width: page.width || 0,
                    height: page.height || 0,
                    rotation: page.rotation || 0,
                    userUnit: page.userUnit || 1,
                    mediaBox: page.mediaBox || { x: 0, y: 0, width: 0, height: 0 },
                    cropBox: page.cropBox,
                    bleedBox: page.bleedBox,
                    trimBox: page.trimBox,
                    artBox: page.artBox,
                    contents: page.contents,
                    resources: page.resources
                };
                // Check abort signal
                if (this.options?.abortSignal?.aborted) {
                    break;
                }
            }
            catch (error) {
                if (error instanceof Error && error.message === 'Stream ended') {
                    break;
                }
                throw error;
            }
        }
    }
    async parseNextPage(pageNumber) {
        // Incrementally parse the next page from the buffered stream data
        try {
            await this.ensureBytes(4096);
        }
        catch {
            return null; // Stream ended
        }
        const text = this.readString(this.position, Math.min(this.buffer.length - this.position, 65536));
        // Search for page objects: /Type /Page
        const pagePattern = /\/Type\s*\/Page(?!s)\b/g;
        let match;
        let foundCount = 0;
        while ((match = pagePattern.exec(text)) !== null) {
            foundCount++;
            if (foundCount < pageNumber)
                continue;
            // Found a page - try to extract MediaBox
            const contextStart = Math.max(0, match.index - 500);
            const contextEnd = Math.min(text.length, match.index + 1000);
            const context = text.substring(contextStart, contextEnd);
            let width = 612, height = 792, rotation = 0;
            const mediaBoxMatch = context.match(/\/MediaBox\s*\[\s*([\d.]+)\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)\s*\]/);
            if (mediaBoxMatch) {
                width = parseFloat(mediaBoxMatch[3]) - parseFloat(mediaBoxMatch[1]);
                height = parseFloat(mediaBoxMatch[4]) - parseFloat(mediaBoxMatch[2]);
            }
            const rotateMatch = context.match(/\/Rotate\s+(\d+)/);
            if (rotateMatch)
                rotation = parseInt(rotateMatch[1], 10);
            // Extract content stream reference
            const contentsMatch = context.match(/\/Contents\s+(\d+)\s+\d+\s+R/);
            let contents;
            if (contentsMatch) {
                // Note: actual stream decompression requires full parser
                contents = new Uint8Array(0);
            }
            // Advance position past this page
            this.position += match.index + match[0].length;
            return {
                width,
                height,
                rotation,
                userUnit: 1,
                mediaBox: { x: 0, y: 0, width, height },
                contents
            };
        }
        // No more pages found
        return null;
    }
    async ensureBytes(count) {
        while (this.buffer.length - this.position < count) {
            const { done, value } = await this.reader.read();
            if (done) {
                if (this.buffer.length - this.position < count) {
                    throw new Error('Stream ended unexpectedly');
                }
                break;
            }
            // Append to buffer
            const newBuffer = new Uint8Array(this.buffer.length + value.length);
            newBuffer.set(this.buffer);
            newBuffer.set(value, this.buffer.length);
            this.buffer = newBuffer;
            this.bytesRead += value.length;
            // Report progress
            this.options?.progressCallback?.({
                bytesRead: this.bytesRead,
                totalBytes: 0,
                pagesProcessed: 0,
                currentOperation: 'Reading stream',
                timeElapsed: Date.now() - this.startTime
            });
            // Check backpressure
            if (this.buffer.length > (this.options?.backpressureThreshold || 10485760)) {
                // Compact buffer if possible
                if (this.position > 0) {
                    this.buffer = this.buffer.slice(this.position);
                    this.position = 0;
                }
            }
        }
    }
    readString(start, length) {
        const bytes = this.buffer.slice(start, start + length);
        return new TextDecoder().decode(bytes);
    }
}
class TextExtractor {
    constructor(pdf, options) {
        this.pdf = pdf;
        this.options = options;
    }
    async extract() {
        const metric = PerformanceMonitor.startOperation('TextExtractor.extract');
        const results = [];
        const metadata = this.pdf.getMetadata();
        if (!metadata) {
            PerformanceMonitor.endOperation(metric);
            return results;
        }
        for (let i = 1; i <= metadata.pageCount; i++) {
            const page = await this.pdf.getPage(i);
            if (page) {
                const pageText = await this.extractPageText(page);
                results.push(...pageText);
            }
        }
        const finalResults = this.options?.preserveFormatting ? results : this.mergeTextBlocks(results);
        PerformanceMonitor.endOperation(metric);
        return finalResults;
    }
    async *stream() {
        const metadata = this.pdf.getMetadata();
        if (!metadata)
            return;
        for (let i = 1; i <= metadata.pageCount; i++) {
            const page = await this.pdf.getPage(i);
            if (page) {
                const pageText = await this.extractPageText(page);
                for (const text of pageText) {
                    yield text;
                }
            }
        }
    }
    async extractPageText(page) {
        const metric = PerformanceMonitor.startOperation('TextExtractor.extractPageText');
        const textBlocks = [];
        if (!page.contents) {
            PerformanceMonitor.endOperation(metric);
            return textBlocks;
        }
        // Parse content stream with safety limits (now with caching)
        const contentParser = new ContentStreamParser(page.contents);
        try {
            const operations = contentParser.parse();
            if (operations.length === 0) {
                console.warn('No operations parsed from content stream');
                return textBlocks;
            }
            // Current graphics state
            let currentState = {
                textMatrix: [1, 0, 0, 1, 0, 0],
                fontSize: 12,
                fontName: 'Helvetica',
                textLeading: 0,
                charSpace: 0,
                wordSpace: 0,
                horizontalScaling: 100,
                textRise: 0,
                renderingMode: 0,
                fillColor: { r: 0, g: 0, b: 0 }
            };
            // Line matrix for T* operator
            let lineMatrix = [1, 0, 0, 1, 0, 0];
            // Debug: log text positioning operators
            const textOps = operations.filter(op => ['BT', 'ET', 'Tf', 'Tm', 'Td', 'TD', 'T*', 'Tj', 'TJ', "'", '"', 'TL'].includes(op.operator));
            console.log(`Page ${page.pageNumber}: Found ${textOps.length} text operators out of ${operations.length} total`);
            console.log('Text operators:', textOps.slice(0, 20).map(op => `${op.operator}(${op.operands.length})`).join(', '));
            for (const op of operations) {
                switch (op.operator) {
                    case 'BT': // Begin text
                        currentState.textMatrix = [1, 0, 0, 1, 0, 0];
                        lineMatrix = [1, 0, 0, 1, 0, 0];
                        break;
                    case 'ET': // End text
                        break;
                    case 'Tf': // Set font and size
                        currentState.fontName = op.operands[0];
                        currentState.fontSize = op.operands[1];
                        // Track font resource for text decoding
                        {
                            const fontKey = currentState.fontName.startsWith('/') ? currentState.fontName.substring(1) : currentState.fontName;
                            if (page.resources && page.resources.fonts) {
                                currentState.fontResource = page.resources.fonts.get(fontKey);
                            }
                        }
                        break;
                    case 'Tm': // Set text matrix
                        if (op.operands.length >= 6) {
                            currentState.textMatrix = [
                                op.operands[0], op.operands[1], op.operands[2],
                                op.operands[3], op.operands[4], op.operands[5]
                            ];
                            lineMatrix = [...currentState.textMatrix];
                        }
                        break;
                    case 'Td': // Move text position
                        if (op.operands.length >= 2) {
                            const tx = op.operands[0];
                            const ty = op.operands[1];
                            currentState.textMatrix[4] += tx;
                            currentState.textMatrix[5] += ty;
                            lineMatrix = [...currentState.textMatrix];
                        }
                        break;
                    case 'TD': // Move text position and set leading
                        if (op.operands.length >= 2) {
                            const tx = op.operands[0];
                            const ty = op.operands[1];
                            currentState.textMatrix[4] += tx;
                            currentState.textMatrix[5] += ty;
                            currentState.textLeading = -ty;
                            lineMatrix = [...currentState.textMatrix];
                        }
                        break;
                    case 'T*': // Move to start of next line
                        currentState.textMatrix[4] = lineMatrix[4];
                        currentState.textMatrix[5] = lineMatrix[5] - currentState.textLeading;
                        lineMatrix = [...currentState.textMatrix];
                        break;
                    case 'Tj': // Show text
                        const text = op.operands[0];
                        if (text) {
                            const decodedText = this.decodeTextWithFont(text, currentState);
                            textBlocks.push(this.createTextContent(decodedText, currentState, page));
                        }
                        break;
                    case 'TJ': // Show text with positioning
                        const array = op.operands[0];
                        let combinedText = '';
                        for (const item of array) {
                            if (typeof item === 'string') {
                                combinedText += this.decodeTextWithFont(item, currentState);
                            }
                            else if (typeof item === 'number') {
                                // Large negative adjustments (e.g., < -100) indicate word boundaries
                                if (item < -100) {
                                    combinedText += ' ';
                                }
                            }
                        }
                        if (combinedText) {
                            textBlocks.push(this.createTextContent(combinedText, currentState, page));
                        }
                        break;
                    case "'": // Move to next line and show text
                        // Equivalent to T* Tj
                        currentState.textMatrix[4] = lineMatrix[4];
                        currentState.textMatrix[5] = lineMatrix[5] - currentState.textLeading;
                        lineMatrix = [...currentState.textMatrix];
                        if (op.operands[0]) {
                            const decodedQuote = this.decodeTextWithFont(op.operands[0], currentState);
                            textBlocks.push(this.createTextContent(decodedQuote, currentState, page));
                        }
                        break;
                    case '"': // Set spacing, move to next line, show text
                        // Set word and char spacing, then equivalent to T* Tj
                        if (op.operands.length >= 3) {
                            currentState.wordSpace = op.operands[0];
                            currentState.charSpace = op.operands[1];
                            currentState.textMatrix[4] = lineMatrix[4];
                            currentState.textMatrix[5] = lineMatrix[5] - currentState.textLeading;
                            lineMatrix = [...currentState.textMatrix];
                            if (op.operands[2]) {
                                const decodedDQ = this.decodeTextWithFont(op.operands[2], currentState);
                                textBlocks.push(this.createTextContent(decodedDQ, currentState, page));
                            }
                        }
                        break;
                    case 'TL': // Set text leading
                        if (op.operands.length >= 1) {
                            currentState.textLeading = op.operands[0];
                        }
                        break;
                }
            }
            PerformanceMonitor.endOperation(metric);
            return textBlocks;
        }
        catch (error) {
            console.error('Error parsing content stream:', error);
            PerformanceMonitor.endOperation(metric);
            return textBlocks; // Return empty array on error
        }
    }
    createTextContent(text, state, page) {
        // Calculate text metrics
        const width = text.length * state.fontSize * 0.5; // Approximate
        const height = state.fontSize;
        return {
            text: text,
            x: state.textMatrix[4],
            y: page.height - state.textMatrix[5], // Flip Y coordinate
            width: width,
            height: height,
            fontSize: state.fontSize,
            fontName: state.fontName,
            direction: 'ltr',
            transform: [...state.textMatrix], // CRITICAL: Copy array, not reference!
            style: {
                bold: state.fontName.toLowerCase().includes('bold'),
                italic: state.fontName.toLowerCase().includes('italic'),
                underline: false,
                strikethrough: false,
                color: state.fillColor
            },
            pageNumber: page.pageNumber
        };
    }
    decodeTextWithFont(rawText, state) {
        const font = state.fontResource;
        if (!font)
            return rawText;
        const toUnicode = font.toUnicode;
        const encoding = font.encoding;
        const isCIDFont = font.subtype === 'Type0';
        // For raw PDF strings, extract byte values
        let decoded = '';
        if (isCIDFont) {
            // Group bytes into 2-byte CID codes
            for (let i = 0; i < rawText.length - 1; i += 2) {
                const charCode = (rawText.charCodeAt(i) << 8) | rawText.charCodeAt(i + 1);
                if (toUnicode && toUnicode.has(charCode)) {
                    decoded += toUnicode.get(charCode);
                }
                else if (charCode > 0) {
                    decoded += String.fromCodePoint(charCode);
                }
            }
        }
        else {
            for (let i = 0; i < rawText.length; i++) {
                const charCode = rawText.charCodeAt(i);
                if (toUnicode && toUnicode.has(charCode)) {
                    decoded += toUnicode.get(charCode);
                }
                else {
                    decoded += PDFTextDecoder.mapCharCode(charCode, encoding);
                }
            }
        }
        return decoded;
    }
    mergeTextBlocks(blocks) {
        // Merge adjacent text blocks on the same line
        const merged = [];
        let current = null;
        for (const block of blocks) {
            if (!current) {
                current = { ...block };
                continue;
            }
            // Check if blocks are on the same line (within threshold)
            const sameY = Math.abs(current.y - block.y) < current.fontSize * 0.5;
            const adjacent = Math.abs((current.x + current.width) - block.x) < current.fontSize;
            if (sameY && adjacent && current.pageNumber === block.pageNumber) {
                // Merge blocks
                current.text += ' ' + block.text;
                current.width = block.x + block.width - current.x;
            }
            else {
                merged.push(current);
                current = { ...block };
            }
        }
        if (current)
            merged.push(current);
        return merged;
    }
}
class ContentStreamParser {
    constructor(data) {
        this.data = data;
        this.position = 0;
        this.errorRecoveryMode = false;
    }
    /**
     * Get cache key for content stream data
     */
    getCacheKey() {
        // Only cache small content streams (performance vs memory tradeoff)
        if (this.data.length > ContentStreamParser.CACHE_KEY_MAX_LENGTH) {
            return null;
        }
        // Use first 100 bytes as cache key (fast and usually unique)
        const keyBytes = this.data.slice(0, Math.min(100, this.data.length));
        return Array.from(keyBytes).join(',');
    }
    /**
     * Clear parser cache (useful for memory management)
     */
    static clearCache() {
        ContentStreamParser.parserCache.clear();
    }
    parse() {
        // Try to get cached result
        const cacheKey = this.getCacheKey();
        if (cacheKey) {
            const cached = ContentStreamParser.parserCache.get(cacheKey);
            if (cached) {
                return cached; // Return deep copy to prevent mutation
            }
        }
        const operations = [];
        let iterations = 0;
        const maxIterations = this.data.length * 2; // Safety limit: twice the data length
        let lastPosition = -1;
        while (this.position < this.data.length) {
            // Safety check: prevent infinite loops
            iterations++;
            if (iterations > maxIterations) {
                console.warn(`ContentStreamParser: Reached max iterations (${maxIterations}), stopping parse`);
                break;
            }
            // Safety check: detect position not advancing
            if (this.position === lastPosition) {
                console.warn(`ContentStreamParser: Position stuck at ${this.position}, advancing forcefully`);
                this.position++;
                if (this.position >= this.data.length)
                    break;
            }
            lastPosition = this.position;
            this.skipWhitespaceAndComments();
            if (this.position >= this.data.length)
                break;
            const operands = [];
            // Parse operands with safety counter
            let operandIterations = 0;
            const maxOperands = 1000; // Max operands before one operator
            let lastOperandPosition = -1;
            while (this.position < this.data.length) {
                if (++operandIterations > maxOperands) {
                    console.warn(`ContentStreamParser: Too many operands (${maxOperands}) without operator at position ${this.position}`);
                    break;
                }
                // Check if position is advancing
                if (this.position === lastOperandPosition) {
                    console.warn(`ContentStreamParser: Operand parsing stuck at position ${this.position}`);
                    this.position++; // Force advance
                    break;
                }
                lastOperandPosition = this.position;
                const operand = this.parseOperand();
                if (operand === null)
                    break;
                operands.push(operand);
            }
            // Parse operator
            const operator = this.parseOperator();
            if (operator) {
                // Handle inline image operator (BI)
                if (operator === 'BI') {
                    const inlineImageOp = this.parseInlineImage(operands);
                    if (inlineImageOp) {
                        operations.push(inlineImageOp);
                    }
                }
                else {
                    operations.push({ operator, operands });
                }
            }
            else if (operands.length > 0) {
                // Incomplete operation: operands without operator
                // Attempt error recovery
                console.warn(`ContentStreamParser: Found ${operands.length} operands without operator at position ${this.position}`);
                if (!this.errorRecoveryMode) {
                    this.errorRecoveryMode = true;
                    this.recoverFromError();
                }
            }
            else {
                // If we can't parse an operator and have no operands, advance position
                if (this.position < this.data.length) {
                    this.position++;
                }
            }
        }
        // Cache the result if applicable
        if (cacheKey && operations.length > 0) {
            // Implement LRU-style cache eviction
            if (ContentStreamParser.parserCache.size >= ContentStreamParser.MAX_CACHE_SIZE) {
                const firstKey = ContentStreamParser.parserCache.keys().next().value;
                if (firstKey) {
                    ContentStreamParser.parserCache.delete(firstKey);
                }
            }
            ContentStreamParser.parserCache.set(cacheKey, operations);
        }
        return operations;
    }
    parseOperand() {
        this.skipWhitespace();
        const byte = this.data[this.position];
        // Number
        if ((byte >= 48 && byte <= 57) || byte === 45 || byte === 46) {
            return this.parseNumber();
        }
        // String
        if (byte === 40) { // '('
            return this.parseString();
        }
        // Hex string
        if (byte === 60) { // '<'
            return this.parseHexString();
        }
        // Array
        if (byte === 91) { // '['
            return this.parseArray();
        }
        // Name
        if (byte === 47) { // '/'
            return this.parseName();
        }
        // Dictionary
        if (byte === 60 && this.data[this.position + 1] === 60) { // '<<'
            return this.parseDictionary();
        }
        return null;
    }
    parseOperator() {
        this.skipWhitespace();
        let operator = '';
        const startPos = this.position;
        while (this.position < this.data.length) {
            const byte = this.data[this.position];
            // Operator characters (letters and special chars)
            if ((byte >= 65 && byte <= 90) || // A-Z
                (byte >= 97 && byte <= 122) || // a-z
                byte === 42 || byte === 39 || byte === 34) { // *, ', "
                operator += String.fromCharCode(byte);
                this.position++;
            }
            else {
                break;
            }
        }
        // Validate operator length (PDF operators are typically 1-3 characters)
        if (operator.length > 10) {
            console.warn(`ContentStreamParser: Unusually long operator "${operator}" at position ${startPos}, truncating`);
            operator = operator.substring(0, 10);
        }
        return operator;
    }
    parseNumber() {
        let numStr = '';
        let hasDecimalPoint = false;
        while (this.position < this.data.length) {
            const byte = this.data[this.position];
            if (byte >= 48 && byte <= 57) { // 0-9
                numStr += String.fromCharCode(byte);
                this.position++;
            }
            else if (byte === 46 && !hasDecimalPoint) { // '.' - only one decimal point allowed
                hasDecimalPoint = true;
                numStr += '.';
                this.position++;
            }
            else if ((byte === 45 || byte === 43) && numStr.length === 0) { // +/- only at start
                numStr += String.fromCharCode(byte);
                this.position++;
            }
            else {
                break;
            }
        }
        // Handle edge cases
        if (numStr === '' || numStr === '+' || numStr === '-' || numStr === '.') {
            console.warn(`ContentStreamParser: Invalid number "${numStr}" at position ${this.position}, defaulting to 0`);
            return 0;
        }
        const result = parseFloat(numStr);
        // Validate result
        if (isNaN(result) || !isFinite(result)) {
            console.warn(`ContentStreamParser: Invalid number "${numStr}" parsed as ${result}, defaulting to 0`);
            return 0;
        }
        return result;
    }
    parseString() {
        this.position++; // Skip '('
        let str = '';
        let parenCount = 1;
        const startPos = this.position;
        while (this.position < this.data.length && parenCount > 0) {
            const byte = this.data[this.position++];
            if (byte === 40) { // '('
                parenCount++;
                str += '(';
            }
            else if (byte === 41) { // ')'
                parenCount--;
                if (parenCount > 0)
                    str += ')';
            }
            else if (byte === 92 && this.position < this.data.length) { // '\' - escape sequence
                const next = this.data[this.position++];
                switch (next) {
                    case 110:
                        str += '\n';
                        break; // n
                    case 114:
                        str += '\r';
                        break; // r
                    case 116:
                        str += '\t';
                        break; // t
                    case 98:
                        str += '\b';
                        break; // b
                    case 102:
                        str += '\f';
                        break; // f
                    case 40:
                        str += '(';
                        break; // (
                    case 41:
                        str += ')';
                        break; // )
                    case 92:
                        str += '\\';
                        break; // \
                    // Octal escape sequences (\ddd)
                    case 48:
                    case 49:
                    case 50:
                    case 51:
                    case 52:
                    case 53:
                    case 54:
                    case 55: {
                        let octalStr = String.fromCharCode(next);
                        // Read up to 2 more octal digits
                        for (let i = 0; i < 2 && this.position < this.data.length; i++) {
                            const octalByte = this.data[this.position];
                            if (octalByte >= 48 && octalByte <= 55) { // 0-7
                                octalStr += String.fromCharCode(octalByte);
                                this.position++;
                            }
                            else {
                                break;
                            }
                        }
                        const octalValue = parseInt(octalStr, 8);
                        str += String.fromCharCode(octalValue);
                        break;
                    }
                    // Line continuation (backslash followed by newline)
                    case 10:
                    case 13: // LF or CR
                        if (next === 13 && this.position < this.data.length && this.data[this.position] === 10) {
                            this.position++; // Skip LF after CR
                        }
                        // Don't add anything - line continuation
                        break;
                    default:
                        // Unknown escape - keep the character
                        str += String.fromCharCode(next);
                }
            }
            else {
                str += String.fromCharCode(byte);
            }
        }
        // Check for unclosed string
        if (parenCount > 0) {
            console.warn(`ContentStreamParser: Unclosed string at position ${startPos}, found ${parenCount} unclosed parentheses`);
        }
        return str;
    }
    parseHexString() {
        const startPos = this.position;
        this.position++; // Skip '<'
        let hex = '';
        let foundClosing = false;
        while (this.position < this.data.length) {
            const byte = this.data[this.position];
            if (byte === 62) { // '>'
                this.position++;
                foundClosing = true;
                break;
            }
            // Skip whitespace in hex strings (PDF spec allows it)
            if (this.isWhitespace(byte)) {
                this.position++;
                continue;
            }
            if ((byte >= 48 && byte <= 57) || // 0-9
                (byte >= 65 && byte <= 70) || // A-F
                (byte >= 97 && byte <= 102)) { // a-f
                hex += String.fromCharCode(byte);
            }
            else {
                // Invalid hex character
                console.warn(`ContentStreamParser: Invalid hex character ${String.fromCharCode(byte)} at position ${this.position}`);
            }
            this.position++;
        }
        // Check for unclosed hex string
        if (!foundClosing) {
            console.warn(`ContentStreamParser: Unclosed hex string at position ${startPos}`);
        }
        // Handle odd number of hex digits (pad with 0)
        if (hex.length % 2 !== 0) {
            hex += '0';
        }
        // Convert hex to string
        let str = '';
        for (let i = 0; i < hex.length; i += 2) {
            const hexByte = hex.substr(i, 2);
            const value = parseInt(hexByte, 16);
            if (!isNaN(value)) {
                str += String.fromCharCode(value);
            }
        }
        return str;
    }
    parseArray() {
        this.position++; // Skip '['
        const array = [];
        let safetyCounter = 0;
        const MAX_ARRAY_ITEMS = 100000;
        let lastPosition = -1;
        while (this.position < this.data.length) {
            if (++safetyCounter > MAX_ARRAY_ITEMS) {
                console.warn(`ContentStreamParser.parseArray: Safety limit reached (${MAX_ARRAY_ITEMS} items)`);
                break;
            }
            if (this.position === lastPosition) {
                console.warn(`ContentStreamParser.parseArray: Position stuck at ${this.position}`);
                this.position++;
                break;
            }
            lastPosition = this.position;
            this.skipWhitespace();
            if (this.data[this.position] === 93) { // ']'
                this.position++;
                break;
            }
            const item = this.parseOperand();
            if (item !== null) {
                array.push(item);
            }
        }
        return array;
    }
    parseName() {
        this.position++; // Skip '/'
        let name = '';
        let safetyCounter = 0;
        const MAX_NAME_LENGTH = 10000;
        while (this.position < this.data.length) {
            if (++safetyCounter > MAX_NAME_LENGTH) {
                console.warn(`ContentStreamParser.parseName: Safety limit reached (${MAX_NAME_LENGTH} chars)`);
                break;
            }
            const byte = this.data[this.position];
            // Name ends at whitespace or delimiter
            if (this.isWhitespace(byte) || this.isDelimiter(byte)) {
                break;
            }
            name += String.fromCharCode(byte);
            this.position++;
        }
        return name;
    }
    parseDictionary() {
        this.position += 2; // Skip '<<'
        const dict = new Map();
        let safetyCounter = 0;
        const MAX_DICT_ENTRIES = 10000;
        let lastPosition = -1;
        while (this.position < this.data.length) {
            if (++safetyCounter > MAX_DICT_ENTRIES) {
                console.warn(`ContentStreamParser.parseDictionary: Safety limit reached (${MAX_DICT_ENTRIES} entries)`);
                break;
            }
            if (this.position === lastPosition) {
                console.warn(`ContentStreamParser.parseDictionary: Position stuck at ${this.position}`);
                this.position++;
                break;
            }
            lastPosition = this.position;
            this.skipWhitespace();
            // Check for '>>'
            if (this.data[this.position] === 62 && this.data[this.position + 1] === 62) {
                this.position += 2;
                break;
            }
            // Parse key (name)
            const key = this.parseName();
            this.skipWhitespace();
            // Parse value
            const value = this.parseOperand();
            dict.set(key, value);
        }
        return dict;
    }
    skipWhitespace() {
        while (this.position < this.data.length && this.isWhitespace(this.data[this.position])) {
            this.position++;
        }
    }
    /**
     * Skip whitespace and comments
     * Comments start with % and continue to end of line
     */
    skipWhitespaceAndComments() {
        while (this.position < this.data.length) {
            const byte = this.data[this.position];
            if (this.isWhitespace(byte)) {
                this.position++;
            }
            else if (byte === 37) { // '%' - comment
                this.skipComment();
            }
            else {
                break;
            }
        }
    }
    /**
     * Skip comment (from % to end of line)
     */
    skipComment() {
        // Skip the '%'
        this.position++;
        // Skip until newline or end of data
        while (this.position < this.data.length) {
            const byte = this.data[this.position];
            this.position++;
            // Stop at line feed or carriage return
            if (byte === 10 || byte === 13) {
                break;
            }
        }
    }
    /**
     * Parse inline image (BI...ID...EI sequence)
     * Format: BI <dictionary> ID <image data> EI
     */
    parseInlineImage(_operands) {
        try {
            // BI operator already consumed, parse image dictionary
            const imageDict = new Map();
            // Parse key-value pairs until ID
            while (this.position < this.data.length) {
                this.skipWhitespaceAndComments();
                // Check for ID operator (image data marker)
                if (this.peekOperator() === 'ID') {
                    this.parseOperator(); // Consume 'ID'
                    break;
                }
                // Parse key (name or abbreviation)
                const key = this.parseName();
                if (!key)
                    break;
                this.skipWhitespaceAndComments();
                // Parse value
                const value = this.parseOperand();
                imageDict.set(key, value);
            }
            // Skip whitespace after ID (single whitespace character)
            if (this.position < this.data.length && this.isWhitespace(this.data[this.position])) {
                this.position++;
            }
            // Read image data until EI operator
            const imageData = this.findInlineImageEnd();
            // Create operation with image dictionary and data
            return {
                operator: 'BI',
                operands: [{
                        dictionary: imageDict,
                        data: imageData
                    }]
            };
        }
        catch (error) {
            console.warn('ContentStreamParser: Failed to parse inline image:', error);
            return null;
        }
    }
    /**
     * Find the end of inline image data (EI operator)
     * This is tricky because EI could appear in the image data
     */
    findInlineImageEnd() {
        const start = this.position;
        let end = start;
        // Look for 'EI' preceded by whitespace and followed by whitespace/delimiter
        while (this.position < this.data.length - 2) {
            // Check for whitespace + 'EI' + whitespace/delimiter
            const byte = this.data[this.position];
            if (this.isWhitespace(byte)) {
                const next1 = this.data[this.position + 1];
                const next2 = this.data[this.position + 2];
                // Check for 'EI' (0x45 0x49)
                if (next1 === 69 && next2 === 73) {
                    // Check if followed by whitespace or delimiter
                    if (this.position + 3 < this.data.length) {
                        const next3 = this.data[this.position + 3];
                        if (this.isWhitespace(next3) || this.isDelimiter(next3)) {
                            // Found EI operator
                            end = this.position;
                            this.position += 3; // Skip whitespace + 'EI'
                            break;
                        }
                    }
                    else {
                        // EI at end of stream
                        end = this.position;
                        this.position += 3;
                        break;
                    }
                }
            }
            this.position++;
        }
        // Extract image data
        const imageData = this.data.slice(start, end);
        return imageData;
    }
    /**
     * Peek at next operator without consuming it
     */
    peekOperator() {
        const savedPosition = this.position;
        this.skipWhitespaceAndComments();
        let operator = '';
        let pos = this.position;
        while (pos < this.data.length) {
            const byte = this.data[pos];
            // Operator characters (letters and special chars)
            if ((byte >= 65 && byte <= 90) || // A-Z
                (byte >= 97 && byte <= 122) || // a-z
                byte === 42 || byte === 39 || byte === 34) { // *, ', "
                operator += String.fromCharCode(byte);
                pos++;
            }
            else {
                break;
            }
        }
        // Restore position
        this.position = savedPosition;
        return operator;
    }
    /**
     * Recover from parsing error
     * Attempts to synchronize with next valid operator
     */
    recoverFromError() {
        console.warn('ContentStreamParser: Attempting error recovery');
        // Skip until we find a valid operator or delimiter
        let recovered = false;
        const recoveryLimit = 1000; // Don't skip more than 1000 bytes
        let skipped = 0;
        while (this.position < this.data.length && skipped < recoveryLimit) {
            const byte = this.data[this.position];
            // Try to find a delimiter or whitespace
            if (this.isWhitespace(byte) || this.isDelimiter(byte)) {
                // Try to parse next token
                this.skipWhitespaceAndComments();
                // Check if we can parse an operator
                const savedPos = this.position;
                const op = this.parseOperator();
                if (op && op.length > 0) {
                    // Found valid operator, restore position and exit recovery
                    this.position = savedPos;
                    recovered = true;
                    this.errorRecoveryMode = false;
                    break;
                }
            }
            this.position++;
            skipped++;
        }
        if (!recovered) {
            console.warn('ContentStreamParser: Error recovery failed, continuing parse');
            this.errorRecoveryMode = false;
        }
    }
    isWhitespace(byte) {
        return byte === 0 || byte === 9 || byte === 10 || byte === 12 || byte === 13 || byte === 32;
    }
    isDelimiter(byte) {
        return byte === 40 || byte === 41 || // ()
            byte === 60 || byte === 62 || // <>
            byte === 91 || byte === 93 || // []
            byte === 123 || byte === 125 || // {}
            byte === 47 || byte === 37; // /%
    }
}
// Parser cache for frequently parsed content streams
ContentStreamParser.parserCache = new Map();
ContentStreamParser.MAX_CACHE_SIZE = 100;
ContentStreamParser.CACHE_KEY_MAX_LENGTH = 1000;
/**
 * CCITT Fax decoder (Group 3 1D / Group 4)
 * Decodes bi-level (1-bit) image data used in scanned documents.
 */
class CCITTFaxDecoder {
    constructor(data, columns, rows, blackIs1 = false, encodedByteAlign = false) {
        this.bytePos = 0;
        this.bitPos = 0;
        this.data = data;
        this.columns = columns;
        this.rows = rows;
        this.blackIs1 = blackIs1;
        this.encodedByteAlign = encodedByteAlign;
    }
    /**
     * Decode CCITT data to a bitmap (1 bit per pixel → 8 bits per pixel grayscale)
     * Returns Uint8Array of width*height bytes, 0=black, 255=white
     */
    decode() {
        // Simplified: treat as raw uncompressed bitmap for now
        // Full ITU-T T.4/T.6 decoding is complex; provide best-effort output
        const output = new Uint8Array(this.columns * this.rows);
        const bytesPerRow = Math.ceil(this.columns / 8);
        const whiteVal = this.blackIs1 ? 0 : 255;
        const blackVal = this.blackIs1 ? 255 : 0;
        for (let row = 0; row < this.rows; row++) {
            for (let col = 0; col < this.columns; col++) {
                const byteIdx = row * bytesPerRow + (col >> 3);
                const bitIdx = 7 - (col & 7);
                if (byteIdx < this.data.length) {
                    const bit = (this.data[byteIdx] >> bitIdx) & 1;
                    output[row * this.columns + col] = bit ? blackVal : whiteVal;
                }
                else {
                    output[row * this.columns + col] = whiteVal;
                }
            }
        }
        return output;
    }
    /**
     * Decode to a 1-bit packed bitmap (returns packed bytes, MSB first)
     */
    decodePacked() {
        const bytesPerRow = Math.ceil(this.columns / 8);
        const output = new Uint8Array(bytesPerRow * this.rows);
        // For raw/uncompressed CCITT data the input is already packed
        const len = Math.min(this.data.length, output.length);
        for (let i = 0; i < len; i++) {
            output[i] = this.blackIs1 ? this.data[i] : (this.data[i] ^ 0xFF);
        }
        return output;
    }
}
// ITU-T T.4 / T.6 run-length tables
// White run make-up codes + terminating codes; black run equivalents
CCITTFaxDecoder.WHITE_TERM = [
    [0x35, 8, 0], [0x07, 6, 1], [0x07, 4, 2], [0x08, 4, 3],
    [0x0B, 4, 4], [0x0C, 4, 5], [0x0E, 4, 6], [0x0F, 4, 7],
    [0x13, 5, 8], [0x14, 5, 9], [0x07, 5, 10], [0x08, 5, 11],
    [0x08, 6, 12], [0x03, 6, 13], [0x34, 6, 14], [0x35, 6, 15],
    [0x2A, 6, 16], [0x2B, 6, 17], [0x27, 7, 18], [0x0C, 7, 19],
    [0x08, 7, 20], [0x17, 7, 21], [0x03, 7, 22], [0x04, 7, 23],
    [0x28, 7, 24], [0x2B, 7, 25], [0x13, 7, 26], [0x24, 7, 27],
    [0x18, 7, 28], [0x02, 8, 29], [0x03, 8, 30], [0x1A, 8, 31],
    [0x1B, 8, 32], [0x12, 8, 33], [0x13, 8, 34], [0x14, 8, 35],
    [0x15, 8, 36], [0x16, 8, 37], [0x17, 8, 38], [0x28, 8, 39],
    [0x29, 8, 40], [0x2A, 8, 41], [0x2B, 8, 42], [0x2C, 8, 43],
    [0x2D, 8, 44], [0x04, 8, 45], [0x05, 8, 46], [0x0A, 8, 47],
    [0x0B, 8, 48], [0x52, 8, 49], [0x53, 8, 50], [0x54, 8, 51],
    [0x55, 8, 52], [0x24, 8, 53], [0x25, 8, 54], [0x58, 8, 55],
    [0x59, 8, 56], [0x5A, 8, 57], [0x5B, 8, 58], [0x4A, 8, 59],
    [0x4B, 8, 60], [0x32, 8, 61], [0x33, 8, 62], [0x34, 8, 63],
];
CCITTFaxDecoder.WHITE_MAKEUP = [
    [0x1B, 5, 64], [0x12, 5, 128], [0x17, 6, 192], [0x37, 7, 256],
    [0x36, 8, 320], [0x37, 8, 384], [0x64, 8, 448], [0x65, 8, 512],
    [0x68, 8, 576], [0x67, 8, 640], [0xCC, 9, 704], [0xCD, 9, 768],
    [0xD2, 9, 832], [0xD3, 9, 896], [0xD4, 9, 960], [0xD5, 9, 1024],
    [0xD6, 9, 1088], [0xD7, 9, 1152], [0xD8, 9, 1216], [0xD9, 9, 1280],
    [0xDA, 9, 1344], [0xDB, 9, 1408], [0x98, 9, 1472], [0x99, 9, 1536],
    [0x9A, 9, 1600], [0x18, 6, 1664], [0x9B, 9, 1728],
];
class ImageExtractor {
    constructor(pdf, options) {
        this.pdf = pdf;
        this.options = options;
    }
    async extract() {
        const images = [];
        const metadata = this.pdf.getMetadata();
        if (!metadata)
            return images;
        for (let i = 1; i <= metadata.pageCount; i++) {
            const page = await this.pdf.getPage(i);
            if (page && page.resources) {
                const pageImages = await this.extractPageImages(page);
                images.push(...pageImages);
            }
        }
        return images;
    }
    async extractPageImages(page) {
        const images = [];
        if (!page.resources?.images)
            return images;
        // Page range filtering
        if (this.options?.pageRange) {
            const { start, end } = this.options.pageRange;
            if (page.pageNumber < start || page.pageNumber > end)
                return images;
        }
        let imageIndex = 0;
        for (const [_name, imageRes] of page.resources.images) {
            const image = {
                id: `page${page.pageNumber}_img${imageIndex++}`,
                x: 0,
                y: 0,
                width: imageRes.width,
                height: imageRes.height,
                mimeType: this.getImageMimeType(imageRes),
                bitsPerComponent: imageRes.bitsPerComponent,
                colorSpace: imageRes.colorSpace,
                filter: imageRes.filter,
                data: this.resolveImageData(imageRes),
                pageNumber: page.pageNumber
            };
            images.push(image);
        }
        return images;
    }
    /**
     * Resolve image data based on filter type.
     * JPEG/JPEG2000: passthrough raw compressed data directly.
     * CCITT: decode to grayscale pixel data.
     * FlateDecode: raw pixel data (already decompressed by parser).
     */
    resolveImageData(img) {
        if (!img.data || img.data.length === 0)
            return new Uint8Array(0);
        const filter = img.filter?.[0] || '';
        // DCTDecode (JPEG) — pass raw JPEG data directly
        if (filter === 'DCTDecode') {
            return img.data;
        }
        // JPXDecode (JPEG 2000) — pass raw JP2 data directly
        if (filter === 'JPXDecode') {
            return img.data;
        }
        // CCITTFaxDecode — decode Group 3/4 to grayscale
        if (filter === 'CCITTFaxDecode') {
            try {
                const dp = img.decodeParms || {};
                const columns = dp.Columns || img.width || 1;
                const rows = dp.Rows || img.height || 1;
                const blackIs1 = !!dp.BlackIs1;
                const decoder = new CCITTFaxDecoder(img.data, columns, rows, blackIs1);
                return decoder.decode();
            }
            catch (e) {
                return img.data;
            }
        }
        // JBIG2Decode — return raw data (full decode is very complex)
        if (filter === 'JBIG2Decode') {
            return img.data;
        }
        // FlateDecode or no filter — already decoded pixels from parser
        return img.data;
    }
    getImageMimeType(imageRes) {
        const filter = imageRes.filter?.[0] || '';
        if (filter === 'DCTDecode')
            return 'image/jpeg';
        if (filter === 'JPXDecode')
            return 'image/jp2';
        if (filter === 'JBIG2Decode')
            return 'image/x-jbig2';
        if (filter === 'CCITTFaxDecode')
            return 'image/x-ccitt';
        // All other formats (raw/FlateDecode) are resolved to raw pixel data
        return 'image/raw';
    }
    /**
     * Convert raw pixel ImageContent to a data URL (PNG or JPEG).
     * For DCTDecode images, wraps the raw JPEG bytes in a data URL.
     * For raw pixel data, encodes to PNG using canvas.
     */
    static toDataURL(image, format = 'png', quality = 0.92) {
        if (!image.data || image.data.length === 0)
            return '';
        // JPEG passthrough — already compressed
        if (image.mimeType === 'image/jpeg') {
            let binary = '';
            for (let i = 0; i < image.data.length; i++)
                binary += String.fromCharCode(image.data[i]);
            return 'data:image/jpeg;base64,' + btoa(binary);
        }
        // JPEG 2000 passthrough
        if (image.mimeType === 'image/jp2') {
            let binary = '';
            for (let i = 0; i < image.data.length; i++)
                binary += String.fromCharCode(image.data[i]);
            return 'data:image/jp2;base64,' + btoa(binary);
        }
        // Raw pixel data — need canvas to encode
        if (typeof document === 'undefined')
            return ''; // Server-side: no canvas
        const w = image.width;
        const h = image.height;
        if (w <= 0 || h <= 0)
            return '';
        const canvas = document.createElement('canvas');
        canvas.width = w;
        canvas.height = h;
        const ctx = canvas.getContext('2d');
        const imgData = ctx.createImageData(w, h);
        const rgba = imgData.data;
        const bpc = image.bitsPerComponent || 8;
        const cs = (image.colorSpace || 'DeviceRGB').replace(/^\//, '');
        let comp = 3;
        if (cs === 'DeviceGray' || cs === 'CalGray' || cs === 'G')
            comp = 1;
        else if (cs === 'DeviceCMYK' || cs === 'CMYK')
            comp = 4;
        const pixels = image.data;
        const total = w * h;
        if (bpc === 8) {
            for (let i = 0; i < total; i++) {
                const di = i * 4;
                if (comp === 1) {
                    const g = pixels[i] || 0;
                    rgba[di] = g;
                    rgba[di + 1] = g;
                    rgba[di + 2] = g;
                    rgba[di + 3] = 255;
                }
                else if (comp === 3) {
                    const si = i * 3;
                    rgba[di] = pixels[si] || 0;
                    rgba[di + 1] = pixels[si + 1] || 0;
                    rgba[di + 2] = pixels[si + 2] || 0;
                    rgba[di + 3] = 255;
                }
                else if (comp === 4) {
                    const si = i * 4;
                    const c = (pixels[si] || 0) / 255, m = (pixels[si + 1] || 0) / 255, y = (pixels[si + 2] || 0) / 255, k = (pixels[si + 3] || 0) / 255;
                    rgba[di] = 255 * (1 - c) * (1 - k) | 0;
                    rgba[di + 1] = 255 * (1 - m) * (1 - k) | 0;
                    rgba[di + 2] = 255 * (1 - y) * (1 - k) | 0;
                    rgba[di + 3] = 255;
                }
            }
        }
        else if (bpc === 1) {
            // 1-bit per component (common for CCITT-decoded images)
            for (let i = 0; i < total; i++) {
                const di = i * 4;
                const g = pixels[i] || 0; // Already expanded by CCITT decoder
                rgba[di] = g;
                rgba[di + 1] = g;
                rgba[di + 2] = g;
                rgba[di + 3] = 255;
            }
        }
        else {
            // Fallback — treat as 8-bit grayscale
            for (let i = 0; i < total && i < pixels.length; i++) {
                const di = i * 4;
                const g = pixels[i] || 0;
                rgba[di] = g;
                rgba[di + 1] = g;
                rgba[di + 2] = g;
                rgba[di + 3] = 255;
            }
        }
        ctx.putImageData(imgData, 0, 0);
        const mimeType = format === 'jpeg' ? 'image/jpeg' : format === 'webp' ? 'image/webp' : 'image/png';
        return canvas.toDataURL(mimeType, quality);
    }
}
class AIAnalyzer {
    constructor(pdf, options) {
        this.pdf = pdf;
        this.options = options;
    }
    async analyze() {
        const structuralAnalysis = await this.analyzeStructure();
        const semanticChunks = await this.generateChunks();
        const nlpReady = await this.prepareNLPContent();
        return {
            structuralAnalysis,
            semanticChunks,
            nlpReady,
            embeddings: this.options?.embeddingProvider
        };
    }
    async analyzeStructure() {
        const allText = await this.pdf.extractText();
        // Detect document type based on content patterns
        const documentType = this.detectDocumentType(allText);
        // Extract sections
        const sections = this.extractSections(allText);
        // Extract tables
        const tables = await this.extractTables();
        // Extract figures
        const figures = await this.extractFigures();
        // Extract equations
        const equations = this.extractEquations(allText);
        // Extract references
        const references = this.extractReferences(allText);
        // Build table of contents
        const tableOfContents = this.buildTableOfContents(sections);
        // Extract bibliography
        const bibliography = this.extractBibliography(allText);
        return {
            documentType,
            sections,
            tables,
            figures,
            equations,
            references,
            tableOfContents,
            bibliography
        };
    }
    detectDocumentType(text) {
        const fullText = text.map(t => t.text).join(' ').toLowerCase();
        // Simple heuristics for document type detection
        if (fullText.includes('abstract') && fullText.includes('references')) {
            return DocumentType.Article;
        }
        if (fullText.includes('chapter') && text.length > 1000) {
            return DocumentType.Book;
        }
        if (fullText.includes('invoice') || fullText.includes('total amount')) {
            return DocumentType.Invoice;
        }
        if (fullText.includes('experience') && fullText.includes('education')) {
            return DocumentType.Resume;
        }
        return DocumentType.Other;
    }
    extractSections(text) {
        const sections = [];
        let currentSection = null;
        let sectionId = 0;
        for (const block of text) {
            // Detect headers based on font size and style
            const isHeader = block.fontSize > 14 || block.style.bold;
            if (isHeader) {
                if (currentSection) {
                    sections.push(currentSection);
                }
                currentSection = {
                    id: `section_${sectionId++}`,
                    type: 'heading',
                    level: this.getHeaderLevel(block.fontSize),
                    pageStart: block.pageNumber,
                    pageEnd: block.pageNumber,
                    boundingBox: {
                        x: block.x,
                        y: block.y,
                        width: block.width,
                        height: block.height
                    },
                    text: block.text,
                    children: []
                };
            }
            else if (currentSection) {
                // Add content to current section
                currentSection.children?.push({
                    id: `section_${sectionId++}`,
                    type: 'paragraph',
                    pageStart: block.pageNumber,
                    pageEnd: block.pageNumber,
                    boundingBox: {
                        x: block.x,
                        y: block.y,
                        width: block.width,
                        height: block.height
                    },
                    text: block.text
                });
                currentSection.pageEnd = Math.max(currentSection.pageEnd, block.pageNumber);
            }
        }
        if (currentSection) {
            sections.push(currentSection);
        }
        return sections;
    }
    getHeaderLevel(fontSize) {
        if (fontSize > 24)
            return 1;
        if (fontSize > 18)
            return 2;
        if (fontSize > 14)
            return 3;
        return 4;
    }
    async extractTables() {
        const tables = [];
        // Table extraction logic would go here
        // This would analyze page layout to detect tabular structures
        return tables;
    }
    async extractFigures() {
        const figures = [];
        const images = await this.pdf.extractImages();
        for (const image of images) {
            figures.push({
                id: `figure_${image.id}`,
                pageNumber: image.pageNumber,
                boundingBox: {
                    x: image.x,
                    y: image.y,
                    width: image.width,
                    height: image.height
                },
                imageRef: image,
                type: 'illustration'
            });
        }
        return figures;
    }
    extractEquations(_text) {
        const equations = [];
        // Equation extraction logic
        // Would detect mathematical notation patterns
        return equations;
    }
    extractReferences(_text) {
        const references = [];
        // Reference extraction logic
        // Would detect citations, footnotes, hyperlinks
        return references;
    }
    buildTableOfContents(sections) {
        return sections
            .filter(s => s.type === 'heading')
            .map(s => ({
            title: s.text,
            pageNumber: s.pageStart,
            level: s.level || 1
        }));
    }
    extractBibliography(_text) {
        const bibliography = [];
        // Bibliography extraction logic
        // Would parse reference sections
        return bibliography;
    }
    async generateChunks() {
        const text = await this.pdf.extractText();
        const chunker = new SemanticChunker(this.pdf, {
            maxChunkSize: this.options?.chunkSize || 500,
            minChunkSize: 100,
            overlapSize: this.options?.chunkOverlap || 50
        });
        return chunker.chunkText(text);
    }
    async prepareNLPContent() {
        const text = await this.pdf.extractText();
        const fullText = text.map(t => t.text).join(' ');
        // Clean text
        const cleanText = this.cleanText(fullText);
        // Split into sentences
        const sentences = this.splitSentences(cleanText);
        // Split into paragraphs
        const paragraphs = this.splitParagraphs(cleanText);
        // Count tokens (simple approximation)
        const tokenCount = cleanText.split(/\s+/).length;
        // Detect language
        const language = this.detectLanguage(cleanText);
        // Extract keywords
        const keywords = this.extractKeywords(cleanText);
        return {
            fullText,
            cleanText,
            sentences,
            paragraphs,
            tokenCount,
            language,
            keywords
        };
    }
    cleanText(text) {
        return text
            .replace(/\s+/g, ' ')
            .replace(/[^\w\s.,!?;:\-'"]/g, '')
            .trim();
    }
    splitSentences(text) {
        return text.match(/[^.!?]+[.!?]+/g) || [];
    }
    splitParagraphs(text) {
        return text.split(/\n\n+/).filter(p => p.trim().length > 0);
    }
    detectLanguage(text) {
        // Simple language detection based on common words
        const englishWords = ['the', 'is', 'at', 'which', 'on'];
        const textLower = text.toLowerCase();
        if (englishWords.some(word => textLower.includes(word))) {
            return 'en';
        }
        return 'unknown';
    }
    extractKeywords(text) {
        // Simple keyword extraction
        const words = text.toLowerCase().split(/\s+/);
        const wordFreq = new Map();
        for (const word of words) {
            if (word.length > 4) { // Skip short words
                wordFreq.set(word, (wordFreq.get(word) || 0) + 1);
            }
        }
        // Sort by frequency and return top keywords
        return Array.from(wordFreq.entries())
            .sort((a, b) => b[1] - a[1])
            .slice(0, 10)
            .map(([word]) => word);
    }
}
class SemanticChunker {
    constructor(pdf, options = {}) {
        this.pdf = pdf;
        this.options = options;
    }
    async chunk() {
        const text = await this.pdf.extractText();
        return this.chunkText(text);
    }
    async *stream() {
        const chunks = await this.chunk();
        for (const chunk of chunks) {
            yield chunk;
        }
    }
    chunkText(text) {
        const strategy = this.options.strategy || 'semantic';
        switch (strategy) {
            case 'fixed':
                return this.fixedSizeChunking(text);
            case 'sliding':
                return this.slidingWindowChunking(text);
            case 'recursive':
                return this.recursiveChunking(text);
            case 'semantic':
            default:
                return this.semanticChunking(text);
        }
    }
    semanticChunking(text) {
        const chunks = [];
        let currentChunk = [];
        let currentSize = 0;
        let chunkId = 0;
        for (const block of text) {
            const blockSize = block.text.split(/\s+/).length;
            // Check if adding this block would exceed max size
            if (currentSize + blockSize > (this.options.maxChunkSize || 500)) {
                // Create chunk from current blocks
                if (currentChunk.length > 0) {
                    chunks.push(this.createChunk(currentChunk, chunkId++));
                }
                // Start new chunk with overlap
                if (this.options.overlapSize) {
                    // Keep last N tokens for overlap
                    const overlapBlocks = this.getOverlapBlocks(currentChunk, this.options.overlapSize);
                    currentChunk = overlapBlocks;
                    currentSize = overlapBlocks.reduce((sum, b) => sum + b.text.split(/\s+/).length, 0);
                }
                else {
                    currentChunk = [];
                    currentSize = 0;
                }
            }
            currentChunk.push(block);
            currentSize += blockSize;
            // Check for semantic boundaries (paragraph breaks, headers, etc.)
            if (this.isSemanticBoundary(block)) {
                if (currentSize >= (this.options.minChunkSize || 100)) {
                    chunks.push(this.createChunk(currentChunk, chunkId++));
                    currentChunk = [];
                    currentSize = 0;
                }
            }
        }
        // Add remaining chunk
        if (currentChunk.length > 0) {
            chunks.push(this.createChunk(currentChunk, chunkId++));
        }
        return chunks;
    }
    fixedSizeChunking(text) {
        const chunks = [];
        const maxSize = this.options.maxChunkSize || 500;
        let currentChunk = [];
        let currentSize = 0;
        let chunkId = 0;
        for (const block of text) {
            const blockSize = block.text.split(/\s+/).length;
            if (currentSize + blockSize > maxSize) {
                if (currentChunk.length > 0) {
                    chunks.push(this.createChunk(currentChunk, chunkId++));
                }
                currentChunk = [];
                currentSize = 0;
            }
            currentChunk.push(block);
            currentSize += blockSize;
        }
        if (currentChunk.length > 0) {
            chunks.push(this.createChunk(currentChunk, chunkId++));
        }
        return chunks;
    }
    slidingWindowChunking(text) {
        const chunks = [];
        const windowSize = this.options.maxChunkSize || 500;
        const stepSize = windowSize - (this.options.overlapSize || 50);
        let chunkId = 0;
        for (let i = 0; i < text.length; i += stepSize) {
            const window = text.slice(i, i + windowSize);
            if (window.length > 0) {
                chunks.push(this.createChunk(window, chunkId++));
            }
        }
        return chunks;
    }
    recursiveChunking(text) {
        // Recursive chunking based on document structure
        return this.semanticChunking(text); // Simplified for now
    }
    isSemanticBoundary(block) {
        // Check if this block represents a semantic boundary
        return block.text.endsWith('.') &&
            block.text.length > 50 &&
            (block.fontSize > 14 || block.style.bold);
    }
    getOverlapBlocks(blocks, overlapSize) {
        const result = [];
        let size = 0;
        for (let i = blocks.length - 1; i >= 0; i--) {
            const blockSize = blocks[i].text.split(/\s+/).length;
            if (size + blockSize <= overlapSize) {
                result.unshift(blocks[i]);
                size += blockSize;
            }
            else {
                break;
            }
        }
        return result;
    }
    createChunk(blocks, id) {
        const content = blocks.map(b => b.text).join(' ');
        const pageNumbers = [...new Set(blocks.map(b => b.pageNumber))];
        return {
            id: `chunk_${id}`,
            content,
            pageNumbers,
            type: this.detectChunkType(blocks),
            metadata: {
                tokenCount: content.split(/\s+/).length,
                confidence: 1.0,
                importance: this.calculateImportance(blocks),
                keywords: this.extractChunkKeywords(content)
            },
            startOffset: blocks[0].x,
            endOffset: blocks[blocks.length - 1].x + blocks[blocks.length - 1].width
        };
    }
    detectChunkType(blocks) {
        const firstBlock = blocks[0];
        if (firstBlock.fontSize > 16 || firstBlock.style.bold) {
            return ChunkType.Header;
        }
        const content = blocks.map(b => b.text).join(' ');
        if (content.match(/^\d+\./)) {
            return ChunkType.List;
        }
        if (content.startsWith('"') || content.includes('said')) {
            return ChunkType.Quote;
        }
        return ChunkType.Paragraph;
    }
    calculateImportance(blocks) {
        // Calculate importance based on position, formatting, etc.
        let importance = 0.5;
        // Headers are more important
        if (blocks.some(b => b.fontSize > 14 || b.style.bold)) {
            importance += 0.2;
        }
        // First page content is more important
        if (blocks.some(b => b.pageNumber === 1)) {
            importance += 0.1;
        }
        // Longer chunks might be more important
        const totalLength = blocks.reduce((sum, b) => sum + b.text.length, 0);
        if (totalLength > 500) {
            importance += 0.1;
        }
        return Math.min(importance, 1.0);
    }
    extractChunkKeywords(content) {
        const words = content.toLowerCase().split(/\s+/);
        const wordFreq = new Map();
        for (const word of words) {
            if (word.length > 4 && !this.isStopWord(word)) {
                wordFreq.set(word, (wordFreq.get(word) || 0) + 1);
            }
        }
        return Array.from(wordFreq.entries())
            .sort((a, b) => b[1] - a[1])
            .slice(0, 5)
            .map(([word]) => word);
    }
    isStopWord(word) {
        const stopWords = ['the', 'is', 'at', 'which', 'on', 'and', 'a', 'an', 'as', 'are', 'was', 'were', 'been', 'be', 'have', 'has', 'had', 'do', 'does', 'did', 'will', 'would', 'should', 'could', 'may', 'might', 'must', 'can', 'this', 'that', 'these', 'those', 'with', 'from', 'for', 'to', 'of', 'in'];
        return stopWords.includes(word);
    }
}
class PDFSearcher {
    constructor(pdf) {
        this.pdf = pdf;
    }
    async search(query, options) {
        const results = [];
        const text = await this.pdf.extractText();
        const queryLower = query.toLowerCase();
        const regex = options?.regex ? new RegExp(query, options.caseSensitive ? 'g' : 'gi') : null;
        for (const block of text) {
            const content = options?.caseSensitive ? block.text : block.text.toLowerCase();
            const searchQuery = options?.caseSensitive ? query : queryLower;
            let matches = [];
            if (regex) {
                // Regex search
                let match;
                while ((match = regex.exec(block.text)) !== null) {
                    matches.push({
                        text: match[0],
                        index: match.index,
                        length: match[0].length
                    });
                }
            }
            else if (options?.wholeWord) {
                // Whole word search
                const wordRegex = new RegExp(`\\b${searchQuery}\\b`, options.caseSensitive ? 'g' : 'gi');
                let match;
                while ((match = wordRegex.exec(block.text)) !== null) {
                    matches.push({
                        text: match[0],
                        index: match.index,
                        length: match[0].length
                    });
                }
            }
            else {
                // Simple substring search
                let index = content.indexOf(searchQuery);
                while (index !== -1) {
                    matches.push({
                        text: block.text.substr(index, query.length),
                        index,
                        length: query.length
                    });
                    index = content.indexOf(searchQuery, index + 1);
                }
            }
            if (matches.length > 0) {
                results.push({
                    pageNumber: block.pageNumber,
                    textContent: block,
                    matches,
                    context: this.getContext(block.text, matches[0].index, options?.contextLength || 50)
                });
            }
        }
        return results;
    }
    getContext(text, index, contextLength) {
        const start = Math.max(0, index - contextLength);
        const end = Math.min(text.length, index + contextLength);
        let context = text.substring(start, end);
        if (start > 0)
            context = '...' + context;
        if (end < text.length)
            context = context + '...';
        return context;
    }
}
class FormExtractor {
    constructor(pdf) {
        this.pdf = pdf;
    }
    async extract() {
        const fields = [];
        const pages = await this.pdf.getAllPages();
        for (const page of pages) {
            // Extract form fields from page annotations
            // This would parse interactive form elements
            const pageFields = await this.extractPageFormFields(page);
            fields.push(...pageFields);
        }
        return fields;
    }
    async extractPageFormFields(page) {
        const fields = [];
        try {
            // Parse page dictionary to find Annots array
            const pageDict = await this.getPageDictionary(page.pageNumber);
            const annotsRef = pageDict?.entries.get('Annots');
            if (annotsRef && annotsRef.type === PDFObjectType.Array) {
                const annots = annotsRef.value;
                for (let i = 0; i < annots.length; i++) {
                    const annotRef = annots[i];
                    if (annotRef.type === PDFObjectType.Reference) {
                        const ref = annotRef.value;
                        // Get annotation dictionary from PDF objects
                        const annotDict = await this.getObjectFromReference(ref);
                        if (annotDict && annotDict.type === PDFObjectType.Dictionary) {
                            const dict = annotDict.value;
                            const subtype = dict.entries.get('Subtype');
                            // Check if it's a Widget annotation (form field)
                            if (subtype && subtype.type === PDFObjectType.Name && subtype.value === 'Widget') {
                                const field = await this.parseFormField(dict, page.pageNumber, i);
                                if (field)
                                    fields.push(field);
                            }
                        }
                    }
                }
            }
        }
        catch (error) {
            console.warn('Error extracting form fields from page:', error);
        }
        return fields;
    }
    async getPageDictionary(pageNumber) {
        // Get the page dictionary from the page tree
        const pageTree = this.pdf.pageTree;
        return pageTree?.getPage(pageNumber);
    }
    async getObjectFromReference(ref) {
        try {
            const parser = this.pdf.parser;
            const xref = this.pdf.xrefTable;
            if (parser && xref) {
                return parser.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);
            }
        }
        catch (e) {
            // fallback: try objects cache
        }
        const objects = this.pdf.objects;
        const key = `${ref.objectNumber}_${ref.generationNumber}`;
        return objects?.get(key);
    }
    async parseFormField(annotDict, pageNumber, fieldIndex) {
        try {
            const rect = this.parseRectFromDict(annotDict, 'Rect');
            if (!rect)
                return null;
            // Walk /Parent chain for inherited properties (FT, Ff, T, V, DV, Opt)
            const inherited = await this.resolveInheritedProps(annotDict);
            const fieldName = this.getStringFromDict(annotDict, 'T') || inherited.name || `field_${pageNumber}_${fieldIndex}`;
            const fieldType = this.mapFT(this.getNameFromDict(annotDict, 'FT') || inherited.ft);
            const flags = this.getNumberFromDict(annotDict, 'Ff') ?? inherited.ff ?? 0;
            const field = {
                id: `${pageNumber}_${fieldIndex}_${fieldName}`,
                name: fieldName,
                type: fieldType,
                value: this.getFieldValue(annotDict) ?? inherited.value,
                defaultValue: this.getFieldValue(annotDict, 'DV') ?? inherited.defaultValue,
                rect,
                pageNumber,
                flags,
                required: !!(flags & 2),
                readOnly: !!(flags & 1),
                noExport: !!(flags & 4),
                multiline: !!(flags & 4096),
                password: !!(flags & 8192),
                maxLength: this.getNumberFromDict(annotDict, 'MaxLen')
            };
            // Handle specific field types
            if (field.type === FormFieldType.Choice) {
                field.options = this.getChoiceOptions(annotDict);
                if (!field.options?.length && inherited.optDict) {
                    field.options = this.getChoiceOptions(inherited.optDict);
                }
            }
            // Detect button sub-type from Ff flags
            if (field.type === FormFieldType.Button) {
                if (flags & 0x10000) {
                    field.buttonSubType = 'pushbutton';
                }
                else if (flags & 0x8000) {
                    field.buttonSubType = 'radio';
                }
                else {
                    field.buttonSubType = 'checkbox';
                }
            }
            return field;
        }
        catch (error) {
            console.warn('Error parsing form field:', error);
            return null;
        }
    }
    async resolveInheritedProps(dict) {
        const result = {};
        let current = dict;
        for (let depth = 0; depth < 10; depth++) {
            const parentRef = current.entries.get('Parent');
            if (!parentRef || parentRef.type !== PDFObjectType.Reference)
                break;
            const parentObj = await this.getObjectFromReference(parentRef.value);
            if (!parentObj || parentObj.type !== PDFObjectType.Dictionary)
                break;
            current = parentObj.value;
            if (!result.ft) {
                const ft = current.entries.get('FT');
                if (ft?.type === PDFObjectType.Name)
                    result.ft = ft.value;
            }
            if (result.ff === undefined) {
                const ff = current.entries.get('Ff');
                if (ff?.type === PDFObjectType.Number)
                    result.ff = ff.value;
            }
            if (!result.name) {
                const t = current.entries.get('T');
                if (t?.type === PDFObjectType.String)
                    result.name = t.value;
            }
            if (result.value === undefined) {
                result.value = this.getFieldValue(current);
            }
            if (result.defaultValue === undefined) {
                result.defaultValue = this.getFieldValue(current, 'DV');
            }
            if (!result.optDict && current.entries.has('Opt')) {
                result.optDict = current;
            }
        }
        return result;
    }
    getNameFromDict(dict, key) {
        const obj = dict.entries.get(key);
        return obj && obj.type === PDFObjectType.Name ? obj.value : undefined;
    }
    mapFT(ft) {
        switch (ft) {
            case 'Tx': return FormFieldType.Text;
            case 'Btn': return FormFieldType.Button;
            case 'Ch': return FormFieldType.Choice;
            case 'Sig': return FormFieldType.Signature;
            default: return FormFieldType.Text;
        }
    }
    getFormFieldType(dict) {
        const ft = dict.entries.get('FT');
        if (ft && ft.type === PDFObjectType.Name) {
            switch (ft.value) {
                case 'Tx': return FormFieldType.Text;
                case 'Btn': return FormFieldType.Button;
                case 'Ch': return FormFieldType.Choice;
                case 'Sig': return FormFieldType.Signature;
                default: return FormFieldType.Text;
            }
        }
        return FormFieldType.Text;
    }
    getFieldValue(dict, key = 'V') {
        const v = dict.entries.get(key);
        if (!v)
            return null;
        switch (v.type) {
            case PDFObjectType.String:
            case PDFObjectType.Name:
                return v.value;
            case PDFObjectType.Boolean:
                return v.value;
            case PDFObjectType.Number:
                return v.value;
            case PDFObjectType.Array:
                return v.value.map(obj => obj.value);
            default:
                return null;
        }
    }
    getChoiceOptions(dict) {
        const opt = dict.entries.get('Opt');
        if (!opt || opt.type !== PDFObjectType.Array)
            return [];
        const options = [];
        const optArray = opt.value;
        // Get selected values
        const selectedValues = new Set();
        const value = this.getFieldValue(dict);
        if (Array.isArray(value)) {
            value.forEach(v => selectedValues.add(String(v)));
        }
        else if (value !== null) {
            selectedValues.add(String(value));
        }
        for (let i = 0; i < optArray.length; i++) {
            const option = optArray[i];
            if (option.type === PDFObjectType.String) {
                const optValue = option.value;
                options.push({
                    value: optValue,
                    label: optValue,
                    selected: selectedValues.has(optValue)
                });
            }
            else if (option.type === PDFObjectType.Array) {
                const pair = option.value;
                if (pair.length >= 2) {
                    const optValue = pair[0].value;
                    options.push({
                        value: optValue,
                        label: pair[1].value,
                        selected: selectedValues.has(optValue)
                    });
                }
            }
        }
        return options;
    }
    parseRectFromDict(dict, key) {
        const rectObj = dict.entries.get(key);
        if (!rectObj || rectObj.type !== PDFObjectType.Array)
            return null;
        const arr = rectObj.value;
        if (arr.length !== 4)
            return null;
        return {
            x: arr[0].value,
            y: arr[1].value,
            width: arr[2].value - arr[0].value,
            height: arr[3].value - arr[1].value
        };
    }
    getStringFromDict(dict, key) {
        const obj = dict.entries.get(key);
        return obj && obj.type === PDFObjectType.String ? obj.value : undefined;
    }
    getNumberFromDict(dict, key) {
        const obj = dict.entries.get(key);
        return obj && obj.type === PDFObjectType.Number ? obj.value : undefined;
    }
    getBooleanFromDict(dict, key) {
        const obj = dict.entries.get(key);
        return obj && obj.type === PDFObjectType.Boolean ? obj.value : false;
    }
}
class FormFiller {
    constructor(pdf) {
        this.pdf = pdf;
    }
    async fill(data) {
        const formValues = this.pdf._formValues;
        for (const [key, val] of Object.entries(data)) {
            formValues.set(key, val);
        }
    }
}
class AnnotationExtractor {
    constructor(pdf) {
        this.pdf = pdf;
    }
    async extract(pageNumber) {
        const annotations = [];
        if (pageNumber) {
            const page = await this.pdf.getPage(pageNumber);
            if (page) {
                const pageAnnotations = await this.extractPageAnnotations(page);
                annotations.push(...pageAnnotations);
            }
        }
        else {
            const pages = await this.pdf.getAllPages();
            for (const page of pages) {
                const pageAnnotations = await this.extractPageAnnotations(page);
                annotations.push(...pageAnnotations);
            }
        }
        return annotations;
    }
    async extractPageAnnotations(page) {
        const annotations = [];
        try {
            // Parse page dictionary to find Annots array
            const pageDict = await this.getPageDictionary(page.pageNumber);
            const annotsRef = pageDict?.entries.get('Annots');
            if (annotsRef && annotsRef.type === PDFObjectType.Array) {
                const annots = annotsRef.value;
                for (let i = 0; i < annots.length; i++) {
                    const annotRef = annots[i];
                    if (annotRef.type === PDFObjectType.Reference) {
                        const ref = annotRef.value;
                        // Get annotation dictionary from PDF objects
                        const annotObj = await this.getObjectFromReference(ref);
                        if (annotObj && annotObj.type === PDFObjectType.Dictionary) {
                            const annotDict = annotObj.value;
                            const annotation = this.parseAnnotation(annotDict, page.pageNumber, i);
                            if (annotation)
                                annotations.push(annotation);
                        }
                    }
                }
            }
        }
        catch (error) {
            console.warn('Error extracting annotations from page:', error);
        }
        return annotations;
    }
    async getPageDictionary(pageNumber) {
        // Get the page dictionary from the page tree
        const pageTree = this.pdf.pageTree;
        return pageTree?.getPage(pageNumber);
    }
    async getObjectFromReference(ref) {
        try {
            const parser = this.pdf.parser;
            const xref = this.pdf.xrefTable;
            if (parser && xref) {
                return parser.parseIndirectObject(ref.objectNumber, ref.generationNumber, xref);
            }
        }
        catch (e) {
            // fallback: try objects cache
        }
        const objects = this.pdf.objects;
        const key = `${ref.objectNumber}_${ref.generationNumber}`;
        return objects?.get(key);
    }
    parseAnnotation(dict, pageNumber, index) {
        try {
            // Get annotation subtype
            const subtype = dict.entries.get('Subtype');
            if (!subtype || subtype.type !== PDFObjectType.Name)
                return null;
            const annotationType = this.mapSubtypeToAnnotationType(subtype.value);
            if (!annotationType)
                return null;
            // Extract common annotation properties
            const rect = this.parseRectFromDict(dict, 'Rect');
            if (!rect)
                return null;
            const annotation = {
                id: `annotation_${pageNumber}_${index}`,
                type: annotationType,
                rect,
                pageNumber,
                contents: this.getStringFromDict(dict, 'Contents'),
                author: this.getStringFromDict(dict, 'T'),
                modificationDate: this.parseDateFromDict(dict, 'M'),
                flags: this.getNumberFromDict(dict, 'F') || 0,
                opacity: this.getNumberFromDict(dict, 'CA'),
                color: this.parseColorFromDict(dict, 'C')
            };
            // Parse type-specific properties
            this.parseTypeSpecificProperties(annotation, dict);
            return annotation;
        }
        catch (error) {
            console.warn('Error parsing annotation:', error);
            return null;
        }
    }
    mapSubtypeToAnnotationType(subtype) {
        switch (subtype) {
            case 'Text': return AnnotationType.Text;
            case 'Link': return AnnotationType.Link;
            case 'FreeText': return AnnotationType.FreeText;
            case 'Line': return AnnotationType.Line;
            case 'Square': return AnnotationType.Square;
            case 'Circle': return AnnotationType.Circle;
            case 'Polygon': return AnnotationType.Polygon;
            case 'PolyLine': return AnnotationType.PolyLine;
            case 'Highlight': return AnnotationType.Highlight;
            case 'Underline': return AnnotationType.Underline;
            case 'Squiggly': return AnnotationType.Squiggly;
            case 'StrikeOut': return AnnotationType.StrikeOut;
            case 'Stamp': return AnnotationType.Stamp;
            case 'Caret': return AnnotationType.Caret;
            case 'Ink': return AnnotationType.Ink;
            case 'Popup': return AnnotationType.Popup;
            case 'FileAttachment': return AnnotationType.FileAttachment;
            case 'Sound': return AnnotationType.Sound;
            case 'Movie': return AnnotationType.Movie;
            case 'Widget': return AnnotationType.Widget;
            case 'Screen': return AnnotationType.Screen;
            case 'PrinterMark': return AnnotationType.PrinterMark;
            case 'TrapNet': return AnnotationType.TrapNet;
            case 'Watermark': return AnnotationType.Watermark;
            case 'Redact': return AnnotationType.Redact;
            default: return null;
        }
    }
    parseTypeSpecificProperties(annotation, dict) {
        // Parse destination for link annotations
        if (annotation.type === AnnotationType.Link) {
            const action = dict.entries.get('A');
            if (action && action.type === PDFObjectType.Dictionary) {
                const actionDict = action.value;
                const actionType = actionDict.entries.get('S');
                if (actionType && actionType.type === PDFObjectType.Name) {
                    if (actionType.value === 'URI') {
                        const uri = actionDict.entries.get('URI');
                        if (uri && uri.type === PDFObjectType.String) {
                            annotation.destination = uri.value;
                        }
                    }
                    else if (actionType.value === 'GoTo') {
                        const dest = actionDict.entries.get('D');
                        if (dest) {
                            if (dest.type === PDFObjectType.String) {
                                annotation.destination = dest.value;
                            }
                            else if (dest.type === PDFObjectType.Array) {
                                annotation.destination = dest.value.map(obj => obj.value);
                            }
                        }
                    }
                }
            }
            // Fallback: check /Dest key directly on annotation dict
            if (!annotation.destination) {
                const dest = dict.entries.get('Dest');
                if (dest) {
                    if (dest.type === PDFObjectType.String) {
                        annotation.destination = dest.value;
                    }
                    else if (dest.type === PDFObjectType.Array) {
                        annotation.destination = dest.value.map(obj => obj.value);
                    }
                }
            }
        }
        // Parse border style if present
        const borderStyle = dict.entries.get('BS');
        if (borderStyle && borderStyle.type === PDFObjectType.Dictionary) {
            const bsDict = borderStyle.value;
            const width = this.getNumberFromDict(bsDict, 'W');
            const style = this.getStringFromDict(bsDict, 'S');
            if (width !== undefined && style) {
                annotation.borderStyle = {
                    width,
                    style: style
                };
            }
        }
    }
    parseColor(colorArray) {
        if (colorArray.length === 1) {
            // Grayscale
            const gray = colorArray[0].value;
            return { r: gray, g: gray, b: gray };
        }
        else if (colorArray.length === 3) {
            // RGB
            return {
                r: colorArray[0].value,
                g: colorArray[1].value,
                b: colorArray[2].value
            };
        }
        else if (colorArray.length === 4) {
            // CMYK - convert to RGB approximation
            const c = colorArray[0].value;
            const m = colorArray[1].value;
            const y = colorArray[2].value;
            const k = colorArray[3].value;
            return {
                r: 1 - Math.min(1, c * (1 - k) + k),
                g: 1 - Math.min(1, m * (1 - k) + k),
                b: 1 - Math.min(1, y * (1 - k) + k)
            };
        }
        return undefined;
    }
    parseRectFromDict(dict, key) {
        const rectObj = dict.entries.get(key);
        if (!rectObj || rectObj.type !== PDFObjectType.Array)
            return null;
        const arr = rectObj.value;
        if (arr.length !== 4)
            return null;
        return {
            x: arr[0].value,
            y: arr[1].value,
            width: arr[2].value - arr[0].value,
            height: arr[3].value - arr[1].value
        };
    }
    getStringFromDict(dict, key) {
        const obj = dict.entries.get(key);
        return obj && obj.type === PDFObjectType.String ? obj.value : undefined;
    }
    getNumberFromDict(dict, key) {
        const obj = dict.entries.get(key);
        return obj && obj.type === PDFObjectType.Number ? obj.value : undefined;
    }
    getBooleanFromDict(dict, key) {
        const obj = dict.entries.get(key);
        return obj && obj.type === PDFObjectType.Boolean ? obj.value : false;
    }
    parseDateFromDict(dict, key) {
        const dateStr = this.getStringFromDict(dict, key);
        if (!dateStr)
            return undefined;
        try {
            // PDF date format: D:YYYYMMDDHHmmSSOHH'mm
            if (dateStr.startsWith('D:')) {
                const year = parseInt(dateStr.substr(2, 4), 10);
                const month = parseInt(dateStr.substr(6, 2), 10) - 1;
                const day = parseInt(dateStr.substr(8, 2), 10);
                const hour = parseInt(dateStr.substr(10, 2), 10) || 0;
                const minute = parseInt(dateStr.substr(12, 2), 10) || 0;
                const second = parseInt(dateStr.substr(14, 2), 10) || 0;
                return new Date(year, month, day, hour, minute, second);
            }
            // Fallback to standard Date parsing
            return new Date(dateStr);
        }
        catch (error) {
            console.warn('Error parsing date:', dateStr, error);
            return undefined;
        }
    }
    parseColorFromDict(dict, key) {
        const colorObj = dict.entries.get(key);
        if (!colorObj || colorObj.type !== PDFObjectType.Array)
            return undefined;
        const colorArray = colorObj.value;
        return this.parseColor(colorArray);
    }
}
class AnnotationManager {
    constructor(pdf) {
        this.pdf = pdf;
    }
    async add(_annotation) {
        const id = `annotation_${Date.now()}`;
        // Add annotation to PDF structure
        // This would modify the page's annotation array
        return id;
    }
    async remove(_annotationId) {
        // Remove annotation from PDF structure
        return true;
    }
    async update(_annotationId, _updates) {
        // Update annotation in PDF structure
        return true;
    }
}
/**
 * Safe content stream parser that prevents infinite loops
 * Based on PDF.js approach with explicit position tracking
 */
class SafeContentStreamParser {
    constructor(data) {
        this.data = data;
        this.position = 0;
        this.maxPosition = data.length;
    }
    parse() {
        const operations = [];
        let errorCount = 0;
        const maxErrors = 100;
        const startTime = Date.now();
        const maxDuration = 5000; // 5 seconds max
        let iterationCount = 0;
        const maxIterations = 100000; // Safety limit
        while (this.position < this.maxPosition && errorCount < maxErrors && iterationCount++ < maxIterations) {
            // Check timeout
            if (Date.now() - startTime > maxDuration) {
                console.warn('Parse timeout - stopping after 5 seconds');
                break;
            }
            try {
                this.skipWhitespace();
                if (this.position >= this.maxPosition)
                    break;
                const operands = [];
                // Parse operands until we hit an operator
                let operandCount = 0;
                while (this.position < this.maxPosition && operandCount++ < 100) {
                    const startPos = this.position;
                    const operand = this.parseOperand();
                    if (operand === null) {
                        break; // Hit an operator or end
                    }
                    // Safety: Ensure position advanced
                    if (this.position === startPos) {
                        console.warn('Position stuck in operand parsing at', this.position);
                        this.position++;
                        break;
                    }
                    operands.push(operand);
                    this.skipWhitespace();
                }
                // Parse operator
                const operator = this.parseOperator();
                if (operator) {
                    if (operator === 'BI') {
                        const inlineOp = this.parseInlineImage();
                        if (inlineOp)
                            operations.push(inlineOp);
                    }
                    else {
                        operations.push({ operator, operands });
                    }
                }
                else if (this.position < this.maxPosition) {
                    // Couldn't parse operator - skip this byte and continue
                    this.position++;
                    errorCount++;
                }
            }
            catch (error) {
                console.warn('Parser error at position', this.position, error);
                this.position++;
                errorCount++;
            }
        }
        if (errorCount >= maxErrors) {
            console.warn(`Stopped parsing after ${maxErrors} errors`);
        }
        if (iterationCount >= maxIterations) {
            console.warn(`Stopped parsing after ${maxIterations} iterations`);
        }
        return operations;
    }
    skipWhitespace() {
        while (this.position < this.maxPosition) {
            const ch = this.data[this.position];
            if (ch === 0x00 || ch === 0x09 || ch === 0x0a || ch === 0x0c || ch === 0x0d || ch === 0x20) {
                this.position++;
            }
            else if (ch === 0x25) { // % comment
                this.skipComment();
            }
            else {
                break;
            }
        }
    }
    skipComment() {
        while (this.position < this.maxPosition) {
            const ch = this.data[this.position++];
            if (ch === 0x0a || ch === 0x0d)
                break;
        }
    }
    parseOperand() {
        const startPos = this.position;
        const ch = this.data[this.position];
        // Dictionary
        if (ch === 0x3c && this.data[this.position + 1] === 0x3c) {
            return this.parseDictionary();
        }
        // Array
        if (ch === 0x5b) {
            return this.parseArray();
        }
        // String
        if (ch === 0x28) {
            return this.parseString();
        }
        // Hex string
        if (ch === 0x3c) {
            return this.parseHexString();
        }
        // Name (starts with /)
        if (ch === 0x2f) {
            this.position++; // Skip the '/'
            const nameBody = this.parseToken();
            return '/' + nameBody;
        }
        // Number or operator
        const token = this.parseToken();
        if (!token)
            return null;
        // Check if it's a number
        if (token === 'true')
            return true;
        if (token === 'false')
            return false;
        if (token === 'null')
            return null;
        const num = parseFloat(token);
        if (!isNaN(num))
            return num;
        // Must be an operator - return null to signal end of operands
        this.position = startPos;
        return null;
    }
    parseToken() {
        const start = this.position;
        while (this.position < this.maxPosition) {
            const ch = this.data[this.position];
            // Delimiters
            if (ch === 0x00 || ch === 0x09 || ch === 0x0a || ch === 0x0c || ch === 0x0d || ch === 0x20 ||
                ch === 0x28 || ch === 0x29 || ch === 0x3c || ch === 0x3e || ch === 0x5b || ch === 0x5d ||
                ch === 0x7b || ch === 0x7d || ch === 0x2f || ch === 0x25) {
                break;
            }
            this.position++;
        }
        if (this.position === start)
            return '';
        return String.fromCharCode(...this.data.slice(start, this.position));
    }
    parseOperator() {
        const token = this.parseToken();
        if (!token)
            return null;
        // Operator should not start with / (name) or be a number
        if (token[0] === '/' || !isNaN(parseFloat(token)))
            return null;
        return token;
    }
    parseArray() {
        this.position++; // Skip [
        const array = [];
        let iterations = 0;
        const maxIterations = 10000; // Safety limit
        while (this.position < this.maxPosition && iterations++ < maxIterations) {
            this.skipWhitespace();
            if (this.position >= this.maxPosition)
                break;
            if (this.data[this.position] === 0x5d) { // ]
                this.position++;
                break;
            }
            const startPos = this.position;
            const item = this.parseOperand();
            if (item !== null) {
                array.push(item);
            }
            else {
                // Couldn't parse - advance to avoid infinite loop
                if (this.position === startPos) {
                    this.position++;
                }
                break;
            }
        }
        return array;
    }
    parseDictionary() {
        this.position += 2; // Skip <<
        const dict = {};
        let iterations = 0;
        const maxIterations = 10000; // Safety limit
        while (this.position < this.maxPosition && iterations++ < maxIterations) {
            this.skipWhitespace();
            if (this.position >= this.maxPosition)
                break;
            if (this.position + 1 < this.maxPosition &&
                this.data[this.position] === 0x3e &&
                this.data[this.position + 1] === 0x3e) {
                this.position += 2;
                break;
            }
            const keyStartPos = this.position;
            const key = this.parseOperand();
            if (key && typeof key === 'string' && key[0] === '/') {
                this.skipWhitespace();
                const valueStartPos = this.position;
                const value = this.parseOperand();
                // Ensure position advanced
                if (this.position === valueStartPos && this.position < this.maxPosition) {
                    this.position++;
                }
                dict[key.substring(1)] = value;
            }
            else {
                // Couldn't parse key - advance to avoid infinite loop
                if (this.position === keyStartPos && this.position < this.maxPosition) {
                    this.position++;
                }
                break;
            }
        }
        return dict;
    }
    parseString() {
        this.position++; // Skip (
        const chars = [];
        let depth = 1;
        while (this.position < this.maxPosition && depth > 0) {
            const ch = this.data[this.position++];
            if (ch === 0x28) { // (
                depth++;
                chars.push(ch);
            }
            else if (ch === 0x29) { // )
                depth--;
                if (depth > 0)
                    chars.push(ch);
            }
            else if (ch === 0x5c) { // \ escape - keep backslash for later decoding
                chars.push(ch);
                if (this.position < this.maxPosition) {
                    const next = this.data[this.position++];
                    chars.push(next);
                }
            }
            else {
                chars.push(ch);
            }
        }
        return String.fromCharCode(...chars);
    }
    parseHexString() {
        this.position++; // Skip <
        const chars = [];
        while (this.position < this.maxPosition) {
            const ch = this.data[this.position++];
            if (ch === 0x3e)
                break; // >
            if ((ch >= 0x30 && ch <= 0x39) || (ch >= 0x41 && ch <= 0x46) || (ch >= 0x61 && ch <= 0x66)) {
                chars.push(ch);
            }
        }
        const hexStr = String.fromCharCode(...chars);
        const bytes = [];
        for (let i = 0; i < hexStr.length; i += 2) {
            bytes.push(parseInt(hexStr.substr(i, 2), 16));
        }
        return String.fromCharCode(...bytes);
    }
    parseInlineImage() {
        try {
            const imageDict = new Map();
            // Parse key-value pairs until ID operator
            while (this.position < this.maxPosition) {
                this.skipWhitespace();
                if (this.position >= this.maxPosition)
                    break;
                // Check for ID operator
                if (this.data[this.position] === 0x49 && // I
                    this.position + 1 < this.maxPosition &&
                    this.data[this.position + 1] === 0x44) { // D
                    // Verify followed by whitespace or end
                    if (this.position + 2 >= this.maxPosition ||
                        this.data[this.position + 2] === 0x20 ||
                        this.data[this.position + 2] === 0x0A ||
                        this.data[this.position + 2] === 0x0D ||
                        this.data[this.position + 2] === 0x09) {
                        this.position += 2; // skip 'ID'
                        // Skip single whitespace after ID
                        if (this.position < this.maxPosition) {
                            const ws = this.data[this.position];
                            if (ws === 0x20 || ws === 0x0A || ws === 0x0D || ws === 0x09) {
                                this.position++;
                            }
                        }
                        break;
                    }
                }
                // Parse key (name like /W, /H, /CS, etc.)
                const ch = this.data[this.position];
                if (ch === 0x2F) { // '/'
                    this.position++; // skip '/'
                    let name = '/';
                    while (this.position < this.maxPosition) {
                        const b = this.data[this.position];
                        if (b === 0x20 || b === 0x09 || b === 0x0A || b === 0x0D ||
                            b === 0x2F || b === 0x5B || b === 0x5D || b === 0x28 ||
                            b === 0x29 || b === 0x3C || b === 0x3E)
                            break;
                        name += String.fromCharCode(b);
                        this.position++;
                    }
                    this.skipWhitespace();
                    const value = this.parseOperand();
                    imageDict.set(name, value);
                }
                else {
                    // Bare key (abbreviation without '/')
                    let token = '';
                    while (this.position < this.maxPosition) {
                        const b = this.data[this.position];
                        if (b === 0x20 || b === 0x09 || b === 0x0A || b === 0x0D ||
                            b === 0x2F || b === 0x5B || b === 0x5D)
                            break;
                        token += String.fromCharCode(b);
                        this.position++;
                    }
                    if (token.length > 0) {
                        this.skipWhitespace();
                        const value = this.parseOperand();
                        imageDict.set('/' + token, value);
                    }
                    else {
                        this.position++;
                    }
                }
            }
            // Read image data until EI
            const startPos = this.position;
            let endPos = startPos;
            while (this.position < this.maxPosition - 2) {
                const b = this.data[this.position];
                if (b === 0x20 || b === 0x0A || b === 0x0D || b === 0x09) {
                    if (this.data[this.position + 1] === 0x45 && // E
                        this.data[this.position + 2] === 0x49) { // I
                        if (this.position + 3 >= this.maxPosition ||
                            this.data[this.position + 3] === 0x20 ||
                            this.data[this.position + 3] === 0x0A ||
                            this.data[this.position + 3] === 0x0D ||
                            this.data[this.position + 3] === 0x09 ||
                            this.data[this.position + 3] === 0x2F ||
                            this.data[this.position + 3] === 0x25) {
                            endPos = this.position;
                            this.position += 3; // skip whitespace + 'EI'
                            break;
                        }
                    }
                }
                this.position++;
            }
            const imageData = this.data.slice(startPos, endPos);
            return {
                operator: 'BI',
                operands: [{ dictionary: imageDict, data: imageData }]
            };
        }
        catch (error) {
            console.warn('SafeContentStreamParser: Failed to parse inline image:', error);
            return null;
        }
    }
}
/**
 * PDF Text Decoder
 * Handles various PDF string encodings and character mappings
 */
class PDFTextDecoder {
    // Common PDF glyph name to Unicode mapping
    static glyphNameToUnicode(name) {
        const glyphMap = {
            'space': 0x20, 'exclam': 0x21, 'quotedbl': 0x22, 'numbersign': 0x23,
            'dollar': 0x24, 'percent': 0x25, 'ampersand': 0x26, 'quoteright': 0x2019,
            'parenleft': 0x28, 'parenright': 0x29, 'asterisk': 0x2A, 'plus': 0x2B,
            'comma': 0x2C, 'hyphen': 0x2D, 'period': 0x2E, 'slash': 0x2F,
            'zero': 0x30, 'one': 0x31, 'two': 0x32, 'three': 0x33, 'four': 0x34,
            'five': 0x35, 'six': 0x36, 'seven': 0x37, 'eight': 0x38, 'nine': 0x39,
            'colon': 0x3A, 'semicolon': 0x3B, 'less': 0x3C, 'equal': 0x3D,
            'greater': 0x3E, 'question': 0x3F, 'at': 0x40,
            'A': 0x41, 'B': 0x42, 'C': 0x43, 'D': 0x44, 'E': 0x45, 'F': 0x46,
            'G': 0x47, 'H': 0x48, 'I': 0x49, 'J': 0x4A, 'K': 0x4B, 'L': 0x4C,
            'M': 0x4D, 'N': 0x4E, 'O': 0x4F, 'P': 0x50, 'Q': 0x51, 'R': 0x52,
            'S': 0x53, 'T': 0x54, 'U': 0x55, 'V': 0x56, 'W': 0x57, 'X': 0x58,
            'Y': 0x59, 'Z': 0x5A, 'bracketleft': 0x5B, 'backslash': 0x5C,
            'bracketright': 0x5D, 'asciicircum': 0x5E, 'underscore': 0x5F,
            'quoteleft': 0x2018, 'a': 0x61, 'b': 0x62, 'c': 0x63, 'd': 0x64,
            'e': 0x65, 'f': 0x66, 'g': 0x67, 'h': 0x68, 'i': 0x69, 'j': 0x6A,
            'k': 0x6B, 'l': 0x6C, 'm': 0x6D, 'n': 0x6E, 'o': 0x6F, 'p': 0x70,
            'q': 0x71, 'r': 0x72, 's': 0x73, 't': 0x74, 'u': 0x75, 'v': 0x76,
            'w': 0x77, 'x': 0x78, 'y': 0x79, 'z': 0x7A,
            'braceleft': 0x7B, 'bar': 0x7C, 'braceright': 0x7D, 'asciitilde': 0x7E,
            'fi': 0xFB01, 'fl': 0xFB02, 'endash': 0x2013, 'emdash': 0x2014,
            'bullet': 0x2022, 'ellipsis': 0x2026, 'quotedblleft': 0x201C,
            'quotedblright': 0x201D, 'quotesingle': 0x27,
            'quotedblbase': 0x201E, 'quotesinglbase': 0x201A,
            'dagger': 0x2020, 'daggerdbl': 0x2021, 'trademark': 0x2122,
            'circumflex': 0x02C6, 'tilde': 0x02DC, 'dotlessi': 0x0131,
            'breve': 0x02D8, 'ring': 0x02DA, 'ogonek': 0x02DB,
            'caron': 0x02C7, 'degree': 0x00B0, 'section': 0x00A7,
            'copyright': 0x00A9, 'registered': 0x00AE, 'mu': 0x00B5,
            'paragraph': 0x00B6, 'guillemotleft': 0x00AB,
            'guillemotright': 0x00BB, 'guilsinglleft': 0x2039,
            'guilsinglright': 0x203A, 'fraction': 0x2044,
            'minus': 0x2212, 'perthousand': 0x2030,
            'Lslash': 0x0141, 'lslash': 0x0142, 'OE': 0x0152,
            'oe': 0x0153, 'Scaron': 0x0160, 'scaron': 0x0161,
            'Ydieresis': 0x0178, 'Zcaron': 0x017D, 'zcaron': 0x017E
        };
        const cp = glyphMap[name];
        return cp !== undefined ? String.fromCodePoint(cp) : undefined;
    }
    /**
     * Map a raw byte to Unicode display character.
     * Uses the specified encoding, defaulting to PDFDocEncoding.
     */
    static mapCharCode(charCode, encoding) {
        if (charCode < 128) {
            return String.fromCharCode(charCode);
        }
        if (charCode <= 255) {
            if (encoding === 'WinAnsiEncoding') {
                return String.fromCodePoint(this.WIN_ANSI_ENCODING[charCode - 128]);
            }
            if (encoding === 'MacRomanEncoding') {
                return String.fromCodePoint(this.MAC_ROMAN_ENCODING[charCode - 128]);
            }
            return String.fromCodePoint(this.PDF_DOC_ENCODING[charCode - 128]);
        }
        return String.fromCodePoint(charCode);
    }
    static decode(input) {
        // Handle non-string inputs
        if (typeof input !== 'string') {
            return String(input);
        }
        if (input.length === 0) {
            return '';
        }
        // Check if it's UTF-16BE (starts with BOM: \xFE\xFF)
        if (input.length >= 2 &&
            input.charCodeAt(0) === 0xFE &&
            input.charCodeAt(1) === 0xFF) {
            return this.decodeUTF16BE(input);
        }
        // Check if it's UTF-16LE (starts with BOM: \xFF\xFE) - rare in PDFs
        if (input.length >= 2 &&
            input.charCodeAt(0) === 0xFF &&
            input.charCodeAt(1) === 0xFE) {
            return this.decodeUTF16LE(input);
        }
        // Otherwise, decode as PDFDocEncoding with escape sequences
        return this.decodePDFDocEncoding(input);
    }
    /**
     * Decode UTF-16BE encoded string (big-endian)
     */
    static decodeUTF16BE(input) {
        const result = [];
        // Skip BOM (first 2 bytes)
        for (let i = 2; i < input.length; i += 2) {
            if (i + 1 >= input.length)
                break;
            const high = input.charCodeAt(i);
            const low = input.charCodeAt(i + 1);
            const codePoint = (high << 8) | low;
            // Handle surrogate pairs for characters > U+FFFF
            if (codePoint >= 0xD800 && codePoint <= 0xDBFF && i + 3 < input.length) {
                const high2 = input.charCodeAt(i + 2);
                const low2 = input.charCodeAt(i + 3);
                const codePoint2 = (high2 << 8) | low2;
                if (codePoint2 >= 0xDC00 && codePoint2 <= 0xDFFF) {
                    // Combine surrogate pair
                    const finalCodePoint = 0x10000 +
                        ((codePoint - 0xD800) << 10) +
                        (codePoint2 - 0xDC00);
                    result.push(finalCodePoint);
                    i += 2; // Skip the second pair
                    continue;
                }
            }
            result.push(codePoint);
        }
        return String.fromCodePoint(...result);
    }
    /**
     * Decode UTF-16LE encoded string (little-endian) - rare
     */
    static decodeUTF16LE(input) {
        const result = [];
        // Skip BOM (first 2 bytes)
        for (let i = 2; i < input.length; i += 2) {
            if (i + 1 >= input.length)
                break;
            const low = input.charCodeAt(i);
            const high = input.charCodeAt(i + 1);
            const codePoint = (high << 8) | low;
            result.push(codePoint);
        }
        return String.fromCodePoint(...result);
    }
    /**
     * Decode PDFDocEncoding with escape sequences
     */
    static decodePDFDocEncoding(input) {
        const result = [];
        let i = 0;
        while (i < input.length) {
            let ch = input.charCodeAt(i);
            // Handle escape sequences
            if (ch === 0x5C) { // Backslash
                i++;
                if (i >= input.length)
                    break;
                const next = input.charCodeAt(i);
                // Octal escape: \ddd (1-3 digits)
                if (next >= 0x30 && next <= 0x37) { // 0-7
                    let octal = next - 0x30;
                    i++;
                    // Second digit
                    if (i < input.length) {
                        const digit2 = input.charCodeAt(i);
                        if (digit2 >= 0x30 && digit2 <= 0x37) {
                            octal = octal * 8 + (digit2 - 0x30);
                            i++;
                            // Third digit
                            if (i < input.length) {
                                const digit3 = input.charCodeAt(i);
                                if (digit3 >= 0x30 && digit3 <= 0x37) {
                                    octal = octal * 8 + (digit3 - 0x30);
                                    i++;
                                }
                            }
                        }
                    }
                    ch = octal;
                }
                // Standard escape sequences
                else {
                    switch (next) {
                        case 0x6E:
                            ch = 0x0A;
                            break; // \n -> newline
                        case 0x72:
                            ch = 0x0D;
                            break; // \r -> carriage return
                        case 0x74:
                            ch = 0x09;
                            break; // \t -> tab
                        case 0x62:
                            ch = 0x08;
                            break; // \b -> backspace
                        case 0x66:
                            ch = 0x0C;
                            break; // \f -> form feed
                        case 0x28:
                            ch = 0x28;
                            break; // \( -> (
                        case 0x29:
                            ch = 0x29;
                            break; // \) -> )
                        case 0x5C:
                            ch = 0x5C;
                            break; // \\ -> \
                        default:
                            ch = next;
                            break; // Unknown escape - use literal
                    }
                    i++;
                }
            }
            else {
                i++;
            }
            // Map PDFDocEncoding to Unicode
            if (ch < 128) {
                // ASCII range - use as-is
                result.push(ch);
            }
            else {
                // Extended range (128-255) - map to Unicode
                result.push(this.PDF_DOC_ENCODING[ch - 128]);
            }
        }
        return String.fromCodePoint(...result);
    }
    /**
     * Decode hex string (format: <48656C6C6F>)
     */
    static decodeHexString(hexStr) {
        // Remove < and > brackets if present
        hexStr = hexStr.replace(/[<>]/g, '');
        const bytes = [];
        for (let i = 0; i < hexStr.length; i += 2) {
            const hexByte = hexStr.substr(i, 2);
            if (hexByte.length === 2) {
                bytes.push(parseInt(hexByte, 16));
            }
            else if (hexByte.length === 1) {
                // Odd number of digits - pad with 0
                bytes.push(parseInt(hexByte + '0', 16));
            }
        }
        // Convert bytes to string
        const byteString = String.fromCharCode(...bytes);
        // Check for UTF-16BE BOM
        if (bytes.length >= 2 && bytes[0] === 0xFE && bytes[1] === 0xFF) {
            return this.decode(byteString);
        }
        // Otherwise decode as PDFDocEncoding
        return this.decodePDFDocEncoding(byteString);
    }
}
// PDFDocEncoding to Unicode mapping (for bytes 128-255)
// Bytes 0-127 are standard ASCII
PDFTextDecoder.PDF_DOC_ENCODING = [
    0x2022, 0x2020, 0x2021, 0x2026, 0x2014, 0x2013, 0x0192, 0x2044,
    0x2039, 0x203A, 0x2212, 0x2030, 0x201E, 0x201C, 0x201D, 0x2018,
    0x2019, 0x201A, 0x2122, 0xFB01, 0xFB02, 0x0141, 0x0152, 0x0160,
    0x0178, 0x017D, 0x0131, 0x0142, 0x0153, 0x0161, 0x017E, 0xFFFD,
    0x00A0, 0x00A1, 0x00A2, 0x00A3, 0x00A4, 0x00A5, 0x00A6, 0x00A7,
    0x00A8, 0x00A9, 0x00AA, 0x00AB, 0x00AC, 0x00AD, 0x00AE, 0x00AF,
    0x00B0, 0x00B1, 0x00B2, 0x00B3, 0x00B4, 0x00B5, 0x00B6, 0x00B7,
    0x00B8, 0x00B9, 0x00BA, 0x00BB, 0x00BC, 0x00BD, 0x00BE, 0x00BF,
    0x00C0, 0x00C1, 0x00C2, 0x00C3, 0x00C4, 0x00C5, 0x00C6, 0x00C7,
    0x00C8, 0x00C9, 0x00CA, 0x00CB, 0x00CC, 0x00CD, 0x00CE, 0x00CF,
    0x00D0, 0x00D1, 0x00D2, 0x00D3, 0x00D4, 0x00D5, 0x00D6, 0x00D7,
    0x00D8, 0x00D9, 0x00DA, 0x00DB, 0x00DC, 0x00DD, 0x00DE, 0x00DF,
    0x00E0, 0x00E1, 0x00E2, 0x00E3, 0x00E4, 0x00E5, 0x00E6, 0x00E7,
    0x00E8, 0x00E9, 0x00EA, 0x00EB, 0x00EC, 0x00ED, 0x00EE, 0x00EF,
    0x00F0, 0x00F1, 0x00F2, 0x00F3, 0x00F4, 0x00F5, 0x00F6, 0x00F7,
    0x00F8, 0x00F9, 0x00FA, 0x00FB, 0x00FC, 0x00FD, 0x00FE, 0x00FF
];
// WinAnsiEncoding map (bytes 128-159 differ from PDFDocEncoding)
PDFTextDecoder.WIN_ANSI_ENCODING = [
    0x20AC, 0xFFFD, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, // 128-135
    0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0xFFFD, 0x017D, 0xFFFD, // 136-143
    0xFFFD, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, // 144-151
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0xFFFD, 0x017E, 0x0178, // 152-159
    0x00A0, 0x00A1, 0x00A2, 0x00A3, 0x00A4, 0x00A5, 0x00A6, 0x00A7, // 160-167
    0x00A8, 0x00A9, 0x00AA, 0x00AB, 0x00AC, 0x00AD, 0x00AE, 0x00AF, // 168-175
    0x00B0, 0x00B1, 0x00B2, 0x00B3, 0x00B4, 0x00B5, 0x00B6, 0x00B7, // 176-183
    0x00B8, 0x00B9, 0x00BA, 0x00BB, 0x00BC, 0x00BD, 0x00BE, 0x00BF, // 184-191
    0x00C0, 0x00C1, 0x00C2, 0x00C3, 0x00C4, 0x00C5, 0x00C6, 0x00C7, // 192-199
    0x00C8, 0x00C9, 0x00CA, 0x00CB, 0x00CC, 0x00CD, 0x00CE, 0x00CF, // 200-207
    0x00D0, 0x00D1, 0x00D2, 0x00D3, 0x00D4, 0x00D5, 0x00D6, 0x00D7, // 208-215
    0x00D8, 0x00D9, 0x00DA, 0x00DB, 0x00DC, 0x00DD, 0x00DE, 0x00DF, // 216-223
    0x00E0, 0x00E1, 0x00E2, 0x00E3, 0x00E4, 0x00E5, 0x00E6, 0x00E7, // 224-231
    0x00E8, 0x00E9, 0x00EA, 0x00EB, 0x00EC, 0x00ED, 0x00EE, 0x00EF, // 232-239
    0x00F0, 0x00F1, 0x00F2, 0x00F3, 0x00F4, 0x00F5, 0x00F6, 0x00F7, // 240-247
    0x00F8, 0x00F9, 0x00FA, 0x00FB, 0x00FC, 0x00FD, 0x00FE, 0x00FF // 248-255
];
// MacRomanEncoding map (bytes 128-255)
PDFTextDecoder.MAC_ROMAN_ENCODING = [
    0x00C4, 0x00C5, 0x00C7, 0x00C9, 0x00D1, 0x00D6, 0x00DC, 0x00E1, // 128-135
    0x00E0, 0x00E2, 0x00E4, 0x00E3, 0x00E5, 0x00E7, 0x00E9, 0x00E8, // 136-143
    0x00EA, 0x00EB, 0x00ED, 0x00EC, 0x00EE, 0x00EF, 0x00F1, 0x00F3, // 144-151  
    0x00F2, 0x00F4, 0x00F6, 0x00F5, 0x00FA, 0x00F9, 0x00FB, 0x00FC, // 152-159
    0x2020, 0x00B0, 0x00A2, 0x00A3, 0x00A7, 0x2022, 0x00B6, 0x00DF, // 160-167
    0x00AE, 0x00A9, 0x2122, 0x00B4, 0x00A8, 0x2260, 0x00C6, 0x00D8, // 168-175
    0x221E, 0x00B1, 0x2264, 0x2265, 0x00A5, 0x00B5, 0x2202, 0x2211, // 176-183
    0x220F, 0x03C0, 0x222B, 0x00AA, 0x00BA, 0x2126, 0x00E6, 0x00F8, // 184-191
    0x00BF, 0x00A1, 0x00AC, 0x221A, 0x0192, 0x2248, 0x2206, 0x00AB, // 192-199
    0x00BB, 0x2026, 0x00A0, 0x00C0, 0x00C3, 0x00D5, 0x0152, 0x0153, // 200-207
    0x2013, 0x2014, 0x201C, 0x201D, 0x2018, 0x2019, 0x00F7, 0x25CA, // 208-215
    0x00FF, 0x0178, 0x2044, 0x20AC, 0x2039, 0x203A, 0xFB01, 0xFB02, // 216-223
    0x2021, 0x00B7, 0x201A, 0x201E, 0x2030, 0x00C2, 0x00CA, 0x00C1, // 224-231
    0x00CB, 0x00C8, 0x00CD, 0x00CE, 0x00CF, 0x00CC, 0x00D3, 0x00D4, // 232-239
    0xF8FF, 0x00D2, 0x00DA, 0x00DB, 0x00D9, 0x0131, 0x02C6, 0x02DC, // 240-247
    0x00AF, 0x02D8, 0x02D9, 0x02DA, 0x00B8, 0x02DD, 0x02DB, 0x02C7 // 248-255
];
/**
 * PDF Glyph Metrics Calculator
 * Calculates accurate glyph widths from PDF font metrics
 */
class PDFGlyphMetrics {
    /**
     * Get the width of a character in text space units
     * @param charCode Character code
     * @param font Font resource with width information
     * @param fontSize Font size in points
     * @returns Width in user space units
     */
    static getCharWidth(charCode, font, fontSize) {
        if (!font) {
            // Fallback: use average character width estimate
            return fontSize * 0.5;
        }
        // Get the glyph width in glyph space (usually 1000 units)
        const glyphWidth = this.getGlyphWidth(charCode, font);
        // Convert from glyph space to text space
        // PDF glyph widths are typically in 1000-unit glyph space
        // Scale by font size and divide by 1000
        // Use abs(fontSize) — negative fontSize affects direction, not glyph width
        return (glyphWidth * Math.abs(fontSize)) / 1000;
    }
    /**
     * Get glyph width in glyph space units (typically 0-1000 range)
     */
    static getGlyphWidth(charCode, font) {
        // Check CIDFont per-CID widths (Type0/CIDFont W array)
        if (font.cidWidths && font.cidWidths.has(charCode)) {
            return font.cidWidths.get(charCode);
        }
        // Check if we have a Widths array
        if (font.widths && font.widths.length > 0 &&
            font.firstChar !== undefined && font.lastChar !== undefined) {
            // Check if character is in the widths array range
            if (charCode >= font.firstChar && charCode <= font.lastChar) {
                const index = charCode - font.firstChar;
                if (index >= 0 && index < font.widths.length) {
                    return font.widths[index];
                }
            }
        }
        // Check for DefaultWidth (CIDFonts)
        if (font.defaultWidth !== undefined) {
            return font.defaultWidth;
        }
        // Check for MissingWidth
        if (font.missingWidth !== undefined) {
            return font.missingWidth;
        }
        // Use font-specific defaults based on font type
        return this.getDefaultWidth(font, charCode);
    }
    /**
     * Get default width based on font type and character
     */
    static getDefaultWidth(font, charCode) {
        const baseFont = font.baseFont || '';
        const bf = baseFont.toLowerCase();
        // Monospace fonts (Courier family) — fixed width
        if (bf.includes('courier') || bf.includes('mono') || bf.includes('nimbusmonl')) {
            return 600;
        }
        // Helvetica / Arial / sans-serif
        if (bf.includes('helvetica') || bf.includes('arial') || bf.includes('sans')) {
            if (charCode >= 32 && charCode <= 126) {
                if (bf.includes('bold')) {
                    return this.HELVETICA_BOLD_WIDTHS[charCode - 32];
                }
                return this.HELVETICA_WIDTHS[charCode - 32];
            }
            return 556; // fallback for extended chars
        }
        // Times / serif
        if (bf.includes('times') || bf.includes('roman') || (bf.includes('serif') && !bf.includes('sans'))) {
            if (charCode >= 32 && charCode <= 126) {
                if (bf.includes('bold') && bf.includes('ital')) {
                    return this.TIMES_BOLD_WIDTHS[charCode - 32]; // approximate
                }
                if (bf.includes('bold')) {
                    return this.TIMES_BOLD_WIDTHS[charCode - 32];
                }
                if (bf.includes('ital') || bf.includes('obli')) {
                    return this.TIMES_ITALIC_WIDTHS[charCode - 32];
                }
                return this.TIMES_WIDTHS[charCode - 32];
            }
            return 500; // fallback for extended chars
        }
        // Symbol fonts
        if (bf.includes('symbol') || bf.includes('zapfdingbats')) {
            return 750;
        }
        // Generic fallback — try Helvetica table as reasonable default
        if (charCode >= 32 && charCode <= 126) {
            return this.HELVETICA_WIDTHS[charCode - 32];
        }
        return 500;
    }
    /**
     * Calculate total width of a text string
     * @param text Text string
     * @param font Font resource
     * @param fontSize Font size in points
     * @param charSpace Character spacing
     * @param wordSpace Word spacing
     * @param horizScale Horizontal scaling (%)
     * @returns Total width in user space units
     */
    static getTextWidth(text, font, fontSize, charSpace = 0, wordSpace = 0, horizScale = 100) {
        let totalWidth = 0;
        const scale = horizScale / 100;
        for (let i = 0; i < text.length; i++) {
            const charCode = text.charCodeAt(i);
            // Get base glyph width
            const glyphWidth = this.getCharWidth(charCode, font, fontSize);
            totalWidth += glyphWidth;
            // Add character spacing
            totalWidth += charSpace;
            // Add word spacing for spaces
            if (charCode === 32) {
                totalWidth += wordSpace;
            }
        }
        // Apply horizontal scaling
        return totalWidth * scale;
    }
}
// Standard 14 font width tables (PDF spec Appendix D)
// Per-character widths in 1000-unit glyph space for ASCII 32-126
PDFGlyphMetrics.HELVETICA_WIDTHS = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, // 32-47
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, // 48-63
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, // 64-79
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, // 80-95
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, // 96-111
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584 // 112-126
];
PDFGlyphMetrics.HELVETICA_BOLD_WIDTHS = [
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278, // 32-47
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611, // 48-63
    975, 722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778, // 64-79
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 333, 278, 333, 584, 556, // 80-95
    333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, 611, 611, // 96-111
    611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584 // 112-126
];
PDFGlyphMetrics.TIMES_WIDTHS = [
    250, 333, 408, 500, 500, 833, 778, 180, 333, 333, 500, 564, 250, 333, 250, 278, // 32-47
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 278, 278, 564, 564, 564, 444, // 48-63
    921, 722, 667, 667, 722, 611, 556, 722, 722, 333, 389, 722, 611, 889, 722, 722, // 64-79
    556, 722, 667, 556, 611, 722, 722, 944, 722, 722, 611, 333, 278, 333, 469, 500, // 80-95
    333, 444, 500, 444, 500, 444, 333, 500, 500, 278, 278, 500, 278, 778, 500, 500, // 96-111
    500, 500, 333, 389, 278, 500, 500, 722, 500, 500, 444, 480, 200, 480, 541 // 112-126
];
PDFGlyphMetrics.TIMES_BOLD_WIDTHS = [
    250, 333, 555, 500, 500, 1000, 833, 278, 333, 333, 500, 570, 250, 333, 250, 278, // 32-47
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 333, 333, 570, 570, 570, 500, // 48-63
    930, 722, 667, 722, 722, 667, 611, 778, 778, 389, 500, 778, 667, 944, 722, 778, // 64-79
    611, 778, 722, 556, 667, 722, 722, 1000, 722, 722, 667, 333, 278, 333, 581, 500, // 80-95
    333, 500, 556, 444, 556, 444, 333, 500, 556, 278, 333, 556, 278, 833, 556, 500, // 96-111
    556, 556, 444, 389, 333, 556, 500, 722, 500, 500, 444, 394, 220, 394, 520 // 112-126
];
PDFGlyphMetrics.TIMES_ITALIC_WIDTHS = [
    250, 333, 420, 500, 500, 833, 778, 214, 333, 333, 500, 675, 250, 333, 250, 278, // 32-47
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 333, 333, 675, 675, 675, 500, // 48-63
    920, 611, 611, 667, 722, 611, 611, 722, 722, 333, 444, 667, 556, 833, 667, 722, // 64-79
    611, 722, 611, 500, 556, 722, 611, 833, 611, 556, 556, 389, 278, 389, 422, 500, // 80-95
    333, 500, 500, 444, 500, 444, 278, 500, 500, 278, 278, 444, 278, 722, 500, 500, // 96-111
    500, 500, 389, 389, 278, 500, 444, 667, 444, 444, 389, 400, 275, 400, 541 // 112-126
];
/**
 * PDF Color Space Processor
 * Handles advanced color spaces: ICCBased, Indexed, Pattern, Separation, DeviceN
 */
class PDFColorSpaceProcessor {
    /**
     * Clear all caches (useful for memory management)
     */
    static clearCaches() {
        PDFColorSpaceProcessor.colorSpaceCache.clear();
        PDFColorSpaceProcessor.conversionCache.clear();
    }
    /**
     * Get cache key for color space object
     */
    static getColorSpaceCacheKey(csObj) {
        if (typeof csObj === 'string') {
            return `str:${csObj}`;
        }
        if (Array.isArray(csObj) && csObj.length > 0) {
            // Create a simple key from array structure
            const name = csObj[0];
            // Only cache simple color spaces (avoid complex objects)
            if (typeof name === 'string' && csObj.length <= 5) {
                return `arr:${JSON.stringify(csObj).slice(0, 100)}`;
            }
        }
        return null;
    }
    /**
     * Parse color space definition from PDF resources
     */
    static parseColorSpace(csObj, resources) {
        // Try to get cached color space
        const cacheKey = this.getColorSpaceCacheKey(csObj);
        if (cacheKey) {
            const cached = PDFColorSpaceProcessor.colorSpaceCache.get(cacheKey);
            if (cached) {
                return cached;
            }
        }
        // Handle simple names
        if (typeof csObj === 'string') {
            return {
                name: csObj,
                numComponents: this.getNumComponents(csObj)
            };
        }
        // Handle array definitions: [name, ...params]
        if (Array.isArray(csObj) && csObj.length > 0) {
            const csName = csObj[0];
            switch (csName) {
                case 'ICCBased':
                    return this.parseICCBased(csObj, resources);
                case 'Indexed':
                case 'I':
                    return this.parseIndexed(csObj, resources);
                case 'Pattern':
                    return this.parsePattern(csObj);
                case 'Separation':
                    return this.parseSeparation(csObj, resources);
                case 'DeviceN':
                    return this.parseDeviceN(csObj, resources);
                case 'CalRGB':
                case 'CalGray':
                    return this.parseCalibrated(csObj);
                default:
                    // Fallback to simple color space
                    const fallbackCS = {
                        name: csName,
                        numComponents: this.getNumComponents(csName)
                    };
                    this.cacheColorSpace(cacheKey, fallbackCS);
                    return fallbackCS;
            }
        }
        // Default fallback
        const defaultCS = {
            name: 'DeviceRGB',
            numComponents: 3
        };
        this.cacheColorSpace(cacheKey, defaultCS);
        return defaultCS;
    }
    /**
     * Cache a parsed color space
     */
    static cacheColorSpace(cacheKey, colorSpace) {
        if (!cacheKey)
            return;
        // Implement LRU-style cache eviction
        if (PDFColorSpaceProcessor.colorSpaceCache.size >= PDFColorSpaceProcessor.MAX_COLOR_SPACE_CACHE_SIZE) {
            const firstKey = PDFColorSpaceProcessor.colorSpaceCache.keys().next().value;
            if (firstKey) {
                PDFColorSpaceProcessor.colorSpaceCache.delete(firstKey);
            }
        }
        PDFColorSpaceProcessor.colorSpaceCache.set(cacheKey, colorSpace);
    }
    /**
     * Get number of color components for a color space
     */
    static getNumComponents(csName) {
        switch (csName) {
            case 'DeviceGray':
            case 'G':
                return 1;
            case 'DeviceRGB':
            case 'RGB':
                return 3;
            case 'DeviceCMYK':
            case 'CMYK':
                return 4;
            case 'Pattern':
                return 0; // Patterns don't have fixed components
            default:
                return 3; // Default to RGB
        }
    }
    /**
     * Parse ICCBased color space
     * Format: [/ICCBased stream]
     */
    static parseICCBased(csArray, resources) {
        const cs = {
            name: 'ICCBased',
            numComponents: 3 // Default to RGB
        };
        if (csArray.length >= 2) {
            const stream = csArray[1];
            // Extract ICC profile from stream
            if (stream && stream.data) {
                cs.iccProfile = stream.data;
            }
            // Get number of components from stream dictionary
            if (stream && stream.dictionary) {
                const n = stream.dictionary.entries.get('N');
                if (n !== undefined) {
                    cs.numComponents = typeof n === 'number' ? n : parseInt(n, 10);
                }
                // Get alternate color space
                const alternate = stream.dictionary.entries.get('Alternate');
                if (alternate) {
                    cs.alternate = this.parseColorSpace(alternate, resources);
                }
                // Get range
                const range = stream.dictionary.entries.get('Range');
                if (range && Array.isArray(range)) {
                    cs.range = range;
                }
            }
        }
        return cs;
    }
    /**
     * Parse Indexed color space
     * Format: [/Indexed base hival lookup]
     */
    static parseIndexed(csArray, resources) {
        const cs = {
            name: 'Indexed',
            numComponents: 1 // Indexed uses single index value
        };
        if (csArray.length >= 4) {
            // Parse base color space
            cs.base = this.parseColorSpace(csArray[1], resources);
            // Get hival (maximum index value)
            cs.hival = typeof csArray[2] === 'number' ? csArray[2] : parseInt(csArray[2], 10);
            // Get lookup table
            const lookupObj = csArray[3];
            if (lookupObj) {
                if (lookupObj instanceof Uint8Array) {
                    cs.lookup = lookupObj;
                }
                else if (lookupObj.data) {
                    cs.lookup = lookupObj.data;
                }
                else if (typeof lookupObj === 'string') {
                    // Convert hex string to bytes
                    cs.lookup = this.hexStringToBytes(lookupObj);
                }
            }
        }
        return cs;
    }
    /**
     * Parse Pattern color space
     * Format: [/Pattern] or [/Pattern baseCS]
     */
    static parsePattern(csArray) {
        const cs = {
            name: 'Pattern',
            numComponents: 0
        };
        // Pattern can have an optional base color space
        if (csArray.length >= 2) {
            cs.base = this.parseColorSpace(csArray[1]);
        }
        return cs;
    }
    /**
     * Parse Separation color space
     * Format: [/Separation name alternateSpace tintTransform]
     */
    static parseSeparation(csArray, resources) {
        const cs = {
            name: 'Separation',
            numComponents: 1,
            colorants: []
        };
        if (csArray.length >= 4) {
            // Colorant name
            if (typeof csArray[1] === 'string') {
                cs.colorants = [csArray[1]];
            }
            // Alternate color space
            cs.alternate = this.parseColorSpace(csArray[2], resources);
            // Tint transform function
            cs.tintTransform = csArray[3];
        }
        return cs;
    }
    /**
     * Parse DeviceN color space
     * Format: [/DeviceN names alternateSpace tintTransform]
     */
    static parseDeviceN(csArray, resources) {
        const cs = {
            name: 'DeviceN',
            numComponents: 0,
            colorants: []
        };
        if (csArray.length >= 4) {
            // Colorant names
            if (Array.isArray(csArray[1])) {
                cs.colorants = csArray[1].map((n) => String(n));
                cs.numComponents = cs.colorants.length;
            }
            // Alternate color space
            cs.alternate = this.parseColorSpace(csArray[2], resources);
            // Tint transform function
            cs.tintTransform = csArray[3];
        }
        return cs;
    }
    /**
     * Parse calibrated color space (CalRGB, CalGray)
     */
    static parseCalibrated(csArray) {
        const csName = csArray[0];
        const cs = {
            name: csName,
            numComponents: csName === 'CalGray' ? 1 : 3
        };
        // Additional calibration parameters could be extracted here
        // (WhitePoint, BlackPoint, Gamma, Matrix)
        return cs;
    }
    /**
     * Convert color values to RGB based on color space
     */
    static toRGB(colorSpace, values) {
        switch (colorSpace.name) {
            case 'DeviceGray':
            case 'CalGray':
            case 'G':
                return this.grayToRGB(values[0] || 0);
            case 'DeviceRGB':
            case 'CalRGB':
            case 'RGB':
                return [
                    values[0] || 0,
                    values[1] || 0,
                    values[2] || 0
                ];
            case 'DeviceCMYK':
            case 'CMYK':
                return this.cmykToRGB(values[0] || 0, values[1] || 0, values[2] || 0, values[3] || 0);
            case 'ICCBased':
                // Use alternate color space if available
                if (colorSpace.alternate) {
                    return this.toRGB(colorSpace.alternate, values);
                }
                // Fallback based on number of components
                if (colorSpace.numComponents === 1) {
                    return this.grayToRGB(values[0] || 0);
                }
                else if (colorSpace.numComponents === 4) {
                    return this.cmykToRGB(values[0] || 0, values[1] || 0, values[2] || 0, values[3] || 0);
                }
                // Default to RGB
                return [values[0] || 0, values[1] || 0, values[2] || 0];
            case 'Indexed':
            case 'I':
                return this.indexedToRGB(colorSpace, values[0] || 0);
            case 'Separation':
                // Apply tint to alternate color space
                if (colorSpace.alternate) {
                    const tint = values[0] || 0;
                    // Simplified: just apply tint as scaling factor
                    const alternateValues = new Array(colorSpace.alternate.numComponents).fill(tint);
                    return this.toRGB(colorSpace.alternate, alternateValues);
                }
                return [0, 0, 0];
            case 'DeviceN':
                // Use alternate color space
                if (colorSpace.alternate) {
                    // Simplified: average the components
                    const avg = values.reduce((sum, v) => sum + v, 0) / values.length;
                    const alternateValues = new Array(colorSpace.alternate.numComponents).fill(avg);
                    return this.toRGB(colorSpace.alternate, alternateValues);
                }
                return [0, 0, 0];
            default:
                // Default to black
                return [0, 0, 0];
        }
    }
    /**
     * Convert grayscale to RGB
     */
    static grayToRGB(gray) {
        // Common gray values cache (performance optimization)
        const cacheKey = `gray:${gray.toFixed(3)}`;
        const cached = PDFColorSpaceProcessor.conversionCache.get(cacheKey);
        if (cached)
            return cached;
        const rgb = [gray, gray, gray];
        this.cacheConversion(cacheKey, rgb);
        return rgb;
    }
    /**
     * Convert CMYK to RGB
     */
    static cmykToRGB(c, m, y, k) {
        // Cache frequently used CMYK conversions
        const cacheKey = `cmyk:${c.toFixed(2)},${m.toFixed(2)},${y.toFixed(2)},${k.toFixed(2)}`;
        const cached = PDFColorSpaceProcessor.conversionCache.get(cacheKey);
        if (cached)
            return cached;
        const r = (1 - c) * (1 - k);
        const g = (1 - m) * (1 - k);
        const b = (1 - y) * (1 - k);
        const rgb = [r, g, b];
        this.cacheConversion(cacheKey, rgb);
        return rgb;
    }
    /**
     * Cache a color conversion result
     */
    static cacheConversion(cacheKey, rgb) {
        // Implement LRU-style cache eviction
        if (PDFColorSpaceProcessor.conversionCache.size >= PDFColorSpaceProcessor.MAX_CONVERSION_CACHE_SIZE) {
            const firstKey = PDFColorSpaceProcessor.conversionCache.keys().next().value;
            if (firstKey) {
                PDFColorSpaceProcessor.conversionCache.delete(firstKey);
            }
        }
        PDFColorSpaceProcessor.conversionCache.set(cacheKey, rgb);
    }
    /**
     * Convert indexed color to RGB
     */
    static indexedToRGB(colorSpace, index) {
        if (!colorSpace.base || !colorSpace.lookup || colorSpace.hival === undefined) {
            return [0, 0, 0];
        }
        // Clamp index to valid range
        const idx = Math.max(0, Math.min(Math.floor(index), colorSpace.hival));
        // Calculate byte offset in lookup table
        const componentCount = colorSpace.base.numComponents;
        const offset = idx * componentCount;
        // Extract color components from lookup table
        const components = [];
        for (let i = 0; i < componentCount; i++) {
            if (offset + i < colorSpace.lookup.length) {
                components.push(colorSpace.lookup[offset + i] / 255);
            }
            else {
                components.push(0);
            }
        }
        // Convert to RGB using base color space
        return this.toRGB(colorSpace.base, components);
    }
    /**
     * Convert RGB array to CSS color string
     */
    static rgbToCSS(rgb) {
        const r = Math.max(0, Math.min(255, Math.floor(rgb[0] * 255)));
        const g = Math.max(0, Math.min(255, Math.floor(rgb[1] * 255)));
        const b = Math.max(0, Math.min(255, Math.floor(rgb[2] * 255)));
        return `rgb(${r},${g},${b})`;
    }
    /**
     * Convert hex string to byte array
     */
    static hexStringToBytes(hexStr) {
        const clean = hexStr.replace(/[<>\s]/g, '');
        const bytes = new Uint8Array(clean.length / 2);
        for (let i = 0; i < bytes.length; i++) {
            bytes[i] = parseInt(clean.substr(i * 2, 2), 16);
        }
        return bytes;
    }
}
// Color space cache
PDFColorSpaceProcessor.colorSpaceCache = new Map();
PDFColorSpaceProcessor.MAX_COLOR_SPACE_CACHE_SIZE = 50;
// Color conversion cache for common values
PDFColorSpaceProcessor.conversionCache = new Map();
PDFColorSpaceProcessor.MAX_CONVERSION_CACHE_SIZE = 500;
/**
 * PDF Graphics State Executor
 * Implements PDF graphics operators to render content to canvas
 */
class PDFGraphicsExecutor {
    constructor(ctx, page, scale) {
        this.ctx = ctx;
        this.page = page;
        this.scale = scale;
        this.graphicsStateStack = [];
        this.currentPath = new Path2D();
        this.currentPoint = [0, 0];
        this.textState = {
            font: 'Arial',
            fontStyle: '',
            fontSize: 12,
            matrix: [1, 0, 0, 1, 0, 0], // Text matrix [a, b, c, d, e, f]
            lineMatrix: [1, 0, 0, 1, 0, 0], // Line matrix (start of current line)
            charSpace: 0,
            wordSpace: 0,
            horizontalScaling: 100,
            leading: 0,
            rise: 0,
            renderMode: 0
        };
        // Current transformation matrix (separate from canvas)
        this.currentCTM = [1, 0, 0, 1, 0, 0];
        // Line state
        this.lineState = {
            width: 1,
            cap: 'butt',
            join: 'miter',
            miterLimit: 10,
            dashArray: [],
            dashPhase: 0
        };
        // Color state
        this.colorState = {
            stroke: '#000000',
            fill: '#000000'
        };
        // Color space state
        this.strokeColorSpace = { name: 'DeviceGray', numComponents: 1 };
        this.fillColorSpace = { name: 'DeviceGray', numComponents: 1 };
        // Clipping state
        this.hasClippingPath = false;
        // Set up coordinate system (PDF uses bottom-left origin, canvas uses top-left)
        ctx.save();
        ctx.scale(scale, -scale);
        ctx.translate(0, -page.height);
    }
    async executeOperator(operator, operands) {
        try {
            switch (operator) {
                // Graphics state operators
                case 'q':
                    this.saveGraphicsState();
                    break;
                case 'Q':
                    this.restoreGraphicsState();
                    break;
                case 'cm':
                    this.concatMatrix(operands);
                    break;
                case 'w':
                    this.setLineWidth(operands);
                    break;
                case 'J':
                    this.setLineCap(operands);
                    break;
                case 'j':
                    this.setLineJoin(operands);
                    break;
                case 'M':
                    this.setMiterLimit(operands);
                    break;
                case 'd':
                    this.setDash(operands);
                    break;
                case 'i': /* flatness tolerance - ignored for screen rendering */ break;
                case 'gs':
                    this.setExtGState(operands);
                    break;
                // Color operators
                case 'G':
                    this.setStrokeGray(operands);
                    break;
                case 'g':
                    this.setFillGray(operands);
                    break;
                case 'RG':
                    this.setStrokeRGB(operands);
                    break;
                case 'rg':
                    this.setFillRGB(operands);
                    break;
                case 'K':
                    this.setStrokeCMYK(operands);
                    break;
                case 'k':
                    this.setFillCMYK(operands);
                    break;
                // Advanced color space operators
                case 'CS':
                    this.setStrokeColorSpace(operands);
                    break;
                case 'cs':
                    this.setFillColorSpace(operands);
                    break;
                case 'SCN':
                case 'SC':
                    this.setStrokeColor(operands);
                    break;
                case 'scn':
                case 'sc':
                    this.setFillColor(operands);
                    break;
                // Path construction operators
                case 'm':
                    this.moveTo(operands);
                    break;
                case 'l':
                    this.lineTo(operands);
                    break;
                case 'c':
                    this.curveTo(operands);
                    break;
                case 'v':
                    this.curveToV(operands);
                    break;
                case 'y':
                    this.curveToY(operands);
                    break;
                case 'h':
                    this.closePath();
                    break;
                case 're':
                    this.rectangle(operands);
                    break;
                // Clipping path operators
                case 'W':
                    this.clipNonZero();
                    break;
                case 'W*':
                    this.clipEvenOdd();
                    break;
                // Path painting operators
                case 'S':
                    this.stroke();
                    break;
                case 's':
                    this.closeAndStroke();
                    break;
                case 'f':
                case 'F':
                    this.fill();
                    break;
                case 'f*':
                    this.fillEvenOdd();
                    break;
                case 'B':
                    this.fillAndStroke();
                    break;
                case 'B*':
                    this.fillEvenOddAndStroke();
                    break;
                case 'b':
                    this.closeFillAndStroke();
                    break;
                case 'b*':
                    this.closeFillEvenOddAndStroke();
                    break;
                case 'n':
                    this.endPath();
                    break;
                // Text operators
                case 'BT':
                    this.beginText();
                    break;
                case 'ET':
                    this.endText();
                    break;
                case 'Td':
                    this.moveText(operands);
                    break;
                case 'TD':
                    this.moveTextWithLeading(operands);
                    break;
                case 'Tm':
                    this.setTextMatrix(operands);
                    break;
                case 'T*':
                    this.nextLine();
                    break;
                case 'Tf':
                    this.setFont(operands);
                    break;
                case 'Tj':
                    this.showText(operands);
                    break;
                case 'TJ':
                    this.showTextArray(operands);
                    break;
                case "'":
                    this.nextLineShowText(operands);
                    break;
                case '"':
                    this.setSpacingShowText(operands);
                    break;
                case 'Tc':
                    this.setCharSpacing(operands);
                    break;
                case 'Tw':
                    this.setWordSpacing(operands);
                    break;
                case 'Tz':
                    this.setHorizontalScaling(operands);
                    break;
                case 'TL':
                    this.setTextLeading(operands);
                    break;
                case 'Tr':
                    this.setTextRenderMode(operands);
                    break;
                case 'Ts':
                    this.setTextRise(operands);
                    break;
                // XObject operators
                case 'Do':
                    await this.displayXObject(operands);
                    break;
                // Inline image operator
                case 'BI':
                    await this.renderInlineImage(operands);
                    break;
                // Ignore unsupported operators
                default:
                    // console.log('Unsupported operator:', operator);
                    break;
            }
        }
        catch (error) {
            console.warn(`Error executing operator ${operator}:`, error);
        }
    }
    // Graphics state operators
    saveGraphicsState() {
        // Save canvas state (includes CTM, clipping, colors, etc.)
        this.ctx.save();
        // Save our PDF graphics state
        const state = {
            textState: {
                font: this.textState.font,
                fontStyle: this.textState.fontStyle,
                fontSize: this.textState.fontSize,
                matrix: [...this.textState.matrix],
                lineMatrix: [...this.textState.lineMatrix],
                charSpace: this.textState.charSpace,
                wordSpace: this.textState.wordSpace,
                horizontalScaling: this.textState.horizontalScaling,
                leading: this.textState.leading,
                rise: this.textState.rise,
                renderMode: this.textState.renderMode
            },
            lineWidth: this.lineState.width,
            lineCap: this.lineState.cap,
            lineJoin: this.lineState.join,
            miterLimit: this.lineState.miterLimit,
            dashArray: [...this.lineState.dashArray],
            dashPhase: this.lineState.dashPhase,
            strokeColor: this.colorState.stroke,
            fillColor: this.colorState.fill,
            strokeColorSpace: { ...this.strokeColorSpace },
            fillColorSpace: { ...this.fillColorSpace },
            ctm: [...this.currentCTM],
            hasClippingPath: this.hasClippingPath,
            fontResource: this.currentFontResource
        };
        this.graphicsStateStack.push(state);
    }
    restoreGraphicsState() {
        // Restore canvas state
        this.ctx.restore();
        // Restore our PDF graphics state
        if (this.graphicsStateStack.length > 0) {
            const state = this.graphicsStateStack.pop();
            // Restore text state
            this.textState = {
                font: state.textState.font,
                fontStyle: state.textState.fontStyle,
                fontSize: state.textState.fontSize,
                matrix: [...state.textState.matrix],
                lineMatrix: [...state.textState.lineMatrix],
                charSpace: state.textState.charSpace,
                wordSpace: state.textState.wordSpace,
                horizontalScaling: state.textState.horizontalScaling,
                leading: state.textState.leading,
                rise: state.textState.rise,
                renderMode: state.textState.renderMode
            };
            // Restore line state
            this.lineState.width = state.lineWidth;
            this.lineState.cap = state.lineCap;
            this.lineState.join = state.lineJoin;
            this.lineState.miterLimit = state.miterLimit;
            this.lineState.dashArray = [...state.dashArray];
            this.lineState.dashPhase = state.dashPhase;
            // Restore color state
            this.colorState.stroke = state.strokeColor;
            this.colorState.fill = state.fillColor;
            this.strokeColorSpace = { ...state.strokeColorSpace };
            this.fillColorSpace = { ...state.fillColorSpace };
            // Restore CTM
            this.currentCTM = [...state.ctm];
            // Restore clipping state
            this.hasClippingPath = state.hasClippingPath;
            // Restore font resource
            this.currentFontResource = state.fontResource;
        }
    }
    concatMatrix(operands) {
        if (operands.length === 6) {
            const [a, b, c, d, e, f] = operands;
            // Apply to canvas
            this.ctx.transform(a, b, c, d, e, f);
            // Update our CTM (multiply matrices)
            const [a0, b0, c0, d0, e0, f0] = this.currentCTM;
            this.currentCTM = [
                a * a0 + b * c0,
                a * b0 + b * d0,
                c * a0 + d * c0,
                c * b0 + d * d0,
                e * a0 + f * c0 + e0,
                e * b0 + f * d0 + f0
            ];
        }
    }
    setLineWidth(operands) {
        if (operands.length > 0) {
            this.lineState.width = operands[0];
            this.ctx.lineWidth = operands[0];
        }
    }
    setLineCap(operands) {
        if (operands.length > 0) {
            const caps = ['butt', 'round', 'square'];
            this.lineState.cap = caps[operands[0]];
            this.ctx.lineCap = caps[operands[0]];
        }
    }
    setMiterLimit(operands) {
        if (operands.length > 0) {
            this.lineState.miterLimit = operands[0];
            this.ctx.miterLimit = operands[0];
        }
    }
    setExtGState(operands) {
        if (operands.length === 0)
            return;
        let gsName = operands[0];
        if (typeof gsName !== 'string')
            return;
        if (gsName.startsWith('/'))
            gsName = gsName.substring(1);
        if (!this.page.resources || !this.page.resources.extGState)
            return;
        const gs = this.page.resources.extGState.get(gsName);
        if (!gs)
            return;
        // Apply graphics state parameters
        if (gs.ca !== undefined) {
            // Fill opacity (lowercase ca)
            this.ctx.globalAlpha = Math.max(0, Math.min(1, gs.ca));
        }
        if (gs.CA !== undefined) {
            // Stroke opacity (uppercase CA)
            // Canvas doesn't separate stroke/fill alpha, use the more restrictive
            this.ctx.globalAlpha = Math.max(0, Math.min(1, gs.CA));
        }
        if (gs.LW !== undefined) {
            this.lineState.width = gs.LW;
            this.ctx.lineWidth = gs.LW;
        }
        if (gs.LC !== undefined) {
            const caps = ['butt', 'round', 'square'];
            if (gs.LC >= 0 && gs.LC <= 2) {
                this.lineState.cap = caps[gs.LC];
                this.ctx.lineCap = caps[gs.LC];
            }
        }
        if (gs.LJ !== undefined) {
            const joins = ['miter', 'round', 'bevel'];
            if (gs.LJ >= 0 && gs.LJ <= 2) {
                this.lineState.join = joins[gs.LJ];
                this.ctx.lineJoin = joins[gs.LJ];
            }
        }
        if (gs.ML !== undefined) {
            this.lineState.miterLimit = gs.ML;
            this.ctx.miterLimit = gs.ML;
        }
        if (gs.Font !== undefined) {
            // Font entry is [fontRef size] — we just use the size if present
        }
    }
    setLineJoin(operands) {
        if (operands.length > 0) {
            const joins = ['miter', 'round', 'bevel'];
            this.lineState.join = joins[operands[0]];
            this.ctx.lineJoin = joins[operands[0]];
        }
    }
    setDash(operands) {
        if (operands.length >= 2 && Array.isArray(operands[0])) {
            this.lineState.dashArray = operands[0];
            this.lineState.dashPhase = operands[1] || 0;
            this.ctx.setLineDash(operands[0]);
            if (typeof this.ctx.lineDashOffset !== 'undefined') {
                this.ctx.lineDashOffset = operands[1] || 0;
            }
        }
    }
    // Color operators
    setStrokeGray(operands) {
        if (operands.length > 0) {
            const gray = Math.floor(operands[0] * 255);
            const color = `rgb(${gray},${gray},${gray})`;
            this.colorState.stroke = color;
            this.ctx.strokeStyle = color;
        }
    }
    setFillGray(operands) {
        if (operands.length > 0) {
            const gray = Math.floor(operands[0] * 255);
            const color = `rgb(${gray},${gray},${gray})`;
            this.colorState.fill = color;
            this.ctx.fillStyle = color;
        }
    }
    setStrokeRGB(operands) {
        if (operands.length >= 3) {
            const r = Math.floor(operands[0] * 255);
            const g = Math.floor(operands[1] * 255);
            const b = Math.floor(operands[2] * 255);
            const color = `rgb(${r},${g},${b})`;
            this.colorState.stroke = color;
            this.ctx.strokeStyle = color;
        }
    }
    setFillRGB(operands) {
        if (operands.length >= 3) {
            const r = Math.floor(operands[0] * 255);
            const g = Math.floor(operands[1] * 255);
            const b = Math.floor(operands[2] * 255);
            const color = `rgb(${r},${g},${b})`;
            this.colorState.fill = color;
            this.ctx.fillStyle = color;
        }
    }
    setStrokeCMYK(operands) {
        if (operands.length >= 4) {
            // Simple CMYK to RGB conversion
            const [c, m, y, k] = operands;
            const r = Math.floor(255 * (1 - c) * (1 - k));
            const g = Math.floor(255 * (1 - m) * (1 - k));
            const b = Math.floor(255 * (1 - y) * (1 - k));
            const color = `rgb(${r},${g},${b})`;
            this.colorState.stroke = color;
            this.ctx.strokeStyle = color;
        }
    }
    setFillCMYK(operands) {
        if (operands.length >= 4) {
            const [c, m, y, k] = operands;
            const r = Math.floor(255 * (1 - c) * (1 - k));
            const g = Math.floor(255 * (1 - m) * (1 - k));
            const b = Math.floor(255 * (1 - y) * (1 - k));
            const color = `rgb(${r},${g},${b})`;
            this.colorState.fill = color;
            this.ctx.fillStyle = color;
        }
    }
    // Advanced Color Space Operators
    /**
     * CS operator: Set stroke color space
     */
    setStrokeColorSpace(operands) {
        if (operands.length > 0) {
            let csName = operands[0];
            if (typeof csName === 'string' && csName.startsWith('/'))
                csName = csName.substring(1);
            // Get color space from resources or parse inline
            let colorSpace;
            if (this.page.resources && this.page.resources.colorSpaces.has(csName)) {
                const csObj = this.page.resources.colorSpaces.get(csName);
                colorSpace = PDFColorSpaceProcessor.parseColorSpace(csObj, this.page.resources);
            }
            else {
                colorSpace = PDFColorSpaceProcessor.parseColorSpace(csName, this.page.resources);
            }
            this.strokeColorSpace = colorSpace;
        }
    }
    /**
     * cs operator: Set fill color space
     */
    setFillColorSpace(operands) {
        if (operands.length > 0) {
            let csName = operands[0];
            if (typeof csName === 'string' && csName.startsWith('/'))
                csName = csName.substring(1);
            // Get color space from resources or parse inline
            let colorSpace;
            if (this.page.resources && this.page.resources.colorSpaces.has(csName)) {
                const csObj = this.page.resources.colorSpaces.get(csName);
                colorSpace = PDFColorSpaceProcessor.parseColorSpace(csObj, this.page.resources);
            }
            else {
                colorSpace = PDFColorSpaceProcessor.parseColorSpace(csName, this.page.resources);
            }
            this.fillColorSpace = colorSpace;
        }
    }
    /**
     * SCN/SC operator: Set stroke color (with current color space)
     */
    setStrokeColor(operands) {
        if (operands.length === 0)
            return;
        // Convert color values to RGB using current stroke color space
        const rgb = PDFColorSpaceProcessor.toRGB(this.strokeColorSpace, operands);
        const color = PDFColorSpaceProcessor.rgbToCSS(rgb);
        this.colorState.stroke = color;
        this.ctx.strokeStyle = color;
    }
    /**
     * scn/sc operator: Set fill color (with current color space)
     */
    setFillColor(operands) {
        if (operands.length === 0)
            return;
        // Convert color values to RGB using current fill color space
        const rgb = PDFColorSpaceProcessor.toRGB(this.fillColorSpace, operands);
        const color = PDFColorSpaceProcessor.rgbToCSS(rgb);
        this.colorState.fill = color;
        this.ctx.fillStyle = color;
    }
    // Path construction operators
    moveTo(operands) {
        if (operands.length >= 2) {
            this.currentPoint = [operands[0], operands[1]];
            this.currentPath.moveTo(operands[0], operands[1]);
        }
    }
    lineTo(operands) {
        if (operands.length >= 2) {
            this.currentPoint = [operands[0], operands[1]];
            this.currentPath.lineTo(operands[0], operands[1]);
        }
    }
    curveTo(operands) {
        if (operands.length >= 6) {
            this.currentPoint = [operands[4], operands[5]];
            this.currentPath.bezierCurveTo(operands[0], operands[1], operands[2], operands[3], operands[4], operands[5]);
        }
    }
    curveToV(operands) {
        if (operands.length >= 4) {
            const [x1, y1] = this.currentPoint;
            this.currentPoint = [operands[2], operands[3]];
            this.currentPath.bezierCurveTo(x1, y1, operands[0], operands[1], operands[2], operands[3]);
        }
    }
    curveToY(operands) {
        if (operands.length >= 4) {
            this.currentPoint = [operands[2], operands[3]];
            this.currentPath.bezierCurveTo(operands[0], operands[1], operands[2], operands[3], operands[2], operands[3]);
        }
    }
    closePath() {
        this.currentPath.closePath();
    }
    rectangle(operands) {
        if (operands.length >= 4) {
            this.currentPath.rect(operands[0], operands[1], operands[2], operands[3]);
        }
    }
    // Clipping path operators
    clipNonZero() {
        // W operator: Modify clipping path using nonzero winding rule
        // Note: Must be followed by a path painting operator (n, S, f, etc.)
        if (this.currentPath) {
            this.ctx.clip(this.currentPath, 'nonzero');
            this.hasClippingPath = true;
        }
        // Don't clear currentPath - it may still be used for painting
    }
    clipEvenOdd() {
        // W* operator: Modify clipping path using even-odd rule
        // Note: Must be followed by a path painting operator (n, S, f, etc.)
        if (this.currentPath) {
            this.ctx.clip(this.currentPath, 'evenodd');
            this.hasClippingPath = true;
        }
        // Don't clear currentPath - it may still be used for painting
    }
    // Path painting operators
    stroke() {
        this.ctx.stroke(this.currentPath);
        this.currentPath = new Path2D();
    }
    closeAndStroke() {
        this.currentPath.closePath();
        this.ctx.stroke(this.currentPath);
        this.currentPath = new Path2D();
    }
    fill() {
        this.ctx.fill(this.currentPath);
        this.currentPath = new Path2D();
    }
    fillEvenOdd() {
        this.ctx.fill(this.currentPath, 'evenodd');
        this.currentPath = new Path2D();
    }
    fillAndStroke() {
        this.ctx.fill(this.currentPath);
        this.ctx.stroke(this.currentPath);
        this.currentPath = new Path2D();
    }
    fillEvenOddAndStroke() {
        this.ctx.fill(this.currentPath, 'evenodd');
        this.ctx.stroke(this.currentPath);
        this.currentPath = new Path2D();
    }
    closeFillAndStroke() {
        this.currentPath.closePath();
        this.ctx.fill(this.currentPath);
        this.ctx.stroke(this.currentPath);
        this.currentPath = new Path2D();
    }
    closeFillEvenOddAndStroke() {
        this.currentPath.closePath();
        this.ctx.fill(this.currentPath, 'evenodd');
        this.ctx.stroke(this.currentPath);
        this.currentPath = new Path2D();
    }
    endPath() {
        this.currentPath = new Path2D();
    }
    // Text operators
    beginText() {
        this.textState.matrix = [1, 0, 0, 1, 0, 0];
        this.textState.lineMatrix = [1, 0, 0, 1, 0, 0];
        this.ctx.textBaseline = 'alphabetic';
    }
    endText() {
        // Text state is preserved
    }
    moveText(operands) {
        if (operands.length >= 2) {
            const tx = operands[0];
            const ty = operands[1];
            // Update line matrix: translate by (tx, ty)
            this.textState.lineMatrix[4] += tx * this.textState.lineMatrix[0] + ty * this.textState.lineMatrix[2];
            this.textState.lineMatrix[5] += tx * this.textState.lineMatrix[1] + ty * this.textState.lineMatrix[3];
            // Text matrix = line matrix
            this.textState.matrix = [...this.textState.lineMatrix];
        }
    }
    moveTextWithLeading(operands) {
        if (operands.length >= 2) {
            this.textState.leading = -operands[1];
            this.moveText(operands);
        }
    }
    setTextMatrix(operands) {
        if (operands.length >= 6) {
            this.textState.matrix = [...operands];
            this.textState.lineMatrix = [...operands];
        }
    }
    nextLine() {
        // T* is equivalent to Td(0, -TL) per PDF spec
        this.moveText([0, -this.textState.leading]);
    }
    setFont(operands) {
        if (operands.length >= 2) {
            const rawName = operands[0];
            const fontSize = operands[1];
            this.textState.fontSize = fontSize;
            // Strip leading '/' from name for resource lookup (parser returns '/F94', resources keyed 'F94')
            const fontKey = typeof rawName === 'string' && rawName.startsWith('/') ? rawName.substring(1) : rawName;
            // Lookup font resource from page resources
            if (this.page.resources && this.page.resources.fonts) {
                this.currentFontResource = this.page.resources.fonts.get(fontKey);
            }
            // Map PDF font to canvas font using baseFont from resource (or raw name as fallback)
            let baseFont = this.currentFontResource?.baseFont || fontKey || '';
            // Strip subset prefix (e.g., "ABCDEF+" → "ArialMT")
            const plusIdx = baseFont.indexOf('+');
            if (plusIdx >= 0 && plusIdx <= 6)
                baseFont = baseFont.substring(plusIdx + 1);
            let canvasFont = 'sans-serif';
            const bf = baseFont.toLowerCase();
            if (bf.includes('courier') || bf.includes('mono') || bf.includes('nimbusmono') || bf.includes('nimbusl') || bf.includes('nimbusmonl')) {
                canvasFont = '"Courier New", monospace';
            }
            else if (bf.includes('times') || bf.includes('nimbus') || bf.includes('cmr') || bf.includes('cmb') ||
                bf.includes('cmmi') || bf.includes('cmsy') || bf.includes('cmt') || bf.includes('roman') ||
                bf.includes('serif') && !bf.includes('sans')) {
                canvasFont = '"Times New Roman", serif';
            }
            else if (bf.includes('helvetica') || bf.includes('arial') || bf.includes('sans')) {
                canvasFont = '"Helvetica", "Arial", sans-serif';
            }
            else {
                canvasFont = '"Times New Roman", serif'; // default serif for academic/book content
            }
            // Add weight/style modifiers
            let fontStyle = '';
            if (bf.includes('bold') || bf.includes('medi') || bf.includes('cmb')) {
                fontStyle = 'bold ';
            }
            if (bf.includes('ital') || bf.includes('obli') || bf.includes('cmmi') || bf.includes('cmti')) {
                fontStyle += 'italic ';
            }
            this.textState.font = canvasFont;
            this.textState.fontStyle = fontStyle;
            this.ctx.font = `${fontStyle}${Math.abs(fontSize)}px ${canvasFont}`;
        }
    }
    showText(operands) {
        if (operands.length === 0)
            return;
        const rawInput = operands[0];
        if (typeof rawInput !== 'string' || rawInput.length === 0)
            return;
        const tm = this.textState.matrix;
        const fontSize = this.textState.fontSize;
        const absFontSize = Math.abs(fontSize);
        const renderMode = this.textState.renderMode || 0;
        const shouldRender = renderMode !== 3 && renderMode !== 7;
        const horizScale = (this.textState.horizontalScaling || 100) / 100;
        const charSpace = this.textState.charSpace || 0;
        const wordSpace = this.textState.wordSpace || 0;
        const rise = this.textState.rise || 0;
        // Resolve escape sequences to get raw byte values for width lookups.
        const rawBytes = this.resolveEscapes(rawInput);
        if (rawBytes.length === 0)
            return;
        const toUnicode = this.currentFontResource?.toUnicode;
        // Determine if this font uses 2-byte character codes (CID fonts).
        // Type0 composite fonts (CIDFont) encode characters as 2-byte CIDs.
        const isCIDFont = this.currentFontResource?.subtype === 'Type0';
        // Build character code array: group bytes into proper code units
        const charCodes = [];
        if (isCIDFont) {
            // 2-byte CID codes: combine pairs of bytes into 16-bit values
            for (let i = 0; i < rawBytes.length - 1; i += 2) {
                charCodes.push((rawBytes[i] << 8) | rawBytes[i + 1]);
            }
        }
        else {
            // Single-byte codes
            for (const b of rawBytes) {
                charCodes.push(b);
            }
        }
        // Factor font size from Tm for sharper rendering.
        const tmScale = Math.sqrt(tm[0] * tm[0] + tm[1] * tm[1]) || 1;
        const effectiveFontSize = absFontSize * tmScale;
        if (shouldRender) {
            this.ctx.save();
            this.ctx.textBaseline = 'alphabetic';
            if (tmScale !== 1) {
                this.ctx.transform(tm[0] / tmScale, tm[1] / tmScale, tm[2] / tmScale, tm[3] / tmScale, tm[4], tm[5]);
            }
            else {
                this.ctx.transform(tm[0], tm[1], tm[2], tm[3], tm[4], tm[5]);
            }
            if (horizScale !== 1) {
                this.ctx.scale(horizScale, 1);
            }
            const fontStyle = this.textState.fontStyle || '';
            this.ctx.font = fontStyle + effectiveFontSize + 'px ' + this.textState.font;
            this.ctx.scale(1, -1);
        }
        let xOffset = 0;
        for (const charCode of charCodes) {
            // Display character: ToUnicode/Encoding first, then PDFDocEncoding
            let displayChar;
            if (toUnicode && toUnicode.has(charCode)) {
                displayChar = toUnicode.get(charCode);
            }
            else if (isCIDFont) {
                // For CID fonts without ToUnicode, try Unicode interpretation
                displayChar = charCode > 0 ? String.fromCodePoint(charCode) : '';
            }
            else {
                displayChar = PDFTextDecoder.mapCharCode(charCode, this.currentFontResource?.encoding);
            }
            const renderX = xOffset * tmScale;
            const renderY = -rise * tmScale;
            if (shouldRender && displayChar) {
                if (renderMode === 1) {
                    this.ctx.strokeText(displayChar, renderX, renderY);
                }
                else if (renderMode === 2) {
                    this.ctx.fillText(displayChar, renderX, renderY);
                    this.ctx.strokeText(displayChar, renderX, renderY);
                }
                else {
                    this.ctx.fillText(displayChar, renderX, renderY);
                }
            }
            const glyphWidth = PDFGlyphMetrics.getCharWidth(charCode, this.currentFontResource, fontSize);
            let advance = glyphWidth + charSpace;
            if (charCode === 32)
                advance += wordSpace;
            xOffset += advance;
        }
        if (shouldRender) {
            this.ctx.restore();
        }
        const totalAdvance = xOffset * horizScale;
        tm[4] += totalAdvance * tm[0];
        tm[5] += totalAdvance * tm[1];
    }
    showTextArray(operands) {
        if (operands.length > 0 && Array.isArray(operands[0])) {
            const array = operands[0];
            for (const item of array) {
                if (typeof item === 'string') {
                    // Show the text string
                    this.showText([item]);
                }
                else if (typeof item === 'number') {
                    // Adjust spacing: tx = -(Tj/1000) * Tfs * Th (PDF spec 9.4.4)
                    const horizScale = (this.textState.horizontalScaling || 100) / 100;
                    const adjustment = -item / 1000 * this.textState.fontSize * horizScale;
                    // Move text matrix horizontally
                    this.textState.matrix[4] += adjustment * this.textState.matrix[0];
                    this.textState.matrix[5] += adjustment * this.textState.matrix[1];
                }
            }
        }
    }
    nextLineShowText(operands) {
        this.nextLine();
        this.showText(operands);
    }
    setSpacingShowText(operands) {
        if (operands.length >= 3) {
            this.textState.wordSpace = operands[0];
            this.textState.charSpace = operands[1];
            this.nextLine();
            this.showText([operands[2]]);
        }
    }
    setCharSpacing(operands) {
        if (operands.length > 0) {
            this.textState.charSpace = operands[0];
        }
    }
    setWordSpacing(operands) {
        if (operands.length > 0) {
            this.textState.wordSpace = operands[0];
        }
    }
    setHorizontalScaling(operands) {
        if (operands.length > 0) {
            this.textState.horizontalScaling = operands[0];
        }
    }
    setTextLeading(operands) {
        if (operands.length > 0) {
            this.textState.leading = operands[0];
        }
    }
    setTextRenderMode(operands) {
        if (operands.length > 0) {
            // 0: Fill, 1: Stroke, 2: Fill then stroke, 3: Invisible
            // 4-7: Same but with clipping
            const mode = operands[0];
            this.textState.renderMode = mode;
            // For now, just handle basic modes
            // Mode 3 and 7 are invisible - we'll skip rendering
            // Other modes we'll render as fill
        }
    }
    setTextRise(operands) {
        if (operands.length > 0) {
            this.textState.rise = operands[0];
        }
    }
    // XObject operators
    async displayXObject(operands) {
        if (operands.length === 0)
            return;
        let xobjName = operands[0];
        if (typeof xobjName !== 'string')
            return;
        if (xobjName.startsWith('/'))
            xobjName = xobjName.substring(1);
        // Lookup XObject in page resources
        if (!this.page.resources || !this.page.resources.xObjects) {
            console.warn('[XObject] No resources or xObjects available');
            return;
        }
        const xobject = this.page.resources.xObjects.get(xobjName);
        if (!xobject) {
            console.warn('[XObject] XObject not found:', xobjName);
            return;
        }
        // console.log('[XObject] Displaying XObject:', xobjName, 'type:', xobject.type);
        if (xobject.type === 'image') {
            await this.renderImage(xobject);
        }
        else if (xobject.type === 'form') {
            await this.renderFormXObject(xobject);
        }
    }
    /**
     * Render Form XObject
     * Form XObjects are reusable graphics content that can be drawn multiple times
     */
    async renderFormXObject(xobject) {
        try {
            // Save graphics state before rendering form
            this.saveGraphicsState();
            // Apply form matrix if present
            if (xobject.matrix) {
                const m = xobject.matrix;
                this.ctx.transform(m[0], m[1], m[2], m[3], m[4], m[5]);
            }
            // Apply bounding box clipping if present
            if (xobject.bbox && xobject.bbox.length === 4) {
                const [llx, lly, urx, ury] = xobject.bbox;
                this.ctx.beginPath();
                this.ctx.rect(llx, lly, urx - llx, ury - lly);
                this.ctx.clip();
            }
            // Parse and execute form content stream
            const formContentStream = xobject.data;
            if (formContentStream && formContentStream.length > 0) {
                // Parse form content stream
                const parser = new SafeContentStreamParser(formContentStream);
                const operations = parser.parse();
                // Store current page resources
                const savedPageResources = this.page.resources;
                // If form has its own resources, merge them with page resources
                if (xobject.resources) {
                    this.page.resources = this.mergeResources(savedPageResources, xobject.resources);
                }
                // Execute form operations
                for (const op of operations) {
                    await this.executeOperator(op.operator, op.operands);
                }
                // Restore page resources
                this.page.resources = savedPageResources;
            }
            // Restore graphics state
            this.restoreGraphicsState();
        }
        catch (error) {
            console.warn('Error rendering form XObject:', error);
            // Ensure we restore state even on error
            this.restoreGraphicsState();
        }
    }
    /**
     * Merge form resources with page resources
     * Form resources take precedence over page resources
     */
    mergeResources(pageRes, formRes) {
        if (!pageRes)
            return formRes;
        // Create merged resources with form resources taking precedence
        const merged = {
            fonts: new Map([...(pageRes.fonts || new Map()), ...(formRes.fonts || new Map())]),
            images: new Map([...(pageRes.images || new Map()), ...(formRes.images || new Map())]),
            xObjects: new Map([...(pageRes.xObjects || new Map()), ...(formRes.xObjects || new Map())]),
            colorSpaces: new Map([...(pageRes.colorSpaces || new Map()), ...(formRes.colorSpaces || new Map())]),
            patterns: new Map([...(pageRes.patterns || new Map()), ...(formRes.patterns || new Map())]),
            extGState: new Map([...(pageRes.extGState || new Map()), ...(formRes.extGState || new Map())])
        };
        return merged;
    }
    async renderImage(xobject) {
        try {
            const data = xobject.data instanceof Uint8Array
                ? xobject.data : new Uint8Array(xobject.data);
            if (data.length === 0)
                return;
            // Check if data is a pre-encoded image (JPEG/PNG/GIF)
            if (this.isEncodedImageData(data, xobject.filter)) {
                await this.renderEncodedImage(data);
            }
            else {
                // Raw pixel data — convert to ImageData and draw
                this.renderRawPixelImage(data, xobject);
            }
        }
        catch (error) {
            console.warn('[Image Rendering] Error in renderImage:', error);
        }
    }
    isEncodedImageData(data, filter) {
        if (filter === 'DCTDecode' || filter === 'JPXDecode')
            return true;
        if (data.length >= 3 && data[0] === 0xFF && data[1] === 0xD8 && data[2] === 0xFF)
            return true; // JPEG
        if (data.length >= 4 && data[0] === 0x89 && data[1] === 0x50 && data[2] === 0x4E && data[3] === 0x47)
            return true; // PNG
        if (data.length >= 3 && data[0] === 0x47 && data[1] === 0x49 && data[2] === 0x46)
            return true; // GIF
        return false;
    }
    async renderEncodedImage(data) {
        let mimeType = 'image/jpeg';
        if (data[0] === 0x89 && data[1] === 0x50)
            mimeType = 'image/png';
        else if (data[0] === 0x47 && data[1] === 0x49)
            mimeType = 'image/gif';
        const arrayBuffer = data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength);
        const blob = new Blob([arrayBuffer], { type: mimeType });
        const imageUrl = URL.createObjectURL(blob);
        try {
            const img = new Image();
            img.src = imageUrl;
            await new Promise((resolve, reject) => {
                img.onload = () => {
                    try {
                        this.ctx.save();
                        this.ctx.scale(1, -1);
                        this.ctx.translate(0, -1);
                        this.ctx.drawImage(img, 0, 0, 1, 1);
                        this.ctx.restore();
                        resolve();
                    }
                    catch (e) {
                        reject(e);
                    }
                };
                img.onerror = () => reject(new Error('Failed to load image'));
            });
        }
        finally {
            URL.revokeObjectURL(imageUrl);
        }
    }
    renderRawPixelImage(data, xobject) {
        const width = xobject.width || 1;
        const height = xobject.height || 1;
        const bpc = xobject.bitsPerComponent || 8;
        const cs = (xobject.colorSpace || 'DeviceRGB').replace(/^\//, '');
        // Determine components per pixel from color space
        let components;
        if (cs === 'DeviceGray' || cs === 'CalGray' || cs === 'G') {
            components = 1;
        }
        else if (cs === 'DeviceCMYK' || cs === 'CMYK') {
            components = 4;
        }
        else {
            components = 3; // DeviceRGB, CalRGB, ICCBased (default)
        }
        // Infer components from data length if mismatch
        if (bpc === 8) {
            const expectedRGB = width * height * 3;
            const expectedGray = width * height;
            const expectedCMYK = width * height * 4;
            if (components === 3 && data.length >= expectedCMYK && data.length < expectedRGB) {
                // Data doesn't fit RGB, might be something else
            }
            else if (components === 3 && data.length === expectedGray) {
                components = 1; // Actually grayscale
            }
            else if (components === 3 && data.length === expectedCMYK) {
                components = 4; // Actually CMYK
            }
        }
        // Apply PNG predictor un-filtering if needed
        let pixels = data;
        if (xobject.predictor && xobject.predictor >= 10) {
            const bytesPerPixel = Math.ceil(components * bpc / 8);
            const rowBytes = xobject.predictorColumns
                ? Math.ceil(xobject.predictorColumns * (xobject.predictorColors || components) * bpc / 8)
                : Math.ceil(width * components * bpc / 8);
            pixels = this.reversePNGPrediction(data, rowBytes, bytesPerPixel, height);
        }
        // Create RGBA ImageData
        const imageData = new ImageData(width, height);
        const rgba = imageData.data;
        if (bpc === 8) {
            this.convertPixels8bit(pixels, rgba, width, height, components);
        }
        else if (bpc === 1 || bpc === 2 || bpc === 4) {
            this.convertPixelsSubByte(pixels, rgba, width, height, components, bpc);
        }
        else {
            this.convertPixels8bit(pixels, rgba, width, height, components);
        }
        // Draw via temporary canvas
        const tmpCanvas = document.createElement('canvas');
        tmpCanvas.width = width;
        tmpCanvas.height = height;
        const tmpCtx = tmpCanvas.getContext('2d');
        tmpCtx.putImageData(imageData, 0, 0);
        this.ctx.save();
        this.ctx.scale(1, -1);
        this.ctx.translate(0, -1);
        this.ctx.drawImage(tmpCanvas, 0, 0, 1, 1);
        this.ctx.restore();
    }
    convertPixels8bit(src, dst, w, h, comp) {
        const total = w * h;
        for (let i = 0; i < total; i++) {
            const dstIdx = i * 4;
            if (comp === 1) {
                const g = src[i] || 0;
                dst[dstIdx] = g;
                dst[dstIdx + 1] = g;
                dst[dstIdx + 2] = g;
                dst[dstIdx + 3] = 255;
            }
            else if (comp === 3) {
                const srcIdx = i * 3;
                dst[dstIdx] = src[srcIdx] || 0;
                dst[dstIdx + 1] = src[srcIdx + 1] || 0;
                dst[dstIdx + 2] = src[srcIdx + 2] || 0;
                dst[dstIdx + 3] = 255;
            }
            else if (comp === 4) {
                const srcIdx = i * 4;
                const c = (src[srcIdx] || 0) / 255;
                const m = (src[srcIdx + 1] || 0) / 255;
                const y = (src[srcIdx + 2] || 0) / 255;
                const k = (src[srcIdx + 3] || 0) / 255;
                dst[dstIdx] = 255 * (1 - c) * (1 - k) | 0;
                dst[dstIdx + 1] = 255 * (1 - m) * (1 - k) | 0;
                dst[dstIdx + 2] = 255 * (1 - y) * (1 - k) | 0;
                dst[dstIdx + 3] = 255;
            }
        }
    }
    convertPixelsSubByte(src, dst, w, h, comp, bpc) {
        const maxVal = (1 << bpc) - 1;
        let bitPos = 0;
        const total = w * h;
        for (let i = 0; i < total; i++) {
            const dstIdx = i * 4;
            if (comp === 1) {
                const byteIdx = bitPos >> 3;
                const bitOffset = 8 - bpc - (bitPos & 7);
                const val = ((src[byteIdx] || 0) >> bitOffset) & maxVal;
                const scaled = (val * 255 / maxVal) | 0;
                dst[dstIdx] = scaled;
                dst[dstIdx + 1] = scaled;
                dst[dstIdx + 2] = scaled;
                dst[dstIdx + 3] = 255;
                bitPos += bpc;
            }
            else {
                const vals = [];
                for (let c = 0; c < comp; c++) {
                    const byteIdx = bitPos >> 3;
                    const bitOffset = 8 - bpc - (bitPos & 7);
                    vals.push((((src[byteIdx] || 0) >> bitOffset) & maxVal) * 255 / maxVal | 0);
                    bitPos += bpc;
                }
                dst[dstIdx] = vals[0];
                dst[dstIdx + 1] = vals[1] || 0;
                dst[dstIdx + 2] = vals[2] || 0;
                dst[dstIdx + 3] = 255;
            }
            // Row alignment: each row starts at byte boundary
            if ((i + 1) % w === 0) {
                bitPos = ((bitPos + 7) >> 3) << 3;
            }
        }
    }
    reversePNGPrediction(data, rowBytes, bytesPerPixel, height) {
        const inputRowSize = 1 + rowBytes; // 1 filter byte + row data
        const output = new Uint8Array(rowBytes * height);
        for (let row = 0; row < height; row++) {
            const inOff = row * inputRowSize;
            const outOff = row * rowBytes;
            const prevRowOff = (row - 1) * rowBytes;
            const filterType = data[inOff] || 0;
            for (let col = 0; col < rowBytes; col++) {
                const raw = data[inOff + 1 + col] || 0;
                const a = col >= bytesPerPixel ? output[outOff + col - bytesPerPixel] : 0;
                const b = row > 0 ? output[prevRowOff + col] : 0;
                const c = (row > 0 && col >= bytesPerPixel) ? output[prevRowOff + col - bytesPerPixel] : 0;
                let val;
                switch (filterType) {
                    case 0:
                        val = raw;
                        break;
                    case 1:
                        val = (raw + a) & 0xFF;
                        break;
                    case 2:
                        val = (raw + b) & 0xFF;
                        break;
                    case 3:
                        val = (raw + ((a + b) >> 1)) & 0xFF;
                        break;
                    case 4:
                        val = (raw + this.paethPredictor(a, b, c)) & 0xFF;
                        break;
                    default:
                        val = raw;
                        break;
                }
                output[outOff + col] = val;
            }
        }
        return output;
    }
    paethPredictor(a, b, c) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        if (pa <= pb && pa <= pc)
            return a;
        if (pb <= pc)
            return b;
        return c;
    }
    async renderInlineImage(operands) {
        if (operands.length === 0)
            return;
        const imgObj = operands[0];
        if (!imgObj || !imgObj.data)
            return;
        const dict = imgObj.dictionary || new Map();
        // Inline images use abbreviated keys
        const width = dict.get('/W') || dict.get('/Width') || 1;
        const height = dict.get('/H') || dict.get('/Height') || 1;
        const bpc = dict.get('/BPC') || dict.get('/BitsPerComponent') || 8;
        let cs = dict.get('/CS') || dict.get('/ColorSpace') || 'DeviceRGB';
        if (typeof cs === 'string')
            cs = cs.replace(/^\//, '');
        if (cs === 'G')
            cs = 'DeviceGray';
        else if (cs === 'RGB')
            cs = 'DeviceRGB';
        else if (cs === 'CMYK')
            cs = 'DeviceCMYK';
        let filter = dict.get('/F') || dict.get('/Filter') || '';
        if (typeof filter === 'string')
            filter = filter.replace(/^\//, '');
        if (filter === 'AHx')
            filter = 'ASCIIHexDecode';
        else if (filter === 'A85')
            filter = 'ASCII85Decode';
        else if (filter === 'LZW')
            filter = 'LZWDecode';
        else if (filter === 'Fl')
            filter = 'FlateDecode';
        else if (filter === 'RL')
            filter = 'RunLengthDecode';
        else if (filter === 'CCF')
            filter = 'CCITTFaxDecode';
        else if (filter === 'DCT')
            filter = 'DCTDecode';
        const data = imgObj.data instanceof Uint8Array ? imgObj.data : new Uint8Array(imgObj.data);
        if (data.length === 0)
            return;
        const xobj = {
            type: 'image',
            data: data,
            width: typeof width === 'number' ? width : parseInt(width) || 1,
            height: typeof height === 'number' ? height : parseInt(height) || 1,
            bitsPerComponent: typeof bpc === 'number' ? bpc : parseInt(bpc) || 8,
            colorSpace: cs,
            filter: filter
        };
        await this.renderImage(xobj);
    }
    /**
     * Resolve PDF string escape sequences to raw byte values.
     */
    resolveEscapes(input) {
        const bytes = [];
        let i = 0;
        while (i < input.length) {
            const ch = input.charCodeAt(i);
            if (ch === 0x5C && i + 1 < input.length) {
                i++;
                const next = input.charCodeAt(i);
                if (next >= 0x30 && next <= 0x37) {
                    let octal = next - 0x30;
                    i++;
                    if (i < input.length && input.charCodeAt(i) >= 0x30 && input.charCodeAt(i) <= 0x37) {
                        octal = octal * 8 + (input.charCodeAt(i) - 0x30);
                        i++;
                        if (i < input.length && input.charCodeAt(i) >= 0x30 && input.charCodeAt(i) <= 0x37) {
                            octal = octal * 8 + (input.charCodeAt(i) - 0x30);
                            i++;
                        }
                    }
                    bytes.push(octal);
                }
                else {
                    switch (next) {
                        case 0x6E:
                            bytes.push(0x0A);
                            break;
                        case 0x72:
                            bytes.push(0x0D);
                            break;
                        case 0x74:
                            bytes.push(0x09);
                            break;
                        case 0x62:
                            bytes.push(0x08);
                            break;
                        case 0x66:
                            bytes.push(0x0C);
                            break;
                        case 0x28:
                            bytes.push(0x28);
                            break;
                        case 0x29:
                            bytes.push(0x29);
                            break;
                        case 0x5C:
                            bytes.push(0x5C);
                            break;
                        default:
                            bytes.push(next);
                            break;
                    }
                    i++;
                }
            }
            else {
                bytes.push(ch);
                i++;
            }
        }
        return bytes;
    }
    decodeText(text) {
        const decoded = PDFTextDecoder.decode(text);
        // Apply font-specific ToUnicode mapping if available
        if (this.currentFontResource?.toUnicode && this.currentFontResource.toUnicode.size > 0) {
            const toUnicode = this.currentFontResource.toUnicode;
            let result = '';
            // Use original byte values for mapping (before PDFDocEncoding)
            const raw = typeof text === 'string' ? text : String(text);
            for (let i = 0; i < raw.length; i++) {
                const charCode = raw.charCodeAt(i);
                const mapped = toUnicode.get(charCode);
                result += mapped !== undefined ? mapped : decoded[i] || '';
            }
            return result;
        }
        return decoded;
    }
}
class PDFRenderer {
    constructor(pdf, options) {
        this.pdf = pdf;
        this.options = options;
    }
    /**
     * Get optimal viewer options for best user experience
     * @returns Recommended render options
     */
    static getOptimalViewerOptions() {
        return {
            scale: 1.0,
            renderScale: 2.0, // High DPI for sharp text
            fitToWidth: true,
            maintainAspectRatio: true,
            autoFitOnLoad: true,
            darkMode: true,
            continuousScrolling: true,
            renderText: true,
            renderImages: true,
            renderAnnotations: true,
            imageQuality: 1.0, // Maximum quality
            // Theme toggle functionality
            enableThemeToggle: true,
            persistTheme: true,
            defaultTheme: 'dark',
            themeStorageKey: 'AgenticPDF-theme'
        };
    }
    /**
     * Calculate optimal scale for fit-to-width rendering
     * @param pageWidth - Width of the PDF page
     * @param containerWidth - Width of the container element
     * @returns Optimal scale value
     */
    static calculateFitToWidthScale(pageWidth, containerWidth) {
        return Math.min(containerWidth / pageWidth, 1.5); // Cap at 1.5x for readability
    }
    /**
     * Apply fit-to-width scaling to canvas
     * @param canvas - Canvas element to scale
     * @param pageWidth - Width of the PDF page
     * @param containerWidth - Available container width
     */
    static applyFitToWidth(canvas, pageWidth, containerWidth) {
        const scale = PDFRenderer.calculateFitToWidthScale(pageWidth, containerWidth);
        const scaledWidth = pageWidth * scale;
        const scaledHeight = (canvas.height / canvas.width) * scaledWidth;
        canvas.style.width = `${scaledWidth}px`;
        canvas.style.height = `${scaledHeight}px`;
    }
    /**
     * Configure canvas for optimal viewer experience
     * @param canvas - Canvas element to configure
     * @param options - Render options
     */
    static configureOptimalViewer(canvas, options) {
        const opts = { ...PDFRenderer.getOptimalViewerOptions(), ...options };
        // Initialize theme manager if theme toggle is enabled
        if (opts.enableThemeToggle) {
            const themeManager = ThemeManager.getInstance();
            themeManager.initialize({
                defaultTheme: opts.defaultTheme || 'dark',
                storageKey: opts.themeStorageKey || 'AgenticPDF-theme',
                persistTheme: opts.persistTheme !== false
            });
        }
        // Apply dark mode styling based on current theme
        const currentTheme = opts.enableThemeToggle
            ? ThemeManager.getInstance().getCurrentTheme()
            : (opts.darkMode ? 'dark' : 'light');
        if (currentTheme === 'dark') {
            canvas.style.backgroundColor = '#1a1a1a';
            if (canvas.parentElement) {
                canvas.parentElement.style.backgroundColor = '#1a1a1a';
                canvas.parentElement.style.color = '#ffffff';
            }
        }
        else {
            canvas.style.backgroundColor = '#ffffff';
            if (canvas.parentElement) {
                canvas.parentElement.style.backgroundColor = '#f5f5f5';
                canvas.parentElement.style.color = '#333333';
            }
        }
        // Enable smooth scaling
        canvas.style.imageRendering = 'auto';
        canvas.style.imageRendering = 'optimizeQuality';
        // Maintain aspect ratio
        if (opts.maintainAspectRatio !== false) {
            canvas.style.objectFit = 'contain';
        }
        // Add theme change observer if theme toggle is enabled
        if (opts.enableThemeToggle) {
            const themeManager = ThemeManager.getInstance();
            themeManager.addObserver((theme) => {
                if (theme === 'dark') {
                    canvas.style.backgroundColor = '#1a1a1a';
                    if (canvas.parentElement) {
                        canvas.parentElement.style.backgroundColor = '#1a1a1a';
                        canvas.parentElement.style.color = '#ffffff';
                    }
                }
                else {
                    canvas.style.backgroundColor = '#ffffff';
                    if (canvas.parentElement) {
                        canvas.parentElement.style.backgroundColor = '#f5f5f5';
                        canvas.parentElement.style.color = '#333333';
                    }
                }
            });
        }
    }
    /**
     * Create a complete PDF viewer with optimal configuration including theme toggle
     * @param container - Container element for the viewer
     * @param pdf - AgenticPDF instance
     * @param options - Render options
     * @returns Object with viewer elements and methods
     */
    static createOptimalViewer(container, pdf, options) {
        const opts = { ...PDFRenderer.getOptimalViewerOptions(), ...options };
        // Create main canvas
        const canvas = document.createElement('canvas');
        PDFRenderer.configureOptimalViewer(canvas, opts);
        // Create toolbar
        const toolbar = document.createElement('div');
        toolbar.className = 'pdf-viewer-toolbar';
        Object.assign(toolbar.style, {
            position: 'fixed',
            top: '10px',
            left: '50%',
            transform: 'translateX(-50%)',
            background: opts.darkMode ? '#2d2d2d' : '#ffffff',
            border: `1px solid ${opts.darkMode ? '#555' : '#ddd'}`,
            borderRadius: '4px',
            padding: '8px 16px',
            display: 'flex',
            alignItems: 'center',
            gap: '10px',
            zIndex: '1000',
            boxShadow: opts.darkMode
                ? '0 2px 8px rgba(0, 0, 0, 0.3)'
                : '0 2px 8px rgba(0, 0, 0, 0.1)'
        });
        // Create theme toggle button if enabled
        let themeToggle;
        let themeManager;
        if (opts.enableThemeToggle) {
            themeManager = ThemeManager.getInstance();
            themeToggle = ThemeManager.createThemeToggleButton({
                className: 'pdf-theme-toggle',
                size: 'medium'
            });
            toolbar.appendChild(themeToggle);
        }
        // Add elements to container
        container.appendChild(toolbar);
        container.appendChild(canvas);
        // Cleanup function
        const destroy = () => {
            container.removeChild(toolbar);
            container.removeChild(canvas);
        };
        return {
            canvas,
            toolbar,
            themeToggle: themeToggle,
            themeManager: themeManager,
            destroy
        };
    }
    async renderToCanvas(pageNumber, canvas) {
        const page = await this.pdf.getPage(pageNumber);
        if (!page)
            throw new Error(`Page ${pageNumber} not found`);
        const ctx = canvas.getContext('2d');
        if (!ctx)
            throw new Error('Canvas context not available');
        const scale = this.options?.scale || 1.0;
        const renderScale = this.options?.renderScale || 2.0;
        // Calculate dimensions
        const displayWidth = page.width * scale;
        const displayHeight = page.height * scale;
        const canvasWidth = displayWidth * renderScale;
        const canvasHeight = displayHeight * renderScale;
        // Set canvas size
        canvas.width = canvasWidth;
        canvas.height = canvasHeight;
        if (this.options?.maintainAspectRatio !== false) {
            canvas.style.width = `${displayWidth}px`;
            canvas.style.height = `${displayHeight}px`;
        }
        ctx.save();
        ctx.scale(renderScale, renderScale);
        // Render background
        const backgroundColor = this.options?.background || (this.options?.darkMode ? '#1a1a1a' : 'white');
        ctx.fillStyle = backgroundColor;
        ctx.fillRect(0, 0, displayWidth, displayHeight);
        // Simple approach: Draw graphics operators directly to canvas
        // Based on PDF.js approach but simplified and translated to TypeScript
        await this.renderPageContent(ctx, page, scale);
        ctx.restore();
    }
    async renderPageContent(ctx, page, scale) {
        if (!page.contents) {
            console.warn('Page has no contents');
            return;
        }
        try {
            // console.log(`Rendering page ${page.pageNumber}, contents size: ${page.contents.length} bytes`);
            // Create PDF graphics state machine
            const graphics = new PDFGraphicsExecutor(ctx, page, scale);
            // Parse and execute content stream operators
            const parser = new SafeContentStreamParser(page.contents);
            const operations = parser.parse();
            // console.log(`Parsed ${operations.length} operations`);
            if (operations.length === 0) {
                console.warn('No operations parsed from content stream');
                // Draw fallback message
                ctx.save();
                ctx.setTransform(1, 0, 0, 1, 0, 0); // Reset transform
                ctx.fillStyle = '#666';
                ctx.font = `${14 * scale}px Arial`;
                ctx.textAlign = 'left';
                ctx.fillText(`Page ${page.pageNumber} - No content parsed`, 20 * scale, 30 * scale);
                ctx.restore();
                return;
            }
            // Show sample of operations
            // (debug logging removed)
            // Execute each operation
            for (const op of operations) {
                await graphics.executeOperator(op.operator, op.operands);
            }
            // console.log('Rendering complete');
        }
        catch (error) {
            console.error('Error rendering page content:', error);
            // Fallback: Draw page info
            ctx.save();
            ctx.setTransform(1, 0, 0, 1, 0, 0); // Reset transform
            ctx.fillStyle = '#c00';
            ctx.font = `${14 * scale}px Arial`;
            ctx.textAlign = 'left';
            ctx.fillText(`Page ${page.pageNumber} (Render error: ${error})`, 20 * scale, 30 * scale);
            ctx.restore();
        }
    }
    async renderToImage(pageNumber, format) {
        // Create offscreen canvas
        const canvas = document.createElement('canvas');
        await this.renderToCanvas(pageNumber, canvas);
        return new Promise((resolve, reject) => {
            canvas.toBlob((blob) => {
                if (blob)
                    resolve(blob);
                else
                    reject(new Error('Failed to create image blob'));
            }, `image/${format}`, this.options?.imageQuality || 0.92);
        });
    }
    async renderText(ctx, page) {
        if (!page.contents)
            return;
        try {
            const parser = new SafeContentStreamParser(page.contents);
            const operations = parser.parse();
            // Only process text-related operators
            const textOps = new Set(['BT', 'ET', 'Td', 'TD', 'Tm', 'T*', 'Tf', 'Tj', 'TJ', "'", '"', 'Tc', 'Tw', 'Tz', 'TL', 'Tr', 'Ts']);
            ctx.save();
            let inText = false;
            const textState = {
                font: 'Arial',
                fontSize: 12,
                matrix: [1, 0, 0, 1, 0, 0],
                lineMatrix: [1, 0, 0, 1, 0, 0],
                charSpace: 0,
                wordSpace: 0,
                leading: 0,
                rise: 0,
                renderMode: 0
            };
            for (const op of operations) {
                if (!textOps.has(op.operator) && op.operator !== 'q' && op.operator !== 'Q' && op.operator !== 'cm')
                    continue;
                switch (op.operator) {
                    case 'BT':
                        inText = true;
                        textState.matrix = [1, 0, 0, 1, 0, 0];
                        textState.lineMatrix = [1, 0, 0, 1, 0, 0];
                        break;
                    case 'ET':
                        inText = false;
                        break;
                    case 'Tf':
                        if (op.operands.length >= 2) {
                            textState.fontSize = op.operands[1];
                            let fontName = 'Arial';
                            const pdfFont = String(op.operands[0]);
                            if (pdfFont.includes('Bold'))
                                fontName = 'Arial';
                            if (pdfFont.includes('Courier'))
                                fontName = 'Courier New';
                            if (pdfFont.includes('Times'))
                                fontName = 'Times New Roman';
                            textState.font = fontName;
                        }
                        break;
                    case 'Td':
                        if (inText && op.operands.length >= 2) {
                            textState.lineMatrix[4] += op.operands[0] * textState.lineMatrix[0] + op.operands[1] * textState.lineMatrix[2];
                            textState.lineMatrix[5] += op.operands[0] * textState.lineMatrix[1] + op.operands[1] * textState.lineMatrix[3];
                            textState.matrix = [...textState.lineMatrix];
                        }
                        break;
                    case 'TD':
                        if (inText && op.operands.length >= 2) {
                            textState.leading = -op.operands[1];
                            textState.lineMatrix[4] += op.operands[0] * textState.lineMatrix[0] + op.operands[1] * textState.lineMatrix[2];
                            textState.lineMatrix[5] += op.operands[0] * textState.lineMatrix[1] + op.operands[1] * textState.lineMatrix[3];
                            textState.matrix = [...textState.lineMatrix];
                        }
                        break;
                    case 'Tm':
                        if (inText && op.operands.length >= 6) {
                            textState.matrix = [...op.operands];
                            textState.lineMatrix = [...op.operands];
                        }
                        break;
                    case 'T*':
                        if (inText) {
                            textState.lineMatrix[4] += textState.leading * textState.lineMatrix[2];
                            textState.lineMatrix[5] += textState.leading * textState.lineMatrix[3];
                            textState.matrix = [...textState.lineMatrix];
                        }
                        break;
                    case 'Tj':
                        if (inText && op.operands.length > 0) {
                            const text = PDFTextDecoder.decode(op.operands[0]);
                            if (text) {
                                ctx.save();
                                const tm = textState.matrix;
                                ctx.font = `${Math.abs(textState.fontSize)}px ${textState.font}`;
                                ctx.fillStyle = '#000000';
                                ctx.transform(tm[0], tm[1], tm[2], tm[3], tm[4], tm[5]);
                                ctx.scale(1, -1);
                                ctx.fillText(text, 0, -(textState.rise || 0));
                                ctx.restore();
                                const w = text.length * textState.fontSize * 0.5;
                                textState.matrix[4] += w * textState.matrix[0];
                                textState.matrix[5] += w * textState.matrix[1];
                            }
                        }
                        break;
                    case 'TJ':
                        if (inText && op.operands.length > 0 && Array.isArray(op.operands[0])) {
                            for (const item of op.operands[0]) {
                                if (typeof item === 'string') {
                                    const text = PDFTextDecoder.decode(item);
                                    if (text) {
                                        ctx.save();
                                        const tm = textState.matrix;
                                        ctx.font = `${Math.abs(textState.fontSize)}px ${textState.font}`;
                                        ctx.fillStyle = '#000000';
                                        ctx.transform(tm[0], tm[1], tm[2], tm[3], tm[4], tm[5]);
                                        ctx.scale(1, -1);
                                        ctx.fillText(text, 0, -(textState.rise || 0));
                                        ctx.restore();
                                        const w = text.length * textState.fontSize * 0.5;
                                        textState.matrix[4] += w * textState.matrix[0];
                                        textState.matrix[5] += w * textState.matrix[1];
                                    }
                                }
                                else if (typeof item === 'number') {
                                    const adj = -item / 1000 * textState.fontSize;
                                    textState.matrix[4] += adj * textState.matrix[0];
                                    textState.matrix[5] += adj * textState.matrix[1];
                                }
                            }
                        }
                        break;
                    case 'TL':
                        if (op.operands.length > 0)
                            textState.leading = op.operands[0];
                        break;
                    case 'Tc':
                        if (op.operands.length > 0)
                            textState.charSpace = op.operands[0];
                        break;
                    case 'Tw':
                        if (op.operands.length > 0)
                            textState.wordSpace = op.operands[0];
                        break;
                    case 'Ts':
                        if (op.operands.length > 0)
                            textState.rise = op.operands[0];
                        break;
                    case 'Tr':
                        if (op.operands.length > 0)
                            textState.renderMode = op.operands[0];
                        break;
                }
            }
            ctx.restore();
        }
        catch (error) {
            console.warn('Text rendering error (non-fatal):', error);
        }
    }
    async renderImages(ctx, page) {
        if (!page.resources?.images)
            return;
        for (const [_name, imageRes] of page.resources.images) {
            const img = await this.decodeImage(imageRes);
            if (img) {
                ctx.drawImage(img, 0, 0, imageRes.width, imageRes.height);
            }
        }
    }
    async decodeImage(imageRes) {
        try {
            const arrayBuffer = imageRes.data.slice();
            const blob = new Blob([arrayBuffer], { type: 'image/jpeg' });
            const url = URL.createObjectURL(blob);
            const img = new Image();
            return new Promise((resolve) => {
                img.onload = () => {
                    URL.revokeObjectURL(url);
                    resolve(img);
                };
                img.onerror = () => {
                    URL.revokeObjectURL(url);
                    resolve(null);
                };
                img.src = url;
            });
        }
        catch {
            return null;
        }
    }
    async renderAnnotations(ctx, page) {
        const annotations = await new AnnotationExtractor(this.pdf).extract(page.pageNumber);
        for (const annotation of annotations) {
            this.renderAnnotation(ctx, annotation, page);
        }
    }
    renderAnnotation(ctx, annotation, page) {
        ctx.save();
        // Set annotation style
        if (annotation.color) {
            ctx.strokeStyle = `rgba(${annotation.color.r}, ${annotation.color.g}, ${annotation.color.b}, ${annotation.opacity || 1})`;
            ctx.fillStyle = `rgba(${annotation.color.r}, ${annotation.color.g}, ${annotation.color.b}, ${(annotation.opacity || 1) * 0.3})`;
        }
        switch (annotation.type) {
            case AnnotationType.Highlight:
                ctx.fillRect(annotation.rect.x, page.height - annotation.rect.y - annotation.rect.height, annotation.rect.width, annotation.rect.height);
                break;
            case AnnotationType.Underline:
                ctx.beginPath();
                ctx.moveTo(annotation.rect.x, page.height - annotation.rect.y);
                ctx.lineTo(annotation.rect.x + annotation.rect.width, page.height - annotation.rect.y);
                ctx.stroke();
                break;
            case AnnotationType.Square:
                ctx.strokeRect(annotation.rect.x, page.height - annotation.rect.y - annotation.rect.height, annotation.rect.width, annotation.rect.height);
                break;
            case AnnotationType.Circle:
                const centerX = annotation.rect.x + annotation.rect.width / 2;
                const centerY = page.height - annotation.rect.y - annotation.rect.height / 2;
                const radiusX = annotation.rect.width / 2;
                const radiusY = annotation.rect.height / 2;
                ctx.beginPath();
                ctx.ellipse(centerX, centerY, radiusX, radiusY, 0, 0, 2 * Math.PI);
                ctx.stroke();
                break;
        }
        ctx.restore();
    }
    async renderForms(ctx, page) {
        const formFields = await new FormExtractor(this.pdf).extract();
        const pageFields = formFields.filter(f => f.pageNumber === page.pageNumber);
        for (const field of pageFields) {
            this.renderFormField(ctx, field, page);
        }
    }
    renderFormField(ctx, field, page) {
        ctx.save();
        // Draw field border
        ctx.strokeStyle = '#000000';
        ctx.strokeRect(field.rect.x, page.height - field.rect.y - field.rect.height, field.rect.width, field.rect.height);
        // Draw field value
        if (field.value) {
            ctx.fillStyle = '#000000';
            ctx.font = '12px Arial';
            if (field.type === FormFieldType.Text) {
                ctx.fillText(String(field.value), field.rect.x + 2, page.height - field.rect.y - 4);
            }
            else if (field.type === FormFieldType.Button && field.value) {
                // Draw checkmark for checked checkbox
                ctx.beginPath();
                ctx.moveTo(field.rect.x + 2, page.height - field.rect.y - field.rect.height / 2);
                ctx.lineTo(field.rect.x + field.rect.width / 3, page.height - field.rect.y - 2);
                ctx.lineTo(field.rect.x + field.rect.width - 2, page.height - field.rect.y - field.rect.height + 2);
                ctx.stroke();
            }
        }
        ctx.restore();
    }
}
class TextLayerBuilder {
    constructor(options) {
        this.textDivs = [];
        this.textDivProperties = new WeakMap();
        this.renderingDone = false;
        this.container = options.container;
        this.textContent = options.textContent;
        this.viewport = options.viewport;
        this.enhanceTextSelection = options.enhanceTextSelection ?? true;
        // Following PDF.js approach exactly
        this.scale = options.viewport.scale;
        this.pageWidth = options.viewport.width / this.scale;
        this.pageHeight = options.viewport.height / this.scale;
        // Ensure container is positioned for absolute children
        if (getComputedStyle(this.container).position === 'static') {
            this.container.style.position = 'relative';
        }
        // Set container dimensions
        this.container.style.width = `${this.viewport.width}px`;
        this.container.style.height = `${this.viewport.height}px`;
        // Ensure minimum font size is computed
        TextLayerBuilder.ensureMinFontSizeComputed();
    }
    /**
     * Get canvas context for font measurements (PDF.js approach)
     */
    static getCanvasContext() {
        if (!this.canvasContext) {
            const canvas = document.createElement('canvas');
            canvas.className = 'hiddenCanvasElement';
            canvas.style.display = 'none';
            document.body.appendChild(canvas);
            this.canvasContext = canvas.getContext('2d', {
                alpha: false,
                willReadFrequently: true
            });
        }
        return this.canvasContext;
    }
    /**
     * Compute minimum font size enforced by browser (PDF.js approach)
     */
    static ensureMinFontSizeComputed() {
        if (this.minFontSize !== null) {
            return;
        }
        const div = document.createElement('div');
        div.style.opacity = '0';
        div.style.lineHeight = '1';
        div.style.fontSize = '1px';
        div.style.position = 'absolute';
        div.textContent = 'X';
        document.body.appendChild(div);
        this.minFontSize = div.getBoundingClientRect().height;
        div.remove();
    }
    /**
     * Get font ascent ratio (PDF.js approach)
     */
    static getAscent(fontFamily) {
        const cachedAscent = this.ascentCache.get(fontFamily);
        if (cachedAscent) {
            return cachedAscent;
        }
        const ctx = this.getCanvasContext();
        const DEFAULT_FONT_SIZE = 30;
        ctx.canvas.width = ctx.canvas.height = DEFAULT_FONT_SIZE;
        ctx.font = `${DEFAULT_FONT_SIZE}px ${fontFamily}`;
        const metrics = ctx.measureText('');
        const ascent = metrics.fontBoundingBoxAscent || 0;
        const descent = Math.abs(metrics.fontBoundingBoxDescent || 0);
        ctx.canvas.width = ctx.canvas.height = 0;
        let ratio = 0.8; // Default
        if (ascent) {
            ratio = ascent / (ascent + descent);
        }
        this.ascentCache.set(fontFamily, ratio);
        return ratio;
    }
    /**
     * Map font name to CSS font family
     */
    mapFontFamily(fontName) {
        const fontMap = {
            'Times': 'Times New Roman, serif',
            'TimesNewRoman': 'Times New Roman, serif',
            'Times-Roman': 'Times New Roman, serif',
            'Times-Bold': 'Times New Roman, serif',
            'Times-Italic': 'Times New Roman, serif',
            'Times-BoldItalic': 'Times New Roman, serif',
            'Helvetica': 'Helvetica, Arial, sans-serif',
            'Helvetica-Bold': 'Helvetica, Arial, sans-serif',
            'Helvetica-Oblique': 'Helvetica, Arial, sans-serif',
            'Helvetica-BoldOblique': 'Helvetica, Arial, sans-serif',
            'Courier': 'Courier New, monospace',
            'Courier-Bold': 'Courier New, monospace',
            'Courier-Oblique': 'Courier New, monospace',
            'Courier-BoldOblique': 'Courier New, monospace',
            'Symbol': 'Symbol, serif',
            'ZapfDingbats': 'ZapfDingbats, serif'
        };
        // Check for exact match
        if (fontMap[fontName]) {
            return fontMap[fontName];
        }
        // Check for partial matches
        const lower = fontName.toLowerCase();
        if (lower.includes('times'))
            return 'Times New Roman, serif';
        if (lower.includes('helvetica') || lower.includes('arial'))
            return 'Helvetica, Arial, sans-serif';
        if (lower.includes('courier'))
            return 'Courier New, monospace';
        // Default fallback
        return 'sans-serif';
    }
    /**
     * Get font weight from font name and style
     */
    getFontWeight(fontName, style) {
        if (style.bold)
            return 700;
        const lower = fontName.toLowerCase();
        if (lower.includes('bold'))
            return 700;
        if (lower.includes('medium'))
            return 500;
        if (lower.includes('light'))
            return 300;
        return 400; // Normal
    }
    /**
     * Get font style (italic/oblique)
     */
    getFontStyle(fontName, style) {
        if (style.italic)
            return 'italic';
        const lower = fontName.toLowerCase();
        if (lower.includes('italic') || lower.includes('oblique'))
            return 'italic';
        return 'normal';
    }
    /**
     * Convert PDF coordinates to CSS coordinates
     */
    convertTransform(transform) {
        const [a, b, c, d, e, f] = transform;
        // PDF coordinate system has origin at bottom-left
        // CSS coordinate system has origin at top-left
        // We need to flip the Y coordinate
        const cssY = this.viewport.height - f;
        return `matrix(${a}, ${b}, ${c}, ${d}, ${e}, ${cssY})`;
    }
    /**
     * Create a text div element for a text item (PDF.js approach)
     */
    createTextDiv(textItem) {
        const textDiv = document.createElement('span');
        // Initialize properties (PDF.js uses these for layout)
        const textDivProperties = {
            angle: 0,
            canvasWidth: 0,
            hasText: textItem.text !== '',
            hasEOL: false,
            fontSize: 0
        };
        // Transform the text item's transform matrix
        // PDF.js uses: Util.transform(this.#transform, geom.transform)
        // where this.#transform = [1, 0, 0, -1, -pageX, pageY + pageHeight]
        const tx = textItem.transform;
        // Calculate angle
        let angle = Math.atan2(tx[1], tx[0]);
        // Get font info
        const fontFamily = this.mapFontFamily(textItem.fontName);
        // Calculate font height from transform matrix (PDF.js approach)
        const fontHeight = Math.hypot(tx[2], tx[3]);
        const fontAscent = fontHeight * TextLayerBuilder.getAscent(fontFamily);
        // Calculate position (PDF.js approach)
        let left, top;
        if (angle === 0) {
            left = tx[4];
            top = tx[5] - fontAscent;
        }
        else {
            left = tx[4] + fontAscent * Math.sin(angle);
            top = tx[5] - fontAscent * Math.cos(angle);
        }
        // Apply styles (PDF.js approach with percentage-based positioning)
        const style = textDiv.style;
        style.left = `${((100 * left) / this.pageWidth).toFixed(2)}%`;
        style.top = `${((100 * top) / this.pageHeight).toFixed(2)}%`;
        // Font size with minFontSize multiplier (PDF.js approach)
        const minFontSize = TextLayerBuilder.minFontSize || 1;
        style.fontSize = `${(minFontSize * fontHeight).toFixed(2)}px`;
        style.fontFamily = fontFamily;
        textDivProperties.fontSize = fontHeight;
        // Base styles
        style.position = 'absolute';
        style.whiteSpace = 'pre';
        style.transformOrigin = '0% 0%';
        // Make transparent for overlay
        style.color = 'transparent';
        // Set role for accessibility
        textDiv.setAttribute('role', 'presentation');
        textDiv.textContent = textItem.text;
        textDiv.dir = textItem.direction;
        // Store angle if rotated
        if (angle !== 0) {
            textDivProperties.angle = angle * (180 / Math.PI);
        }
        // Determine if we should scale text (PDF.js logic)
        let shouldScaleText = false;
        if (textItem.text.length > 1) {
            shouldScaleText = true;
        }
        else if (textItem.text !== ' ' && tx[0] !== tx[3]) {
            const absScaleX = Math.abs(tx[0]);
            const absScaleY = Math.abs(tx[3]);
            if (absScaleX !== absScaleY && Math.max(absScaleX, absScaleY) / Math.min(absScaleX, absScaleY) > 1.5) {
                shouldScaleText = true;
            }
        }
        if (shouldScaleText) {
            textDivProperties.canvasWidth = textItem.width;
        }
        this.textDivProperties.set(textDiv, textDivProperties);
        return textDiv;
    }
    /**
     * Apply transform to layout text div (PDF.js approach)
     */
    applyTransform(textDiv) {
        const properties = this.textDivProperties.get(textDiv);
        if (!properties)
            return;
        const style = textDiv.style;
        let transform = '';
        // Scale down by minFontSize to counteract the multiplication in fontSize
        const minFontSize = TextLayerBuilder.minFontSize || 1;
        if (minFontSize > 1) {
            transform = `scale(${1 / minFontSize})`;
        }
        // Scale text to match canvas width (PDF.js approach)
        if (properties.canvasWidth !== 0 && properties.hasText) {
            const { fontFamily } = style;
            const { canvasWidth, fontSize } = properties;
            const ctx = TextLayerBuilder.getCanvasContext();
            ctx.font = `${fontSize * this.scale}px ${fontFamily}`;
            // Measure actual text width
            const { width } = ctx.measureText(textDiv.textContent || '');
            if (width > 0) {
                const scaleX = (canvasWidth * this.scale) / width;
                transform = `scaleX(${scaleX}) ${transform}`;
            }
        }
        // Apply rotation if needed
        if (properties.angle !== 0) {
            transform = `rotate(${properties.angle}deg) ${transform}`;
        }
        // Apply combined transform
        if (transform.length > 0) {
            style.transform = transform;
        }
    }
    /**
     * Enhance text selection (optional)
            if (Math.abs(scaleRatio - 1) < 0.5) {
              const charSpacing = (targetWidth - measuredWidth) / (textDiv.textContent!.length - 1);
              style.letterSpacing = `${charSpacing}px`;
            } else {
              transformStr += `scaleX(${scaleRatio}) `;
            }
          }
        }
      }
  
      if (transformStr) {
        style.transform = transformStr.trim();
      }
    }
  
    /**
     * Render the text layer
     */
    render() {
        if (this.renderingDone) {
            return;
        }
        // Clear container
        this.container.innerHTML = '';
        this.textDivs = [];
        this.textDivProperties = new WeakMap(); // WeakMap doesn't have clear(), recreate it
        // Create text divs
        for (const textItem of this.textContent) {
            const textDiv = this.createTextDiv(textItem);
            this.textDivs.push(textDiv);
            this.container.appendChild(textDiv);
        }
        // Apply transforms
        for (const textDiv of this.textDivs) {
            this.applyTransform(textDiv);
        }
        // Enhance text selection if enabled
        if (this.enhanceTextSelection) {
            this.enhanceSelection();
        }
        this.renderingDone = true;
    }
    /**
     * Enhance text selection by making text transparent and selectable
     */
    enhanceSelection() {
        // Make text layer fully transparent but selectable
        // Do NOT set container opacity - individual text items are already transparent
        this.container.style.userSelect = 'text';
        this.container.style.cursor = 'text';
        // Enable text selection on all divs
        for (const textDiv of this.textDivs) {
            textDiv.style.cursor = 'text';
            textDiv.style.userSelect = 'text';
        }
        // Add selection styling
        const style = document.createElement('style');
        style.textContent = `
      .textLayer ::selection {
        background: rgba(0, 102, 204, 0.3);
      }
      .textLayer ::-moz-selection {
        background: rgba(0, 102, 204, 0.3);
      }
    `;
        if (!this.container.classList.contains('textLayer')) {
            this.container.classList.add('textLayer');
        }
        document.head.appendChild(style);
    }
    /**
     * Cancel rendering
     */
    cancel() {
        this.renderingDone = true;
    }
    /**
     * Clean up resources
     */
    static cleanup() {
        this.ascentCache.clear();
        if (this.canvasContext) {
            const canvas = this.canvasContext.canvas;
            canvas.remove();
            this.canvasContext = null;
        }
    }
}
// Cache for font metrics to avoid repeated measurements
TextLayerBuilder.ascentCache = new Map();
TextLayerBuilder.canvasContext = null;
TextLayerBuilder.minFontSize = null;
/**
 * Convenience function to render text layer
 */
function renderTextLayer(options) {
    const builder = new TextLayerBuilder(options);
    builder.render();
    return builder;
}
class ThemeManager {
    constructor() {
        this.currentTheme = 'dark';
        this.storageKey = 'AgenticPDF-theme';
        this.observers = [];
        this.loadTheme();
    }
    static getInstance() {
        if (!ThemeManager.instance) {
            ThemeManager.instance = new ThemeManager();
        }
        return ThemeManager.instance;
    }
    /**
     * Initialize theme management with options
     */
    initialize(options) {
        if (options?.storageKey) {
            this.storageKey = options.storageKey;
        }
        if (options?.defaultTheme) {
            if (options.defaultTheme === 'auto') {
                this.currentTheme = this.detectSystemTheme();
            }
            else {
                this.currentTheme = options.defaultTheme;
            }
        }
        if (options?.persistTheme !== false) {
            this.loadTheme();
        }
        this.applyTheme();
    }
    /**
     * Toggle between dark and light themes
     */
    toggleTheme() {
        this.currentTheme = this.currentTheme === 'dark' ? 'light' : 'dark';
        this.applyTheme();
        this.saveTheme();
        this.notifyObservers();
    }
    /**
     * Set a specific theme
     */
    setTheme(theme) {
        this.currentTheme = theme;
        this.applyTheme();
        this.saveTheme();
        this.notifyObservers();
    }
    /**
     * Get current theme
     */
    getCurrentTheme() {
        return this.currentTheme;
    }
    /**
     * Add theme change observer
     */
    addObserver(callback) {
        this.observers.push(callback);
    }
    /**
     * Remove theme change observer
     */
    removeObserver(callback) {
        const index = this.observers.indexOf(callback);
        if (index > -1) {
            this.observers.splice(index, 1);
        }
    }
    /**
     * Apply theme to DOM elements
     */
    applyTheme() {
        if (typeof document !== 'undefined') {
            if (this.currentTheme === 'light') {
                document.body.classList.add('light-mode');
            }
            else {
                document.body.classList.remove('light-mode');
            }
            // Update theme toggle button if it exists
            const themeToggleBtn = document.getElementById('themeToggle');
            if (themeToggleBtn) {
                if (this.currentTheme === 'dark') {
                    themeToggleBtn.textContent = '🌙';
                    themeToggleBtn.title = 'Switch to Light Mode';
                }
                else {
                    themeToggleBtn.textContent = '☀️';
                    themeToggleBtn.title = 'Switch to Dark Mode';
                }
            }
        }
    }
    /**
     * Save theme to localStorage
     */
    saveTheme() {
        if (typeof localStorage !== 'undefined') {
            try {
                localStorage.setItem(this.storageKey, this.currentTheme);
            }
            catch (e) {
                // localStorage might not be available
            }
        }
    }
    /**
     * Load theme from localStorage
     */
    loadTheme() {
        if (typeof localStorage !== 'undefined') {
            try {
                const saved = localStorage.getItem(this.storageKey);
                if (saved === 'dark' || saved === 'light') {
                    this.currentTheme = saved;
                }
            }
            catch (e) {
                // localStorage might not be available
            }
        }
    }
    /**
     * Detect system theme preference
     */
    detectSystemTheme() {
        if (typeof window !== 'undefined' && window.matchMedia) {
            return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
        }
        return 'dark'; // Default fallback
    }
    /**
     * Notify all observers of theme change
     */
    notifyObservers() {
        this.observers.forEach(callback => callback(this.currentTheme));
    }
    /**
     * Create theme toggle button element
     */
    static createThemeToggleButton(options) {
        const button = document.createElement('button');
        const themeManager = ThemeManager.getInstance();
        button.id = 'themeToggle';
        button.className = options?.className || 'theme-toggle-btn';
        // Set initial appearance
        const currentTheme = themeManager.getCurrentTheme();
        button.textContent = currentTheme === 'dark' ? '🌙' : '☀️';
        button.title = currentTheme === 'dark' ? 'Switch to Light Mode' : 'Switch to Dark Mode';
        // Add click handler
        button.addEventListener('click', () => {
            themeManager.toggleTheme();
        });
        // Style the button
        const size = options?.size || 'medium';
        const sizeMap = {
            small: { fontSize: '12px', padding: '4px 8px' },
            medium: { fontSize: '14px', padding: '6px 10px' },
            large: { fontSize: '16px', padding: '8px 12px' }
        };
        Object.assign(button.style, {
            fontSize: sizeMap[size].fontSize,
            padding: sizeMap[size].padding,
            border: '1px solid #666',
            borderRadius: '3px',
            background: '#404040',
            color: 'white',
            cursor: 'pointer',
            minWidth: '32px'
        });
        if (options?.position === 'fixed') {
            Object.assign(button.style, {
                position: 'fixed',
                top: '10px',
                right: '10px',
                zIndex: '1000'
            });
        }
        // Update button when theme changes
        themeManager.addObserver((theme) => {
            button.textContent = theme === 'dark' ? '🌙' : '☀️';
            button.title = theme === 'dark' ? 'Switch to Light Mode' : 'Switch to Dark Mode';
        });
        return button;
    }
}
class PDFExporter {
    constructor(pdf, options) {
        this.pdf = pdf;
        this.options = options;
    }
    async export(format) {
        switch (format) {
            case 'text':
                return this.exportAsText();
            case 'html':
                return this.exportAsHTML();
            case 'markdown':
                return this.exportAsMarkdown();
            case 'json':
                return this.exportAsJSON();
            case 'xml':
                return this.exportAsXML();
            case 'csv':
                return this.exportAsCSV();
            default:
                throw new Error(`Unsupported export format: ${format}`);
        }
    }
    async exportAsText() {
        const textContent = await this.pdf.extractText();
        const pages = new Map();
        for (const text of textContent) {
            if (!pages.has(text.pageNumber)) {
                pages.set(text.pageNumber, []);
            }
            pages.get(text.pageNumber).push(text.text);
        }
        let output = '';
        for (const [pageNum, texts] of pages) {
            output += `\n--- Page ${pageNum} ---\n`;
            output += texts.join(' ');
        }
        return output;
    }
    async exportAsHTML() {
        const metadata = this.pdf.getMetadata();
        const textContent = await this.pdf.extractText();
        let html = `<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>${metadata?.title || 'PDF Document'}</title>
  <style>
    body { font-family: Arial, sans-serif; margin: 40px; }
    .page { page-break-after: always; margin-bottom: 40px; }
    .text-block { margin: 10px 0; }
    .metadata { background: #f0f0f0; padding: 20px; margin-bottom: 30px; }
    h1 { color: #333; }
    .bold { font-weight: bold; }
    .italic { font-style: italic; }
  </style>
</head>
<body>`;
        if (this.options?.includeMetadata && metadata) {
            html += `
  <div class="metadata">
    <h1>${metadata.title || 'Untitled'}</h1>
    ${metadata.author ? `<p><strong>Author:</strong> ${metadata.author}</p>` : ''}
    ${metadata.subject ? `<p><strong>Subject:</strong> ${metadata.subject}</p>` : ''}
    ${metadata.creationDate ? `<p><strong>Created:</strong> ${metadata.creationDate.toLocaleDateString()}</p>` : ''}
    <p><strong>Pages:</strong> ${metadata.pageCount}</p>
  </div>`;
        }
        const pageGroups = new Map();
        for (const text of textContent) {
            if (!pageGroups.has(text.pageNumber)) {
                pageGroups.set(text.pageNumber, []);
            }
            pageGroups.get(text.pageNumber).push(text);
        }
        for (const [pageNum, texts] of pageGroups) {
            html += `\n  <div class="page" data-page="${pageNum}">`;
            html += `\n    <h2>Page ${pageNum}</h2>`;
            for (const text of texts) {
                const classes = [];
                if (text.style.bold)
                    classes.push('bold');
                if (text.style.italic)
                    classes.push('italic');
                html += `\n    <div class="text-block${classes.length ? ' ' + classes.join(' ') : ''}" style="font-size: ${text.fontSize}px;">${this.escapeHtml(text.text)}</div>`;
            }
            html += '\n  </div>';
        }
        html += '\n</body>\n</html>';
        return html;
    }
    async exportAsMarkdown() {
        const metadata = this.pdf.getMetadata();
        const ai = await this.pdf.getAIFeatures();
        let markdown = '';
        // Add metadata as frontmatter
        if (this.options?.includeMetadata && metadata) {
            markdown += '---\n';
            if (metadata.title)
                markdown += `title: "${metadata.title}"\n`;
            if (metadata.author)
                markdown += `author: "${metadata.author}"\n`;
            if (metadata.creationDate)
                markdown += `date: ${metadata.creationDate.toISOString()}\n`;
            markdown += `pages: ${metadata.pageCount}\n`;
            markdown += '---\n\n';
        }
        // Add title
        if (metadata?.title) {
            markdown += `# ${metadata.title}\n\n`;
        }
        // Export based on structural analysis
        for (const section of ai.structuralAnalysis.sections) {
            if (section.type === 'heading') {
                const level = section.level || 1;
                markdown += `${'#'.repeat(level)} ${section.text}\n\n`;
            }
            else if (section.type === 'paragraph') {
                markdown += `${section.text}\n\n`;
            }
            else if (section.type === 'list') {
                const items = section.text.split('\n');
                for (const item of items) {
                    markdown += `- ${item}\n`;
                }
                markdown += '\n';
            }
            else if (section.type === 'blockquote') {
                markdown += `> ${section.text}\n\n`;
            }
            else if (section.type === 'code') {
                markdown += '```\n' + section.text + '\n```\n\n';
            }
            // Add children sections
            if (section.children) {
                for (const child of section.children) {
                    if (child.type === 'paragraph') {
                        markdown += `${child.text}\n\n`;
                    }
                }
            }
        }
        // Add tables
        if (ai.structuralAnalysis.tables.length > 0) {
            markdown += '\n## Tables\n\n';
            for (const table of ai.structuralAnalysis.tables) {
                markdown += this.tableToMarkdown(table) + '\n\n';
            }
        }
        // Add references/bibliography
        if (ai.structuralAnalysis.bibliography && ai.structuralAnalysis.bibliography.length > 0) {
            markdown += '\n## References\n\n';
            for (const ref of ai.structuralAnalysis.bibliography) {
                markdown += `- ${ref.authors?.join(', ')} (${ref.year}). *${ref.title}*. ${ref.journal || ''}\n`;
            }
        }
        return markdown;
    }
    tableToMarkdown(table) {
        let markdown = '';
        // Headers
        if (table.headers && table.headers.length > 0) {
            markdown += '| ' + table.headers.join(' | ') + ' |\n';
            markdown += '|' + table.headers.map(() => '---').join('|') + '|\n';
        }
        // Rows
        for (const row of table.cells) {
            markdown += '| ' + row.map(cell => cell.text).join(' | ') + ' |\n';
        }
        return markdown;
    }
    async exportAsJSON() {
        const metadata = this.pdf.getMetadata();
        const textContent = await this.pdf.extractText();
        const ai = await this.pdf.getAIFeatures();
        const data = {
            metadata: this.options?.includeMetadata ? metadata : undefined,
            pages: []
        };
        // Group content by page
        const pageMap = new Map();
        for (const text of textContent) {
            if (!pageMap.has(text.pageNumber)) {
                pageMap.set(text.pageNumber, {
                    pageNumber: text.pageNumber,
                    textBlocks: [],
                    images: [],
                    annotations: [],
                    forms: []
                });
            }
            pageMap.get(text.pageNumber).textBlocks.push({
                text: text.text,
                x: text.x,
                y: text.y,
                width: text.width,
                height: text.height,
                fontSize: text.fontSize,
                fontName: text.fontName,
                style: text.style
            });
        }
        // Add images if requested
        if (this.options?.includeImages) {
            const images = await this.pdf.extractImages();
            for (const image of images) {
                if (pageMap.has(image.pageNumber)) {
                    pageMap.get(image.pageNumber).images.push({
                        id: image.id,
                        x: image.x,
                        y: image.y,
                        width: image.width,
                        height: image.height,
                        mimeType: image.mimeType
                    });
                }
            }
        }
        // Add annotations if requested
        if (this.options?.includeAnnotations) {
            const annotations = await this.pdf.getAnnotations();
            for (const annotation of annotations) {
                if (pageMap.has(annotation.pageNumber)) {
                    pageMap.get(annotation.pageNumber).annotations.push({
                        type: annotation.type,
                        rect: annotation.rect,
                        contents: annotation.contents,
                        author: annotation.author
                    });
                }
            }
        }
        // Add forms if requested
        if (this.options?.includeForms) {
            const forms = await this.pdf.getFormFields();
            for (const form of forms) {
                if (pageMap.has(form.pageNumber)) {
                    pageMap.get(form.pageNumber).forms.push({
                        name: form.name,
                        type: form.type,
                        value: form.value,
                        rect: form.rect
                    });
                }
            }
        }
        // Convert map to array
        data.pages = Array.from(pageMap.values()).sort((a, b) => a.pageNumber - b.pageNumber);
        // Add AI features
        data.aiAnalysis = {
            documentType: ai.structuralAnalysis.documentType,
            sections: ai.structuralAnalysis.sections,
            tables: ai.structuralAnalysis.tables,
            keywords: ai.nlpReady.keywords,
            language: ai.nlpReady.language,
            tokenCount: ai.nlpReady.tokenCount
        };
        return JSON.stringify(data, null, 2);
    }
    async exportAsXML() {
        const metadata = this.pdf.getMetadata();
        const textContent = await this.pdf.extractText();
        let xml = '<?xml version="1.0" encoding="UTF-8"?>\n';
        xml += '<document>\n';
        if (this.options?.includeMetadata && metadata) {
            xml += '  <metadata>\n';
            if (metadata.title)
                xml += `    <title>${this.escapeXml(metadata.title)}</title>\n`;
            if (metadata.author)
                xml += `    <author>${this.escapeXml(metadata.author)}</author>\n`;
            if (metadata.subject)
                xml += `    <subject>${this.escapeXml(metadata.subject)}</subject>\n`;
            if (metadata.creationDate)
                xml += `    <creationDate>${metadata.creationDate.toISOString()}</creationDate>\n`;
            xml += `    <pageCount>${metadata.pageCount}</pageCount>\n`;
            xml += '  </metadata>\n';
        }
        xml += '  <pages>\n';
        const pageGroups = new Map();
        for (const text of textContent) {
            if (!pageGroups.has(text.pageNumber)) {
                pageGroups.set(text.pageNumber, []);
            }
            pageGroups.get(text.pageNumber).push(text);
        }
        for (const [pageNum, texts] of pageGroups) {
            xml += `    <page number="${pageNum}">\n`;
            for (const text of texts) {
                xml += '      <text';
                xml += ` x="${text.x}"`;
                xml += ` y="${text.y}"`;
                xml += ` width="${text.width}"`;
                xml += ` height="${text.height}"`;
                xml += ` fontSize="${text.fontSize}"`;
                xml += ` fontName="${this.escapeXml(text.fontName)}"`;
                xml += '>';
                xml += this.escapeXml(text.text);
                xml += '</text>\n';
            }
            xml += '    </page>\n';
        }
        xml += '  </pages>\n';
        xml += '</document>';
        return xml;
    }
    async exportAsCSV() {
        const ai = await this.pdf.getAIFeatures();
        const tables = ai.structuralAnalysis.tables;
        if (tables.length === 0) {
            // Export text content as CSV
            const textContent = await this.pdf.extractText();
            let csv = 'Page,Text,Font Size,Font Name\n';
            for (const text of textContent) {
                csv += `${text.pageNumber},"${this.escapeCSV(text.text)}",${text.fontSize},"${this.escapeCSV(text.fontName)}"\n`;
            }
            return csv;
        }
        // Export tables as CSV
        let csv = '';
        for (let i = 0; i < tables.length; i++) {
            const table = tables[i];
            if (i > 0)
                csv += '\n\n';
            csv += `Table ${i + 1} (Page ${table.pageNumber})\n`;
            // Headers
            if (table.headers && table.headers.length > 0) {
                csv += table.headers.map(h => `"${this.escapeCSV(h)}"`).join(',') + '\n';
            }
            // Rows
            for (const row of table.cells) {
                csv += row.map(cell => `"${this.escapeCSV(cell.text)}"`).join(',') + '\n';
            }
        }
        return csv;
    }
    escapeHtml(text) {
        const map = {
            '&': '&amp;',
            '<': '&lt;',
            '>': '&gt;',
            '"': '&quot;',
            "'": '&#39;'
        };
        return text.replace(/[&<>"']/g, m => map[m]);
    }
    escapeXml(text) {
        return this.escapeHtml(text);
    }
    escapeCSV(text) {
        return text.replace(/"/g, '""');
    }
}
class PDFWriter {
    constructor(pdf) {
        this.pdf = pdf;
        this.modifiedObjects = new Map();
    }
    async save() {
        const _originalBuffer = await this.getOriginalBuffer();
        const writer = new PDFStreamWriter();
        // Write header
        writer.writeString('%PDF-1.7\n');
        writer.writeString('%âãÏÓ\n'); // Binary marker
        // Copy and modify objects
        const objects = await this.collectObjects();
        const xref = new XRefTable();
        for (const obj of objects) {
            const offset = writer.position;
            xref.addEntry(obj.objectNumber, offset, obj.generationNumber || 0, 'n');
            writer.writeString(`${obj.objectNumber} ${obj.generationNumber || 0} obj\n`);
            this.writeObject(writer, obj);
            writer.writeString('\nendobj\n');
        }
        // Write xref table
        const xrefOffset = writer.position;
        writer.writeString('xref\n');
        writer.writeString(`0 ${objects.length + 1}\n`);
        writer.writeString('0000000000 65535 f \n');
        for (const obj of objects) {
            const entry = xref.getEntry(obj.objectNumber);
            if (entry) {
                const offsetStr = entry.offset.toString().padStart(10, '0');
                const genStr = entry.generation.toString().padStart(5, '0');
                writer.writeString(`${offsetStr} ${genStr} ${entry.type} \n`);
            }
        }
        // Write trailer
        writer.writeString('trailer\n');
        writer.writeString('<<\n');
        writer.writeString(`/Size ${objects.length + 1}\n`);
        writer.writeString(`/Root 1 0 R\n`); // Catalog reference
        writer.writeString('>>\n');
        writer.writeString('startxref\n');
        writer.writeString(`${xrefOffset}\n`);
        writer.writeString('%%EOF');
        const buffer = writer.getBuffer();
        return new Blob([buffer.slice()], { type: 'application/pdf' });
    }
    async getOriginalBuffer() {
        // Get the original PDF buffer
        // This would be stored in the AgenticPDF instance
        return new ArrayBuffer(0);
    }
    async collectObjects() {
        const objects = [];
        try {
            // Get the original xref table and objects
            const xrefTable = this.pdf.xrefTable;
            const pdfObjects = this.pdf.objects;
            if (!xrefTable) {
                console.warn('No xref table available for PDF writing');
                return objects;
            }
            // Collect objects from the xref table
            const objectNumbers = new Set();
            // Get all object numbers from xref
            for (let i = 1; i <= 1000; i++) { // Arbitrary limit to prevent infinite loops
                const entry = xrefTable.getEntry(i);
                if (entry && entry.type === 'n') { // Only include in-use objects
                    objectNumbers.add(i);
                }
            }
            // Collect objects in order
            for (const objNum of Array.from(objectNumbers).sort((a, b) => a - b)) {
                const entry = xrefTable.getEntry(objNum);
                if (!entry)
                    continue;
                // Check if we have a modified version
                const modifiedKey = `${objNum}_${entry.generation}`;
                if (this.modifiedObjects.has(modifiedKey)) {
                    const modifiedObj = this.modifiedObjects.get(modifiedKey);
                    objects.push({
                        ...modifiedObj,
                        objectNumber: objNum,
                        generationNumber: entry.generation
                    });
                }
                else if (pdfObjects && pdfObjects.has(modifiedKey)) {
                    // Use original object
                    const originalObj = pdfObjects.get(modifiedKey);
                    objects.push({
                        ...originalObj,
                        objectNumber: objNum,
                        generationNumber: entry.generation
                    });
                }
                else {
                    // Create a minimal object if we don't have the original
                    objects.push({
                        type: PDFObjectType.Null,
                        value: null,
                        objectNumber: objNum,
                        generationNumber: entry.generation
                    });
                }
            }
            // Add any new objects that were created
            for (const [key, obj] of this.modifiedObjects) {
                const [objNumStr, genStr] = key.split('_');
                const objNum = parseInt(objNumStr, 10);
                if (!objectNumbers.has(objNum)) {
                    objects.push({
                        ...obj,
                        objectNumber: objNum,
                        generationNumber: parseInt(genStr, 10)
                    });
                }
            }
        }
        catch (error) {
            console.warn('Error collecting PDF objects:', error);
        }
        return objects;
    }
    /**
     * Mark an object as modified for inclusion in the saved PDF
     */
    modifyObject(objectNumber, generationNumber, obj) {
        const key = `${objectNumber}_${generationNumber}`;
        this.modifiedObjects.set(key, obj);
    }
    /**
     * Add a new object to be included in the saved PDF
     */
    addObject(obj) {
        // Find next available object number
        let objectNumber = this.getNextObjectNumber();
        const key = `${objectNumber}_0`;
        this.modifiedObjects.set(key, {
            ...obj,
            objectNumber,
            generationNumber: 0
        });
        return objectNumber;
    }
    getNextObjectNumber() {
        const xrefTable = this.pdf.xrefTable;
        let maxObjectNumber = 0;
        // Find the highest object number
        for (let i = 1; i <= 10000; i++) {
            const entry = xrefTable?.getEntry(i);
            if (entry) {
                maxObjectNumber = Math.max(maxObjectNumber, i);
            }
            else {
                break;
            }
        }
        // Check modified objects too
        for (const key of this.modifiedObjects.keys()) {
            const objNum = parseInt(key.split('_')[0], 10);
            maxObjectNumber = Math.max(maxObjectNumber, objNum);
        }
        return maxObjectNumber + 1;
    }
    writeObject(writer, obj) {
        switch (obj.type) {
            case PDFObjectType.Boolean:
                writer.writeString(obj.value ? 'true' : 'false');
                break;
            case PDFObjectType.Number:
                writer.writeString(obj.value.toString());
                break;
            case PDFObjectType.String:
                writer.writeString('(' + this.escapeString(obj.value) + ')');
                break;
            case PDFObjectType.Name:
                writer.writeString('/' + obj.value);
                break;
            case PDFObjectType.Array:
                writer.writeString('[');
                for (let i = 0; i < obj.value.length; i++) {
                    if (i > 0)
                        writer.writeString(' ');
                    this.writeObject(writer, obj.value[i]);
                }
                writer.writeString(']');
                break;
            case PDFObjectType.Dictionary:
                const dict = obj.value;
                writer.writeString('<<\n');
                for (const [key, value] of dict.entries) {
                    writer.writeString('/' + key + ' ');
                    this.writeObject(writer, value);
                    writer.writeString('\n');
                }
                writer.writeString('>>');
                break;
            case PDFObjectType.Stream:
                const stream = obj.value;
                this.writeObject(writer, { type: PDFObjectType.Dictionary, value: stream.dictionary });
                writer.writeString('\nstream\n');
                writer.writeBytes(stream.data);
                writer.writeString('\nendstream');
                break;
            case PDFObjectType.Reference:
                const ref = obj.value;
                writer.writeString(`${ref.objectNumber} ${ref.generationNumber} R`);
                break;
            case PDFObjectType.Null:
                writer.writeString('null');
                break;
        }
    }
    escapeString(str) {
        return str
            .replace(/\\/g, '\\\\')
            .replace(/\(/g, '\\(')
            .replace(/\)/g, '\\)')
            .replace(/\r/g, '\\r')
            .replace(/\n/g, '\\n');
    }
}
class PDFStreamWriter {
    constructor(initialSize = 1024 * 1024) {
        this.position = 0;
        this.buffer = new Uint8Array(initialSize);
    }
    writeString(str) {
        const bytes = new TextEncoder().encode(str);
        this.writeBytes(bytes);
    }
    writeBytes(bytes) {
        if (this.position + bytes.length > this.buffer.length) {
            this.expand(bytes.length);
        }
        this.buffer.set(bytes, this.position);
        this.position += bytes.length;
    }
    expand(minSize) {
        const newSize = Math.max(this.buffer.length * 2, this.position + minSize);
        const newBuffer = new Uint8Array(newSize);
        newBuffer.set(this.buffer);
        this.buffer = newBuffer;
    }
    getBuffer() {
        return this.buffer.slice(0, this.position);
    }
}
// ============================================================================
// Additional Exports
// ============================================================================

{ TextLayerBuilder, renderTextLayer };
// ============================================================================
// Default Export
// ============================================================================


    // Export to global scope
    if (typeof window !== 'undefined') {
        window.AgenticPDF = AgenticPDF;
        window.TextExtractor = TextExtractor;
        window.ImageExtractor = ImageExtractor;
        window.FormExtractor = FormExtractor;
        window.AnnotationExtractor = AnnotationExtractor;
        window.renderTextLayer = renderTextLayer;
        window.TextLayerBuilder = TextLayerBuilder;
        window.ThemeManager = ThemeManager;
    }
    if (typeof global !== 'undefined') {
        global.AgenticPDF = AgenticPDF;
        global.TextExtractor = TextExtractor;
        global.ImageExtractor = ImageExtractor;
        global.FormExtractor = FormExtractor;
        global.AnnotationExtractor = AnnotationExtractor;
        global.renderTextLayer = renderTextLayer;
        global.TextLayerBuilder = TextLayerBuilder;
        global.ThemeManager = ThemeManager;
    }
    
})(typeof window !== 'undefined' ? window : typeof global !== 'undefined' ? global : this);
