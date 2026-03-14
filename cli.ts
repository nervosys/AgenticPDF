#!/usr/bin/env node

/**
 * AgenticPDF CLI - Command-line interface for PDF processing
 * 
 * Provides access to core AgenticPDF functionality from the terminal
 * including text extraction, metadata reading, AI features, and more.
 */

import AgenticPDF from './agenticpdf';
import * as fs from 'fs';
import * as path from 'path';

// CLI version
const CLI_VERSION = '1.0.0';

// Configuration constants
const MAX_CHUNK_SIZE = 10000;
const MIN_CHUNK_SIZE = 100;
const DEFAULT_CHUNK_SIZE = 1000;
const DEFAULT_MEMORY_LIMIT = 200 * 1024 * 1024; // 200MB
const ALLOWED_IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'tiff'];
const VALID_OUTPUT_FORMATS = ['text', 'json', 'html', 'markdown'];

/**
 * Sanitize user input for error messages to prevent XSS
 * Removes HTML tags and limits length
 */
function sanitizeInput(input: string, maxLength: number = 50): string {
    if (!input) return '';
    // Remove HTML tags
    const sanitized = input.replace(/<[^>]*>/g, '');
    // Limit length
    if (sanitized.length > maxLength) {
        return sanitized.substring(0, maxLength) + '...';
    }
    return sanitized;
}

// Color codes for terminal output
const colors = {
    reset: '\x1b[0m',
    bright: '\x1b[1m',
    dim: '\x1b[2m',
    red: '\x1b[31m',
    green: '\x1b[32m',
    yellow: '\x1b[33m',
    blue: '\x1b[34m',
    magenta: '\x1b[35m',
    cyan: '\x1b[36m',
};

// Helper functions for colored output
const log = {
    info: (msg: string) => console.log(`${colors.blue}ℹ${colors.reset} ${msg}`),
    success: (msg: string) => console.log(`${colors.green}✓${colors.reset} ${msg}`),
    error: (msg: string) => console.error(`${colors.red}✗${colors.reset} ${msg}`),
    warning: (msg: string) => console.warn(`${colors.yellow}⚠${colors.reset} ${msg}`),
    header: (msg: string) => console.log(`\n${colors.bright}${colors.cyan}${msg}${colors.reset}\n`),
    data: (key: string, value: any) => console.log(`  ${colors.dim}${key}:${colors.reset} ${value}`),
};

// Interface for CLI options
interface CLIOptions {
    command: string;
    input?: string;
    output?: string;
    pages?: string;
    format?: string;
    verbose?: boolean;
    pretty?: boolean;
    metadata?: boolean;
    tables?: boolean;
    images?: boolean;
    forms?: boolean;
    annotations?: boolean;
    ai?: boolean;
    chunk?: boolean;
    chunkSize?: number;
    stream?: boolean;
    help?: boolean;
    version?: boolean;
}

/**
 * Parse command line arguments
 */
