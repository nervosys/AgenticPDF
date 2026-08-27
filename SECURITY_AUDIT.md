# AgenticPDF — Security Audit Report

> **Audit Date:** 2026-08-26 (Revision 3)  
> **Previous Audit:** 2026-04-01  
> **Version:** 1.0.0  
> **Auditor:** Automated security analysis + manual code review  
> **Classification:** UNCLASSIFIED // FOUO  
> **Applicable Frameworks:** CVE, MITRE ATT&CK, NIST FIPS 140-3, CMMC 2.0 Level 2  
> **Scope:** Core library (`agenticpdf.ts`), CLI (`cli.ts`), Rust/WASM (`agenticpdf-rs/`), dev server (`server.cjs`), website (`website/`)

---

## Executive Summary

AgenticPDF is a zero-dependency, single-file TypeScript PDF processing library with a companion Rust CLI/WASM module and Next.js documentation website. This revision updates the 2026-03-14 audit with new findings from expanded scope (CLI file operations, dev server, website CSP, regex safety, PRNG usage) and verifies all previously identified controls.

**Revision 3 rating: LOW.** Two HIGH advisories exist in the dependency graph
and neither is reachable from document input; the crypto surface is larger than
Revisions 1-2 claimed and is now inventoried honestly; the four decoders added
by the render-correctness branch are bounded and adversarially tested. The one
item a reader should not skip is §3.1: **this build is not FIPS 140-3 compliant
and the earlier revisions' statement that it contains no cryptography was
wrong.**

**Revision 2 rating (retained for history): LOW-MEDIUM** — Zero-dependency core architecture eliminates supply chain risk. All parsing operations include bounds checking, recursion limits, and size constraints. New findings identify path traversal weaknesses in CLI output handling (CWE-22), missing security headers in the dev server (CWE-693), use of `Math.random()` for ID generation (CWE-338), and a user-controlled regex injection surface (CWE-1333). None are exploitable in the core library's default configuration.

### Changes Since Previous Audit

Revision 3 covers the render-correctness branch (`fix/security-hardening`, 91
commits). That work added four new decoders that read attacker-controlled bytes,
so the scope grows accordingly, and it is the first revision to run automated
advisory scanning rather than review dependency lists by hand.

| Area                  | Change                                                                                                    |
| --------------------- | --------------------------------------------------------------------------------------------------------- |
| Rust test suite       | 597 crate + 45 integration + 75 reader tests; clippy `-D warnings` and `cargo fmt --check` clean            |
| New parsing surface   | Coons/tensor patch meshes, tiling and shading patterns, inline images, image resampling and polygon masking |
| Advisory scanning     | `cargo audit` and `npm audit` run for the first time; findings triaged by **reachability**, not by count    |
| New findings          | 2 HIGH (transitive, unreachable), 1 MEDIUM (dev supply chain), 2 INFORMATIONAL (see §1.3)                   |
| Memory safety         | Confirmed: **zero `unsafe` in the core library**; all 22 uses confined to the two FFI shims                 |
| Adversarial testing   | Malformed-input tests added for the new decoders, run under debug (overflow checks on) and release          |
| Platform scope        | Android shell built and exercised on an emulator; iOS remains unverified for want of macOS                  |

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

### 1.3a Advisory Scan Results (Revision 3)

Run on 2026-08-26. **The count is not the finding; reachability is.**

**Rust — `cargo audit` over 466 crate dependencies: 2 vulnerabilities, 2 warnings.**

| Advisory              | Crate / Version    | CVSS      | Reachable here?                                                                      |
| --------------------- | ------------------ | --------- | ------------------------------------------------------------------------------------ |
| **RUSTSEC-2026-0194** | `quick-xml` 0.30.0 | 7.5 HIGH  | **No.** Quadratic run time on duplicate attribute names. DoS.                        |
| **RUSTSEC-2026-0195** | `quick-xml` 0.30.0 | 7.5 HIGH  | **No.** Unbounded namespace-declaration allocation. Memory-exhaustion DoS.           |
| RUSTSEC-2024-0436     | `paste` 1.0.15     | —         | Unmaintained. Proc-macro, compile time only.                                          |
| RUSTSEC-2026-0192     | `ttf-parser` 0.25.1| —         | Unmaintained. Not a vulnerability.                                                    |

