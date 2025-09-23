/**
 * Basic PDF Processing Example
 * 
 * This example demonstrates fundamental PDF operations:
 * - Loading PDFs from different sources
 * - Extracting text content
 * - Getting metadata and page information
 * - Basic memory management
 */

import ModernPDF from '../modernpdf';

async function basicPDFProcessing() {
    console.log('=== Basic PDF Processing Example ===\n');

    // Example 1: Load from File API (browser)
    if (typeof window !== 'undefined') {
        const fileInput = document.createElement('input');
        fileInput.type = 'file';
        fileInput.accept = '.pdf';

        fileInput.onchange = async (event) => {
            const file = (event.target as HTMLInputElement).files?.[0];
            if (!file) return;

            await processPDFFile(file);
        };

        document.body.appendChild(fileInput);
        console.log('File input created. Select a PDF file to process.');
    }

    // Example 2: Load from URL
    try {
        console.log('Loading PDF from URL...');
        const pdf = await ModernPDF.fromUrl('https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf');
        await processPDF(pdf, 'URL PDF');
    } catch (error) {
        console.log('URL loading failed (expected in some environments):', (error as Error).message);
    }

    // Example 3: Load from ArrayBuffer (Node.js or after fetch)
    // This would typically come from fs.readFile() in Node.js
    console.log('\nNote: In Node.js, you would load from buffer like this:');
    console.log(`
  import fs from 'fs';
  const buffer = fs.readFileSync('document.pdf');
  const pdf = await ModernPDF.fromBuffer(buffer.buffer);
  `);
}

async function processPDFFile(file: File) {
    console.log(`\nProcessing file: ${file.name} (${(file.size / 1024 / 1024).toFixed(2)} MB)`);

    let pdf: ModernPDF | null = null;

    try {
        // Load PDF with basic options
        pdf = await ModernPDF.fromFile(file, {
            lazyLoad: true, // Load pages on-demand
            maxMemoryUsage: 50 * 1024 * 1024, // 50MB limit
            cachePages: false // Don't cache pages in memory
        });

        await processPDF(pdf, file.name);

    } catch (error) {
        console.error('Error processing PDF:', error);
    } finally {
        // Always cleanup resources
        pdf?.close();
        console.log('PDF resources cleaned up');
    }
}

async function processPDF(pdf: ModernPDF, source: string) {
    console.log(`\n--- Processing PDF from ${source} ---`);

    // Get basic metadata
    const metadata = pdf.getMetadata();
    if (metadata) {
        console.log('📄 Metadata:');
        console.log(`  Title: ${metadata.title || 'Unknown'}`);
        console.log(`  Author: ${metadata.author || 'Unknown'}`);
        console.log(`  Pages: ${metadata.pageCount}`);
        console.log(`  File Size: ${(metadata.fileSize / 1024 / 1024).toFixed(2)} MB`);
        console.log(`  PDF Version: ${metadata.version}`);
        console.log(`  Encrypted: ${metadata.isEncrypted ? 'Yes' : 'No'}`);
        console.log(`  Creation Date: ${metadata.creationDate?.toLocaleDateString() || 'Unknown'}`);
    }

    // Extract text from first page
    console.log('\n📝 Text Extraction:');
    try {
        const textContent = await pdf.extractText({
            preserveFormatting: true,
            normalizeWhitespace: true,
            pageRange: { start: 1, end: 1 } // Just first page for demo
        });

        if (textContent.length > 0) {
            const firstPageText = textContent
                .filter(content => content.pageNumber === 1)
                .map(content => content.text)
                .join(' ');

            console.log(`  First page text (${firstPageText.length} chars):`);
            console.log(`  "${firstPageText.substring(0, 200)}${firstPageText.length > 200 ? '...' : ''}"`);
        } else {
            console.log('  No text content found on first page');
        }
    } catch (error) {
        console.log('  Text extraction failed:', (error as Error).message);
    }

    // Get page information
    console.log('\n📋 Page Information:');
    try {
        const firstPage = await pdf.getPage(1);
        if (firstPage) {
            console.log(`  Page 1 dimensions: ${firstPage.width} x ${firstPage.height} pts`);
            console.log(`  Media box: ${JSON.stringify(firstPage.mediaBox)}`);
            console.log(`  Rotation: ${firstPage.rotation}°`);
        }
    } catch (error) {
        console.log('  Could not get page information:', (error as Error).message);
    }

    // Basic search
    console.log('\n🔍 Search Demo:');
    try {
        const searchResults = await pdf.search('the');
        console.log(`  Found ${searchResults.length} occurrences of "the" (showing first 3)`);
        searchResults.slice(0, 3).forEach((result, index) => {
            console.log(`    ${index + 1}. Page ${result.pageNumber}: "${result.context}"`);
        });
    } catch (error) {
        console.log('  Search failed:', (error as Error).message);
    }
}

// Run the example
if (typeof window !== 'undefined') {
    // Browser environment
    document.addEventListener('DOMContentLoaded', basicPDFProcessing);
} else {
    // Node.js environment
    basicPDFProcessing().catch(console.error);
}

export { basicPDFProcessing, processPDFFile };