#!/usr/bin/env tsx
/**
 * ModernPDF Examples Runner
 * 
 * A comprehensive script to run all ModernPDF examples with interactive CLI interface.
 * 
 * Usage:
 *   npm run examples                    # Interactive mode
 *   npm run examples -- --file=test.pdf # Run with specific file
 *   npm run examples -- --all          # Run all examples
 *   npm run examples -- --help         # Show help
 */

import * as fs from 'fs';
import * as path from 'path';
import * as readline from 'readline';
import { fileURLToPath } from 'url';

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

// Progress indicator
function showProgress(message: string): void {
    process.stdout.write(`${colorize('⏳', 'yellow')} ${message}...`);
}

function clearProgress(): void {
    process.stdout.write('\r\x1b[K'); // Clear line
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
async function selectExamples(availableExamples: any[]): Promise<number[]> {
    console.log(colorize('\n📋 Available Examples:', 'bright'));
    availableExamples.forEach((example, index) => {
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
                resolve(availableExamples.map((_, index) => index));
                return;
            }

            try {
                const selected = trimmed
                    .split(',')
                    .map(s => parseInt(s.trim()) - 1)
                    .filter(n => n >= 0 && n < availableExamples.length);

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

// Create File object from Node.js file path
async function createFileFromPath(filePath: string): Promise<File> {
    const buffer = await fs.promises.readFile(filePath);
    const fileName = path.basename(filePath);

    // Convert Buffer to Uint8Array for File constructor compatibility
    const uint8Array = new Uint8Array(buffer);

    // Create a File-like object that's compatible with the examples
    return new File([uint8Array], fileName, {
        type: 'application/pdf',
        lastModified: Date.now()
    });
}

// Enhanced logging for better example output
function setupLogging(): void {
    const originalLog = console.log;
    const originalError = console.error;
    const originalWarn = console.warn;

    console.log = (...args: any[]) => {
        const message = args.join(' ');
        originalLog(colorize('📝', 'blue'), message);
    };

    console.error = (...args: any[]) => {
        const message = args.join(' ');
        originalError(colorize('❌', 'red'), colorize(message, 'red'));
    };

    console.warn = (...args: any[]) => {
        const message = args.join(' ');
        originalWarn(colorize('⚠️', 'yellow'), colorize(message, 'yellow'));
    };
}

// Run a single example with error handling
async function runSingleExample(
    example: any,
    file: File,
    apiKey?: string,
    additionalInputs?: any
): Promise<boolean> {
    console.log(colorize(`\n${'='.repeat(80)}`, 'cyan'));
    console.log(colorize(`🧪 Running: ${example.name}`, 'bright'));
    console.log(colorize(`📝 Description: ${example.description}`, 'white'));
    console.log(colorize(`${'='.repeat(80)}`, 'cyan'));

    const startTime = Date.now();

    try {
        const inputs: any = { file, ...additionalInputs };
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

        if (process.env.DEBUG) {
            console.error(colorize((error as Error).stack || '', 'red'));
        }

        return false;
    }
}

// Main CLI interface
async function runInteractiveMode(): Promise<void> {
    console.log(colorize('🚀 ModernPDF Examples Runner', 'bright'));
    console.log(colorize('=====================================\n', 'cyan'));

    try {
        // Load examples
        showProgress('Loading examples');
        const examplesModule = await import('./examples/index.ts');
        clearProgress();

        const { examples } = examplesModule;
        console.log(colorize(`✅ Loaded ${examples.length} examples\n`, 'green'));

        // Get PDF file
        const filePath = await selectPDFFile();
        console.log(colorize(`✅ Selected: ${path.basename(filePath)}\n`, 'green'));

        // Get API key
        const apiKey = await getAPIKey();
        if (apiKey) {
            console.log(colorize('✅ API key provided for AI features\n', 'green'));
        } else {
            console.log(colorize('ℹ️  No API key provided - AI features will use mock data\n', 'yellow'));
        }

        // Select examples
        const selectedIndices = await selectExamples(examples);
        const selectedExamples = selectedIndices.map(i => examples[i]);

        console.log(colorize(`\n🎯 Will run ${selectedExamples.length} example(s)\n`, 'bright'));

        // Create File object
        showProgress('Loading PDF file');
        const file = await createFileFromPath(filePath);
        clearProgress();

        // Setup enhanced logging
        setupLogging();

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
            console.log(colorize('💡 Try running with DEBUG=1 for more detailed error information', 'blue'));
        }

    } catch (error) {
        clearProgress();
        console.error(colorize('\n❌ Error in interactive mode:', 'red'));
        console.error(colorize((error as Error).message, 'red'));
        process.exit(1);
    }
}

// Command line argument parsing
function parseArgs(): { file?: string; all?: boolean; help?: boolean; example?: string } {
    const args = process.argv.slice(2);
    const parsed: any = {};

    for (const arg of args) {
        if (arg === '--help' || arg === '-h') {
            parsed.help = true;
        } else if (arg === '--all') {
            parsed.all = true;
        } else if (arg.startsWith('--file=')) {
            parsed.file = arg.split('=')[1];
        } else if (arg.startsWith('--example=')) {
            parsed.example = arg.split('=')[1];
        }
    }

    return parsed;
}

// Show help
function showHelp(): void {
    console.log(colorize('ModernPDF Examples Runner', 'bright'));
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
            // Load examples
            const examplesModule = await import('./examples/index.ts');
            const { examples, runAllExamples } = examplesModule;

            // Validate file
            const filePath = path.resolve(args.file);
            if (!fs.existsSync(filePath)) {
                throw new Error(`File not found: ${filePath}`);
            }

            const file = await createFileFromPath(filePath);
            const apiKey = process.env.API_KEY;

            setupLogging();

            if (args.all) {
                console.log(colorize('🎯 Running all examples...', 'bright'));
                await runAllExamples(file, apiKey);
            } else {
                console.log(colorize('🎯 Running first example...', 'bright'));
                await runSingleExample(examples[0], file, apiKey);
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

process.on('uncaughtException', (error) => {
    console.error(colorize('\n💥 Uncaught exception:', 'red'));
    console.error(colorize(error.message, 'red'));
    if (process.env.DEBUG) {
        console.error(colorize(error.stack || '', 'red'));
    }
    process.exit(1);
});

// Run the script
if (import.meta.url === `file://${process.argv[1]}`) {
    main().catch((error) => {
        console.error(colorize('💥 Fatal error:', 'red'));
        console.error(colorize(error.message, 'red'));
        process.exit(1);
    });
}

export { main, runSingleExample, createFileFromPath };