# AgenticPDF — Security Audit Report

> **Audit Date:** 2026-04-01 (Revision 2)  
> **Previous Audit:** 2026-03-14  
> **Version:** 1.0.0  
> **Auditor:** Automated security analysis + manual code review  
> **Classification:** UNCLASSIFIED // FOUO  
> **Applicable Frameworks:** CVE, MITRE ATT&CK, NIST FIPS 140-3, CMMC 2.0 Level 2  
> **Scope:** Core library (`agenticpdf.ts`), CLI (`cli.ts`), Rust/WASM (`agenticpdf-rs/`), dev server (`server.cjs`), website (`website/`)

---

## Executive Summary

AgenticPDF is a zero-dependency, single-file TypeScript PDF processing library with a companion Rust CLI/WASM module and Next.js documentation website. This revision updates the 2026-03-14 audit with new findings from expanded scope (CLI file operations, dev server, website CSP, regex safety, PRNG usage) and verifies all previously identified controls.

**Overall Risk Rating: LOW-MEDIUM** — Zero-dependency core architecture eliminates supply chain risk. All parsing operations include bounds checking, recursion limits, and size constraints. New findings identify path traversal weaknesses in CLI output handling (CWE-22), missing security headers in the dev server (CWE-693), use of `Math.random()` for ID generation (CWE-338), and a user-controlled regex injection surface (CWE-1333). None are exploitable in the core library's default configuration.

### Changes Since Previous Audit

| Area                 | Change                                                                    |
| -------------------- | ------------------------------------------------------------------------- |
| Test suite           | 924 tests across 24 suites (was 560 tests, 16 suites)                     |
| Website dependencies | Added `shiki` ^4.0.2 for syntax highlighting                              |
| Scope expansion      | CLI file I/O, dev server, website CSP, regex patterns now audited         |
| New findings         | 5 MEDIUM, 2 LOW (see §1.3)                                                |
| Resolved             | All prior recommendations unchanged; test coverage significantly improved |

---

## 1. CVE Vulnerability Assessment

### 1.1 Known PDF Parsing CVEs

The following CVE categories are relevant to PDF parsers. Each is assessed against AgenticPDF's implementation.

| CVE Category                                       | Risk | Mitigation Status                                                                                                                                                             |
| -------------------------------------------------- | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Buffer overflow in stream decompression**        | LOW  | Custom `Inflate` class with bounds-checked operations. No raw pointer arithmetic. TypeScript's `Uint8Array` prevents out-of-bounds writes. Rust `miniz_oxide` is memory-safe. |
| **Integer overflow in xref parsing**               | LOW  | `MAX_XREF_SUBSECTIONS` (1000) limit. Count values validated (`< 0 \| > 100000` rejected). Position advancement verified per iteration.                                        |
| **Infinite loop in object parsing**                | LOW  | Safety counters on all loops: `MAX_DICT_ENTRIES` (10000), `MAX_NAME_LENGTH` (10000), `MAX_XREF_SUBSECTIONS` (1000). Position-not-advancing detection.                         |
| **Stack overflow via recursive objects**           | LOW  | Recursion depth limited in `resolveReference()` and form field parent chain walking. `MAX_RECURSION_DEPTH` = 64 in Rust.                                                      |
| **JavaScript execution in PDF**                    | NONE | No JavaScript evaluation engine. `/JS` and `/JavaScript` actions are parsed as data only, never executed. `SecurityConfig.allowJavaScript = false` blocks JS-containing PDFs. |
| **Embedded file extraction**                       | LOW  | File attachments parsed as metadata. No automatic file extraction to disk. `validateSecurityConstraints()` detects `/EF` entries.                                             |
| **JBIG2 decoder vulnerabilities** (CVE-2021-30860) | NONE | JBIG2 data is passed through without decoding. Browser-level rendering only. No custom JBIG2 decoder.                                                                         |
| **Heap corruption in font parsing**                | LOW  | Font data used as lookup tables only. No TrueType/OpenType instruction VM. No glyph outline processing.                                                                       |

### 1.2 AgenticPDF-Specific Attack Surfaces

