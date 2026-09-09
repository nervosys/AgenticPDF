# IronDocuments CLI Documentation

Complete command-line interface for PDF processing with IronDocuments.

## Installation

```bash
# Global installation (recommended)
npm install -g irondocuments

# Or use with npx (no installation)
npx idoc <command>

# Or run directly from source
npm run cli -- <command>
```

## Quick Start

```bash
# Display PDF information
idoc info document.pdf

# Extract text to console
idoc extract -i document.pdf

# Extract text to file
idoc extract -i document.pdf -o output.txt

# Convert to JSON with metadata
idoc convert -i document.pdf -f json --metadata --pretty
```

## Commands

### info - Display PDF Information

Show document metadata and properties.

```bash
idoc info <input-file> [options]

# Examples
idoc info document.pdf
idoc info -i document.pdf -v
```

**Output includes:**
- Title, Author, Subject
- Creator, Producer
- Page count, PDF version
- Creation and modification dates
- Encryption status
- Keywords

### extract - Extract Text Content

Extract text from PDF with formatting options.

```bash
idoc extract -i <input-file> [options]

# Examples
idoc extract -i document.pdf
idoc extract -i document.pdf -o output.txt
idoc extract -i document.pdf -p 1-5 --tables
idoc extract -i large.pdf --stream -v
```

**Options:**
- `-o, --output <file>` - Save to file (default: console)
- `-p, --pages <range>` - Page range (e.g., 1-5, 1,3,5)
- `--tables` - Extract tables
- `--stream` - Use streaming mode for large files
- `-m, --metadata` - Include metadata statistics
- `-v, --verbose` - Show progress

### convert - Convert to Different Formats

Convert PDF to text, JSON, HTML, or Markdown.

```bash
idoc convert -i <input-file> -f <format> [options]

# Examples
idoc convert -i document.pdf -f text -o output.txt
idoc convert -i document.pdf -f json --pretty -o output.json
idoc convert -i document.pdf -f html --tables -o output.html
idoc convert -i document.pdf -f markdown --images -o output.md
```

**Formats:**
- `text` - Plain text
- `json` - Structured JSON
- `html` - HTML format
- `markdown` - Markdown format

**Options:**
- `-o, --output <file>` - Output file
- `-p, --pages <range>` - Page range
- `--pretty` - Pretty-print JSON
- `-m, --metadata` - Include metadata
- `--tables` - Extract tables
- `--annotations` - Include annotations

### analyze - AI-Powered Analysis

Perform intelligent document analysis using AI features.

```bash
idoc analyze -i <input-file> [options]

# Examples
idoc analyze -i document.pdf
idoc analyze -i document.pdf --ai -o analysis.json
idoc analyze -i document.pdf --pretty -v
```

**Features:**
- Document type detection
- Structural analysis (sections, tables, figures)
- Entity extraction (with `--ai`)
- Keyword extraction
- Document summarization (with `--ai`)

**Options:**
- `-o, --output <file>` - Save analysis
- `--ai` - Enable advanced NER and summarization
- `--pretty` - Pretty-print output
- `-v, --verbose` - Show progress

### chunk - Generate Semantic Chunks

Create semantic chunks optimized for RAG systems.

```bash
idoc chunk -i <input-file> [options]

# Examples
idoc chunk -i document.pdf -o chunks.json
idoc chunk -i document.pdf --chunk-size 1000 --pretty
idoc chunk -i large.pdf --stream -v -o chunks.json
```

**Options:**
- `-o, --output <file>` - Save chunks to file
- `--chunk-size <size>` - Maximum chunk size (default: 1000)
- `--stream` - Use streaming mode
- `--pretty` - Pretty-print JSON
- `-v, --verbose` - Show progress

**Output format:**
```json
[
  {
    "content": "chunk text content...",
    "pageNumbers": [1, 2],
    "chunkType": "paragraph",
    "metadata": {
      "confidence": 0.95,
      "wordCount": 150
    }
  }
]
```

### images - Extract Images

