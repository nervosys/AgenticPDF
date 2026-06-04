#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 NERVOSYS, LLC. Dual-licensed under the GNU AGPLv3 or a
// commercial license; see LICENSE and LICENSE-AGPL.txt.

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
const VALID_OUTPUT_FORMATS = ['text', 'json', 'html', 'markdown', 'xml', 'csv', 'apdf'];
const MAX_OUTPUT_PATH_LENGTH = 4096;

/**
 * Validate and resolve an output file path to prevent directory traversal (CWE-22).
 * Ensures the resolved path is within the current working directory.
 */
function validateOutputPath(outputPath: string): string {
    if (outputPath.length > MAX_OUTPUT_PATH_LENGTH) {
        throw new Error('Output path exceeds maximum length');
    }
    const resolved = path.resolve(outputPath);
    const cwd = path.resolve(process.cwd());
    if (!resolved.startsWith(cwd + path.sep) && resolved !== cwd) {
        throw new Error('Output path must be within current working directory');
    }
    return resolved;
}

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
    css?: boolean;
    accessible?: boolean;
    printCss?: boolean;
    socialMeta?: boolean;
    pageUrl?: string;
    encrypt?: boolean;
    password?: string;
    ndjson?: boolean;
    toolSchema?: string;
    includeText?: boolean;
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
            case '--css':
                options.css = true;
                break;
            case '--accessible':
                options.accessible = true;
                break;
            case '--print-css':
                options.printCss = true;
                break;
            case '--social-meta':
                options.socialMeta = true;
                break;
            case '--page-url':
                if (!nextArg) {
                    throw new Error('--page-url requires a URL');
                }
                options.pageUrl = nextArg;
                i++;
                break;
            case '--encrypt':
                options.encrypt = true;
                break;
            case '--password':
                if (!nextArg) {
                    throw new Error('--password requires a value');
                }
                options.password = nextArg;
                i++;
                break;
            case '--ndjson':
                options.ndjson = true;
                break;
            case '--tool-schema':
                if (!nextArg || nextArg.startsWith('-')) {
                    throw new Error('--tool-schema requires a format: openai, anthropic, generic, or mcp');
                }
                options.toolSchema = nextArg.toLowerCase();
                i++;
                break;
            case '--include-text':
                options.includeText = true;
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
  apdf <command> [options]
  agenticpdf <command> [options]

${colors.bright}COMMANDS:${colors.reset}
  ${colors.green}info${colors.reset}       Display PDF information and metadata
  ${colors.green}extract${colors.reset}    Extract text content from PDF
  ${colors.green}render${colors.reset}     Render PDF pages to images
  ${colors.green}convert${colors.reset}    Convert PDF to different formats
  ${colors.green}analyze${colors.reset}    AI-powered document analysis
  ${colors.green}chunk${colors.reset}      Generate semantic chunks for RAG
  ${colors.green}ingest${colors.reset}     ${colors.bright}Unified AI ingestion${colors.reset} (metadata + chunks + stats in one call)
  ${colors.green}forms${colors.reset}      Extract or fill form fields
  ${colors.green}images${colors.reset}     Extract images from PDF
  ${colors.green}merge${colors.reset}      Merge multiple PDF files
  ${colors.green}split${colors.reset}      Split PDF into separate files
  ${colors.green}metadata${colors.reset}   Generate aPDF metadata envelope
  ${colors.green}generate${colors.reset}   Generate aPDF binary container from a PDF file
  ${colors.green}typeset${colors.reset}    Generate CSS, HTML, or meta tags from aPDF display hints
  ${colors.green}tool-schema${colors.reset} Output tool/function schemas for AI agent integration
  ${colors.green}help${colors.reset}       Display this help message
  ${colors.green}version${colors.reset}    Show version information

${colors.bright}OPTIONS:${colors.reset}
  ${colors.yellow}-i, --input${colors.reset} <file>      Input PDF file (required)
  ${colors.yellow}-o, --output${colors.reset} <file>     Output file path
  ${colors.yellow}-p, --pages${colors.reset} <range>     Page range (e.g., 1-5, 1,3,5)
  ${colors.yellow}-f, --format${colors.reset} <format>   Output format (text, json, html, markdown, xml, csv, apdf)
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
  ${colors.yellow}--css${colors.reset}                   Generate CSS stylesheet (typeset command)
  ${colors.yellow}--accessible${colors.reset}            Generate accessible HTML reading view (typeset command)
  ${colors.yellow}--print-css${colors.reset}             Generate print-ready CSS (typeset command)
  ${colors.yellow}--social-meta${colors.reset}           Generate Open Graph / Twitter Card meta tags (typeset command)
  ${colors.yellow}--page-url${colors.reset} <url>        Public URL for social meta tags
  ${colors.yellow}--encrypt${colors.reset}               Encrypt the aPDF binary container (generate command)
  ${colors.yellow}--password${colors.reset} <pass>       Password for encryption / decryption
  ${colors.yellow}--ndjson${colors.reset}                Output as newline-delimited JSON (ingest command)
  ${colors.yellow}--tool-schema${colors.reset} <fmt>     Output tool schemas (openai, anthropic, generic, mcp)
  ${colors.yellow}--include-text${colors.reset}          Include per-page raw text (ingest command)
  ${colors.yellow}-h, --help${colors.reset}              Display help
  ${colors.yellow}--version${colors.reset}               Show version