| Surface              | Description                                  | Mitigation                                                                |
| -------------------- | -------------------------------------------- | ------------------------------------------------------------------------- |
| `parseXRefTable()`   | Xref entry parsing with subsection iteration | Safety counter, position advancement check, count bounds                  |
| `parseDictionary()`  | Dictionary key-value pair parsing            | 10000 entry limit, position advancement check                             |
| `Inflate.inflate()`  | FlateDecode decompression                    | Fixed output buffer allocation, bounds-checked reads                      |
| `parseFormField()`   | Form field parent chain walking              | Depth-limited recursion                                                   |
| `parseInlineImage()` | Inline image data extraction                 | `findInlineImageEnd()` with bounded search                                |
| `CCITTFaxDecoder`    | CCITT Group 3/4 bi-level decoding            | Fixed output buffer size from declared dimensions                         |
| `searchText()`       | User-supplied regex patterns                 | **NEW:** Try-catch wrapper, but no ReDoS complexity guard (see §1.3)      |
| CLI `--output`       | File write to user-specified path            | **NEW:** Partial path validation; `startsWith` bypass possible (see §1.3) |

### 1.3 New Findings (This Revision)

| ID        | CWE      | Severity   | Location                                     | Description                                                                                                                                                                                                                    |
| --------- | -------- | ---------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **F-001** | CWE-22   | **MEDIUM** | `cli.ts:1290-1297`                           | Path traversal in image extraction — `startsWith(outputDir)` without trailing `path.sep` allows writes to sibling directories (e.g., `/data/images-secret/` passes check for `/data/images`).                                  |
| **F-002** | CWE-22   | **MEDIUM** | `cli.ts:531,581,624,681,748,1174,1236,1346`  | Path traversal in text/JSON/HTML output — 8 `writeFileSync()` calls accept `options.output` with no path normalization or directory confinement. CLI user can specify `../../etc/crontab`.                                     |
| **F-003** | CWE-693  | **MEDIUM** | `server.cjs:49`                              | Dev server missing security headers — No `X-Content-Type-Options`, `X-Frame-Options`, `Content-Security-Policy`, or `Referrer-Policy` headers on responses.                                                                    |
| **F-004** | CWE-693  | **MEDIUM** | `website/next.config.mjs`                    | Website has no CSP headers configured — No middleware or headers config for Content-Security-Policy. Static export relies on hosting provider headers.                                                                         |
| **F-005** | CWE-338  | **MEDIUM** | `agenticpdf.ts:5119,19072,19094,19117,19139` | `Math.random()` used for session and annotation IDs — 5 call sites use `Date.now() + Math.random()` for ID generation. While not used for security tokens, IDs are predictable and could collide.                              |
| **F-006** | CWE-1333 | **LOW**    | `agenticpdf.ts:13242`                        | ReDoS via user-controlled regex — `searchText()` with `options.regex=true` passes user input directly to `new RegExp()`. A try-catch handles syntax errors, but no complexity/length guard prevents catastrophic backtracking. |
| **F-007** | CWE-1333 | **LOW**    | `agenticpdf.ts:20754`                        | Unescaped PDF content in regex — `firstName` from parsed author metadata interpolated into `new RegExp()` without escaping. Malicious author names with regex metacharacters could cause unexpected matching or ReDoS.         |

### 1.4 Recommended Fixes

**F-001 / F-002 — CLI Path Traversal (CWE-22)**:
```typescript
// Fix for image extraction (F-001):
const resolvedDir = path.resolve(outputDir) + path.sep;
const resolvedPath = path.resolve(filepath);
if (!resolvedPath.startsWith(resolvedDir)) {
    throw new Error(`Invalid output path detected: ${filepath}`);
}

// Fix for all writeFileSync calls (F-002):
function validateOutputPath(outputPath: string): string {
    const resolved = path.resolve(outputPath);
    const cwd = path.resolve(process.cwd());
    if (!resolved.startsWith(cwd + path.sep) && resolved !== cwd) {
        throw new Error('Output path must be within current working directory');
    }
    return resolved;
}
```