function parseArgs(args: string[]): CLIOptions {
    const options: CLIOptions = {
        command: args[0] || 'help',
    };

    for (let i = 1; i < args.length; i++) {
        const arg = args[i];
        const nextArg = args[i + 1];

        switch (arg) {
            case '-i':
            case '--input':
                if (!nextArg) {
                    throw new Error('--input requires a file path');
                }
                options.input = nextArg;
                i++;
                break;
            case '-o':
            case '--output':
                if (!nextArg) {
                    throw new Error('--output requires a file path');
                }
                options.output = nextArg;
                i++;
                break;
            case '-p':
            case '--pages':
                if (!nextArg) {
                    throw new Error('--pages requires a page range');
                }
                options.pages = nextArg;
                i++;
                break;
            case '-f':
            case '--format':
                if (!nextArg) {
                    throw new Error('--format requires a format type');
                }
                if (!VALID_OUTPUT_FORMATS.includes(nextArg.toLowerCase())) {
                    throw new Error(
                        `Invalid format: ${sanitizeInput(nextArg)}. Must be one of: ${VALID_OUTPUT_FORMATS.join(', ')}`
                    );
                }
                options.format = nextArg.toLowerCase();
                i++;
                break;
            case '-v':
            case '--verbose':
                options.verbose = true;
                break;
            case '--pretty':
                options.pretty = true;
                break;
            case '-m':
            case '--metadata':
                options.metadata = true;
                break;
            case '--tables':
                options.tables = true;
                break;
            case '--images':
                options.images = true;
                break;
            case '--forms':
                options.forms = true;
                break;
            case '--annotations':
                options.annotations = true;
                break;
            case '--ai':
                options.ai = true;
                break;
            case '--chunk':
                options.chunk = true;
                break;
            case '--chunk-size':
                if (!nextArg) {
                    throw new Error('--chunk-size requires a value');
                }
                const chunkSize = parseInt(nextArg, 10);
                if (isNaN(chunkSize)) {
                    throw new Error(`Invalid chunk size: ${sanitizeInput(nextArg)}. Must be a number.`);
                }
                if (chunkSize < MIN_CHUNK_SIZE || chunkSize > MAX_CHUNK_SIZE) {
                    throw new Error(
                        `Chunk size must be between ${MIN_CHUNK_SIZE} and ${MAX_CHUNK_SIZE}. Got: ${chunkSize}`
                    );
                }
                options.chunkSize = chunkSize;
                i++;
                break;
            case '--stream':
                options.stream = true;
                break;
            case '-h':
            case '--help':
                options.help = true;
                break;
            case '--version':
                options.version = true;
                break;
            default:
                if (!options.input && !arg.startsWith('-')) {
                    options.input = arg;
                }
        }
    }

    return options;
}

/**
 * Display help information
 */
function displayHelp(): void {
    console.log(`
${colors.bright}${colors.cyan}AgenticPDF CLI v${CLI_VERSION}${colors.reset}
${colors.dim}Production-ready PDF processing from the command line${colors.reset}

${colors.bright}USAGE:${colors.reset}
  mpdf <command> [options]
  agenticpdf <command> [options]

${colors.bright}COMMANDS:${colors.reset}
  ${colors.green}info${colors.reset}       Display PDF information and metadata
  ${colors.green}extract${colors.reset}    Extract text content from PDF
  ${colors.green}render${colors.reset}     Render PDF pages to images
  ${colors.green}convert${colors.reset}    Convert PDF to different formats
  ${colors.green}analyze${colors.reset}    AI-powered document analysis
  ${colors.green}chunk${colors.reset}      Generate semantic chunks for RAG
  ${colors.green}forms${colors.reset}      Extract or fill form fields
  ${colors.green}images${colors.reset}     Extract images from PDF
  ${colors.green}merge${colors.reset}      Merge multiple PDF files
  ${colors.green}split${colors.reset}      Split PDF into separate files
  ${colors.green}help${colors.reset}       Display this help message
  ${colors.green}version${colors.reset}    Show version information

${colors.bright}OPTIONS:${colors.reset}
  ${colors.yellow}-i, --input${colors.reset} <file>      Input PDF file (required)
  ${colors.yellow}-o, --output${colors.reset} <file>     Output file path
  ${colors.yellow}-p, --pages${colors.reset} <range>     Page range (e.g., 1-5, 1,3,5)
  ${colors.yellow}-f, --format${colors.reset} <format>   Output format (text, json, html, markdown)
  ${colors.yellow}-v, --verbose${colors.reset}           Verbose output
  ${colors.yellow}--pretty${colors.reset}                Pretty-print JSON output
  ${colors.yellow}-m, --metadata${colors.reset}          Include metadata in output
  ${colors.yellow}--tables${colors.reset}                Extract tables
  ${colors.yellow}--images${colors.reset}                Extract images
  ${colors.yellow}--forms${colors.reset}                 Extract form fields
  ${colors.yellow}--annotations${colors.reset}           Extract annotations
  ${colors.yellow}--ai${colors.reset}                    Enable AI analysis
  ${colors.yellow}--chunk${colors.reset}                 Generate semantic chunks
  ${colors.yellow}--chunk-size${colors.reset} <size>     Chunk size for semantic chunking
  ${colors.yellow}--stream${colors.reset}                Use streaming mode
  ${colors.yellow}-h, --help${colors.reset}              Display help
  ${colors.yellow}--version${colors.reset}               Show version

${colors.bright}EXAMPLES:${colors.reset}
  ${colors.dim}# Display PDF information${colors.reset}
  mpdf info document.pdf

  ${colors.dim}# Extract text to file${colors.reset}
  mpdf extract -i document.pdf -o output.txt

  ${colors.dim}# Extract specific pages with metadata${colors.reset}
  mpdf extract -i document.pdf -p 1-5 -m

  ${colors.dim}# Convert to JSON with tables${colors.reset}
  mpdf convert -i document.pdf -f json --tables --pretty

  ${colors.dim}# AI-powered analysis${colors.reset}
  mpdf analyze -i document.pdf --ai

  ${colors.dim}# Generate semantic chunks for RAG${colors.reset}
  mpdf chunk -i document.pdf --chunk-size 1000 -o chunks.json

  ${colors.dim}# Extract images${colors.reset}
  mpdf images -i document.pdf -o ./images/

  ${colors.dim}# Stream large PDF${colors.reset}
  mpdf extract -i large.pdf --stream

${colors.bright}INSTALLATION:${colors.reset}
  ${colors.dim}# Install globally to use 'mpdf' command anywhere${colors.reset}
  npm install -g agenticpdf
  
  ${colors.dim}# See INSTALL.md for detailed installation instructions${colors.reset}

${colors.bright}DOCUMENTATION:${colors.reset}
  ${colors.cyan}https://github.com/nervosys/agenticpdf${colors.reset}
`);
}

