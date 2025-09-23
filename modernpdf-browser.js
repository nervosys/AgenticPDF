// Browser wrapper for ModernPDF - Pure JavaScript implementation
// This provides the core PDF processing capabilities without external dependencies

class ModernPDF {
    constructor(data, options = {}) {
        this.pdfData = data;
        this.options = {
            lazyLoad: false,
            maxMemoryUsage: 100 * 1024 * 1024, // 100MB
            ...options
        };
        this.pages = [];
        this.metadata = {};
        this.textContent = new Map();
        this.isLoaded = false;
    }

    // Factory methods
    static async fromFile(file, options = {}) {
        const arrayBuffer = await file.arrayBuffer();
        const pdf = new ModernPDF(new Uint8Array(arrayBuffer), options);
        await pdf.parse();
        return pdf;
    }

    static async fromUrl(url, options = {}) {
        const response = await fetch(url);
        if (!response.ok) {
            throw new Error(`Failed to load PDF from URL: ${response.status}`);
        }
        const arrayBuffer = await response.arrayBuffer();
        const pdf = new ModernPDF(new Uint8Array(arrayBuffer), options);
        await pdf.parse();
        return pdf;
    }

    static fromBuffer(buffer, options = {}) {
        const pdf = new ModernPDF(buffer, options);
        return pdf.parse().then(() => pdf);
    }

    // Core parsing functionality
    async parse() {
        if (this.isLoaded) return this;

        // Basic PDF validation
        const header = new TextDecoder().decode(this.pdfData.slice(0, 8));
        if (!header.startsWith('%PDF-')) {
            throw new Error('Invalid PDF file - missing PDF header');
        }

        // Extract version
        this.version = header.substring(5, 8);

        // Mock parsing for demo - in real implementation this would parse the PDF structure
        this.metadata = {
            title: 'Sample PDF Document',
            author: 'ModernPDF',
            creator: 'ModernPDF Library',
            producer: 'ModernPDF v1.0.0',
            creationDate: new Date(),
            modificationDate: new Date(),
            pages: this.generateMockPages(),
            version: this.version
        };

        this.pages = this.metadata.pages;
        this.isLoaded = true;

        return this;
    }

    generateMockPages() {
        // Generate mock pages for demonstration
        const pageCount = 5; // Simulate 5 pages
        const pages = [];

        for (let i = 1; i <= pageCount; i++) {
            pages.push({
                pageNumber: i,
                width: 612, // Standard letter size
                height: 792,
                content: this.generateMockPageContent(i),
                textContent: this.generateMockTextContent(i)
            });
        }

        return pages;
    }

    generateMockPageContent(pageNum) {
        return `This is page ${pageNum} of the ModernPDF document. 
        
        ModernPDF is a modern, TypeScript-native PDF processing library designed to replace PDF.js with better performance, streaming capabilities, and AI integration.
        
        Key features include:
        • Streaming-first architecture
        • AI-ready with built-in semantic chunking
        • Memory-efficient processing
        • Full TypeScript support
        • Theme-aware rendering
        
        This page demonstrates the text search and highlighting capabilities. Try searching for words like "ModernPDF", "streaming", or "TypeScript" to see the highlighting in action.
        
        Page ${pageNum} content continues here with more text for search testing. The search functionality highlights all occurrences of your search term across all pages and allows navigation between results.`;
    }

    generateMockTextContent(pageNum) {
        const content = this.generateMockPageContent(pageNum);
        this.textContent.set(pageNum, content.toLowerCase());
        return content;
    }

    // API methods
    getMetadata() {
        return this.metadata;
    }

    getPageCount() {
        return this.pages.length;
    }

    async getPage(pageNumber) {
        if (pageNumber < 1 || pageNumber > this.pages.length) {
            throw new Error(`Page ${pageNumber} does not exist`);
        }
        return this.pages[pageNumber - 1];
    }

    async extractText(options = {}) {
        const { pageRange } = options;
        let text = '';

        const startPage = pageRange?.start || 1;
        const endPage = pageRange?.end || this.pages.length;

        for (let i = startPage; i <= endPage; i++) {
            const page = await this.getPage(i);
            text += page.textContent + '\n\n';
        }

        return text;
    }

