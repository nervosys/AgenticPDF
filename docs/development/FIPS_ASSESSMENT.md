<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# FIPS 140-3: what it would cost, and what it would not

Groundwork for a decision that is still open (see [`../../ROADMAP.md`](../../ROADMAP.md)).
This assesses the work; it does not make the call, because whether FIPS is a
product requirement depends on who the buyer is.

**Summary.** Only two primitives actually have to move. The obstacle is not the
cryptography — it is that a validated module is a heavy native dependency, and
"single static binary, no runtime, runs in WASM" is the project's stated wedge.

---

## What the crate uses today

All of it is implemented in-crate, with no third-party cryptographic
dependency:

| Primitive | Where | Used for | Is it a security boundary? |
| --- | --- | --- | --- |
| RC4 (40/128-bit) | `src/crypt/rc4.rs` | Reading PDFs encrypted with the standard security handler, revisions 2–4 | **No** |
| MD5 | `src/crypt/md5.rs` | Key derivation for the same | **No** |
| AES-128 | `src/crypt/aes.rs` | The same, revision 4 | **No** |
| SHA-256 | `src/adf/sha256.rs` | ADF provenance commitments | **Yes** |
| SHA-256, truncated to 32 bits | `src/adf/mod.rs` | ADF container checksums | No — corruption detection |

The crate performs no signing and no signature verification.

---

## The two questions FIPS asks

### 1. Are the algorithms approved?

RC4 and MD5 are not, and never will be. AES and SHA-256 are.

This sounds fatal for PDF decryption and is not. The standard security handler
with an empty user password is **not a security control**: the file hands the
key to anyone who opens it, and the crate's own module comment says so — "a
wish, not a lock". FIPS permits non-approved algorithms used for
non-security-relevant purposes, and decoding a document format that happens to
be obfuscated with RC4 is exactly that. It has to be *documented* as such, not
hidden.

The alternative — refusing to open those files in a FIPS build — is worse than
it sounds. Eleven of the corpus's 288 documents are encrypted this way, and all
of them are ordinary documents that merely ask not to be printed.

### 2. Are the implementations validated?

This is the real cost. FIPS 140-3 requires an algorithm to come from a
*validated module*, not merely to be the right algorithm. Hand-written AES and
SHA-256 do not qualify however correct they are.

So a FIPS build must route two things through a validated provider:

- **AES-128**, for decryption. Non-security-relevant by the argument above, so
  arguably out of scope — but a reviewer will ask, and "our own AES" is a harder
  conversation than "AES from the validated module".
- **SHA-256**, for ADF provenance. This one is a genuine integrity commitment
  and is squarely in scope. It has to move.

---

## What that costs

The candidates are `aws-lc-rs` in FIPS mode or the OpenSSL 3 FIPS provider.
Either one:

- adds a **native build dependency** (a C toolchain, and CMake for aws-lc-rs),
  against a crate that currently builds with `cargo build` and nothing else;
- **does not run in WASM**, so the browser and edge targets would need the
  in-crate implementations anyway — meaning two code paths, not one;
- makes the binary no longer trivially static and cross-compilable, which is
  what the Android and iOS targets depend on.

That is the whole trade: FIPS costs the footprint and reach that the project
positions itself on.

---

## The shape it would take, if the answer is yes

Not a rewrite. Two primitives behind a trait, chosen by a Cargo feature:

1. Define `trait Digest` and `trait BlockCipher` over the two call sites —
   `adf::provenance` and `crypt::mod`. Both are small and already isolated.
2. Default feature: the in-crate implementations, unchanged. Builds everywhere,
   including WASM.
3. `--features fips`: `aws-lc-rs` in FIPS mode. Native targets only, gated in
   CI so a WASM build cannot silently select it.
4. Document the RC4/MD5 carve-out explicitly in `SECURITY.md`, naming it as
   non-security-relevant use for format decoding, and state what a FIPS build
   refuses.

The work is real but bounded: the call sites are few, and the awkward part is
CI and packaging rather than cryptography.

---

## What is not assessed here

Whether any of this is needed. FIPS matters for US federal procurement and some
regulated buyers, and for nobody else. Without a buyer asking, this would add a
native toolchain dependency and a second code path to satisfy a requirement that
may never arrive — which is why the decision sits in the roadmap rather than in
a branch.