The two HIGH advisories arrive by this path, established with `Cargo.lock`
rather than assumed:

```
quick-xml 0.30.0 <- zbus_xml <- zbus-lockstep <- atspi-common
                 <- atspi <- accesskit_unix <- accesskit_winit
                 <- egui-winit <- eframe <- apdf-reader
```

`accesskit_unix` is the **Linux AT-SPI accessibility backend**. Three
consequences follow, and all three are what downgrade this from critical to
tracked:

1. **It never sees a document.** This crate's own XML is a hand-written parser
   (`agenticpdf-rs/src/xml.rs`) with no `quick-xml` dependency; `quick-xml` here
   parses D-Bus introspection data, not OOXML, ODF or EPUB. Untrusted document
   input cannot reach it.
2. **It is absent from every shipped artefact except the Linux desktop build.**
   Not in the WASM bundle, the Android or iOS libraries, the CLI on Windows or
   macOS, or the published npm package.
3. **It cannot be fixed here.** The version is pinned transitively through
   `eframe`. Remediation is an upstream `accesskit`/`eframe` bump; a local
   `cargo update` does not move it. Tracked, not patched.

**JavaScript — `npm audit`.**

| Scope                    | critical | high | moderate | low |
| ------------------------ | -------- | ---- | -------- | --- |
| **Production (shipped)** | 0        | 0    | 0        | 0   |
| Development only         | 1        | 12   | 1        | 2   |

The package ships one runtime dependency (`tsx`) and a `files` allowlist, so
none of the sixteen advisories reach a consumer. They are a **build-chain**
concern: `handlebars` (critical, JS injection via AST type confusion),
`undici` (TLS validation bypass via SOCKS5 proxy), `glob` (command injection in
its CLI), `js-yaml`, `minimatch`, `picomatch`, `brace-expansion`, `flatted` and
the `@typescript-eslint` family. These execute on developer and CI machines,
which is exactly the surface CMMC SR and NIST SSDF care about (see §4.6).

### 1.3b New Parsing Surface (Revision 3)

Four decoders were added that read numbers an attacker chooses — counts, bit
widths, step sizes, repeat factors — and loop on them. Each is bounded by a
constant that does not depend on the input being reasonable:

| Decoder                              | Attacker-controlled input                              | Bound                                                     |
| ------------------------------------ | ------------------------------------------------------ | --------------------------------------------------------- |
| Coons/tensor patch mesh (`patches`)  | `BitsPerCoordinate`, `BitsPerComponent`, `BitsPerFlag`, patch count | Widths >32 rejected; a zero-width field ends the stream; `MAX_PATCH_QUADS` 60,000 |
| Tiling pattern (`paint_pattern`)     | `XStep`/`YStep`, `BBox`, nested patterns               | `MAX_TILES` 4,096; `MAX_FORM_DEPTH` recursion limit        |
| Inline image (`take_inline_image_data`) | Sample bytes, dictionary                            | Scan bounded by the stream; operand stack capped at 64     |
| Image placement/masking (`placed`, `cut_to_shapes`) | Placement matrix, clip polygons          | Output clamped to a 4-megapixel budget and 4096 per axis   |

**Evidence, not assertion.** `malformed_meshes_and_patterns_terminate_within_bounds`
drives six mesh configurations — including 64-bit widths, 1-bit widths and the
degenerate all-zero case — with 40 KB of deterministic noise, and
`a_pattern_with_a_vanishing_step_is_capped` asks for a 600×600 fill tiled at a
0.0001-unit step. Both assert only that the work terminates and that the display
list stays bounded. Both pass in **0.23 s under debug**, where Rust's integer
overflow checks are enabled, and 0.08 s under release. A zero `BitsPerFlag` is
the case worth naming: read as "a value of no bits" it loops forever, and it is
read as end-of-stream.