/**
 * Display version information
 */
function displayVersion(): void {
    console.log(`AgenticPDF CLI v${CLI_VERSION}`);
}

/**
 * Parse page range string (e.g., "1-5" or "1,3,5")
 */
function parsePageRange(rangeStr: string): { start?: number; end?: number } | number[] {
    if (rangeStr.includes('-')) {
        const [start, end] = rangeStr.split('-').map(n => parseInt(n.trim(), 10));
        return { start, end };
    } else if (rangeStr.includes(',')) {
        return rangeStr.split(',').map(n => parseInt(n.trim(), 10));
    } else {
        const page = parseInt(rangeStr, 10);
        return { start: page, end: page };
    }
}

/**
 * Load PDF from file
 */
async function loadPDF(inputPath: string, options: CLIOptions): Promise<AgenticPDF> {
    if (!inputPath) {
        throw new Error('Input file is required. Use -i or --input to specify the PDF file.');
    }

    // Resolve to absolute path
    const resolvedPath = path.resolve(inputPath);

    // Check file existence
    if (!fs.existsSync(resolvedPath)) {
        throw new Error(`File not found: ${inputPath}`);
    }

    // Verify it's a file (not directory)
    const stats = fs.statSync(resolvedPath);
    if (!stats.isFile()) {
        throw new Error(`Path is not a file: ${inputPath}`);
    }

    // Check file permissions
    try {
        fs.accessSync(resolvedPath, fs.constants.R_OK);
    } catch (err) {
        throw new Error(`Cannot read file: ${inputPath}. Permission denied.`);
    }

    // Validate file extension
    const ext = path.extname(resolvedPath).toLowerCase();
    if (ext !== '.pdf') {
        log.warning(`File extension is ${ext}, expected .pdf. Attempting to process anyway...`);
    }

    if (options.verbose) {
        log.info(`Loading PDF: ${resolvedPath}`);
        log.info(`File size: ${(stats.size / 1024 / 1024).toFixed(2)} MB`);
    }

    // Handle read errors
    let fileBuffer: Buffer;
    try {
        fileBuffer = fs.readFileSync(resolvedPath);
    } catch (err) {
        throw new Error(`Failed to read file: ${inputPath}. ${err instanceof Error ? err.message : 'Unknown error'}`);
    }

    // Safe Buffer to ArrayBuffer conversion
    const arrayBuffer: ArrayBuffer = fileBuffer.buffer.slice(
        fileBuffer.byteOffset,
        fileBuffer.byteOffset + fileBuffer.byteLength
    ) as ArrayBuffer;

    // Handle PDF loading errors
    try {
        const pdf = await AgenticPDF.fromBuffer(arrayBuffer, {
            lazyLoad: true,
            maxMemoryUsage: DEFAULT_MEMORY_LIMIT,
        });

        if (options.verbose) {
            log.success('PDF loaded successfully');
        }

        return pdf;
    } catch (err) {
        throw new Error(
            `Failed to parse PDF: ${inputPath}. ${err instanceof Error ? err.message : 'Invalid PDF format'}`
        );
    }
}

