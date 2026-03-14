# Security Patches for AgenticPDF CLI

## Critical Security Fixes

This document contains patches for the two critical security vulnerabilities found in the code review.

---

## Patch 1: Fix Command Injection (cli.js)

### Issue
Using `shell: true` in `spawn()` can lead to command injection attacks.

### Location
`cli.js` line 29

### Current Code
```javascript
const child = spawn('npx', ['--yes', 'tsx', cliPath, ...args], {
    stdio: 'inherit',
    shell: true,  // ⚠️ SECURITY RISK
    env: process.env
});
```

### Fixed Code
```javascript
const child = spawn('npx', ['--yes', 'tsx', cliPath, ...args], {
    stdio: 'inherit',
    shell: false,  // ✅ Safe - no shell interpretation
    env: process.env
});
```

### Impact
- **Severity:** Critical
- **Risk:** Command injection if malicious arguments are passed
- **Fix Complexity:** Trivial (1 line change)

---

## Patch 2: Fix Path Traversal (cli.ts)

### Issue
No validation of file paths in image extraction, allowing potential path traversal attacks.

### Location
`cli.ts` lines 558-585 (commandImages function)

### Current Code
```typescript
async function commandImages(options: CLIOptions): Promise<void> {
    const pdf = await loadPDF(options.input!, options);

    try {
        log.header('🖼️  Extracting Images');

        const images = await pdf.extractImages();

        if (images.length === 0) {
            log.warning('No images found in PDF');
            return;
        }

        const outputDir = options.output || './images';

        if (!fs.existsSync(outputDir)) {
            fs.mkdirSync(outputDir, { recursive: true });
        }

        if (options.verbose) {
            log.info(`Found ${images.length} images`);
        }

        for (let i = 0; i < images.length; i++) {
            const image = images[i];
            const ext = image.mimeType?.split('/')[1] || 'png';  // ⚠️ SECURITY RISK
            const outputPath = path.join(outputDir, `image_${i + 1}.${ext}`);

            if (image.data) {
                const buffer = Buffer.from(image.data);
                fs.writeFileSync(outputPath, buffer);

                if (options.verbose) {
                    log.success(`Saved: ${outputPath}`);
                }
            }
        }

        log.success(`Extracted ${images.length} images to: ${outputDir}`);
    } finally {
        pdf.close();
    }
}
```

### Fixed Code
```typescript
async function commandImages(options: CLIOptions): Promise<void> {
    const pdf = await loadPDF(options.input!, options);

    try {
        log.header('🖼️  Extracting Images');

        const images = await pdf.extractImages();

        if (images.length === 0) {
            log.warning('No images found in PDF');
            return;
        }

        // ✅ SECURITY: Resolve to absolute path to prevent traversal
        const outputDir = path.resolve(options.output || './images');
        
        // ✅ SECURITY: Validate output directory is not sensitive location
        const sensitivePathsRegex = /(etc|windows|system32|program files)/i;
        if (sensitivePathsRegex.test(outputDir)) {
            throw new Error('Cannot write to system directories');
        }

        if (!fs.existsSync(outputDir)) {
            fs.mkdirSync(outputDir, { recursive: true });
        }

        if (options.verbose) {
            log.info(`Found ${images.length} images`);
        }

        // ✅ SECURITY: Whitelist of allowed image extensions
        const ALLOWED_IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'tiff'];

        for (let i = 0; i < images.length; i++) {
            const image = images[i];
            
            // ✅ SECURITY: Sanitize file extension
            const rawExt = image.mimeType?.split('/')[1] || 'png';
            const safeExt = rawExt.replace(/[^a-z0-9]/gi, '').toLowerCase();
            
            // ✅ SECURITY: Validate extension is allowed
            const ext = ALLOWED_IMAGE_EXTENSIONS.includes(safeExt) ? safeExt : 'png';
            
            // ✅ SECURITY: Use safe filename
            const safeName = `image_${i + 1}.${ext}`;
            const outputPath = path.join(outputDir, safeName);
            
            // ✅ SECURITY: Ensure output path is within intended directory
            const resolvedPath = path.resolve(outputPath);
            if (!resolvedPath.startsWith(outputDir)) {
                throw new Error(`Invalid output path detected: ${outputPath}`);
            }

            if (image.data) {
                const buffer = Buffer.from(image.data);
                fs.writeFileSync(resolvedPath, buffer);

                if (options.verbose) {
                    log.success(`Saved: ${resolvedPath}`);
                }
            }
        }

        log.success(`Extracted ${images.length} images to: ${outputDir}`);
    } finally {
        pdf.close();
    }
}
```

### Impact
- **Severity:** Critical
- **Risk:** Path traversal allowing writes to arbitrary filesystem locations
- **Fix Complexity:** Medium (adds validation layer)