### 1.3 New Findings (Revision 2)

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
| **T1083**     | File and Directory Discovery       | **LOW**         | CLI accepts user-specified output paths. Path traversal in F-001/F-002 could allow writing outside intended directories. Not exploitable remotely — requires local CLI access.             |
| **T1499**     | Endpoint Denial of Service         | **LOW** *(R3)*  | The primary risk of the new decoders: a document chooses the patch count, tile step and bit widths the renderer loops on. Every loop is bounded by a constant (§1.3b) and the bounds are exercised by adversarial tests under debug-mode overflow checks. |
| **T1195.001** | Supply Chain: Software Dependencies| **LOW** *(R3)*  | Sixteen advisories in **development** dependencies, one critical (`handlebars` JS injection). None ship. They execute during build and CI, so they are a compromise path to a signed artefact rather than to a user. Tracked in §4.6. |
| **T1211**     | Exploitation for Defense Evasion   | NONE *(R3)*     | The renderer declines what it cannot decode rather than guessing: an unresolvable pattern paints nothing, an undecodable image draws a visible frame. A crafted document cannot make content silently disappear *and* look correct. |

### 2.2 Defensive Coverage

| ATT&CK Defense                | Implementation                                                                                                                                                                                                                                  |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Input Validation**          | `validateSecurityConstraints()` checks file size, header, JavaScript presence, embedded files                                                                                                                                                   |
| **Execution Prevention**      | No eval(), no Function constructor, no dynamic code execution                                                                                                                                                                                   |
| **Memory Protection**         | TypeScript managed memory, no raw pointers, bounds-checked arrays                                                                                                                                                                               |
| **Process Isolation**         | Optional Web Worker support via `useWebWorkers` configuration                                                                                                                                                                                   |
| **SSRF Protection** *(NEW)*   | `isPrivateHost()` blocks localhost, private RFC 1918 ranges, link-local, and cloud metadata endpoints (169.254.169.254, metadata.google.internal). Protocol restricted to http/https. Single-hop redirect limit with re-validation at each hop. |
| **Path Traversal Prevention** | `server.cjs` uses `path.resolve() + path.sep` correctly. CLI has partial protection (needs F-001/F-002 fixes).                                                                                                                                  |
| **Confused-Deputy Confinement** *(R3)* | MCP file access is confined to roots by **canonicalization** (`agenticpdf-rs/src/sandbox.rs`), which resolves `..` and follows symlinks before the prefix test. A textual prefix check is fooled by both. The CLI is deliberately not confined: its operator already has a shell. |
| **Memory Safety** *(R3)*      | **Zero `unsafe` in the core library.** All 22 occurrences are in the two FFI shims (`android.rs`, `apple.rs`), where `extern "C"`/JNI require it, and every entry point wraps its body in `catch_unwind` so a panic cannot unwind across the boundary. |
| **Resource Bounding** *(R3)*  | Constant caps on every attacker-driven loop in the new decoders (§1.3b), verified by adversarial tests rather than by inspection.                                                                                                                |

---

## 3. NIST FIPS 140-3 Compliance Review

### 3.1 Cryptographic Module Assessment

> **Correction to Revisions 1–2.** Those revisions stated that AgenticPDF
> "does **not** implement cryptographic operations" and that PDF encryption was
> "a planned Phase 17 feature". **That is no longer true, and the claim is
> withdrawn.** The Rust crate implements four cryptographic primitives today.
> An audit that understates its own crypto surface is worse than no audit, so
> the inventory below replaces the claim rather than qualifying it.

**Cryptographic inventory (`agenticpdf-rs/`), as built:**

| Primitive   | Location               | Purpose                                                        | FIPS-approved algorithm? |
| ----------- | ---------------------- | -------------------------------------------------------------- | ------------------------ |
| SHA-256     | `src/adf/sha256.rs`    | ADF provenance and integrity commitments                        | ✅ Yes (FIPS 180-4)       |
| AES-128     | `src/crypt/aes.rs`     | Decrypting PDFs using the standard security handler, rev. 4     | ✅ Yes (FIPS 197)         |
| RC4 40/128  | `src/crypt/rc4.rs`     | Decrypting PDFs using the standard security handler, rev. 2–3   | ❌ No                     |
| MD5         | `src/crypt/md5.rs`     | Key derivation for the standard security handler                | ❌ No                     |

**Assessment against FIPS 140-3:**