${colors.bright}EXAMPLES:${colors.reset}
  ${colors.dim}# Display PDF information${colors.reset}
  apdf info document.pdf

  ${colors.dim}# Extract text to file${colors.reset}
  apdf extract -i document.pdf -o output.txt

  ${colors.dim}# Extract specific pages with metadata${colors.reset}
  apdf extract -i document.pdf -p 1-5 -m

  ${colors.dim}# Convert to JSON with tables${colors.reset}
  apdf convert -i document.pdf -f json --tables --pretty

  ${colors.dim}# Export as aPDF metadata (agentic AI envelope)${colors.reset}
  apdf convert -i paper.pdf -f apdf -o paper.apdf.json

  ${colors.dim}# AI-powered analysis${colors.reset}
  apdf analyze -i document.pdf --ai

  ${colors.dim}# Generate semantic chunks for RAG${colors.reset}
  apdf chunk -i document.pdf --chunk-size 1000 -o chunks.json

  ${colors.dim}# Extract images${colors.reset}
  apdf images -i document.pdf -o ./images/

  ${colors.dim}# Stream large PDF${colors.reset}
  apdf extract -i large.pdf --stream

  ${colors.dim}# Generate aPDF metadata envelope${colors.reset}
  apdf metadata -i paper.pdf -o paper.apdf.json

  ${colors.dim}# Generate aPDF binary container (PDF + metadata)${colors.reset}
  apdf generate -i paper.pdf -o paper.apdf

  ${colors.dim}# Generate encrypted aPDF binary container${colors.reset}
  apdf generate -i paper.pdf -o paper.apdf --encrypt --password mySecret

  ${colors.dim}# Generate CSS from aPDF display hints${colors.reset}
  apdf typeset -i paper.pdf --css -o styles.css

  ${colors.dim}# Generate responsive HTML article${colors.reset}
  apdf typeset -i paper.pdf -o article.html

  ${colors.dim}# Generate accessible reading view${colors.reset}
  apdf typeset -i paper.pdf --accessible -o readable.html

  ${colors.dim}# Generate print-ready stylesheet${colors.reset}
  apdf typeset -i paper.pdf --print-css -o print.css

  ${colors.dim}# Generate social sharing meta tags${colors.reset}
  apdf typeset -i paper.pdf --social-meta --page-url https://example.com/paper

  ${colors.dim}# Unified AI ingestion (single JSON output)${colors.reset}
  apdf ingest -i document.pdf -o ingested.json

  ${colors.dim}# Streaming AI ingestion (NDJSON to stdout)${colors.reset}
  apdf ingest -i document.pdf --ndjson

  ${colors.dim}# Ingest with custom chunk size and per-page text${colors.reset}
  apdf ingest -i document.pdf --chunk-size 500 --include-text -o result.json

  ${colors.dim}# Output OpenAI function-calling schemas${colors.reset}
  apdf tool-schema --tool-schema openai

  ${colors.dim}# Output MCP manifest${colors.reset}
  apdf tool-schema --tool-schema mcp

${colors.bright}INSTALLATION:${colors.reset}
  ${colors.dim}# Install globally to use 'apdf' command anywhere${colors.reset}
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
            const safePath = validateOutputPath(options.output);
            fs.writeFileSync(safePath, text, 'utf-8');
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
            const safePath = validateOutputPath(options.output);
            fs.writeFileSync(safePath, output, 'utf-8');
            log.success(`Converted file saved to: ${options.output}`);
        } else {
            console.log('\n' + output);
        }
    } finally {
        pdf.close();
    }
}

/**
 * Command: Generate aPDF metadata envelope
 */
async function commandApdf(options: CLIOptions): Promise<void> {
    const pdf = await loadPDF(options.input!, options);

    try {
        log.header('📋 Generating aPDF Metadata');

        if (options.verbose) {
            log.info('Running structural analysis, identifier extraction, artifact detection...');
        }

        const apdf = await pdf.generateAPDFMetadata();

        // Summary output
        log.info(`Title:      ${apdf.metadata.title}`);
        log.info(`Type:       ${apdf['@type']}`);
        log.info(`Pages:      ${apdf.metadata.pageCount}`);

        if (apdf.metadata.identifiers.doi) log.info(`DOI:        ${apdf.metadata.identifiers.doi}`);
        if (apdf.metadata.identifiers.arxivId) log.info(`arXiv:      ${apdf.metadata.identifiers.arxivId}`);
        if (apdf.metadata.identifiers.huggingFaceId) log.info(`HuggingFace: ${apdf.metadata.identifiers.huggingFaceId}`);
        if (apdf.authors.length > 0) log.info(`Authors:    ${apdf.authors.map(a => a.name).join(', ')}`);
        if (apdf.artifacts.length > 0) log.info(`Artifacts:  ${apdf.artifacts.length} linked`);
        log.info(`Chunks:     ${apdf.aiContent.chunks.length}`);
        log.info(`Keywords:   ${apdf.aiContent.keywords.slice(0, 8).join(', ')}`);

        const output = options.pretty !== false
            ? JSON.stringify(apdf, null, 2)
            : JSON.stringify(apdf);

        if (options.output) {
            const safePath = validateOutputPath(options.output);
            fs.writeFileSync(safePath, output, 'utf-8');
            log.success(`aPDF metadata saved to: ${options.output}`);
        } else {
            console.log('\n' + output);
        }
    } finally {
        pdf.close();
    }
}