---

## Patch 3: Add Input Validation (cli.ts)

### Issue
Missing validation for numeric inputs and missing argument checks.

### Location
`cli.ts` lines 65-150 (parseArgs function)

### Current Code (Excerpt)
```typescript
case '--chunk-size':
    options.chunkSize = parseInt(nextArg, 10);  // ⚠️ No validation
    i++;
    break;
```

### Fixed Code
```typescript
// Add constants at top of file
const MAX_CHUNK_SIZE = 10000;
const MIN_CHUNK_SIZE = 100;
const DEFAULT_CHUNK_SIZE = 1000;

// In parseArgs function
case '--chunk-size':
    if (!nextArg) {
        throw new Error('--chunk-size requires a value');
    }
    const chunkSize = parseInt(nextArg, 10);
    if (isNaN(chunkSize)) {
        throw new Error(`Invalid chunk size: ${nextArg}. Must be a number.`);
    }
    if (chunkSize < MIN_CHUNK_SIZE || chunkSize > MAX_CHUNK_SIZE) {
        throw new Error(
            `Chunk size must be between ${MIN_CHUNK_SIZE} and ${MAX_CHUNK_SIZE}. Got: ${chunkSize}`
        );
    }
    options.chunkSize = chunkSize;
    i++;
    break;

// Similarly for other value flags
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
    const validFormats = ['text', 'json', 'html', 'markdown'];
    if (!validFormats.includes(nextArg.toLowerCase())) {
        throw new Error(
            `Invalid format: ${nextArg}. Must be one of: ${validFormats.join(', ')}`
        );
    }
    options.format = nextArg.toLowerCase();
    i++;
    break;
```

### Impact
- **Severity:** Medium
- **Risk:** Application crashes or unexpected behavior from invalid inputs
- **Fix Complexity:** Medium (adds validation for all input parameters)

---

## Patch 4: Fix Unsafe Type Assertion (cli.ts)

### Issue
Direct casting of Node.js Buffer to ArrayBuffer without proper conversion.

### Location
`cli.ts` line 275 (loadPDF function)

### Current Code
```typescript
const fileBuffer = fs.readFileSync(inputPath);
const pdf = await AgenticPDF.fromBuffer(
    fileBuffer.buffer as ArrayBuffer,  // ⚠️ Unsafe
    { lazyLoad: true, maxMemoryUsage: 200 * 1024 * 1024 }
);
```

### Fixed Code
```typescript
const fileBuffer = fs.readFileSync(inputPath);

// ✅ Safe: Properly convert Buffer to ArrayBuffer
const arrayBuffer = fileBuffer.buffer.slice(
    fileBuffer.byteOffset,
    fileBuffer.byteOffset + fileBuffer.byteLength
);

const pdf = await AgenticPDF.fromBuffer(arrayBuffer, {
    lazyLoad: true,
    maxMemoryUsage: 200 * 1024 * 1024
});
```

### Impact
- **Severity:** Medium
- **Risk:** Potential memory corruption or unexpected behavior
- **Fix Complexity:** Trivial (proper conversion)

---

## Patch 5: Enhanced Error Handling (cli.ts)

### Issue
Missing error handling for file system operations.

### Location
`cli.ts` lines 255-280 (loadPDF function)

### Current Code
```typescript
async function loadPDF(inputPath: string, options: CLIOptions): Promise<AgenticPDF> {
    if (!inputPath) {
        throw new Error('Input file is required. Use -i or --input to specify the PDF file.');
    }

    if (!fs.existsSync(inputPath)) {
        throw new Error(`File not found: ${inputPath}`);
    }

    if (options.verbose) {
        log.info(`Loading PDF: ${inputPath}`);
    }

    const fileBuffer = fs.readFileSync(inputPath);  // ⚠️ Can throw
    // ...
}
```

