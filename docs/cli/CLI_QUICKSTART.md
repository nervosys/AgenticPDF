# AgenticPDF CLI - Quick Start Guide

Get started with the AgenticPDF command-line interface in under 5 minutes!

## Installation

```bash
# Install dependencies (first time only)
npm install

# Test the CLI
npm run cli -- help
```

## Your First Commands

### 1. Display PDF Information

```bash
npm run cli -- info demos/sample.pdf
```

**Output:**
```
📄 PDF Information
  Title: Sample Document
  Author: John Doe
  Pages: 10
  PDF Version: 1.7
```

### 2. Extract Text

```bash
# Extract to console
npm run cli -- extract -i demos/sample.pdf

# Save to file
npm run cli -- extract -i demos/sample.pdf -o output.txt

# Extract specific pages
npm run cli -- extract -i demos/sample.pdf -p 1-5
```

### 3. Convert to JSON

```bash
npm run cli -- convert -i demos/sample.pdf -f json --pretty -o output.json
```

### 4. AI Analysis

```bash
npm run cli -- analyze -i demos/sample.pdf --ai --pretty
```

### 5. Generate Chunks for RAG

```bash
npm run cli -- chunk -i demos/sample.pdf --chunk-size 1000 -o chunks.json
```

## Quick Reference

| Command   | Description      | Example                                           |
| --------- | ---------------- | ------------------------------------------------- |
| `info`    | Display metadata | `npm run cli -- info file.pdf`                    |
| `extract` | Extract text     | `npm run cli -- extract -i file.pdf -o out.txt`   |
| `convert` | Convert format   | `npm run cli -- convert -i file.pdf -f json`      |
| `analyze` | AI analysis      | `npm run cli -- analyze -i file.pdf --ai`         |
| `chunk`   | Generate chunks  | `npm run cli -- chunk -i file.pdf -o chunks.json` |
| `images`  | Extract images   | `npm run cli -- images -i file.pdf -o ./images/`  |
| `forms`   | Extract forms    | `npm run cli -- forms -i file.pdf`                |

## Common Options

- `-i, --input <file>` - Input PDF file (required)
- `-o, --output <file>` - Output file path
- `-p, --pages <range>` - Page range (e.g., `1-5`, `1,3,5`)
- `-v, --verbose` - Verbose output
- `--pretty` - Pretty-print JSON

## Platform-Specific Usage

### Windows (PowerShell)

```powershell
# Run CLI directly
npm run cli -- extract -i demos/sample.pdf

# Or use the PowerShell wrapper
.\agenticpdf-cli.ps1 extract -Input demos/sample.pdf -Verbose

# Batch process all PDFs
Get-ChildItem *.pdf | ForEach-Object {
  npm run cli -- extract -i $_.Name -o "$($_.BaseName).txt"
}
```

### Linux/macOS (Bash)

```bash
# Run CLI directly
npm run cli -- extract -i demos/sample.pdf

# Or use the bash wrapper
./agenticpdf-cli.sh extract -i demos/sample.pdf -v

# Batch process all PDFs
for pdf in *.pdf; do
  npm run cli -- extract -i "$pdf" -o "${pdf%.pdf}.txt"
done
```

## Real-World Examples

### Extract Text from Research Paper

```bash
npm run cli -- extract -i research-paper.pdf \
  -o paper.txt \
  --metadata \
  --tables \
  -v
```

### Create RAG-Ready Chunks

```bash
npm run cli -- chunk -i document.pdf \
  --chunk-size 1000 \
  --pretty \
  -o chunks.json
```

**Output format:**
```json
[
  {
    "content": "This is the content of the first chunk...",
    "pageNumbers": [1, 2],
    "chunkType": "paragraph",
    "metadata": {
      "confidence": 0.95,
      "wordCount": 150
    }
  }
]
```

### Extract All Images

```bash
npm run cli -- images -i document.pdf -o ./extracted-images/ -v
```

### AI-Powered Document Analysis

```bash
npm run cli -- analyze -i document.pdf \
  --ai \
  --pretty \
  -o analysis.json
```

**Output includes:**
- Document type detection
- Structural analysis (sections, tables, figures)
- Entity extraction
- Keywords
- Summary

## Testing the CLI

We've included a test script to verify everything works:

```bash
# Run automated tests
node test-cli.js

# Or run the interactive examples
pwsh examples/cli-examples.ps1
```

## Troubleshooting

### "File not found" error

Make sure to provide the correct path to your PDF:

```bash
# Absolute path
npm run cli -- info "C:\Users\YourName\Documents\document.pdf"

# Relative path
npm run cli -- info ./demos/sample.pdf
```

### Out of memory errors

Use streaming mode for large PDFs:

```bash
npm run cli -- extract -i large-file.pdf --stream -o output.txt
```

### Command not recognized

Make sure you're using the correct syntax with `npm run cli --`:

```bash
# ✅ Correct
npm run cli -- extract -i file.pdf

# ❌ Wrong
npm run cli extract -i file.pdf
```

## Next Steps

- **Full Documentation**: See [CLI.md](./CLI.md) for complete command reference
- **Code Examples**: Check the [examples/](./examples/) directory
- **API Documentation**: See [README.md](./README.md) for programmatic usage
- **Interactive Demos**: Open `demos/examples-demo.html` in your browser

## Getting Help

```bash
# General help
npm run cli -- help

# Version info
npm run cli -- version

# Verbose output for debugging
npm run cli -- extract -i file.pdf -v
```

## Tips & Tricks

1. **Use verbose mode** (`-v`) to see progress on long operations
2. **Pretty-print JSON** with `--pretty` for readable output
3. **Specify page ranges** (`-p 1-5`) to process only what you need
4. **Stream large files** with `--stream` to avoid memory issues
5. **Batch processing** - Use shell loops for multiple files

## Support

- 📖 [Full CLI Documentation](./CLI.md)
- 🐛 [Report Issues](https://github.com/nervosys/agenticpdf/issues)
- 💡 [Request Features](https://github.com/nervosys/agenticpdf/issues/new)

---

**Ready to process some PDFs?** Try running:

```bash
npm run cli -- info demos/sample.pdf
```