/**
 * Command: Generate — create an aPDF binary container from a PDF file.
 * Packages the original PDF bytes with rich aPDF metadata into a single .apdf file.
 * Now supports v1.1 streaming format with optional AES-256-GCM encryption.
 */
async function commandGenerate(options: CLIOptions): Promise<void> {
    const pdf = await loadPDF(options.input!, options);

    try {
        log.header('📦 Generating aPDF Binary Container');

        // Build encryption options if --encrypt flag is present
        let binaryOptions: Parameters<typeof pdf.generateAPDFBinary>[0] | undefined;
        if (options.encrypt) {
            if (!options.password) {
                throw new Error('--encrypt requires --password <password>');
            }
            binaryOptions = {
                encryption: {
                    password: options.password,
                    encryptPDF: true,
                    encryptMetadata: false,
                },
            };
            log.info('Encryption: AES-256-GCM with PBKDF2-SHA256 key derivation');
        }

        if (options.verbose) {
            log.info('Running structural analysis, identifier extraction, artifact detection...');
            log.info('Packaging PDF + metadata into aPDF v1.1 binary format...');
        }

        const binary = await pdf.generateAPDFBinary(binaryOptions);
        const metadata = await pdf.generateAPDFMetadata();

        // Summary output
        log.info(`Title:      ${metadata.metadata.title}`);
        log.info(`Type:       ${metadata['@type']}`);
        log.info(`Pages:      ${metadata.metadata.pageCount}`);
        if (metadata.authors.length > 0) log.info(`Authors:    ${metadata.authors.map(a => a.name).join(', ')}`);
        if (metadata.artifacts.length > 0) log.info(`Artifacts:  ${metadata.artifacts.length} linked`);
        log.info(`Chunks:     ${metadata.aiContent.chunks.length}`);
        log.info(`Container:  ${(binary.length / 1024).toFixed(1)} KB`);
        log.info(`Format:     aPDF v1.1 (streaming-optimized)`);
        if (options.encrypt) log.info(`Encrypted:  PDF data (AES-256-GCM)`);

        const outputPath = validateOutputPath(options.output || options.input!.replace(/\.pdf$/i, '.apdf'));
        fs.writeFileSync(outputPath, binary);
        log.success(`aPDF binary saved to: ${outputPath}`);
    } finally {
        pdf.close();
    }
}

/**
 * Command: Typeset — generate CSS, HTML, or meta tags from aPDF display hints
 */
async function commandTypeset(options: CLIOptions): Promise<void> {
    const pdf = await loadPDF(options.input!, options);

    try {
        log.header('🖨️  Typesetting from aPDF Display Hints');

        const apdf = await pdf.generateAPDFMetadata();
        const d = apdf.display;

        log.info(`Title:         ${apdf.metadata.title}`);
        log.info(`Reading order: ${d.readingOrder}`);
        log.info(`Theme:         ${d.suggestedTheme || 'general'}`);
        log.info(`Math:          ${d.hasMath}`);
        log.info(`Color:         ${d.hasColor}`);
        log.info(`Fonts:         ${d.fonts.map(f => `${f.name} (${f.role})`).join(', ')}`);

        let output: string;
        let defaultExt: string;

        if (options.css) {
            // CSS-only output
            log.info('\nGenerating CSS stylesheet...');
            output = generateTypesetCSS(apdf);
            defaultExt = '.css';
            log.success(`Generated ${output.length} chars of CSS`);

        } else if (options.accessible) {
            // Accessible reading view
            log.info('\nGenerating accessible reading view...');
            output = generateAccessibleHTML(apdf);
            defaultExt = '.html';
            log.success(`Generated accessible HTML (${output.length} chars)`);

        } else if (options.printCss) {
            // Print-ready stylesheet
            log.info('\nGenerating print-ready stylesheet...');
            output = generatePrintCSS(apdf);
            defaultExt = '.css';
            log.success(`Generated print CSS (${output.length} chars)`);

        } else if (options.socialMeta) {
            // Social meta tags
            const pageUrl = options.pageUrl || 'https://example.com/document';
            log.info(`\nGenerating social meta tags (URL: ${pageUrl})...`);
            output = generateSocialMeta(apdf, pageUrl);
            defaultExt = '.html';
            log.success(`Generated ${output.split('\n').filter(l => l.startsWith('<meta')).length} meta tags + JSON-LD`);

        } else {
            // Default: full responsive HTML article
            log.info('\nGenerating responsive HTML article...');
            output = generateResponsiveHTML(apdf);
            defaultExt = '.html';
            log.success(`Generated responsive HTML (${output.length} chars)`);
        }

        if (options.output) {
            const safePath = validateOutputPath(options.output);
            fs.writeFileSync(safePath, output, 'utf-8');
            log.success(`Output saved to: ${options.output}`);
        } else {
            console.log('\n' + output);
        }
    } finally {
        pdf.close();
    }
}