### Fixed Code
```typescript
async function loadPDF(inputPath: string, options: CLIOptions): Promise<AgenticPDF> {
    if (!inputPath) {
        throw new Error('Input file is required. Use -i or --input to specify the PDF file.');
    }

    // ✅ Resolve to absolute path
    const resolvedPath = path.resolve(inputPath);

    // ✅ Check file existence
    if (!fs.existsSync(resolvedPath)) {
        throw new Error(`File not found: ${inputPath}`);
    }

    // ✅ Verify it's a file (not directory)
    const stats = fs.statSync(resolvedPath);
    if (!stats.isFile()) {
        throw new Error(`Path is not a file: ${inputPath}`);
    }

    // ✅ Check file permissions
    try {
        fs.accessSync(resolvedPath, fs.constants.R_OK);
    } catch (err) {
        throw new Error(`Cannot read file: ${inputPath}. Permission denied.`);
    }

    // ✅ Validate file extension
    const ext = path.extname(resolvedPath).toLowerCase();
    if (ext !== '.pdf') {
        log.warning(`File extension is ${ext}, expected .pdf. Attempting to process anyway...`);
    }

    if (options.verbose) {
        log.info(`Loading PDF: ${resolvedPath}`);
        log.info(`File size: ${(stats.size / 1024 / 1024).toFixed(2)} MB`);
    }

    // ✅ Handle read errors
    let fileBuffer: Buffer;
    try {
        fileBuffer = fs.readFileSync(resolvedPath);
    } catch (err) {
        throw new Error(`Failed to read file: ${inputPath}. ${err instanceof Error ? err.message : 'Unknown error'}`);
    }

    // ✅ Safe Buffer to ArrayBuffer conversion
    const arrayBuffer = fileBuffer.buffer.slice(
        fileBuffer.byteOffset,
        fileBuffer.byteOffset + fileBuffer.byteLength
    );

    // ✅ Handle PDF loading errors
    try {
        const pdf = await AgenticPDF.fromBuffer(arrayBuffer, {
            lazyLoad: true,
            maxMemoryUsage: 200 * 1024 * 1024
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
```

### Impact
- **Severity:** Medium
- **Risk:** Unclear error messages, potential crashes
- **Fix Complexity:** Medium (comprehensive error handling)

---

## Application Instructions

### 1. Apply cli.js patch
```bash
# Edit cli.js line 29
# Change: shell: true
# To:     shell: false
```

### 2. Apply cli.ts patches
```bash
# Apply all patches to cli.ts:
# - commandImages function (lines 558-585)
# - parseArgs function (lines 65-150)
# - loadPDF function (lines 255-280)
```

### 3. Add constants
```typescript
// At top of cli.ts after imports
const MAX_CHUNK_SIZE = 10000;
const MIN_CHUNK_SIZE = 100;
const DEFAULT_CHUNK_SIZE = 1000;
const DEFAULT_MEMORY_LIMIT = 200 * 1024 * 1024; // 200MB
const ALLOWED_IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'tiff'];
const VALID_OUTPUT_FORMATS = ['text', 'json', 'html', 'markdown'];
```

### 4. Test after applying patches
```bash
# Run all tests
npm test

# Run security-focused tests
npm test -- --testNamePattern="security"

# Manual security testing
mpdf images -i test.pdf -o "../../../../etc/passwd"  # Should fail
mpdf chunk -i test.pdf --chunk-size -1               # Should fail
mpdf chunk -i test.pdf --chunk-size 99999999         # Should fail
```

### 5. Update tests
Add security tests to verify fixes work:

```typescript
// tests/integration/cli-security.test.ts
describe('CLI - Security', () => {
    it('should prevent path traversal in output paths', async () => {
        const result = await runCLI([
            'images',
            '-i', 'sample.pdf',
            '-o', '../../../etc/passwd'
        ]);
        expect(result.exitCode).toBe(1);
        expect(result.stderr).toContain('Invalid output path');
    });

    it('should reject invalid chunk sizes', async () => {
        const result = await runCLI([
            'chunk',
            '-i', 'sample.pdf',
            '--chunk-size', '-1'
        ]);
        expect(result.exitCode).toBe(1);
        expect(result.stderr).toContain('Invalid chunk size');
    });

    it('should validate file extensions', async () => {
        const result = await runCLI([
            'convert',
            '-i', 'sample.pdf',
            '-f', 'invalid_format'
        ]);
        expect(result.exitCode).toBe(1);
        expect(result.stderr).toContain('Invalid format');
    });
});
```

---

## Verification Checklist

- [ ] Command injection vulnerability fixed (cli.js)
- [ ] Path traversal vulnerability fixed (cli.ts commandImages)
- [ ] Input validation added (cli.ts parseArgs)
- [ ] Unsafe type assertion fixed (cli.ts loadPDF)
- [ ] Enhanced error handling implemented (cli.ts loadPDF)
- [ ] Constants extracted (magic numbers removed)
- [ ] Security tests added
- [ ] All existing tests still pass
- [ ] Manual security testing completed
- [ ] Code review performed
- [ ] SECURITY.md created with vulnerability reporting process

---

## Post-Patch Actions

1. **Update version**: Bump to v1.0.1 in package.json
2. **Create security advisory**: Document vulnerabilities and fixes
3. **Notify users**: If any users are using pre-release versions
4. **Update CHANGELOG.md**: Document security fixes
5. **Tag release**: `git tag v1.0.1 -m "Security fixes"`
6. **Publish**: `npm publish`

---

**Patches Ready for Application** ✅

*These patches address all critical and medium security vulnerabilities found in the code review.*
