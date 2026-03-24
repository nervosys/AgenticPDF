#!/usr/bin/env npx tsx

/**
 * Test script for aPDF v1.1 round-trip: unencrypted + encrypted.
 */

import AgenticPDF from '../agenticpdf';
import * as fs from 'fs';

async function main(): Promise<void> {
  console.log('=== Test 1: Unencrypted v1.1 round-trip ===\n');

  const apdfData = new Uint8Array(fs.readFileSync('demos/sample-v11.apdf'));
  const header = AgenticPDF.readAPDFHeader(apdfData);
  console.log('Version:', header.version);
  console.log('Flags:', header.flags);
  console.log('PDF encrypted:', header.pdfEncrypted);
  console.log('Meta encrypted:', header.metadataEncrypted);
  console.log('Meta offset:', header.metadataOffset, '(expected 64)');
  console.log('Meta length:', header.metadataLength);
  console.log('PDF offset:', header.pdfOffset);
  console.log('PDF length:', header.pdfLength);
  console.log('Total size:', header.totalSize, '== file:', apdfData.length);

  const { metadata, pdfData } = await AgenticPDF.readAPDF(apdfData);
  console.log('Title:', metadata.metadata.title);
  console.log('Pages:', metadata.metadata.pageCount);

  const originalPdf = new Uint8Array(fs.readFileSync('demos/sample.pdf'));
  const match = pdfData.length === originalPdf.length && pdfData.every((b, i) => b === originalPdf[i]);
  console.log('PDF byte-for-byte match:', match);
  if (!match) throw new Error('PDF round-trip mismatch!');

  const metaOnly = await AgenticPDF.readAPDFMetadata(apdfData);
  console.log('Streaming metadata read OK:', metaOnly.metadata.title);
  console.log('\n✅ Unencrypted v1.1 tests passed!\n');

  // Test 2: Encrypted v1.1 round-trip
  console.log('=== Test 2: Encrypted v1.1 round-trip ===\n');

  const pdfBuf = fs.readFileSync('demos/sample.pdf');
  const arrayBuffer = pdfBuf.buffer.slice(pdfBuf.byteOffset, pdfBuf.byteOffset + pdfBuf.byteLength) as ArrayBuffer;
  const pdf = await AgenticPDF.fromBuffer(arrayBuffer, { lazyLoad: true });

  try {
    const encBinary = await pdf.generateAPDFBinary({
      encryption: { password: 'test-password-123', encryptPDF: true, encryptMetadata: false },
    });

    console.log('Encrypted container:', encBinary.length, 'bytes');

    const encHeader = AgenticPDF.readAPDFHeader(encBinary);
    console.log('Version:', encHeader.version);
    console.log('PDF encrypted:', encHeader.pdfEncrypted);
    console.log('Meta encrypted:', encHeader.metadataEncrypted);
    console.log('Meta offset:', encHeader.metadataOffset, '(expected 126 = 64 header + 62 enc header)');

    // Read unencrypted metadata without password
    const encMeta = await AgenticPDF.readAPDFMetadata(encBinary);
    console.log('Metadata readable without password:', encMeta.metadata.title);

    // Decrypt PDF with correct password
    const { metadata: decMeta, pdfData: decPdf } = await AgenticPDF.readAPDF(encBinary, 'test-password-123');
    console.log('Decrypted title:', decMeta.metadata.title);

    const encMatch = decPdf.length === originalPdf.length && decPdf.every((b, i) => b === originalPdf[i]);
    console.log('Decrypted PDF byte-for-byte match:', encMatch);
    if (!encMatch) throw new Error('Encrypted PDF round-trip mismatch!');

    // Test wrong password
    try {
      await AgenticPDF.readAPDF(encBinary, 'wrong-password');
      throw new Error('Should have thrown on wrong password!');
    } catch (e: any) {
      if (e.message === 'Should have thrown on wrong password!') throw e;
      console.log('Wrong password correctly rejected:', e.message.substring(0, 50));
    }

    // Test no password on encrypted PDF
    try {
      const result = await AgenticPDF.readAPDF(encBinary);
      throw new Error('Should have thrown without password!');
    } catch (e: any) {
      if (e.message === 'Should have thrown without password!') throw e;
      console.log('No password correctly rejected:', e.message.substring(0, 60));
    }

    console.log('\n✅ Encrypted v1.1 tests passed!\n');

    // Test 3: Both metadata + PDF encrypted
    console.log('=== Test 3: Full encryption (metadata + PDF) ===\n');

    const fullEncBinary = await pdf.generateAPDFBinary({
      encryption: { password: 'full-enc-pass', encryptPDF: true, encryptMetadata: true },
    });

    const fullEncHeader = AgenticPDF.readAPDFHeader(fullEncBinary);
    console.log('PDF encrypted:', fullEncHeader.pdfEncrypted);
    console.log('Meta encrypted:', fullEncHeader.metadataEncrypted);

    // Metadata should NOT be readable without password
    try {
      await AgenticPDF.readAPDFMetadata(fullEncBinary);
      throw new Error('Should have required password for encrypted metadata!');
    } catch (e: any) {
      if (e.message === 'Should have required password for encrypted metadata!') throw e;
      console.log('Encrypted metadata correctly requires password');
    }

    // Full decrypt
    const { metadata: fullMeta, pdfData: fullPdf } = await AgenticPDF.readAPDF(fullEncBinary, 'full-enc-pass');
    const fullMatch = fullPdf.length === originalPdf.length && fullPdf.every((b, i) => b === originalPdf[i]);
    console.log('Full decrypt PDF match:', fullMatch);
    if (!fullMatch) throw new Error('Full encryption round-trip mismatch!');

    console.log('\n✅ Full encryption tests passed!');
    console.log('\n🎉 All aPDF v1.1 tests passed!');
  } finally {
    pdf.close();
  }
}

main().catch((err) => {
  console.error('❌ FAIL:', err.message || err);
  process.exit(1);
});