| Requirement                          | Status                                                                                                                              |
| ------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| Validated cryptographic **module**   | ❌ **NOT COMPLIANT.** All four primitives are implemented in-crate and are not part of a CMVP-validated module.                       |
| Approved **algorithms** only         | ❌ RC4 and MD5 are present. SHA-256 and AES-128 are approved.                                                                        |
| Algorithm correctness                | ✅ SHA-256 is tested against the published FIPS 180-4 vectors, the empty input, multi-block inputs, and both sides of the 55/56/64-byte padding boundaries. |
| Key management / zeroization         | ⚠️ Decryption keys are derived per document and dropped with their owning value; no explicit zeroization.                            |
| No self-modifying code               | ✅ Static code, no `eval`, no dynamic codegen.                                                                                       |
| Module boundary                      | ✅ `crate::crypt` and `crate::adf::sha256` are the only crypto entry points.                                                          |

**The distinction that matters.** FIPS 140-3 validates a *module*, not an
algorithm. Implementing SHA-256 correctly — even provably, against the
published vectors — does not make a build FIPS-compliant, and this document
does not claim it does. A deployment with a genuine FIPS obligation must
substitute a validated module.

**Why RC4 and MD5 are present, and why that is not a policy failure.** They are
not chosen protections; they are **read-only interoperability**. ISO 32000
specifies them for the standard security handler at revisions 2–4, so a reader
that refuses them cannot open documents that already exist. Nothing in this
codebase *encrypts* anything, derives a key to protect data it produces, or
offers RC4 or MD5 to a caller as a security service. The distinction is between
using a weak algorithm to protect data — which this does not do — and decoding
data someone else protected weakly, which is the whole job of a reader.

**Path to a FIPS-capable build**, if one is ever required:

1. Put crypto behind a `fips` feature that binds AES and SHA-256 to a validated
   module (AWS-LC-FIPS or the OpenSSL 3 FIPS provider) instead of the in-crate code.
2. Under that feature, **refuse** RC4- and MD5-protected documents with a clear
   diagnostic rather than decoding them — a refusal is the correct behaviour when
   the algorithm is disallowed, and it is what the corpus notes call the safe direction.
3. Zeroize derived keys explicitly on drop.
4. Record the CMVP certificate number for the chosen module in this section.

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
| No custom hashing         | ❌ **CORRECTED (R3)** — SHA-256 and MD5 are implemented in-crate. See §3.1. SHA-256 is verified against FIPS 180-4 vectors; MD5 exists only to derive PDF decryption keys as ISO 32000 specifies.          |
| No key derivation         | ❌ **CORRECTED (R3)** — the standard security handler's key derivation is implemented (`src/crypt/mod.rs`). It is not PBKDF2/scrypt/Argon2; it is the padded-MD5 construction the PDF format mandates.     |
| Constant-time operations  | ⚠️ Not implemented, and documented as such at `src/adf/sha256.rs`: it hashes public content with no key, no nonce and no secret-dependent branch. The note explicitly warns against reusing it for HMAC or password hashing without revisiting that. |
| Random number generation  | ⚠️ **UPDATED** — 5 call sites use `Math.random()` for session/annotation IDs (F-005). Non-cryptographic use, but should migrate to `crypto.getRandomValues()` for FIPS alignment and collision resistance. |
| No certificate validation | ✅ Not applicable until signature support (Phase 19)                                                                                                                                                       |

---

## 4. CMMC 2.0 Level 2 Compliance Checklist

CMMC 2.0 Level 2 maps to NIST SP 800-171 Rev 2 with 110 security controls across 14 domains. Below are the controls applicable to a PDF processing library component.

> **Scoping note (R3), which governs everything below.** CMMC applies to systems
> that store, process or transmit **CUI**. This repository is a *component*, and
> it is **public** (`github.com/nervosys/AgenticPDF`). No CUI is present in the
> repository and none should be introduced: the render-correctness test corpus is
> the maintainer's own personal documents held **outside** the repository and
> deliberately never committed, and this revision re-verified that nothing from
> it — no filename of a personal record, no extracted content, no local path, no
> credential — has entered the tree or the commit history (§10). The controls
> below describe what the component offers an integrator who *does* handle CUI.
> They are not a claim that this repository is an assessed CMMC boundary, and a
> public repository could not be one.

### 4.1 Access Control (AC)