**F-003 — Server Headers (CWE-693)**:
```javascript
res.writeHead(200, {
    'Content-Type': contentType,
    'X-Content-Type-Options': 'nosniff',
    'X-Frame-Options': 'SAMEORIGIN',
    'X-XSS-Protection': '1; mode=block',
    'Referrer-Policy': 'strict-origin-when-cross-origin'
});
```

**F-005 — PRNG for IDs (CWE-338)**:
```typescript
// Replace Math.random() with crypto:
import { randomBytes } from 'crypto';
const id = `session_${Date.now()}_${randomBytes(4).toString('hex')}`;
// Or in browser: crypto.getRandomValues(new Uint8Array(4))
```

**F-006 — ReDoS Guard (CWE-1333)**:
```typescript
// Add length and complexity guard before RegExp construction:
if (query.length > 200) throw new Error('Regex pattern too long');
if (/(\.\*){3,}|(\(.*\)){5,}/.test(query)) throw new Error('Regex too complex');
```

**F-007 — Escape PDF Content Before Regex Use**:
```typescript
const escapedName = firstName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const orcidPattern = new RegExp(escapedName + '[\\s\\S]{0,200}(\\d{4}-\\d{4}-\\d{4}-\\d{3}[\\dX])', 'i');
```

---

## 2. MITRE ATT&CK Mapping

### 2.1 Relevant Techniques

| ATT&CK ID     | Technique                          | Applicability   | Assessment                                                                                                                                                                                 |
| ------------- | ---------------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **T1203**     | Exploitation for Client Execution  | MEDIUM          | PDF parsers are a known exploitation target. AgenticPDF has no execution engine — all content is extracted as data, never executed. No spawning of child processes.                        |
| **T1204.002** | User Execution: Malicious File     | MEDIUM          | Users may open crafted PDFs. Mitigated by `validateSecurityConstraints()` pre-check, size limits, and JavaScript detection.                                                                |
| **T1059**     | Command and Scripting Interpreter  | NONE            | No script execution capability. JavaScript in PDFs is parsed as an opaque string, never evaluated.                                                                                         |
| **T1566.001** | Phishing: Spearphishing Attachment | LOW             | PDF is a common phishing vector. Library does not handle file download or email integration — that is the caller's responsibility.                                                         |
| **T1027**     | Obfuscated Files or Information    | LOW             | PDF streams are decompressed (FlateDecode) for text extraction. Obfuscated content is made readable, not hidden.                                                                           |
| **T1005**     | Data from Local System             | NONE            | Library does not access the file system except through explicit caller-provided paths. No directory enumeration or file scanning.                                                          |
| **T1020**     | Automated Exfiltration             | NONE            | No network capability. Library does not make HTTP requests, open sockets, or send data externally.                                                                                         |
| **T1071**     | Application Layer Protocol         | NONE            | No network layer implemented. `fromUrl()` uses caller's `fetch()` — network policy enforcement is the caller's responsibility.                                                             |
| **T1499.004** | Application or System DoS: Regex   | **LOW** *(NEW)* | `searchText()` accepts user-supplied regex without complexity limits. Crafted patterns with catastrophic backtracking could cause CPU exhaustion. Mitigated by application-level timeouts. |
| **T1083**     | File and Directory Discovery       | **LOW** *(NEW)* | CLI accepts user-specified output paths. Path traversal in F-001/F-002 could allow writing outside intended directories. Not exploitable remotely — requires local CLI access.             |

### 2.2 Defensive Coverage

| ATT&CK Defense                | Implementation                                                                                                                                                                                                                                  |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Input Validation**          | `validateSecurityConstraints()` checks file size, header, JavaScript presence, embedded files                                                                                                                                                   |
| **Execution Prevention**      | No eval(), no Function constructor, no dynamic code execution                                                                                                                                                                                   |
| **Memory Protection**         | TypeScript managed memory, no raw pointers, bounds-checked arrays                                                                                                                                                                               |
| **Process Isolation**         | Optional Web Worker support via `useWebWorkers` configuration                                                                                                                                                                                   |
| **SSRF Protection** *(NEW)*   | `isPrivateHost()` blocks localhost, private RFC 1918 ranges, link-local, and cloud metadata endpoints (169.254.169.254, metadata.google.internal). Protocol restricted to http/https. Single-hop redirect limit with re-validation at each hop. |
| **Path Traversal Prevention** | `server.cjs` uses `path.resolve() + path.sep` correctly. CLI has partial protection (needs F-001/F-002 fixes).                                                                                                                                  |

