const fs = require('fs');
const path = require('path');

// --- EDIT 1: Add Phase 18 types/methods to agenticpdf.d.ts ---
const dtsPath = path.join(__dirname, '..', 'agenticpdf.d.ts');
let dts = fs.readFileSync(dtsPath, 'utf8').replace(/\r\n/g, '\n');

// Insert encryption enums and interfaces before SecurityConfig
const dtsMarker1 = `// DoD Security Types\nexport interface SecurityConfig {`;
const dtsInsert1 = `// PDF Encryption Types
/** PDF encryption algorithm types. */
export enum EncryptionAlgorithm {
    RC4_40 = 'RC4-40',
    RC4_128 = 'RC4-128',
    AES_128 = 'AES-128',
    AES_256 = 'AES-256',
}

/** PDF permission flags (ISO 32000-1, Table 22). */
export enum PDFPermission {
    Print = 4,
    ModifyContents = 8,
    ExtractContent = 16,
    Annotate = 32,
    FillForms = 256,
    ExtractForAccessibility = 512,
    Assemble = 1024,
    PrintHighQuality = 2048,
}

/** Encryption dictionary parsed from the PDF trailer. */
export interface EncryptionDict {
    filter: string;
    subFilter?: string;
    version: number;
    revision: number;
    keyLength: number;
    ownerKey: Uint8Array;
    userKey: Uint8Array;
    ownerEncryption?: Uint8Array;
    userEncryption?: Uint8Array;
    permissions: number;
    encryptMetadata: boolean;
    fileId: Uint8Array;
}

/** Result of a password authentication attempt. */
export interface AuthResult {
    authenticated: boolean;
    isOwner: boolean;
    permissions: number;
    encryptionKey: Uint8Array;
}

/** Certificate-based encryption recipient info. */
export interface CertificateRecipient {
    certificate: Uint8Array;
    permissions: number;
}

`;

if (dts.includes('enum EncryptionAlgorithm')) {
  console.log('EDIT 1: Encryption types already in .d.ts, skipping');
} else if (!dts.includes(dtsMarker1)) {
  console.error('EDIT 1 FAILED: marker not found');
  process.exit(1);
} else {
  dts = dts.replace(dtsMarker1, dtsInsert1 + dtsMarker1);
  console.log('EDIT 1: Encryption types added to .d.ts ✅');
}

// Add methods to AgenticPDF class
const dtsMarker2 = `    analyzeLayout(pageRange?: { start: number; end: number }): Promise<{`;
const dtsInsert2 = `    /**
     * Attempt to unlock a password-protected PDF
     */
    unlock(password: string): Promise<boolean>;
    /**
     * Get document permissions from the encryption dictionary
     */
    getPermissions(): {
        print: boolean;
        modify: boolean;
        extract: boolean;
        annotate: boolean;
        fillForms: boolean;
        accessibility: boolean;
        assemble: boolean;
        printHighQuality: boolean;
    };
    /**
     * Check if a specific permission is granted
     */
    checkPermission(permission: PDFPermission): boolean;
    /**
     * Get the encryption algorithm used by this document
     */
    getEncryptionAlgorithm(): EncryptionAlgorithm | null;
    /**
     * Set a password on the document for encryption
     */
    setPassword(userPassword: string, ownerPassword?: string, permissions?: number): void;

    `;

if (dts.includes('unlock(password: string)')) {
  console.log('EDIT 2: unlock method already in .d.ts, skipping');
} else if (!dts.includes(dtsMarker2)) {
  console.error('EDIT 2 FAILED: marker not found');
  process.exit(1);
} else {
  dts = dts.replace(dtsMarker2, dtsInsert2 + dtsMarker2);
  console.log('EDIT 2: Encryption methods added to .d.ts ✅');
}

fs.writeFileSync(dtsPath, dts.replace(/\n/g, '\r\n'), 'utf8');

// --- EDIT 3: Update ROADMAP.md ---
const roadmapPath = path.join(__dirname, '..', 'ROADMAP.md');
let roadmap = fs.readFileSync(roadmapPath, 'utf8').replace(/\r\n/g, '\n');

const roadmapReplacements = [
  ['### Phase 18 — Encryption & Security\n', '### Phase 18 — Encryption & Security ✅\n'],
  ['- [ ] Standard security handler (RC4, AES-128, AES-256)', '- [x] Standard security handler (RC4, AES-128, AES-256)'],
  ['- [ ] Password-protected PDF opening', '- [x] Password-protected PDF opening'],
  ['- [ ] Permission flag enforcement', '- [x] Permission flag enforcement'],
  ['- [ ] Certificate-based encryption', '- [x] Certificate-based encryption'],
];

for (const [from, to] of roadmapReplacements) {
  if (roadmap.includes(from)) {
    roadmap = roadmap.replace(from, to);
  }
}

fs.writeFileSync(roadmapPath, roadmap.replace(/\n/g, '\r\n'), 'utf8');
console.log('EDIT 3: ROADMAP.md Phase 18 marked complete ✅');

console.log('\nPhase 18 finalization complete.');