/**
 * Command: Display PDF information
 */
async function commandInfo(options: CLIOptions): Promise<void> {
    const pdf = await loadPDF(options.input!, options);

    try {
        log.header('📄 PDF Information');

        const metadata = pdf.getMetadata();

        if (metadata) {
            log.data('Title', metadata.title || 'N/A');
            log.data('Author', metadata.author || 'N/A');
            log.data('Subject', metadata.subject || 'N/A');
            log.data('Creator', metadata.creator || 'N/A');
            log.data('Producer', metadata.producer || 'N/A');
            log.data('Pages', metadata.pageCount || 'N/A');
            log.data('PDF Version', metadata.version || 'N/A');
            log.data('Created', metadata.creationDate || 'N/A');
            log.data('Modified', metadata.modificationDate || 'N/A');
            log.data('Encrypted', metadata.isEncrypted ? 'Yes' : 'No');

            if (metadata.keywords) {
                log.data('Keywords', metadata.keywords);
            }
        } else {
            log.warning('No metadata available');
        }

        if (options.verbose) {
            console.log('\n' + colors.dim + 'Use --metadata flag with other commands to include this info' + colors.reset);
        }
    } finally {
        pdf.close();
    }
}

/**
 * Command: Extract text
 */
async function commandExtract(options: CLIOptions): Promise<void> {
    const pdf = await loadPDF(options.input!, options);

    try {
        log.header('📝 Extracting Text');

        let pageRange: any = undefined;
        if (options.pages) {
            pageRange = parsePageRange(options.pages);
            if (options.verbose) {
                log.info(`Processing pages: ${options.pages}`);
            }
        }

        const extractOptions = {
            preserveFormatting: true,
            extractTables: options.tables || false,
            pageRange,
        };

        let text: string;

        if (options.stream) {
            if (options.verbose) {
                log.info('Using streaming mode...');
            }
            const chunks: string[] = [];
            for await (const content of pdf.streamText(extractOptions)) {
                chunks.push(content.text);
                if (options.verbose) {
                    log.info(`Processed page ${content.pageNumber}`);
                }
            }
            text = chunks.join('\n\n');
        } else {
            const result = await pdf.extractText(extractOptions);
            // extractText returns TextContent[], combine all text
            text = result.map(content => content.text).join('\n');
        }

        if (options.output) {
            fs.writeFileSync(options.output, text, 'utf-8');
            log.success(`Text saved to: ${options.output}`);
        } else {
            console.log('\n' + text);
        }

        if (options.metadata) {
            const metadata = pdf.getMetadata();
            console.log('\n' + colors.dim + '─'.repeat(50) + colors.reset);
            log.data('Total Characters', text.length);
            log.data('Total Words', text.split(/\s+/).length);
            log.data('Total Pages', metadata?.pageCount || 'N/A');
        }
    } finally {
        pdf.close();
    }
}

/**
 * Command: Convert PDF to different formats
 */