| Control      | Requirement                                    | Status                                                                                  |
| ------------ | ---------------------------------------------- | --------------------------------------------------------------------------------------- |
| AC.L2-3.1.1  | Limit system access to authorized users        | ✅ Library does not manage users — caller enforces access                                |
| AC.L2-3.1.2  | Limit system access to authorized transactions | ✅ All operations explicitly invoked by caller                                           |
| AC.L2-3.1.3  | Control CUI flow                               | ✅ No data exfiltration capability. No network access. SSRF protection on `fromUrl()`.   |
| AC.L2-3.1.5  | Least privilege                                | ✅ No elevated permissions required. No filesystem access beyond caller-provided buffers |
| AC.L2-3.1.19 | Encrypt CUI on mobile devices                  | N/A — Library component                                                                 |
| AC.L2-3.1.20 | Verify connections to external systems         | ✅ *(R3)* MCP file access is confined to canonicalized roots (`src/sandbox.rs`), which is the confused-deputy boundary: there the *model* chooses the path and a document can argue for one. The CLI is deliberately unconfined — its operator already has a shell. |

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

*(R3) Additions for this revision:*

| Control     | Requirement                                   | Status                                                                                                                                     |
| ----------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| SI.L2-3.14.1 | Identify and correct flaws in a timely manner | ⚠️ Advisory scanning now run and triaged (§1.3a); not yet in CI. The two HIGH findings cannot be remediated locally — they are pinned upstream through `eframe`. |
| SI.L2-3.14.2 | Protect against malicious code                | ✅ Documents are parsed as data and never executed. Unresolvable constructs are **declined** rather than guessed at, and an undecodable image draws a visible frame rather than vanishing, so a crafted file cannot silently remove content and still look correct. |
| SI.L2-3.14.6 | Monitor for attacks                            | N/A for a library component; telemetry is opt-**in** and off by default.                                                                   |
| SI.L2-3.14.7 | Identify unauthorized use                     | N/A — no session or user model.                                                                                                            |

*Original Revision 2 table follows.*


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
| **Vulnerability scanning**       | ⚠️ **PARTIAL (R3)** — `npm audit` and `cargo audit` are now *run* and their results triaged in §1.3a, but they are still not wired into CI. Wiring them in is the outstanding half. |
| **Rust crate audit**             | ⚠️ **CORRECTED (R3)** — the claim "no known CVEs" no longer holds: 466 transitive crates carry two HIGH advisories in `quick-xml` 0.30.0, reachable only from the Linux desktop accessibility stack (§1.3a). "Widely used" is not the same as "unaffected", and the earlier row read as though it were. |
| **Dev-chain exposure** *(R3)*    | ⚠️ 16 npm advisories (1 critical, 12 high) in **development** dependencies. None ship — the package declares one runtime dependency and a `files` allowlist — but they execute during build and CI, which is a path to a compromised artefact rather than to a user. |
| **Unmaintained crates** *(R3)*   | ℹ️ `paste` 1.0.15 and `ttf-parser` 0.25.1 are flagged unmaintained. Neither is a vulnerability; both are worth a replacement plan before they become one. |
| **Memory safety** *(R3)*         | ✅ **Zero `unsafe` in the core library.** All 22 occurrences are in the Android and iOS FFI shims, where the calling convention requires it, and every entry point wraps its body in `catch_unwind`. |

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
| `cfb`               | 0.14              | ✅ SAFE     | OLE2 compound files (legacy Office)  |
| `encoding_rs`       | 0.8               | ✅ SAFE     | Windows code pages for legacy Office |
| `deweygui` *(reader)* | pinned rev      | ⚠️ *(R3)*  | Desktop/mobile painter. Pulls `eframe` on desktop, which is the source of the two HIGH advisories in §1.3a. Pinned to a git revision rather than a branch so the build cannot move underneath it. |

*(R3) The table above lists **direct** dependencies. The full graph is 466
crates; "widely used" was doing more work in Revisions 1-2 than it should
have. See §1.3a for what an actual advisory scan found.*

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

**~~No known CVEs detected across any dependency.~~ Withdrawn (R3).** That
sentence was true of the *direct* dependency list it was written against and
false of the graph. An actual `cargo audit` over all 466 crates finds two HIGH
advisories, and `npm audit` finds sixteen in the development chain. See §1.3a
for both, and for why neither is reachable from a document.

---

## 10. Private-Data Exposure Review *(NEW Section, R3)*

The render-correctness work measures against a corpus of **the maintainer's own
personal documents** — receipts, agreements, tickets, account applications,
property and tax records among them — held in `~/Documents` and `~/Desktop`.
The repository is public. Before publishing 91 commits, the diff, the working
tree and every commit message were scanned for anything that corpus could have
leaked. Method and result:

