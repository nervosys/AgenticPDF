#!/usr/bin/env npx tsx

/**
 * PDF-to-aPDF Generator
 *
 * Reads a PDF file, generates rich aPDF metadata via AgenticPDF,
 * and writes a binary .apdf container that packages the original
 * PDF bytes together with the metadata envelope.
 *
 * Supports the v1.1 streaming-optimized format with optional
 * AES-256-GCM encryption.
 *
 * Usage:
 *   npx tsx scripts/generate-apdf.ts <input.pdf> [output.apdf] [--encrypt --password <pass>]
 *
 * If no output path is given, replaces the .pdf extension with .apdf.
 */

import AgenticPDF from '../agenticpdf';
import * as fs from 'fs';
import * as path from 'path';

async function main(): Promise<void> {
  const args = process.argv.slice(2);

  if (args.length === 0 || args[0] === '--help' || args[0] === '-h') {
    console.log('Usage: npx tsx scripts/generate-apdf.ts <input.pdf> [output.apdf] [--encrypt --password <pass>]');
    console.log('       npx tsx scripts/generate-apdf.ts demos/sample.pdf demos/sample.apdf');
    console.log('       npx tsx scripts/generate-apdf.ts demos/sample.pdf demos/sample.apdf --encrypt --password secret');
    process.exit(args.length === 0 ? 1 : 0);
  }

  // Parse positional and flag arguments
  let inputArg: string | undefined;
  let outputArg: string | undefined;
  let encrypt = false;
  let password: string | undefined;

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--encrypt') {
      encrypt = true;
    } else if (args[i] === '--password') {
      password = args[++i];
    } else if (!inputArg) {
      inputArg = args[i];
    } else if (!outputArg) {
      outputArg = args[i];
    }
  }

  if (!inputArg) {
    console.error('Error: No input PDF specified');
    process.exit(1);
  }

  const inputPath = path.resolve(inputArg);
  const outputPath = outputArg
    ? path.resolve(outputArg)
    : inputPath.replace(/\.pdf$/i, '.apdf');

  if (!fs.existsSync(inputPath)) {
    console.error(`Error: File not found: ${inputPath}`);
    process.exit(1);
  }

  if (encrypt && !password) {
    console.error('Error: --encrypt requires --password <password>');
    process.exit(1);
  }

  console.log(`📄 Loading PDF: ${inputPath}`);
  const fileBuffer = fs.readFileSync(inputPath);
  const arrayBuffer = fileBuffer.buffer.slice(
    fileBuffer.byteOffset,
    fileBuffer.byteOffset + fileBuffer.byteLength
  ) as ArrayBuffer;

  const pdf = await AgenticPDF.fromBuffer(arrayBuffer, { lazyLoad: true });

  try {
    console.log('🔍 Analyzing document structure...');

    const binaryOptions = encrypt
      ? { encryption: { password: password!, encryptPDF: true, encryptMetadata: false } }
      : undefined;

    const binary = await pdf.generateAPDFBinary(binaryOptions);
    const metadata = await pdf.generateAPDFMetadata();

    // Summary
    console.log(`   Title:    ${metadata.metadata.title}`);
    console.log(`   Type:     ${metadata['@type']}`);
    console.log(`   Pages:    ${metadata.metadata.pageCount}`);
    if (metadata.authors.length > 0) {
      console.log(`   Authors:  ${metadata.authors.map(a => a.name).join(', ')}`);
    }
    if (metadata.artifacts.length > 0) {
      console.log(`   Artifacts: ${metadata.artifacts.length} linked`);
    }
    console.log(`   Chunks:   ${metadata.aiContent.chunks.length}`);
    console.log(`   Format:   aPDF v1.1 (streaming-optimized)`);
    if (encrypt) console.log(`   Encrypted: PDF data (AES-256-GCM)`);

    console.log(`📦 Writing aPDF binary container (${(binary.length / 1024).toFixed(1)} KB)...`);
    fs.writeFileSync(outputPath, binary);
    console.log(`✅ Saved to: ${outputPath}`);
  } finally {
    pdf.close();
  }
}

main().catch((err) => {
  console.error('Error:', err.message || err);
  process.exit(1);
});