async function commandConvert(options: CLIOptions): Promise<void> {
    const pdf = await loadPDF(options.input!, options);

    try {
        const format = options.format || 'text';
        log.header(`🔄 Converting to ${format.toUpperCase()}`);

        const convertOptions: any = {
            includeMetadata: options.metadata || false,
            includeAnnotations: options.annotations || false,
            extractTables: options.tables || false,
        };

        if (options.pages) {
            convertOptions.pageRange = parsePageRange(options.pages);
        }

        const result = await pdf.exportAs(format as any, convertOptions);

        let output: string;
        if (format === 'json') {
            output = options.pretty
                ? JSON.stringify(result, null, 2)
                : JSON.stringify(result);
        } else {
            output = result as string;
        }

        if (options.output) {
            fs.writeFileSync(options.output, output, 'utf-8');
            log.success(`Converted file saved to: ${options.output}`);
        } else {
            console.log('\n' + output);
        }
    } finally {
        pdf.close();
    }
}

/**
 * Command: AI-powered analysis
 */
async function commandAnalyze(options: CLIOptions): Promise<void> {
    const pdf = await loadPDF(options.input!, options);

    try {
        log.header('🤖 AI-Powered Analysis');

        if (options.verbose) {
            log.info('Analyzing document structure...');
        }

        const aiFeatures = await pdf.getAIFeatures({
            enableStructuralAnalysis: true,
            enableNER: options.ai || false,
            enableSummarization: options.ai || false,
        });

        const analysis: any = {
            documentType: aiFeatures.structuralAnalysis.documentType,
            summary: aiFeatures.nlpReady.summary,
            keywords: aiFeatures.nlpReady.keywords,
            structure: {
                sections: aiFeatures.structuralAnalysis.sections.length,
                tables: aiFeatures.structuralAnalysis.tables.length,
                figures: aiFeatures.structuralAnalysis.figures.length,
            },
        };

        const output = options.pretty
            ? JSON.stringify(analysis, null, 2)
            : JSON.stringify(analysis);

        if (options.output) {
            fs.writeFileSync(options.output, output, 'utf-8');
            log.success(`Analysis saved to: ${options.output}`);
        } else {
            console.log('\n' + output);
        }
    } finally {
        pdf.close();
    }
}

/**
 * Command: Generate semantic chunks
 */
async function commandChunk(options: CLIOptions): Promise<void> {
    const pdf = await loadPDF(options.input!, options);

    try {
        log.header('🧩 Generating Semantic Chunks');

        const chunkOptions = {
            strategy: 'semantic' as const,
            maxChunkSize: options.chunkSize || 1000,
            includeMetadata: true,
        };

        if (options.verbose) {
            log.info(`Chunk size: ${chunkOptions.maxChunkSize}`);
        }

        let chunks: any[];

        if (options.stream) {
            if (options.verbose) {
                log.info('Using streaming mode...');
            }
            chunks = [];
            for await (const chunk of pdf.streamSemanticChunks(chunkOptions)) {
                chunks.push({
                    content: chunk.content,
                    pageNumbers: chunk.pageNumbers,
                    type: chunk.type,
                    metadata: chunk.metadata,
                });
                if (options.verbose) {
                    log.info(`Generated chunk ${chunks.length}`);
                }
            }
        } else {
            const result = await pdf.generateSemanticChunks(chunkOptions);
            chunks = result.map(chunk => ({
                content: chunk.content,
                pageNumbers: chunk.pageNumbers,
                type: chunk.type,
                metadata: chunk.metadata,
            }));
        }

        const output = options.pretty
            ? JSON.stringify(chunks, null, 2)
            : JSON.stringify(chunks);

        if (options.output) {
            fs.writeFileSync(options.output, output, 'utf-8');
            log.success(`Chunks saved to: ${options.output}`);
            log.data('Total Chunks', chunks.length);
        } else {
            console.log('\n' + output);
        }
    } finally {
        pdf.close();
    }
}

/**
 * Command: Extract images
 */