| Check                                                    | Method                                                                                          | Result |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ------ |
| Credentials, private keys, cloud tokens                  | Pattern scan for `sk-`, `ghp_`, `github_pat_`, `AKIA`, `xox[baprs]-`, `AIza`, PEM private keys   | ✅ None |
| Assigned secrets (`api_key = "..."`, `token: "..."`)     | Pattern scan excluding placeholders and `process.env` / `std::env` reads                          | ✅ None |
| Telemetry endpoint and bearer token                      | `.env.example` inspected                                                                          | ✅ Placeholders only (`YOUR_TOKEN_HERE`, `your-otel-collector.example.com`) |
| Local filesystem paths revealing a username              | Scan for `C:/Users/<name>`, `/c/Users/<name>`                                                     | ✅ Only the generic forms `C:/Users/...` and `~/Documents`, used as documentation placeholders |
| Author identity                                          | `git log --format='%an <%ae>'` over all 91 commits                                                | ✅ `NERVOSYS <opensource@nervosys.ai>` throughout; no personal address |
| Personal e-mail in content                               | Address scan over the full diff                                                                    | ✅ Only `john@example.com`, a documentation example |
| **Names of personal records**                            | Scan of tree and all commit messages for receipts, agreements, tickets, disclosures, applications | ✅ None |
| Names of corpus documents that *do* appear               | Manual review of every document identifier in the docs and commit messages                        | ✅ Five, all **publicly distributed** technical literature: `ADA617071` (DISTRIBUTION STATEMENT A — approved for public release), `atmosphere-10-00549` (MDPI open access), `SolarBlack` (vendor datasheet), `podc`, `design-article-optimizing-bldc-motor-control` |
| Extracted document **content**                           | Reviewed: no page text, no rendered pixels, no reference images committed                          | ✅ None |
| Binary fixture (`apps/reader/document.adf`)              | Strings inspection of the committed ADF                                                           | ✅ Synthetic only ("Report", "Revenue grew across EMEA.", "Hiring on plan") |

**One disclosure is present by design and is worth stating rather than
burying:** `docs/development/HANDOFF.md` records that the corpus lives in
`~/Documents` and `~/Desktop`. That reveals the corpus is personal; it reveals
nothing about which documents it contains. It is kept because the reproduction
instructions are useless without it.

**Standing rule for this work.** Corpus documents are referred to by *kind* —
"a prepress page", "property records" — and never by filename, except where the
document is itself public. Rendered comparisons, reference images and sweep
outputs stay in scratch space and are never committed.

---

## 11. Recommendations

### 11.0 Revision 3 Actions

| # | Action                                                                                                                     | Priority | Owner-decision? |
| - | -------------------------------------------------------------------------------------------------------------------------- | -------- | --------------- |
| 1 | **Wire `cargo audit` and `npm audit` into CI** so the next stale "no known CVEs" line cannot be written by hand.            | HIGH     | No              |
| 2 | Track `accesskit`/`eframe` for an upgrade that moves `quick-xml` past 0.30.0; it cannot be fixed from this repository.       | MEDIUM   | No              |
| 3 | Reduce the dev chain: 16 advisories, one critical, all in tooling that runs on developer and CI machines.                    | MEDIUM   | No              |
| 4 | Decide whether a **FIPS-capable build** is a product requirement. If it is, §3.1 lists the four steps; if it is not, say so in `SECURITY.md` so the question stops being asked. | —        | **Yes**         |
| 5 | Plan replacements for `paste` and `ttf-parser` before "unmaintained" becomes "vulnerable".                                   | LOW      | No              |
| 6 | Keep the corpus convention in §10: documents by kind, never by filename, no rendered output committed.                       | ONGOING  | No              |

### 11.1 Immediate (Priority: HIGH)

1. **Fix CLI path traversal (F-001, F-002)** — Apply `path.sep`-suffixed `startsWith` check to image extraction. Add `validateOutputPath()` to all 8 `writeFileSync` calls.
2. **Add security headers to dev server (F-003)** — Set `X-Content-Type-Options: nosniff`, `X-Frame-Options: SAMEORIGIN`. Low urgency since dev-only.
3. **Replace `Math.random()` with `crypto` (F-005)** — Migrate 5 call sites to `crypto.getRandomValues()` or `crypto.randomBytes()`.