---

## 3. NIST FIPS 140-3 Compliance Review

### 3.1 Cryptographic Module Assessment

AgenticPDF v1.0.0 does **not** implement cryptographic operations. PDF encryption (RC4, AES-128, AES-256) is a planned Phase 17 feature.

| FIPS 140-3 Level | Requirement                | Status                                |
| ---------------- | -------------------------- | ------------------------------------- |
| **Level 1**      | Approved algorithms only   | N/A — No crypto module present        |
| **Level 1**      | No self-modifying code     | ✅ COMPLIANT — Static code, no eval    |
| **Level 1**      | Module boundary definition | ✅ Single-file boundary with typed API |
| **Level 2**      | Role-based authentication  | N/A — Library, not a service          |
| **Level 2**      | Tamper evidence            | ✅ Integrity verifiable via checksums  |
| **Level 3**      | Key management             | N/A — No key material handled         |

### 3.2 Recommendations for Future Crypto Module (Phase 17)

When implementing PDF encryption support:

1. **Use Web Crypto API** (`crypto.subtle`) for AES-128/AES-256 — FIPS 140-2/3 validated in most browsers and Node.js
2. **Reject RC4** in FIPS mode — RC4 is not approved per NIST SP 800-131A Rev 2
3. **Use approved PRNG** — `crypto.getRandomValues()` for key derivation salts
4. **Key zeroization** — Clear decryption keys from memory after use via `TypedArray.fill(0)`
5. **Algorithm agility** — Support algorithm negotiation based on PDF version and security handler

### 3.3 FIPS-Related Code Practices (Current)

| Practice                  | Status                                                                                                                                                                                                    |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| No custom hashing         | ✅ No MD5/SHA used (would need FIPS-approved implementations)                                                                                                                                              |
| No key derivation         | ✅ No PBKDF2/scrypt/Argon2                                                                                                                                                                                 |
| Random number generation  | ⚠️ **UPDATED** — 5 call sites use `Math.random()` for session/annotation IDs (F-005). Non-cryptographic use, but should migrate to `crypto.getRandomValues()` for FIPS alignment and collision resistance. |
| No certificate validation | ✅ Not applicable until signature support (Phase 19)                                                                                                                                                       |

---

## 4. CMMC 2.0 Level 2 Compliance Checklist

CMMC 2.0 Level 2 maps to NIST SP 800-171 Rev 2 with 110 security controls across 14 domains. Below are the controls applicable to a PDF processing library component.

### 4.1 Access Control (AC)

| Control      | Requirement                                    | Status                                                                                  |
| ------------ | ---------------------------------------------- | --------------------------------------------------------------------------------------- |
| AC.L2-3.1.1  | Limit system access to authorized users        | ✅ Library does not manage users — caller enforces access                                |
| AC.L2-3.1.2  | Limit system access to authorized transactions | ✅ All operations explicitly invoked by caller                                           |
| AC.L2-3.1.3  | Control CUI flow                               | ✅ No data exfiltration capability. No network access. SSRF protection on `fromUrl()`.   |
| AC.L2-3.1.5  | Least privilege                                | ✅ No elevated permissions required. No filesystem access beyond caller-provided buffers |
| AC.L2-3.1.19 | Encrypt CUI on mobile devices                  | N/A — Library component                                                                 |

### 4.2 Audit & Accountability (AU)

| Control     | Requirement                       | Status                                                                                                           |
| ----------- | --------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| AU.L2-3.3.1 | Create and retain audit logs      | ⚠️ PARTIAL — `PerformanceMonitor` tracks operations. No persistent audit log. Caller should log tool invocations. |
| AU.L2-3.3.2 | Trace actions to individual users | N/A — Library does not manage user identity                                                                      |
| AU.L2-3.3.4 | Alert on audit process failure    | N/A — No audit daemon                                                                                            |

