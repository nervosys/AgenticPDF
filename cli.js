#!/usr/bin/env node

/**
 * AgenticPDF CLI Entry Point (apdf)
 * 
 * This file is the executable entry point for the CLI when installed via npm.
 * It launches the TypeScript CLI using tsx.
 * 
 * Usage:
 *   apdf <command> [options]
 *   agenticpdf <command> [options]
 * 
 * After installing via: npm install -g agenticpdf
 */

import { spawn } from 'child_process';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { existsSync } from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Path to the TypeScript CLI implementation
const cliPath = join(__dirname, 'cli.ts');

// Get command-line arguments (skip node and script path)
const args = process.argv.slice(2);

// Try to find tsx module (local node_modules or global)
const isWindows = process.platform === 'win32';
const tsxModulePath = join(__dirname, 'node_modules', 'tsx', 'dist', 'cli.mjs');

// Use Node.js to execute tsx directly (avoid shell)
let child;
if (existsSync(tsxModulePath)) {
    // Use local tsx via node
    child = spawn(process.execPath, [tsxModulePath, cliPath, ...args], {
        stdio: 'inherit',
        shell: false,  // Security: Never use shell
        env: process.env
    });
} else {
    // Try global tsx installation
    const tsxBinPath = isWindows ? 'tsx.cmd' : 'tsx';
    child = spawn(tsxBinPath, [cliPath, ...args], {
        stdio: 'inherit',
        shell: false,
        env: process.env
    });
}

// Forward exit code
child.on('exit', (code) => {
    process.exit(code || 0);
});

// Handle errors
child.on('error', (error) => {
    console.error('\x1b[31m✗\x1b[0m Error running AgenticPDF CLI:', error.message);
    console.error('\x1b[33m⚠\x1b[0m Make sure tsx is installed:');
    console.error('  Local: npm install tsx');
    console.error('  Global: npm install -g tsx');
    process.exit(1);
});
