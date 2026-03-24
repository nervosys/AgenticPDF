#!/usr/bin/env node
/**
 * Quick CLI test script
 * Demonstrates CLI functionality with sample PDF
 */

const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

console.log('╔════════════════════════════════════════════════╗');
console.log('║       AgenticPDF CLI - Quick Test             ║');
console.log('╚════════════════════════════════════════════════╝\n');

const samplePdf = path.join(__dirname, 'demos', 'sample.pdf');

if (!fs.existsSync(samplePdf)) {
    console.log('⚠️  Sample PDF not found at demos/sample.pdf');
    console.log('   Create a sample PDF to test the CLI\n');
    process.exit(1);
}

console.log('Running CLI tests with sample PDF...\n');

// Test 1: Info command
console.log('1️⃣  Testing: info command');
try {
    execSync(`npx tsx cli.ts info -i "${samplePdf}"`, { stdio: 'inherit' });
    console.log('   ✅ Info command works\n');
} catch (e) {
    console.log('   ❌ Info command failed\n');
}

// Test 2: Extract command (to console)
console.log('2️⃣  Testing: extract command');
try {
    execSync(`npx tsx cli.ts extract -i "${samplePdf}" -p 1`, { stdio: 'inherit' });
    console.log('   ✅ Extract command works\n');
} catch (e) {
    console.log('   ❌ Extract command failed\n');
}

// Test 3: Help command
console.log('3️⃣  Testing: help command');
try {
    execSync(`npx tsx cli.ts help`, { stdio: 'inherit' });
    console.log('   ✅ Help command works\n');
} catch (e) {
    console.log('   ❌ Help command failed\n');
}

console.log('╔════════════════════════════════════════════════╗');
console.log('║       CLI Tests Complete!                      ║');
console.log('╚════════════════════════════════════════════════╝\n');

console.log('Try these commands:');
console.log('  • npm run cli -- info demos/sample.pdf');
console.log('  • npm run cli -- extract -i demos/sample.pdf');
console.log('  • npm run cli -- help\n');