**Recommendation:** Integrators should log all `AgenticPDF` method calls (especially `fromFile`, `extractText`, `getFormData`, `exportAs`) in their audit trail per NIST SP 800-171 AU requirements.

### 4.3 Identification & Authentication (IA)

| Control     | Requirement                 | Status                                       |
| ----------- | --------------------------- | -------------------------------------------- |
| IA.L2-3.5.1 | Identify system users       | N/A — Library component                      |
| IA.L2-3.5.2 | Authenticate users          | N/A — Library does not handle authentication |
| IA.L2-3.5.3 | Multi-factor authentication | N/A                                          |

### 4.4 System & Communications Protection (SC)

| Control       | Requirement                                 | Status                                                                                                                    |
| ------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| SC.L2-3.13.1  | Monitor & control communications            | ✅ No outbound communications. `fromUrl()` SSRF-protected (protocol whitelist, private IP blocking, single-hop redirects). |
| SC.L2-3.13.2  | Architectural designs with defense-in-depth | ✅ Layered validation: header check → xref validation → object bounds → stream limits                                      |
| SC.L2-3.13.5  | Implement subnetworks for CUI               | N/A — Library, not a network service                                                                                      |
| SC.L2-3.13.8  | Implement cryptographic mechanisms          | ⚠️ FUTURE — Crypto planned for Phase 17                                                                                    |
| SC.L2-3.13.11 | Employ FIPS-validated cryptography          | ⚠️ FUTURE — Will use Web Crypto API when implemented                                                                       |

### 4.5 System & Information Integrity (SI)

| Control      | Requirement                            | Status                                                                                                   |
| ------------ | -------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| SI.L2-3.14.1 | Identify and remediate flaws           | ✅ **UPDATED** — Active test suite (924 tests, 24 suites). Continuous validation.                         |
| SI.L2-3.14.2 | Provide protection from malicious code | ✅ No code execution from PDFs. JavaScript actions blocked. No eval().                                    |
| SI.L2-3.14.3 | Monitor security alerts                | ⚠️ PARTIAL — `validateSecurityConstraints()` returns violations array. Caller should integrate with SIEM. |
| SI.L2-3.14.6 | Monitor system security                | ✅ `PerformanceMonitor` class tracks all operations                                                       |
| SI.L2-3.14.7 | Identify unauthorized use              | N/A — Library does not manage sessions                                                                   |

### 4.6 Supply Chain Risk Management

| Aspect                           | Status                                                                                                                    |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| **Runtime dependencies**         | ✅ **ZERO** — Single-file architecture, no `node_modules` at runtime                                                       |
| **Build dependencies**           | Jest, TypeScript, ESLint (dev only, not shipped)                                                                          |
| **Website dependencies** *(NEW)* | `next` ^15.3.0, `react` ^19.1.0, `shiki` ^4.0.2, `tailwindcss` ^4.1.0 — all current, no known CVEs                        |
| **Rust dependencies** *(NEW)*    | `wasm-bindgen` 0.2, `serde` 1.0, `serde_json` 1.0, `miniz_oxide` 0.8, `clap` 4.0 — all stable, widely used, no known CVEs |
| **SBOM generation**              | ✅ `AgenticPDF.generateSBOM()` — CycloneDX format                                                                          |
| **Provenance**                   | ✅ Source at `github.com/nervosys/AgenticPDF`                                                                              |
| **Code signing**                 | ⚠️ RECOMMENDED — npm package signing via `npm publish --provenance`                                                        |
| **Vulnerability scanning**       | ⚠️ RECOMMENDED — Integrate `npm audit` and `cargo audit` in CI/CD                                                          |
| **Rust crate audit**             | ✅ All crates widely used, pure Rust, no unsafe code                                                                       |

---

## 5. SSRF Protection Assessment *(NEW Section)*

The `fromUrl()` / `loadFromUrl()` implementation at `agenticpdf.ts:1334-1380` provides comprehensive SSRF protection:

### 5.1 Protocol Validation
- Only `http:` and `https:` protocols accepted
- Blocks `file://`, `ftp://`, `gopher://`, `data:`, and all other schemes

