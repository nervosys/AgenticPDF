#!/usr/bin/env tsx
/**
 * AgenticPDF Examples Runner (Simplified)
 * 
 * A lightweight script to run AgenticPDF examples with Node.js compatibility.
 * This version includes fallbacks for Node.js environment.
 */

import * as fs from 'fs';
import * as path from 'path';
import * as readline from 'readline';

// CLI Colors for better output
const colors = {
    reset: '\x1b[0m',
    bright: '\x1b[1m',
    red: '\x1b[31m',
    green: '\x1b[32m',
    yellow: '\x1b[33m',
    blue: '\x1b[34m',
    magenta: '\x1b[35m',
    cyan: '\x1b[36m',
    white: '\x1b[37m'
};

function colorize(text: string, color: keyof typeof colors): string {
    return `${colors[color]}${text}${colors.reset}`;
}

// Mock examples for Node.js environment
const mockExamples = [
    {
        name: 'Basic Processing',
        description: 'Fundamental PDF operations: loading, text extraction, metadata, and basic search',
        requiredInputs: ['file: File'],
        run: async (inputs: any) => {
            console.log('🔄 Mock: Loading PDF file...');
            await new Promise(resolve => setTimeout(resolve, 1000));
            console.log(`✅ Mock: Processed ${inputs.file?.name || 'PDF file'}`);
            console.log('📄 Mock: Extracted 1,234 characters of text');
            console.log('📊 Mock: Found 5 pages, 2 images, 1 table');
        }
    },
    {
        name: 'AI Integration',
        description: 'AI-powered features: semantic chunking, embedding providers, document analysis',
        requiredInputs: ['file: File', 'apiKey?: string'],
        run: async (inputs: any) => {
            console.log('🤖 Mock: Initializing AI features...');
            await new Promise(resolve => setTimeout(resolve, 1500));
            if (inputs.apiKey) {
                console.log('✅ Mock: Using provided API key for real AI processing');
            } else {
                console.log('⚠️ Mock: No API key - using simulated AI processing');
            }
            console.log('🧠 Mock: Generated 8 semantic chunks');
            console.log('📊 Mock: Structural analysis found: 3 sections, 1 table, 2 figures');
        }
    },
    {
        name: 'Streaming to LLM',
        description: 'Stream PDF content to Large Language Models with context management',
        requiredInputs: ['file: File', 'apiKey?: string'],
        run: async (inputs: any) => {
            console.log('🌊 Mock: Setting up streaming pipeline...');
            await new Promise(resolve => setTimeout(resolve, 800));
            console.log('📡 Mock: Connected to LLM endpoint');

            for (let i = 1; i <= 5; i++) {
                console.log(`📤 Mock: Streaming chunk ${i}/5...`);
                await new Promise(resolve => setTimeout(resolve, 600));
            }
            console.log('✅ Mock: All chunks processed by LLM');
        }
    },
    {
        name: 'Batch Processing',
        description: 'Efficient processing of multiple PDFs with memory management and progress tracking',
        requiredInputs: ['files: File[]'],
        run: async (inputs: any) => {
            const fileCount = inputs.files?.length || 1;
            console.log(`📁 Mock: Processing ${fileCount} file(s) in batch...`);

            for (let i = 0; i < fileCount; i++) {
                console.log(`🔄 Mock: Processing file ${i + 1}/${fileCount}...`);
                await new Promise(resolve => setTimeout(resolve, 1000));
                console.log(`✅ Mock: File ${i + 1} completed`);
            }
            console.log(`📊 Mock: Batch completed - processed ${fileCount} files`);
        }
    },
    {
        name: 'Real-time WebSocket',
        description: 'Real-time PDF processing with live progress updates via WebSocket',
        requiredInputs: ['file: File', 'wsUrl?: string'],
        run: async (inputs: any) => {
            console.log('🔌 Mock: Establishing WebSocket connection...');
            await new Promise(resolve => setTimeout(resolve, 800));
            console.log('✅ Mock: WebSocket connected');

            const steps = ['Loading', 'Parsing', 'Extracting', 'Analyzing', 'Finalizing'];
            for (let i = 0; i < steps.length; i++) {
                console.log(`🔄 Mock: ${steps[i]}... (${((i + 1) / steps.length * 100).toFixed(0)}%)`);
                await new Promise(resolve => setTimeout(resolve, 1000));
            }
            console.log('✅ Mock: Real-time processing completed');
        }
    }
];

// Show help
function showHelp(): void {
    console.log(colorize('AgenticPDF Examples Runner', 'bright'));
    console.log(colorize('========================\n', 'cyan'));
    console.log('Usage:');
    console.log('  npm run examples                     # Interactive mode');
    console.log('  npm run examples -- --file=test.pdf  # Run with specific file');
    console.log('  npm run examples -- --all            # Run all examples');
    console.log('  npm run examples -- --help           # Show this help\n');
    console.log('Environment Variables:');
    console.log('  DEBUG=1                              # Show detailed error information');
    console.log('  API_KEY=your-key                     # Provide API key for AI features\n');
    console.log('Examples:');
    console.log('  npm run examples -- --file=./sample.pdf');
    console.log('  DEBUG=1 npm run examples -- --file=./test.pdf --all');
    console.log('  API_KEY=sk-... npm run examples\n');
    console.log(colorize('Note: This is a simplified runner with mock examples for Node.js compatibility.', 'yellow'));
    console.log(colorize('For full functionality, use the browser demo: npm run examples:demo', 'yellow'));
}