Extract all images from PDF.

```bash
idoc images -i <input-file> [options]

# Examples
idoc images -i document.pdf -o ./output-images/
idoc images -i document.pdf -v
```

**Options:**
- `-o, --output <directory>` - Output directory for images
- `-v, --verbose` - Show each image saved

**Output:**
- Images saved as `image_1.png`, `image_2.jpg`, etc.
- Format preserved from PDF (PNG, JPEG, etc.)

### forms - Extract Form Fields

Extract form field information.

```bash
idoc forms -i <input-file> [options]

# Examples
idoc forms -i form.pdf
idoc forms -i form.pdf -o fields.json --pretty
```

**Output format:**
```json
[
  {
    "name": "firstName",
    "type": "text",
    "value": "John",
    "required": true
  }
]
```

### typeset - Typesetting & Web Display

Generate CSS, HTML, accessible views, or social meta tags from aPDF display hints.

```bash
idoc typeset -i <input-file> [options]

# Generate responsive HTML article (default)
idoc typeset -i paper.pdf -o article.html

# CSS stylesheet from display hints
idoc typeset -i paper.pdf --css -o styles.css

# Accessible reading view with ARIA landmarks
idoc typeset -i paper.pdf --accessible -o readable.html

# Print-ready stylesheet
idoc typeset -i paper.pdf --print-css -o print.css

# Social meta tags (OG, Twitter Card, JSON-LD)
idoc typeset -i paper.pdf --social-meta --page-url https://example.com/paper
```

**Modes:**
- Default — responsive HTML article with TOC, bibliography, KaTeX math
- `--css` — scoped CSS driven by fonts, theme, reading order, page dimensions
- `--accessible` — semantic HTML with skip navigation and reading-level info
- `--print-css` — `@page` rules, orphan/widow control, URL-after-link printing
- `--social-meta` — Open Graph, Twitter Card, citation `<meta>`, Schema.org JSON-LD

## Global Options

These options work with all commands:

- `-i, --input <file>` - Input PDF file (required)
- `-o, --output <file>` - Output file path
- `-v, --verbose` - Verbose output with progress
- `-h, --help` - Display help
- `--version` - Show version

### Typeset Options

These options apply to the `typeset` command:

- `--css` - Output a CSS stylesheet instead of HTML
- `--accessible` - Generate an accessible reading view with ARIA landmarks
- `--print-css` - Generate a print-ready CSS stylesheet
- `--social-meta` - Generate Open Graph / Twitter Card meta tags and JSON-LD
- `--page-url <url>` - Public document URL for social meta tags

## Examples

### Basic Workflow

```bash
# 1. Check PDF info
idoc info document.pdf

# 2. Extract text from specific pages
idoc extract -i document.pdf -p 1-10 -o chapter1.txt

# 3. Convert to JSON with all features
idoc convert -i document.pdf -f json \
  --metadata --tables --annotations --pretty \
  -o document.json
```

### RAG System Integration

```bash
# Generate semantic chunks optimized for embeddings
idoc chunk -i research-paper.pdf \
  --chunk-size 1000 \
  --pretty \
  -o chunks.json

# Then use chunks with your embedding model
# Output ready for vector databases like Pinecone, Weaviate, etc.
```

### Large Document Processing

```bash
# Use streaming mode for large PDFs
idoc extract -i large-document.pdf \
  --stream \
  -v \
  -o output.txt

# Stream semantic chunks
idoc chunk -i large-document.pdf \
  --stream \
  --chunk-size 1500 \
  -o chunks.json
```

### AI-Powered Analysis

```bash
# Full document analysis
idoc analyze -i document.pdf \
  --ai \
  --pretty \
  -o analysis.json

# Output includes:
# - Document type (research, legal, technical, etc.)
# - Structural analysis (sections, tables, figures)
# - Extracted entities
# - Keywords
# - Summary
```

### Batch Processing