### 5.2 Private IP Blocking (`isPrivateHost()` at lines 1310-1327)
| Range                                    | Blocked |
| ---------------------------------------- | ------- |
| `localhost`, `127.0.0.1`, `::1`, `[::1]` | ✅       |
| `10.0.0.0/8`                             | ✅       |
| `172.16.0.0/12`                          | ✅       |
| `192.168.0.0/16`                         | ✅       |
| `127.0.0.0/8`                            | ✅       |
| `0.0.0.0/8`                              | ✅       |
| `169.254.0.0/16` (link-local)            | ✅       |
| `169.254.169.254` (AWS/GCP metadata)     | ✅       |
| `metadata.google.internal`               | ✅       |

### 5.3 Redirect Handling
- `redirect: 'manual'` prevents automatic following
- Redirect targets re-validated for protocol and private IP
- **Single-hop limit** — no redirect chaining
- Missing or invalid `Location` headers rejected

### 5.4 Known Limitation
DNS rebinding attacks (initial DNS lookup → public IP, subsequent → private IP) are not mitigated. This is acceptable as it is primarily a browser-layer concern and standard for library-level SSRF protection.

**Assessment: EXCELLENT** — Covers OWASP SSRF prevention checklist.

---

## 6. Input Validation Summary

### 6.1 Core Library Protections

| Protection                 | Location                                | Limit                    |
| -------------------------- | --------------------------------------- | ------------------------ |
| Xref subsection count      | `parseXRefTable()`                      | 1,000 subsections        |
| Xref entry count bounds    | `parseXRefTable()`                      | 0–100,000 per subsection |
| Dictionary entry count     | `parseDictionary()`                     | 10,000 entries           |
| Name token length          | `parseName()`                           | 10,000 characters        |
| Position advancement check | `parseXRefTable()`, `parseDictionary()` | Breaks on no-advance     |
| Recursion depth            | `resolveReference()`, form fields       | Limited depth            |
| Performance metrics buffer | `PerformanceMonitor`                    | 1,000 entries            |
| Stream decompression size  | `MAX_STREAM_SIZE`                       | 256 MB                   |

### 6.2 Security Framework Protections (Phase 15)

| Protection              | Method                          | Configuration             |
| ----------------------- | ------------------------------- | ------------------------- |
| File size validation    | `validateSecurityConstraints()` | `maxFileSize`: 500 MB     |
| PDF header validation   | `validateSecurityConstraints()` | `%PDF-` required          |
| JavaScript detection    | `validateSecurityConstraints()` | Blocked by default        |
| Embedded file detection | `validateSecurityConstraints()` | Flagged as violation      |
| SBOM generation         | `generateSBOM()`                | CycloneDX 1.5 format      |
| Security configuration  | `getSecurityConfig()`           | `DEFAULT_SECURITY_CONFIG` |

### 6.3 CLI Input Validation *(NEW)*

| Protection                       | Location           | Status                                            |
| -------------------------------- | ------------------ | ------------------------------------------------- |
| Input sanitization               | `cli.ts:31-41`     | ✅ HTML stripped, length limited                   |
| Output format whitelist          | `cli.ts:148`       | ✅ Only allowed formats accepted                   |
| Chunk size bounds                | `cli.ts:173-180`   | ✅ MIN=100, MAX=10000                              |
| File extension validation        | `cli.ts:1284-1286` | ✅ MIME-based extension mapping                    |
| Image output path validation     | `cli.ts:1290-1297` | ⚠️ Incomplete — missing `path.sep` (F-001)         |
| Text/JSON output path validation | `cli.ts:531+`      | ⚠️ Missing — no validation on 8 call sites (F-002) |
| No command injection             | Full file          | ✅ No `exec()`, `spawn()`, or `child_process`      |

---

## 7. Memory Safety Assessment

### 7.1 TypeScript (Primary Implementation)