// Parse command line arguments
function parseArgs(): { file?: string; all?: boolean; help?: boolean } {
    const args = process.argv.slice(2);
    const parsed: any = {};

    for (const arg of args) {
        if (arg === '--help' || arg === '-h') {
            parsed.help = true;
        } else if (arg === '--all') {
            parsed.all = true;
        } else if (arg.startsWith('--file=')) {
            parsed.file = arg.split('=')[1];
        }
    }

    return parsed;
}

// Create mock File object
function createMockFile(filePath: string): any {
    const fileName = path.basename(filePath);
    const stats = fs.statSync(filePath);

    return {
        name: fileName,
        size: stats.size,
        type: 'application/pdf',
        lastModified: stats.mtime.getTime(),
        // Add mock methods that might be needed
        arrayBuffer: async () => fs.promises.readFile(filePath),
        stream: () => fs.createReadStream(filePath)
    };
}

// Interactive file picker
async function selectPDFFile(): Promise<string> {
    const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout
    });

    return new Promise((resolve, reject) => {
        rl.question(colorize('📁 Enter path to PDF file: ', 'cyan'), (filePath) => {
            rl.close();

            if (!filePath.trim()) {
                reject(new Error('No file path provided'));
                return;
            }

            const absolutePath = path.resolve(filePath.trim());

            if (!fs.existsSync(absolutePath)) {
                reject(new Error(`File not found: ${absolutePath}`));
                return;
            }

            if (!absolutePath.toLowerCase().endsWith('.pdf')) {
                reject(new Error('File must be a PDF'));
                return;
            }

            resolve(absolutePath);
        });
    });
}

// Interactive API key input
async function getAPIKey(): Promise<string | undefined> {
    const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout
    });

    return new Promise((resolve) => {
        rl.question(colorize('🔑 Enter API key for AI features (optional, press Enter to skip): ', 'cyan'), (apiKey) => {
            rl.close();
            resolve(apiKey.trim() || undefined);
        });
    });
}

// Interactive example selection
async function selectExamples(): Promise<number[]> {
    console.log(colorize('\n📋 Available Examples:', 'bright'));
    mockExamples.forEach((example, index) => {
        console.log(`  ${colorize((index + 1).toString(), 'yellow')}. ${colorize(example.name, 'green')}`);
        console.log(`     ${example.description}`);
        console.log(`     ${colorize('Required:', 'blue')} ${example.requiredInputs.join(', ')}\n`);
    });

    const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout
    });

    return new Promise((resolve, reject) => {
        rl.question(colorize('Select examples to run (comma-separated numbers, or "all" for all): ', 'cyan'), (input) => {
            rl.close();

            const trimmed = input.trim().toLowerCase();

            if (trimmed === 'all' || trimmed === '') {
                resolve(mockExamples.map((_, index) => index));
                return;
            }

            try {
                const selected = trimmed
                    .split(',')
                    .map(s => parseInt(s.trim()) - 1)
                    .filter(n => n >= 0 && n < mockExamples.length);

                if (selected.length === 0) {
                    reject(new Error('No valid examples selected'));
                    return;
                }

                resolve(selected);
            } catch (error) {
                reject(new Error('Invalid selection format'));
            }
        });
    });
}

// Run a single example
async function runSingleExample(example: any, file: any, apiKey?: string): Promise<boolean> {
    console.log(colorize(`\n${'='.repeat(80)}`, 'cyan'));
    console.log(colorize(`🧪 Running: ${example.name}`, 'bright'));
    console.log(colorize(`📝 Description: ${example.description}`, 'white'));
    console.log(colorize(`${'='.repeat(80)}`, 'cyan'));

    const startTime = Date.now();

    try {
        const inputs: any = { file };
        if (apiKey) inputs.apiKey = apiKey;

        // Special handling for batch processing
        if (example.name.includes('Batch')) {
            inputs.files = [file];
        }

        await example.run(inputs);

        const duration = Date.now() - startTime;
        console.log(colorize(`\n✅ ${example.name} completed successfully in ${duration}ms`, 'green'));
        return true;

    } catch (error) {
        const duration = Date.now() - startTime;
        console.error(colorize(`\n❌ ${example.name} failed after ${duration}ms:`, 'red'));
        console.error(colorize((error as Error).message, 'red'));
        return false;
    }
}