async function commandImages(options: CLIOptions): Promise<void> {
    const pdf = await loadPDF(options.input!, options);

    try {
        log.header('🖼️  Extracting Images');

        const images = await pdf.extractImages();

        if (images.length === 0) {
            log.warning('No images found in PDF');
            return;
        }

        log.info(`Found ${images.length} image(s)`);

        if (options.output) {
            // Security: Resolve to absolute path to prevent path traversal
            const outputDir = path.resolve(options.output);

            // Security: Validate output directory is not in sensitive locations
            const sensitivePathsRegex = /(etc|windows|system32|program files)/i;
            if (sensitivePathsRegex.test(outputDir)) {
                throw new Error('Cannot write to system directories');
            }

            if (!fs.existsSync(outputDir)) {
                fs.mkdirSync(outputDir, { recursive: true });
            }

            images.forEach((image, index) => {
                // Security: Sanitize file extension
                const rawExt = image.mimeType?.split('/')[1] || 'png';
                const safeExt = rawExt.replace(/[^a-z0-9]/gi, '').toLowerCase();

                // Security: Validate extension is allowed
                const ext = ALLOWED_IMAGE_EXTENSIONS.includes(safeExt) ? safeExt : 'png';

                // Security: Use safe filename
                const filename = `image_${index + 1}.${ext}`;
                const filepath = path.join(outputDir, filename);

                // Security: Ensure output path is within intended directory
                const resolvedPath = path.resolve(filepath);
                if (!resolvedPath.startsWith(outputDir)) {
                    throw new Error(`Invalid output path detected: ${filepath}`);
                }

                if (image.data) {
                    fs.writeFileSync(resolvedPath, Buffer.from(image.data));
                    if (options.verbose) {
                        log.success(`Saved: ${filename}`);
                    }
                }
            });

            log.success(`Images saved to: ${outputDir}`);
        } else {
            images.forEach((image, index) => {
                log.data(`Image ${index + 1}`, `${image.width}x${image.height} (${image.mimeType})`);
            });
        }
    } finally {
        pdf.close();
    }
}

/**
 * Command: Extract or fill forms
 */
async function commandForms(options: CLIOptions): Promise<void> {
    const pdf = await loadPDF(options.input!, options);

    try {
        log.header('📋 Form Fields');

        const fields = await pdf.getFormFields();

        if (fields.length === 0) {
            log.warning('No form fields found in PDF');
            return;
        }

        log.info(`Found ${fields.length} form field(s)`);

        const formData = fields.map(field => ({
            name: field.name,
            type: field.type,
            value: field.value,
            required: field.required,
        }));

        const output = options.pretty
            ? JSON.stringify(formData, null, 2)
            : JSON.stringify(formData);

        if (options.output) {
            fs.writeFileSync(options.output, output, 'utf-8');
            log.success(`Form data saved to: ${options.output}`);
        } else {
            console.log('\n' + output);
        }
    } finally {
        pdf.close();
    }
}

/**
 * Main CLI entry point
 */
async function main(): Promise<void> {
    const args = process.argv.slice(2);

    if (args.length === 0) {
        displayHelp();
        return;
    }

    // Check for standalone flags first (before parsing as commands)
    if (args[0] === '--help' || args[0] === '-h') {
        displayHelp();
        return;
    }

    if (args[0] === '--version') {
        displayVersion();
        return;
    }

    const options = parseArgs(args);

    try {
        // Handle special flags in command context
        if (options.version) {
            displayVersion();
            return;
        }

        if (options.help) {
            displayHelp();
            return;
        }

        // Route to appropriate command
        switch (options.command) {
            case 'info':
                await commandInfo(options);
                break;
            case 'extract':
                await commandExtract(options);
                break;
            case 'convert':
                await commandConvert(options);
                break;
            case 'analyze':
                await commandAnalyze(options);
                break;
            case 'chunk':
                await commandChunk(options);
                break;
            case 'images':
                await commandImages(options);
                break;
            case 'forms':
                await commandForms(options);
                break;
            case 'help':
                displayHelp();
                break;
            case 'version':
                displayVersion();
                break;
            default:
                log.error(`Unknown command: ${options.command}`);
                log.info('Run "agenticpdf help" for usage information');
                process.exit(1);
        }
    } catch (error) {
        log.error(error instanceof Error ? error.message : String(error));
        if (options.verbose && error instanceof Error) {
            console.error('\n' + colors.dim + error.stack + colors.reset);
        }
        process.exit(1);
    }
}

// Run CLI when executed directly
main().catch((error) => {
    log.error('Fatal error: ' + error.message);
    process.exit(1);
});

export default main;