| Category              | Assessment                                                                                |
| --------------------- | ----------------------------------------------------------------------------------------- |
| **Buffer overflows**  | ✅ SAFE — `Uint8Array` and `DataView` enforce bounds. No raw memory access.                |
| **Integer overflows** | ✅ SAFE — JavaScript `Number` is IEEE 754 double. No integer wraparound for values < 2^53. |
| **Use-after-free**    | ✅ SAFE — Garbage-collected runtime. `close()` nullifies references.                       |
| **Double-free**       | ✅ SAFE — Not applicable to managed memory.                                                |
| **Dangling pointers** | ✅ SAFE — No raw pointers in TypeScript.                                                   |
| **Stack overflow**    | ⚠️ MITIGATED — Recursion depth limits prevent unlimited stack growth.                      |

### 7.2 Rust (CLI & WASM)

| Category              | Assessment                                                            |
| --------------------- | --------------------------------------------------------------------- |
| **Buffer overflows**  | ✅ SAFE — Slice bounds checking. No `unsafe` blocks.                   |
| **Integer overflows** | ✅ SAFE — Debug builds panic on overflow. Release uses `wrapping_add`. |
| **Use-after-free**    | ✅ SAFE — Ownership system prevents use-after-free at compile time.    |
| **Memory leaks**      | ✅ SAFE — RAII ensures cleanup. No `mem::forget` usage.                |
| **Data races**        | ✅ SAFE — No shared mutable state. No `unsafe` concurrency.            |
| **Stream size DoS**   | ✅ MITIGATED — `MAX_STREAM_SIZE` = 256 MB limit before decompression.  |

---

## 8. Dev Server & Website Security *(NEW Section)*

### 8.1 Dev Server (`server.cjs`)

| Check             | Status            | Notes                                                                     |
| ----------------- | ----------------- | ------------------------------------------------------------------------- |
| Path traversal    | ✅ SAFE            | `path.resolve()` + `path.sep` suffix — correct implementation             |
| Security headers  | ⚠️ MISSING (F-003) | No `X-Content-Type-Options`, `X-Frame-Options`, CSP, or `Referrer-Policy` |
| CORS              | ✅ SAFE            | No permissive CORS headers set                                            |
| Open redirects    | ✅ SAFE            | No redirect logic present                                                 |
| Error disclosure  | ✅ SAFE            | Generic error messages (403, 404, 500) — no stack traces or paths leaked  |
| Directory listing | ✅ SAFE            | No directory listing capability                                           |

**Note:** `server.cjs` is a development-only static file server, not intended for production. The missing headers finding (F-003) is LOW risk in practice.

### 8.2 Website (`website/`)

| Check                     | Status                     | Notes                                                                                                                                                                     |
| ------------------------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CSP headers               | ⚠️ MISSING (F-004)          | No `Content-Security-Policy` in Next.js config or middleware                                                                                                              |
| `dangerouslySetInnerHTML` | ✅ SAFE (context-dependent) | Used in `ui.tsx:38` for Shiki syntax highlighting. Content is server-rendered from hardcoded code strings, NOT user input. **Risk is effectively NONE in current usage.** |
| Dependencies              | ✅ SAFE                     | All packages current: Next.js 15.3, React 19.1, Shiki 4.0, Tailwind 4.1                                                                                                   |
| Static export             | ✅ SAFE                     | `output: 'export'` — no server-side routes, no API endpoints, no dynamic content                                                                                          |

---

## 9. Dependency Audit Summary *(NEW Section)*

### 9.1 Rust Crates (`agenticpdf-rs/Cargo.toml`)

| Crate               | Version           | Status     | Notes                                |
| ------------------- | ----------------- | ---------- | ------------------------------------ |
| `wasm-bindgen`      | 0.2               | ✅ SAFE     | Standard WASM binding layer          |
| `serde`             | 1.0 (with derive) | ✅ SAFE     | De facto Rust serialization standard |
| `serde_json`        | 1.0               | ✅ SAFE     | Standard JSON library                |
| `miniz_oxide`       | 0.8               | ✅ SAFE     | Pure Rust deflate, no unsafe         |
| `clap`              | 4.0 (with derive) | ✅ SAFE     | Standard CLI parser                  |
| `wasm-bindgen-test` | 0.3               | ✅ DEV ONLY | Testing harness                      |

### 9.2 Website npm Packages (`website/package.json`)