// Main interactive mode
async function runInteractiveMode(): Promise<void> {
    console.log(colorize('🚀 AgenticPDF Examples Runner (Simplified)', 'bright'));
    console.log(colorize('========================================\n', 'cyan'));
    console.log(colorize('ℹ️  Running in Node.js compatibility mode with mock examples', 'yellow'));
    console.log(colorize('💡 For full functionality, use: npm run examples:demo\n', 'blue'));

    try {
        // Get PDF file
        const filePath = await selectPDFFile();
        console.log(colorize(`✅ Selected: ${path.basename(filePath)}\n`, 'green'));

        // Get API key
        const apiKey = await getAPIKey();
        if (apiKey) {
            console.log(colorize('✅ API key provided for AI features\n', 'green'));
        } else {
            console.log(colorize('ℹ️  No API key provided - using mock data\n', 'yellow'));
        }

        // Select examples
        const selectedIndices = await selectExamples();
        const selectedExamples = selectedIndices.map(i => mockExamples[i]);

        console.log(colorize(`\n🎯 Will run ${selectedExamples.length} example(s)\n`, 'bright'));

        // Create mock File object
        const file = createMockFile(filePath);

        // Run selected examples
        let successCount = 0;
        let totalCount = selectedExamples.length;

        for (let i = 0; i < selectedExamples.length; i++) {
            const example = selectedExamples[i];
            console.log(colorize(`\n[${i + 1}/${totalCount}] Starting ${example.name}...`, 'magenta'));

            const success = await runSingleExample(example, file, apiKey);
            if (success) successCount++;

            // Pause between examples
            if (i < selectedExamples.length - 1) {
                console.log(colorize('\n⏸️  Pausing for 2 seconds...\n', 'yellow'));
                await new Promise(resolve => setTimeout(resolve, 2000));
            }
        }

        // Final summary
        console.log(colorize(`\n${'='.repeat(80)}`, 'cyan'));
        console.log(colorize('📊 EXECUTION SUMMARY', 'bright'));
        console.log(colorize(`${'='.repeat(80)}`, 'cyan'));
        console.log(colorize(`✅ Successful: ${successCount}/${totalCount}`, 'green'));
        console.log(colorize(`❌ Failed: ${totalCount - successCount}/${totalCount}`, 'red'));
        console.log(colorize(`📁 File: ${path.basename(filePath)}`, 'white'));
        console.log(colorize(`🔑 API Key: ${apiKey ? 'Provided' : 'Not provided'}`, 'white'));
        console.log(colorize(`${'='.repeat(80)}`, 'cyan'));

        if (successCount === totalCount) {
            console.log(colorize('\n🎉 All examples completed successfully!', 'green'));
        } else {
            console.log(colorize('\n⚠️  Some examples failed - check output above for details', 'yellow'));
        }

    } catch (error) {
        console.error(colorize('\n❌ Error in interactive mode:', 'red'));
        console.error(colorize((error as Error).message, 'red'));
        process.exit(1);
    }
}

// Run all examples with a file
async function runAllExamples(filePath: string, apiKey?: string): Promise<void> {
    console.log(colorize('🎯 Running all examples...', 'bright'));

    const file = createMockFile(filePath);
    let successCount = 0;

    for (let i = 0; i < mockExamples.length; i++) {
        const example = mockExamples[i];
        console.log(colorize(`\n[${i + 1}/${mockExamples.length}] ${example.name}`, 'magenta'));

        const success = await runSingleExample(example, file, apiKey);
        if (success) successCount++;

        // Brief pause between examples
        if (i < mockExamples.length - 1) {
            await new Promise(resolve => setTimeout(resolve, 1000));
        }
    }

    console.log(colorize(`\n🎉 Completed ${successCount}/${mockExamples.length} examples successfully!`, 'green'));
}

// Main execution
async function main(): Promise<void> {
    const args = parseArgs();

    if (args.help) {
        showHelp();
        return;
    }

    if (args.file) {
        // Non-interactive mode with file
        console.log(colorize('🚀 Running examples in non-interactive mode\n', 'bright'));

        try {
            // Validate file
            const filePath = path.resolve(args.file);
            if (!fs.existsSync(filePath)) {
                throw new Error(`File not found: ${filePath}`);
            }

            const apiKey = process.env.API_KEY;

            if (args.all) {
                console.log(colorize('🎯 Running all examples...', 'bright'));
                await runAllExamples(filePath, apiKey);
            } else {
                console.log(colorize('🎯 Running first example...', 'bright'));
                const file = createMockFile(filePath);
                await runSingleExample(mockExamples[0], file, apiKey);
            }

        } catch (error) {
            console.error(colorize('❌ Error:', 'red'));
            console.error(colorize((error as Error).message, 'red'));
            process.exit(1);
        }
    } else {
        // Interactive mode
        await runInteractiveMode();
    }
}

// Handle process signals
process.on('SIGINT', () => {
    console.log(colorize('\n\n👋 Interrupted by user. Goodbye!', 'yellow'));
    process.exit(0);
});

// Run the script
main().catch((error) => {
    console.error(colorize('💥 Fatal error:', 'red'));
    console.error(colorize(error.message, 'red'));
    process.exit(1);
});

export { main, runSingleExample, createMockFile };