// ---- Typesetting helpers ----

function escapeHTMLStr(text: string): string {
    return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function escapeAttrStr(text: string): string {
    return text.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function escapeCSSStr(text: string): string {
    return text.replace(/[\\/"']/g, '\\$&');
}

function truncateStr(text: string, maxLen: number): string {
    const cleaned = text.replace(/\s+/g, ' ').trim();
    if (cleaned.length <= maxLen) return cleaned;
    return cleaned.substring(0, maxLen - 1) + '\u2026';
}

/** Build a font-family CSS value from an aPDF font role */
function fontStack(fontName: string, role: string): string {
    const fallback =
        role === 'mono' ? ', "Fira Code", "Consolas", monospace' :
        role === 'heading' ? ', "Georgia", serif' :
        ', "Charter", "Source Serif Pro", serif';
    return `"${fontName}"${fallback}`;
}

/**
 * Generate a scoped CSS stylesheet from aPDF display hints
 */
function generateTypesetCSS(apdf: any, scopeSelector = '.apdf-document'): string {
    const d = apdf.display;
    const { width: pageW, height: pageH } = d.pageDimensions;
    const theme: string = d.suggestedTheme || 'general';

    const themeVars: Record<string, Record<string, string>> = {
        academic: { '--apdf-bg': '#fffff8', '--apdf-fg': '#1a1a1a', '--apdf-heading': '#222', '--apdf-link': '#1a5276', '--apdf-rule-color': '#ccc', '--apdf-code-bg': '#f5f2e8' },
        technical: { '--apdf-bg': '#fafafa', '--apdf-fg': '#333', '--apdf-heading': '#0d47a1', '--apdf-link': '#1565c0', '--apdf-rule-color': '#e0e0e0', '--apdf-code-bg': '#f0f4f8' },
        general: { '--apdf-bg': '#ffffff', '--apdf-fg': '#222', '--apdf-heading': '#111', '--apdf-link': '#0645ad', '--apdf-rule-color': '#ddd', '--apdf-code-bg': '#f6f6f6' },
    };

    const vars = themeVars[theme] || themeVars.general;
    const lines: string[] = [];

    lines.push(`/* Generated from aPDF display hints — ${escapeCSSStr(apdf.metadata.title)} */`);
    lines.push(`${scopeSelector} {`);
    lines.push(`  --apdf-page-width: ${pageW}pt;`);
    lines.push(`  --apdf-page-height: ${pageH}pt;`);
    lines.push(`  max-width: 900px; margin: 0 auto; line-height: 1.6;`);
    if (d.readingOrder === 'multi-column') {
        lines.push(`  column-count: 2; column-gap: 2rem; column-rule: 1px solid var(--apdf-rule-color, #ddd);`);
    }
    for (const [prop, val] of Object.entries(vars)) {
        lines.push(`  ${prop}: ${val};`);
    }
    lines.push(`  background: var(--apdf-bg); color: var(--apdf-fg);`);
    lines.push(`}`);
    lines.push('');

    // Dark mode
    lines.push(`@media (prefers-color-scheme: dark) {`);
    lines.push(`  ${scopeSelector} { --apdf-bg: #1e1e1e; --apdf-fg: #d4d4d4; --apdf-heading: #e0e0e0; --apdf-link: #6cb4ee; --apdf-rule-color: #444; --apdf-code-bg: #2d2d2d; }`);
    lines.push(`}`);
    lines.push('');

    // Font stacks
    for (const font of d.fonts) {
        const sel =
            font.role === 'body' ? `${scopeSelector} p, ${scopeSelector} li` :
            font.role === 'heading' ? `${scopeSelector} h1, ${scopeSelector} h2, ${scopeSelector} h3, ${scopeSelector} h4` :
            font.role === 'mono' ? `${scopeSelector} code, ${scopeSelector} pre` : null;
        if (sel) {
            lines.push(`${sel} { font-family: ${fontStack(font.name, font.role)}; }`);
        }
    }
    lines.push('');

    lines.push(`${scopeSelector} h1, ${scopeSelector} h2, ${scopeSelector} h3, ${scopeSelector} h4 { color: var(--apdf-heading); margin-top: 1.5em; margin-bottom: 0.5em; }`);
    lines.push(`${scopeSelector} a { color: var(--apdf-link); text-decoration: none; }`);
    lines.push(`${scopeSelector} a:hover { text-decoration: underline; }`);
    lines.push(`${scopeSelector} code { background: var(--apdf-code-bg); padding: 0.15em 0.3em; border-radius: 3px; font-size: 0.9em; }`);

    if (d.hasMath) {
        lines.push(`${scopeSelector} .math-block { display: block; text-align: center; margin: 1em 0; overflow-x: auto; }`);
    }

    // Print
    lines.push('');
    lines.push(`@media print {`);
    lines.push(`  ${scopeSelector} { max-width: none; font-size: 10pt; }`);
    lines.push(`  @page { size: ${pageW}pt ${pageH}pt; margin: 2cm; }`);
    lines.push(`}`);

    return lines.join('\n');
}

/**
 * Generate a full responsive HTML article from aPDF
 */
function generateResponsiveHTML(apdf: any): string {
    const css = generateTypesetCSS(apdf, '.article');
    const html: string[] = [];

    html.push('<!DOCTYPE html>');
    html.push(`<html lang="${escapeAttrStr(apdf.metadata.language)}">`);
    html.push('<head>');
    html.push('  <meta charset="utf-8">');
    html.push('  <meta name="viewport" content="width=device-width, initial-scale=1">');
    html.push(`  <title>${escapeHTMLStr(apdf.metadata.title)}</title>`);

    if (apdf.metadata.identifiers.doi) {
        html.push(`  <meta name="citation_doi" content="${escapeAttrStr(apdf.metadata.identifiers.doi)}">`);
    }
    for (const author of apdf.authors) {
        html.push(`  <meta name="citation_author" content="${escapeAttrStr(author.name)}">`);
    }

    if (apdf.display.hasMath) {
        html.push('  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css" integrity="sha384-n8MVd4RsNIU0tQ2/19gy7bdV4CSjkCEQAo3GrJpc7b/dQSzuqoDP9pwp1SUbJkm8" crossorigin="anonymous">');
        html.push('  <script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.js" integrity="sha384-XjKyOOlGwcjNTAIQHIpgOno0Ola1kmFsp8Ro+2LQWtuGCicnlao/VUfR8rLJ33Eo" crossorigin="anonymous"></script>');
        html.push('  <script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/contrib/auto-render.min.js" integrity="sha384-+VBxd8OG0CnQQRXaISmihC2MG4qXMPSC4e+un7F1GA1iqtVrAmGnDnI9ow3NfJut" crossorigin="anonymous"></script>');
    }

    html.push('  <style>');
    html.push(css);
    html.push('  @media (max-width: 600px) { .article { column-count: 1 !important; padding: 1rem; font-size: 0.95rem; } .article h1 { font-size: 1.4rem; } }');
    html.push('  .toc { background: var(--apdf-code-bg); padding: 1rem 1.5rem; border-radius: 6px; margin: 1.5rem 0; }');
    html.push('  .toc ul { list-style: none; padding-left: 1.2em; }');
    html.push('  .toc > ul { padding-left: 0; }');
    html.push('  </style>');
    html.push('</head>');
    html.push('<body>');
    html.push('<article class="article" role="article">');

    // Title
    html.push(`  <header>`);
    html.push(`    <h1>${escapeHTMLStr(apdf.metadata.title)}</h1>`);
    if (apdf.authors.length > 0) {
        const authorStr = apdf.authors.map((a: any) => escapeHTMLStr(a.name)).join(', ');
        html.push(`    <p class="authors">${authorStr}</p>`);
    }
    html.push(`  </header>`);

    // Abstract
    if (apdf.metadata.abstract) {
        html.push(`  <section class="abstract" aria-label="Abstract"><h2>Abstract</h2><p>${escapeHTMLStr(apdf.metadata.abstract)}</p></section>`);
    }

    // TOC
    if (apdf.structure.tableOfContents.length > 0) {
        html.push('  <nav class="toc" aria-label="Table of Contents"><strong>Contents</strong><ul>');
        for (const entry of apdf.structure.tableOfContents) {
            const anchor = entry.title.toLowerCase().replace(/[^a-z0-9]+/g, '-');
            html.push(`    <li><a href="#${escapeAttrStr(anchor)}">${escapeHTMLStr(entry.title)}</a></li>`);
        }
        html.push('  </ul></nav>');
    }

    // Body chunks
    for (const chunk of apdf.aiContent.chunks) {
        html.push(`  <p>${escapeHTMLStr(chunk.content.trim())}</p>`);
    }

    // Bibliography
    if (apdf.structure.bibliography.length > 0) {
        html.push('  <section class="bibliography" aria-label="References"><h2>References</h2><ol>');
        for (const bib of apdf.structure.bibliography) {
            const authors = bib.authors?.join(', ') || 'Unknown';
            const year = bib.year ? ` (${bib.year})` : '';
            let entry = escapeHTMLStr(`${authors}${year}. "${bib.title}"`);
            if (bib.venue) entry += `. <em>${escapeHTMLStr(bib.venue)}</em>`;
            if (bib.doi) entry += `. <a href="https://doi.org/${escapeAttrStr(bib.doi)}">doi:${escapeHTMLStr(bib.doi)}</a>`;
            html.push(`    <li>${entry}</li>`);
        }
        html.push('  </ol></section>');
    }

    html.push('</article>');

    if (apdf.display.hasMath) {
        html.push('<script>');
        html.push('document.addEventListener("DOMContentLoaded", function() {');
        html.push('  if (typeof renderMathInElement === "function") {');
        html.push('    renderMathInElement(document.querySelector(".article"), {');
        html.push('      delimiters: [{ left: "$$", right: "$$", display: true }, { left: "$", right: "$", display: false }]');
        html.push('    });');
        html.push('  }');
        html.push('});');
        html.push('</script>');
    }

    html.push('</body></html>');
    return html.join('\n');
}

/**
 * Generate accessible HTML reading view from aPDF
 */
function generateAccessibleHTML(apdf: any): string {
    const stats = apdf.aiContent.stats;
    const readingLevel = stats.readingLevel != null
        ? (stats.readingLevel <= 8 ? 'Easy' : stats.readingLevel <= 12 ? 'Moderate' : 'Advanced')
        : 'Unknown';

    const html: string[] = [];

    html.push('<!DOCTYPE html>');
    html.push(`<html lang="${escapeAttrStr(apdf.metadata.language)}">`);
    html.push('<head>');
    html.push('  <meta charset="utf-8">');
    html.push('  <meta name="viewport" content="width=device-width, initial-scale=1">');
    html.push(`  <title>${escapeHTMLStr(apdf.metadata.title)} \u2014 Accessible View</title>`);
    html.push('  <style>');
    html.push('    body { font-family: system-ui, sans-serif; max-width: 45em; margin: 2rem auto; padding: 0 1rem; line-height: 1.7; }');
    html.push('    .skip-link { position: absolute; top: -40px; left: 0; background: #000; color: #fff; padding: 8px; z-index: 100; }');
    html.push('    .skip-link:focus { top: 0; }');
    html.push('    .doc-info { background: #f5f5f5; padding: 1rem; border-radius: 6px; margin-bottom: 2rem; }');
    html.push('    .chunk[data-importance="high"] { border-left: 3px solid #4CAF50; padding-left: 1em; }');
    html.push('    @media (prefers-color-scheme: dark) { body { background: #1a1a1a; color: #d4d4d4; } .doc-info { background: #2a2a2a; } }');
    html.push('  </style>');
    html.push('</head>');
    html.push('<body>');

    html.push('  <a class="skip-link" href="#main-content">Skip to content</a>');

    html.push('  <aside class="doc-info" role="complementary" aria-label="Document information"><dl>');
    html.push(`    <dt>Title</dt><dd>${escapeHTMLStr(apdf.metadata.title)}</dd>`);
    html.push(`    <dt>Pages</dt><dd>${apdf.metadata.pageCount}</dd>`);
    html.push(`    <dt>Reading level</dt><dd>${readingLevel}${stats.readingLevel != null ? ' (grade ' + stats.readingLevel + ')' : ''}</dd>`);
    html.push(`    <dt>Estimated reading time</dt><dd>${Math.ceil(stats.tokenCount / 250)} min</dd>`);
    html.push(`    <dt>Language</dt><dd>${escapeHTMLStr(apdf.metadata.language)}</dd>`);
    html.push('  </dl></aside>');

    // TOC
    if (apdf.structure.tableOfContents.length > 0) {
        html.push('  <nav aria-label="Table of contents"><h2>Contents</h2><ol>');
        for (const entry of apdf.structure.tableOfContents) {
            const anchor = entry.title.toLowerCase().replace(/[^a-z0-9]+/g, '-');
            html.push(`    <li><a href="#${escapeAttrStr(anchor)}">${escapeHTMLStr(entry.title)}</a></li>`);
        }
        html.push('  </ol></nav>');
    }

    html.push('  <main id="main-content" role="main">');
    for (const chunk of apdf.aiContent.chunks) {
        const importance = chunk.importance >= 0.7 ? 'high' : chunk.importance >= 0.4 ? 'medium' : 'low';
        html.push(`    <div class="chunk" data-importance="${importance}"><p>${escapeHTMLStr(chunk.content.trim())}</p></div>`);
    }
    html.push('  </main>');

    html.push(`  <footer role="contentinfo"><p><small>Generated from aPDF v${escapeHTMLStr(apdf.apdfVersion)} by ${escapeHTMLStr(apdf.provenance.generator)}</small></p></footer>`);
    html.push('</body></html>');

    return html.join('\n');
}

/**
 * Generate print-ready CSS from aPDF display hints
 */
function generatePrintCSS(apdf: any): string {
    const d = apdf.display;
    const { width: pageW, height: pageH } = d.pageDimensions;
    const pageSizeName =
        (Math.abs(pageW - 612) < 5 && Math.abs(pageH - 792) < 5) ? 'letter' :
        (Math.abs(pageW - 595) < 5 && Math.abs(pageH - 842) < 5) ? 'A4' :
        `${pageW}pt ${pageH}pt`;

    const bodyFont = d.fonts.find((f: any) => f.role === 'body');
    const headingFont = d.fonts.find((f: any) => f.role === 'heading');
    const monoFont = d.fonts.find((f: any) => f.role === 'mono');

    const lines: string[] = [];

    lines.push(`/* aPDF Print-Ready Stylesheet — ${escapeCSSStr(apdf.metadata.title)} */`);
    lines.push(`@page { size: ${pageSizeName}; margin: 2.5cm 2cm; @bottom-center { content: counter(page); font-size: 9pt; } }`);
    lines.push(`@page :first { margin-top: 4cm; @bottom-center { content: none; } }`);
    lines.push('');

    lines.push(`body {`);
    lines.push(`  font-family: "${bodyFont?.name || 'Georgia'}", "Noto Serif", serif;`);
    lines.push(`  font-size: 10pt; line-height: 1.5; color: #000;`);
    lines.push(`  orphans: 3; widows: 3; hyphens: auto; text-align: justify;`);
    lines.push(`}`);

    if (d.readingOrder === 'multi-column') {
        lines.push(`.content { column-count: 2; column-gap: 0.6cm; column-rule: 0.5pt solid #ccc; }`);
    }

    lines.push(`h1, h2, h3, h4 { font-family: "${headingFont?.name || bodyFont?.name || 'Georgia'}", serif; page-break-after: avoid; break-after: avoid; }`);
    lines.push(`h1 { font-size: 16pt; margin-top: 0; }`);
    lines.push(`h2 { font-size: 13pt; margin-top: 1.5em; }`);
    lines.push(`h3 { font-size: 11pt; margin-top: 1.2em; }`);
    lines.push(`figure, table { page-break-inside: avoid; break-inside: avoid; margin: 1em 0; }`);
    lines.push(`figcaption { font-size: 9pt; text-align: center; margin-top: 0.5em; }`);
    lines.push(`pre, code { font-family: "${monoFont?.name || 'Courier New'}", "Fira Code", monospace; font-size: 8.5pt; }`);
    lines.push(`pre { background: #f8f8f8; padding: 0.5em; border: 0.5pt solid #ddd; page-break-inside: avoid; break-inside: avoid; }`);

    if (d.hasMath) {
        lines.push(`.math-block, .katex-display { page-break-inside: avoid; break-inside: avoid; margin: 0.8em 0; }`);
    }

    lines.push(`.bibliography { font-size: 9pt; line-height: 1.4; }`);
    lines.push(`a[href^="http"]::after { content: " (" attr(href) ")"; font-size: 8pt; color: #666; word-break: break-all; }`);
    lines.push(`.abstract { font-size: 9.5pt; font-style: italic; margin: 1em 2em; }`);

    return lines.join('\n');
}

/**
 * Generate Open Graph / Twitter Card / JSON-LD meta tags from aPDF
 */
function generateSocialMeta(apdf: any, pageUrl: string): string {
    const title = apdf.metadata.title;
    const description = apdf.metadata.abstract
        || apdf.aiContent.summary
        || apdf.aiContent.chunks[0]?.content.substring(0, 200)
        || '';

    const tags: string[] = [];

    tags.push('<!-- Open Graph -->');
    tags.push(`<meta property="og:type" content="article">`);
    tags.push(`<meta property="og:title" content="${escapeAttrStr(title)}">`);
    tags.push(`<meta property="og:description" content="${escapeAttrStr(truncateStr(description, 300))}">`);
    tags.push(`<meta property="og:url" content="${escapeAttrStr(pageUrl)}">`);
    if (apdf.metadata.datePublished) {
        tags.push(`<meta property="article:published_time" content="${escapeAttrStr(apdf.metadata.datePublished)}">`);
    }
    for (const author of apdf.authors) {
        tags.push(`<meta property="article:author" content="${escapeAttrStr(author.name)}">`);
    }
    for (const kw of apdf.aiContent.keywords.slice(0, 5)) {
        tags.push(`<meta property="article:tag" content="${escapeAttrStr(kw)}">`);
    }

    tags.push('');
    tags.push('<!-- Twitter Card -->');
    tags.push(`<meta name="twitter:card" content="summary_large_image">`);
    tags.push(`<meta name="twitter:title" content="${escapeAttrStr(title)}">`);
    tags.push(`<meta name="twitter:description" content="${escapeAttrStr(truncateStr(description, 200))}">`);

    tags.push('');
    tags.push('<!-- Citation metadata -->');
    tags.push(`<meta name="citation_title" content="${escapeAttrStr(title)}">`);
    for (const author of apdf.authors) {
        tags.push(`<meta name="citation_author" content="${escapeAttrStr(author.name)}">`);
    }
    if (apdf.metadata.identifiers.doi) {
        tags.push(`<meta name="citation_doi" content="${escapeAttrStr(apdf.metadata.identifiers.doi)}">`);
    }
    if (apdf.metadata.identifiers.arxivId) {
        tags.push(`<meta name="citation_arxiv_id" content="${escapeAttrStr(apdf.metadata.identifiers.arxivId)}">`);
    }

    tags.push('');

    const jsonLd = {
        '@context': 'https://schema.org',
        '@type': apdf['@type'],
        headline: title,
        author: apdf.authors.map((a: any) => ({
            '@type': 'Person',
            name: a.name,
            ...(a.orcid ? { url: 'https://orcid.org/' + a.orcid } : {}),
        })),
        datePublished: apdf.metadata.datePublished,
        description: truncateStr(description, 300),
        keywords: apdf.aiContent.keywords.slice(0, 10).join(', '),
        url: pageUrl,
        ...(apdf.metadata.identifiers.doi ? { sameAs: 'https://doi.org/' + apdf.metadata.identifiers.doi } : {}),
    };

    tags.push('<script type="application/ld+json">');
    tags.push(JSON.stringify(jsonLd, null, 2));
    tags.push('</script>');

    return tags.join('\n');
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
            const safePath = validateOutputPath(options.output);
            fs.writeFileSync(safePath, output, 'utf-8');
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
            const safePath = validateOutputPath(options.output);
            fs.writeFileSync(safePath, output, 'utf-8');
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

                // Security: Ensure output path is within intended directory (CWE-22)
                const resolvedDir = path.resolve(outputDir) + path.sep;
                const resolvedPath = path.resolve(filepath);
                if (!resolvedPath.startsWith(resolvedDir)) {
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
            const safePath = validateOutputPath(options.output);
            fs.writeFileSync(safePath, output, 'utf-8');
            log.success(`Form data saved to: ${options.output}`);
        } else {
            console.log('\n' + output);
        }
    } finally {
        pdf.close();
    }
}

/**
 * Unified AI ingestion command.
 *
 * Default mode: output a single IngestResult JSON object.
 * --ndjson mode: stream one JSON object per line (header, chunks, footer).
 */
async function commandIngest(options: CLIOptions): Promise<void> {
    const pdf = await loadPDF(options.input!, options);

    try {
        log.header('Unified AI Ingestion');

        const ingestOpts: any = {
            strategy: 'semantic',
            maxChunkSize: options.chunkSize ?? DEFAULT_CHUNK_SIZE,
            overlapSize: Math.min(Math.round((options.chunkSize ?? DEFAULT_CHUNK_SIZE) / 10), 200),
            includeStructure: true,
            includePageText: options.includeText ?? false,
        };

        if (options.pages) {
            const range = parsePageRange(options.pages);
            if (!Array.isArray(range)) {
                ingestOpts.pageRange = range;
            }
        }

        if (options.ndjson) {
            // NDJSON streaming mode — one JSON object per line to stdout
            for await (const record of pdf.streamIngest(ingestOpts)) {
                process.stdout.write(JSON.stringify(record) + '\n');
            }
        } else {
            // Single JSON result
            const result = await pdf.ingest(ingestOpts);
            const output = options.pretty
                ? JSON.stringify(result, null, 2)
                : JSON.stringify(result);

            if (options.output) {
                const safePath = validateOutputPath(options.output);
                fs.writeFileSync(safePath, output, 'utf-8');
                log.success(`Ingest result saved to: ${options.output}`);
                log.data('Chunks', result.chunks.length.toString());
                log.data('Total tokens', result.stats.totalTokens.toString());
                log.data('Processing time', `${result.stats.processingTimeMs}ms`);
            } else {
                console.log(output);
            }
        }
    } finally {
        pdf.close();
    }
}

/**
 * Output tool/function-calling schemas for AI agent integration.
 * Supports openai, anthropic, generic, and mcp formats.
 */
async function commandToolSchema(options: CLIOptions): Promise<void> {
    const format = options.toolSchema ?? 'openai';
    const validFormats = ['openai', 'anthropic', 'generic', 'mcp'];
    if (!validFormats.includes(format)) {
        throw new Error(`Invalid tool-schema format: ${sanitizeInput(format)}. Must be one of: ${validFormats.join(', ')}`);
    }

    let output: string;
    if (format === 'mcp') {
        const manifest = AgenticPDF.getMCPManifest();
        output = options.pretty ? JSON.stringify(manifest, null, 2) : JSON.stringify(manifest);
    } else {
        const schemas = AgenticPDF.getToolSchemas(format as any);
        output = options.pretty ? JSON.stringify(schemas, null, 2) : JSON.stringify(schemas);
    }

    if (options.output) {
        const safePath = validateOutputPath(options.output);
        fs.writeFileSync(safePath, output, 'utf-8');
        log.success(`Tool schemas (${format}) saved to: ${options.output}`);
    } else {
        console.log(output);
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

    // Standalone --tool-schema flag (no file needed)
    if (args.includes('--tool-schema')) {
        const opts = parseArgs(['tool-schema', ...args]);
        await commandToolSchema(opts);
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
            case 'metadata':
                await commandApdf(options);
                break;
            case 'typeset':
                await commandTypeset(options);
                break;
            case 'generate':
                await commandGenerate(options);
                break;
            case 'ingest':
                await commandIngest(options);
                break;
            case 'tool-schema':
                await commandToolSchema(options);
                break;
            case 'help':
                displayHelp();
                break;
            case 'version':
                displayVersion();
                break;
            default:
                log.error(`Unknown command: ${options.command}`);
                log.info('Run "apdf help" for usage information');
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