| Package                                | Version | Status    |
| -------------------------------------- | ------- | --------- |
| `next`                                 | ^15.3.0 | ✅ Current |
| `react` / `react-dom`                  | ^19.1.0 | ✅ Current |
| `shiki`                                | ^4.0.2  | ✅ Current |
| `tailwindcss` / `@tailwindcss/postcss` | ^4.1.0  | ✅ Current |
| `typescript`                           | ^5.8.0  | ✅ Current |

### 9.3 Root Dev Dependencies (`package.json`)

| Package            | Status                  |
| ------------------ | ----------------------- |
| `@opentelemetry/*` | ✅ Latest stable         |
| `tsx` ^4.20.5      | ✅ Latest                |
| `jest` / `ts-jest` | ✅ Dev only, not shipped |

**No known CVEs detected across any dependency.**

---

## 10. Recommendations

### 10.1 Immediate (Priority: HIGH)

1. **Fix CLI path traversal (F-001, F-002)** — Apply `path.sep`-suffixed `startsWith` check to image extraction. Add `validateOutputPath()` to all 8 `writeFileSync` calls.
2. **Add security headers to dev server (F-003)** — Set `X-Content-Type-Options: nosniff`, `X-Frame-Options: SAMEORIGIN`. Low urgency since dev-only.
3. **Replace `Math.random()` with `crypto` (F-005)** — Migrate 5 call sites to `crypto.getRandomValues()` or `crypto.randomBytes()`.

### 10.2 Short-Term (Priority: MEDIUM)

4. **Add ReDoS guard to `searchText()` (F-006)** — Limit regex pattern length (200 chars) and reject patterns with known catastrophic backtracking patterns.
5. **Escape PDF-sourced strings before regex interpolation (F-007)** — Apply `replace(/[.*+?^${}()|[\]\\]/g, '\\$&')` to `firstName` before `new RegExp()`.
6. **Configure CSP for website (F-004)** — If deploying beyond static hosting, add CSP via Next.js middleware or hosting provider headers.
7. **Add `npm audit` and `cargo audit` to CI/CD pipeline.**
8. **Implement npm provenance** signing for published packages.

### 10.3 Long-Term (Priority: LOW)

9. **Add FIPS-validated crypto** when implementing Phase 17 (use `crypto.subtle`).
10. **Pursue FIPS 140-3 Level 1 validation** for the cryptographic module.
11. **Obtain CMMC Level 2 assessment** from an authorized C3PAO.
12. **Implement persistent audit logging** (syslog/structured JSON) for SI/AU controls.
13. **Add fuzzing** via `cargo-fuzz` for the Rust parser and `jsfuzz` for TypeScript.
14. **Add rate limiting** to streaming APIs for DoS prevention.

---

## 11. Conclusion

AgenticPDF presents a **low-medium risk profile** for deployment in DoD and regulated environments:

- **Zero runtime dependencies** eliminate supply chain attack vectors
- **No code execution** from parsed PDFs prevents exploitation via T1203/T1059
- **Comprehensive input validation** with safety counters, bounds checks, and size limits
- **TypeScript + Rust** provide memory safety guarantees without `unsafe` code
- **Excellent SSRF protection** with private IP blocking, protocol whitelisting, and single-hop redirects
- **SBOM generation** via `generateSBOM()` supports CMMC 2.0 supply chain requirements
- **924 tests across 24 suites** provide continuous regression coverage

The upgrade from LOW to LOW-MEDIUM reflects expanded audit scope revealing CLI path traversal weaknesses (F-001/F-002) and missing security headers (F-003/F-004), none of which affect the core library. The primary remaining gap is cryptographic module support (planned Phase 17), which will require FIPS-validated algorithm selection when implemented.

---

*Revision History:*
- *2026-04-01 (Rev 2): Expanded scope to CLI, dev server, website, SSRF, regex, PRNG. Added findings F-001 through F-007. Updated test counts, dependency audit, CMMC/NIST sections.*
- *2026-03-14 (Rev 1): Initial audit — CVE, MITRE ATT&CK, NIST FIPS 140-3, CMMC 2.0 Level 2.*

*This document should be reviewed and updated with each major release. For questions regarding DoD deployment, contact the maintaining organization (NERVOSYS, LLC) for a formal security assessment.*
