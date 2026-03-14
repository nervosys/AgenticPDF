# AgenticPDF — Rust CLI & WASM

High-performance PDF processing CLI and WebAssembly library for agentic AI workflows.

## Build

### CLI (native binary)

```bash
cargo build --release
```

### WASM (browser integration)

```bash
# Install wasm-pack if needed
cargo install wasm-pack

# Build WASM module
wasm-pack build --target web --features wasm --no-default-features
```

## CLI Usage

```bash
# Extract text
agenticpdf text document.pdf
agenticpdf text document.pdf --pages 1-5 --format json

# Show metadata
agenticpdf meta document.pdf
agenticpdf meta document.pdf --format json

# Generate semantic chunks for RAG
agenticpdf chunk document.pdf --size 500 --overlap 50 --format json

# List images
agenticpdf images document.pdf

# Show library info
agenticpdf info
```

## WASM API

```javascript
import init, { parsePdfMetadata, extractText, generateChunks } from './pkg/agenticpdf.js';

await init();

const response = await fetch('document.pdf');
const bytes = new Uint8Array(await response.arrayBuffer());

// Extract text
const text = extractText(bytes);

// Get metadata
const meta = JSON.parse(parsePdfMetadata(bytes));

// Generate RAG chunks
const chunks = JSON.parse(generateChunks(bytes, 500, 50));
```

## Performance

The Rust implementation provides significant performance improvements over the TypeScript version for:

- **FlateDecode decompression**: ~5-10x faster via `miniz_oxide`
- **Text extraction**: ~3-5x faster with zero-copy parsing
- **Semantic chunking**: ~2-3x faster with efficient string operations
- **Memory usage**: ~50% lower through stack allocation and borrowing

## Architecture

```
src/
├── lib.rs      # Core types (PdfDocument, PdfPage, TextBlock, SemanticChunk)
├── parser.rs   # PDF parser (header, xref, objects, streams, text operators)
├── main.rs     # CLI entry point with clap argument parsing
└── wasm.rs     # wasm-bindgen exports for browser integration
```

## License

AGPL-3.0-or-later / Commercial dual license — Nervosys, LLC