    async renderPage(pageNumber, options = {}) {
        const page = await this.getPage(pageNumber);
        const { scale = 1.0, theme = 'dark' } = options;

        // Create canvas for rendering
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');

        // Set canvas dimensions
        canvas.width = page.width * scale * 2; // High DPI
        canvas.height = page.height * scale * 2;
        canvas.style.width = `${page.width * scale}px`;
        canvas.style.height = `${page.height * scale}px`;

        // Scale for high DPI
        ctx.scale(2, 2);

        // Set theme-based background
        const bgColor = theme === 'dark' ? '#2d2d2d' : '#ffffff';
        const textColor = theme === 'dark' ? '#ffffff' : '#000000';

        ctx.fillStyle = bgColor;
        ctx.fillRect(0, 0, page.width * scale, page.height * scale);

        // Render text content
        ctx.fillStyle = textColor;
        ctx.font = `${12 * scale}px Arial`;

        const lines = page.textContent.split('\n');
        const lineHeight = 16 * scale;
        let y = 40 * scale;

        lines.forEach(line => {
            const words = line.split(' ');
            let currentLine = '';
            let x = 40 * scale;

            words.forEach(word => {
                const testLine = currentLine + word + ' ';
                const metrics = ctx.measureText(testLine);
                const testWidth = metrics.width;

                if (testWidth > (page.width - 80) * scale && currentLine !== '') {
                    ctx.fillText(currentLine, x, y);
                    currentLine = word + ' ';
                    y += lineHeight;
                } else {
                    currentLine = testLine;
                }
            });

            ctx.fillText(currentLine, x, y);
            y += lineHeight;
        });

        return canvas;
    }

    // Search functionality
    async search(query, options = {}) {
        const results = [];
        const searchQuery = query.toLowerCase();

        for (let pageNum = 1; pageNum <= this.pages.length; pageNum++) {
            const pageText = this.textContent.get(pageNum);
            if (pageText) {
                let index = 0;
                while ((index = pageText.indexOf(searchQuery, index)) !== -1) {
                    results.push({
                        pageNum,
                        index,
                        text: query,
                        length: query.length,
                        context: this.getSearchContext(pageText, index, query.length)
                    });
                    index += 1;
                }
            }
        }

        return results;
    }

    getSearchContext(text, index, length, contextLength = 50) {
        const start = Math.max(0, index - contextLength);
        const end = Math.min(text.length, index + length + contextLength);
        const before = text.substring(start, index);
        const match = text.substring(index, index + length);
        const after = text.substring(index + length, end);

        return {
            before: start > 0 ? '...' + before : before,
            match,
            after: end < text.length ? after + '...' : after
        };
    }

    // Theme management integration
    getOptimalViewerConfig(theme = 'dark') {
        return {
            theme,
            backgroundColor: theme === 'dark' ? '#1a1a1a' : '#f5f5f5',
            pageBackground: theme === 'dark' ? '#2d2d2d' : '#ffffff',
            textColor: theme === 'dark' ? '#ffffff' : '#000000',
            highlightColor: theme === 'dark' ? 'rgba(255, 255, 0, 0.4)' : 'rgba(255, 235, 59, 0.6)',
            currentHighlightColor: theme === 'dark' ? 'rgba(255, 100, 0, 0.7)' : 'rgba(255, 152, 0, 0.8)',
            toolbarBackground: theme === 'dark' ? '#2d2d2d' : '#ffffff',
            toolbarBorder: theme === 'dark' ? '#555' : '#ddd',
            buttonBackground: theme === 'dark' ? '#404040' : '#f0f0f0',
            buttonColor: theme === 'dark' ? '#ffffff' : '#000000'
        };
    }

    // Memory management
    close() {
        this.pages = [];
        this.textContent.clear();
        this.pdfData = null;
        this.isLoaded = false;
    }
}

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { ModernPDF };
} else if (typeof window !== 'undefined') {
    window.ModernPDF = ModernPDF;
}