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
import { createRequire } from 'module';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Path to the TypeScript CLI implementation
const cliPath = join(__dirname, 'cli.ts');

// Get command-line arguments (skip node and script path)
const args = process.argv.slice(2);

// Resolve tsx through Node's module resolution rather than a hardcoded
// `./node_modules/tsx` path. npm hoists dependencies to the top-level
// node_modules, so when this package is installed as a dependency (or
// globally) tsx does not sit under our own directory. Resolution walks the
// parent directories the way `import` would, so it finds tsx wherever npm
// actually put it, and stays correct if tsx changes its dist layout.
const require = createRequire(import.meta.url);
let tsxCliPath = null;
try {
    tsxCliPath = require.resolve('tsx/cli');
} catch {
    // Not installed alongside us; fall back to a tsx on PATH below.
}

// Use Node.js to execute tsx directly (avoid shell)
let child;
if (tsxCliPath) {
    child = spawn(process.execPath, [tsxCliPath, cliPath, ...args], {
        stdio: 'inherit',
        shell: false,  // Security: Never use shell
        env: process.env
    });
} else {
    // Try global tsx installation
    const isWindows = process.platform === 'win32';
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