```bash
# Process multiple PDFs (PowerShell)
Get-ChildItem *.pdf | ForEach-Object {
  idoc extract -i $_.Name -o "$($_.BaseName).txt"
}

# Bash/zsh
for pdf in *.pdf; do
  idoc extract -i "$pdf" -o "${pdf%.pdf}.txt"
done
```

### Image Extraction

```bash
# Extract all images
idoc images -i document.pdf -o ./images/

# With verbose output
idoc images -i document.pdf -o ./images/ -v
```

## Output Formats

### Text Format
Plain text with optional formatting preservation.

### JSON Format
Structured JSON with nested objects:
```json
{
  "metadata": { ... },
  "pages": [ ... ],
  "content": [ ... ]
}
```

### HTML Format
Clean HTML with semantic structure:
```html
<article>
  <h1>Title</h1>
  <p>Content...</p>
</article>
```

### Markdown Format
GitHub-flavored Markdown:
```markdown
# Title

Content...

## Section

More content...
```

## Performance Tips

1. **Use streaming for large files** (> 50MB):
   ```bash
   idoc extract -i large.pdf --stream
   ```

2. **Specify page ranges** to process only needed pages:
   ```bash
   idoc extract -i document.pdf -p 1-10
   ```

3. **Enable verbose mode** for progress tracking:
   ```bash
   idoc chunk -i document.pdf -v
   ```

4. **Use appropriate chunk sizes** for RAG:
   - Small models: `--chunk-size 500`
   - Medium models: `--chunk-size 1000` (default)
   - Large models: `--chunk-size 2000`

## Error Handling

The CLI provides clear error messages:

```bash
# File not found
✗ File not found: document.pdf

# Invalid page range
✗ Invalid page range: 10-5

# No output specified for images
✗ Output directory required for image extraction
```

Use `--verbose` for detailed error information including stack traces.

## Integration Examples

### With LLM APIs

```bash
# Extract and analyze
idoc analyze -i paper.pdf --ai -o analysis.json

# Use with LLM (example with curl)
cat analysis.json | curl -X POST https://api.openai.com/v1/chat/completions \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d @-
```

### With Vector Databases

```bash
# Generate chunks
idoc chunk -i document.pdf --chunk-size 1000 -o chunks.json

# Import to Pinecone, Weaviate, etc.
# Use your vector database's import tool with chunks.json
```

### With CI/CD Pipelines

```yaml
# GitHub Actions example
- name: Extract PDF content
  run: |
    npm install -g irondocuments
    idoc extract -i document.pdf -o output.txt
    idoc analyze -i document.pdf -o analysis.json
```

## Advanced Usage

### Custom Page Ranges

```bash
# Single page
idoc extract -i doc.pdf -p 5

# Range
idoc extract -i doc.pdf -p 1-10

# Multiple ranges (comma-separated)
idoc extract -i doc.pdf -p 1,5,10,15

# Mix of both
idoc extract -i doc.pdf -p 1-5,10,15-20
```

### Combining Options

```bash
# Full-featured extraction
idoc extract -i document.pdf \
  -o output.txt \
  -p 1-50 \
  --tables \
  --metadata \
  --stream \
  -v

# Complete conversion pipeline
idoc convert -i document.pdf \
  -f json \
  --pretty \
  --metadata \
  --tables \
  --annotations \
  --images \
  -o complete-export.json
```

## Troubleshooting

### Out of Memory Errors

Use streaming mode:
```bash
idoc extract -i large.pdf --stream -o output.txt
```

### Slow Processing

Enable verbose mode to see progress:
```bash
idoc chunk -i document.pdf -v
```

### Missing Text

Try without formatting preservation:
```bash
# Default extraction includes formatting
# If issues occur, report as a bug
```

## Support

- **Documentation**: [README.md](./README.md)
- **GitHub Issues**: [Report bugs or request features](https://github.com/nervosys/IronDocuments/issues)
- **Examples**: See `examples/` directory for code samples

## Version

```bash
idoc --version
# IronDocuments CLI v1.0.0
```

## License

MIT License - See [LICENSE](./LICENSE) file for details.