### 11.2 Short-Term (Priority: MEDIUM)

4. **Add ReDoS guard to `searchText()` (F-006)** — Limit regex pattern length (200 chars) and reject patterns with known catastrophic backtracking patterns.
5. **Escape PDF-sourced strings before regex interpolation (F-007)** — Apply `replace(/[.*+?^${}()|[\]\\]/g, '\\$&')` to `firstName` before `new RegExp()`.
6. **Configure CSP for website (F-004)** — If deploying beyond static hosting, add CSP via Next.js middleware or hosting provider headers.
7. **Add `npm audit` and `cargo audit` to CI/CD pipeline.**
8. **Implement npm provenance** signing for published packages.

### 11.3 Long-Term (Priority: LOW)

9. **Add FIPS-validated crypto** — *(R3: no longer "when implementing Phase 17". Cryptography is implemented today; see §3.1. This is now a substitution, not a greenfield choice.)*
10. **Pursue FIPS 140-3 Level 1 validation** for the cryptographic module.
11. **Obtain CMMC Level 2 assessment** from an authorized C3PAO.
12. **Implement persistent audit logging** (syslog/structured JSON) for SI/AU controls.
13. **Add fuzzing** via `cargo-fuzz` for the Rust parser and `jsfuzz` for TypeScript.
14. **Add rate limiting** to streaming APIs for DoS prevention.

---

## 12. Conclusion

AgenticPDF presents a **low-medium risk profile** for deployment in DoD and regulated environments:

- **Zero runtime dependencies** eliminate supply chain attack vectors
- **No code execution** from parsed PDFs prevents exploitation via T1203/T1059
- **Comprehensive input validation** with safety counters, bounds checks, and size limits
- **TypeScript + Rust** provide memory safety guarantees; the Rust core contains **zero `unsafe`**, and the 22 uses in the codebase are confined to the two FFI shims, each entry point wrapped in `catch_unwind`
- **Excellent SSRF protection** with private IP blocking, protocol whitelisting, and single-hop redirects
- **SBOM generation** via `generateSBOM()` supports CMMC 2.0 supply chain requirements
- **924 TypeScript tests; 597 crate + 45 integration + 75 reader tests in Rust**, clippy `-D warnings` clean

**What Revision 3 changed, stated plainly rather than folded into a rating.**
Three claims in the earlier revisions were wrong and are withdrawn, not softened:

1. *"AgenticPDF does not implement cryptographic operations."* It implements
   four primitives. §3.1 inventories them. The build is **not FIPS 140-3
   compliant** — approved algorithms are not a validated module — and RC4 and MD5
   are present as read-only format interoperability, not as chosen protections.
2. *"No known CVEs detected across any dependency."* An actual scan finds two
   HIGH advisories among 466 crates and sixteen in the npm dev chain. Neither
   set is reachable from a document, and §1.3a shows the dependency path rather
   than asserting it.
3. *"No MD5/SHA used."* Both are used. See §3.3.

Set against that, the render-correctness branch adds four decoders that read
attacker-chosen counts and bit widths, and every one of them is bounded by a
constant and exercised with malformed input under debug-mode overflow checks
(§1.3b). The residual risk is unchanged in kind and better evidenced than
before: a crafted document can waste bounded work, and cannot execute code,
escape a path, or reach the network.

**An audit is only worth the claims it is willing to retract.** These three were
retracted because they were checked, and checking them is the substance of this
revision.

---

*Revision History:*
- 2026-08-26 (R3): Advisory scanning run for the first time; FIPS section corrected; private-data exposure review added ahead of publishing 91 commits to a public repository.
- *2026-04-01 (Rev 2): Expanded scope to CLI, dev server, website, SSRF, regex, PRNG. Added findings F-001 through F-007. Updated test counts, dependency audit, CMMC/NIST sections.*
- *2026-03-14 (Rev 1): Initial audit — CVE, MITRE ATT&CK, NIST FIPS 140-3, CMMC 2.0 Level 2.*

*This document should be reviewed and updated with each major release. For questions regarding DoD deployment, contact the maintaining organization (NERVOSYS, LLC) for a formal security assessment.*