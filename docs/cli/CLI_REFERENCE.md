# AgenticPDF CLI - Quick Reference Card

## 🚀 Getting Started

```bash
npm install          # First time setup
npm run cli -- help  # Show help
```

## 📋 Commands

| Command   | Description       | Example                                              |
| --------- | ----------------- | ---------------------------------------------------- |
| `info`    | Show PDF metadata | `npm run cli -- info file.pdf`                       |
| `extract` | Extract text      | `npm run cli -- extract -i file.pdf -o out.txt`      |
| `convert` | Convert format    | `npm run cli -- convert -i file.pdf -f json`         |
| `analyze` | AI analysis       | `npm run cli -- analyze -i file.pdf --ai`            |
| `chunk`   | Generate chunks   | `npm run cli -- chunk -i file.pdf -o chunks.json`    |
| `images`  | Extract images    | `npm run cli -- images -i file.pdf -o ./images/`     |
| `forms`   | Extract forms     | `npm run cli -- forms -i file.pdf`                   |
| `typeset` | Typeset for web   | `npm run cli -- typeset -i file.pdf -o article.html` |
| `help`    | Show help         | `npm run cli -- help`                                |
| `version` | Show version      | `npm run cli -- version`                             |

## 🎛️ Common Options

| Option          | Short | Description                | Example                |
| --------------- | ----- | -------------------------- | ---------------------- |
| `--input`       | `-i`  | Input PDF file             | `-i document.pdf`      |
| `--output`      | `-o`  | Output file                | `-o output.txt`        |
| `--pages`       | `-p`  | Page range                 | `-p 1-5` or `-p 1,3,5` |
| `--format`      | `-f`  | Output format              | `-f json`              |
| `--verbose`     | `-v`  | Verbose output             | `-v`                   |
| `--pretty`      |       | Pretty JSON                | `--pretty`             |
| `--metadata`    | `-m`  | Include metadata           | `-m`                   |
| `--stream`      |       | Stream large files         | `--stream`             |
| `--css`         |       | CSS output (typeset)       | `--css`                |
| `--accessible`  |       | Accessible HTML (typeset)  | `--accessible`         |
| `--print-css`   |       | Print stylesheet (typeset) | `--print-css`          |
| `--social-meta` |       | OG/Twitter tags (typeset)  | `--social-meta`        |
| `--page-url`    |       | URL for social tags        | `--page-url https://…` |

## 🔥 Quick Examples

### Extract Text
```bash
npm run cli -- extract -i document.pdf -o output.txt
```

### Extract Pages 1-5
```bash
npm run cli -- extract -i document.pdf -p 1-5 -o pages.txt
```

### Convert to JSON
```bash
npm run cli -- convert -i document.pdf -f json --pretty -o data.json
```

### AI Analysis
```bash
npm run cli -- analyze -i document.pdf --ai --pretty -o analysis.json
```

### RAG Chunks
```bash
npm run cli -- chunk -i document.pdf --chunk-size 1000 -o chunks.json
```

### Extract Images
```bash
npm run cli -- images -i document.pdf -o ./images/ -v
```

### Stream Large PDF
```bash
npm run cli -- extract -i large.pdf --stream -o output.txt
```

### Typeset: CSS from Display Hints
```bash
npm run cli -- typeset -i paper.pdf --css -o styles.css
```

### Typeset: Responsive HTML Article
```bash
npm run cli -- typeset -i paper.pdf -o article.html
```

### Typeset: Accessible Reading View
```bash
npm run cli -- typeset -i paper.pdf --accessible -o readable.html
```

### Typeset: Print-Ready Stylesheet
```bash
npm run cli -- typeset -i paper.pdf --print-css -o print.css
```

### Typeset: Social Meta Tags
```bash
npm run cli -- typeset -i paper.pdf --social-meta --page-url https://example.com/paper
```

## 📤 Output Formats

- `text` - Plain text
- `json` - JSON format
- `html` - HTML format
- `markdown` - Markdown format

## 🔄 Batch Processing

### PowerShell
```powershell
Get-ChildItem *.pdf | ForEach-Object {
  npm run cli -- extract -i $_.Name -o "$($_.BaseName).txt"
}
```

### Bash
```bash
for pdf in *.pdf; do
  npm run cli -- extract -i "$pdf" -o "${pdf%.pdf}.txt"
done
```

## 🎨 Using Wrapper Scripts

### PowerShell
```powershell
.\agenticpdf-cli.ps1 extract -Input document.pdf -Output output.txt -Verbose
```

### Bash
```bash
./agenticpdf-cli.sh extract -i document.pdf -o output.txt -v
```

## 🔍 Page Ranges

- Single page: `-p 5`
- Range: `-p 1-10`
- Multiple: `-p 1,5,10`
- Mixed: `-p 1-5,10,15-20`

## 💡 Tips

1. Use `-v` for progress on long operations
2. Use `--pretty` for readable JSON
3. Use `--stream` for files > 50MB
4. Specify `-p` to process only needed pages
5. Use absolute paths to avoid issues

## 📖 Documentation

- **Quick Start**: `CLI_QUICKSTART.md`
- **Full Reference**: `CLI.md`
- **Status**: `CLI_STATUS.md`
- **Summary**: `CLI_SUMMARY.md`
- **Inline Help**: `npm run cli -- help`

## 🐛 Troubleshooting

### File not found
```bash
# Use absolute or correct relative path
npm run cli -- info "C:\path\to\file.pdf"
```

### Out of memory
```bash
# Use streaming mode
npm run cli -- extract -i large.pdf --stream -o output.txt
```

### Command not working
```bash
# Make sure to use -- before CLI args
npm run cli -- extract -i file.pdf
```

## 🎯 Common Workflows

### Document Analysis Pipeline
```bash
# 1. Get info
npm run cli -- info document.pdf

# 2. Extract text
npm run cli -- extract -i document.pdf -o text.txt

# 3. Analyze
npm run cli -- analyze -i document.pdf --ai -o analysis.json

# 4. Generate chunks
npm run cli -- chunk -i document.pdf -o chunks.json
```

### RAG Preparation
```bash
# Generate embedding-ready chunks
npm run cli -- chunk -i document.pdf \
  --chunk-size 1000 \
  --pretty \
  -o chunks.json
```

### Multi-format Export
```bash
npm run cli -- convert -i doc.pdf -f text -o doc.txt
npm run cli -- convert -i doc.pdf -f json --pretty -o doc.json
npm run cli -- convert -i doc.pdf -f html -o doc.html
npm run cli -- convert -i doc.pdf -f markdown -o doc.md
```

## 📞 Getting Help

- Run `npm run cli -- help`
- Check `CLI_QUICKSTART.md`
- Read `CLI.md` for details
- See examples in `examples/cli-examples.ps1`

## 🚀 Next Steps

1. Try: `npm run cli -- info demos/sample.pdf`
2. Read: `CLI_QUICKSTART.md`
3. Explore: `CLI.md`
4. Practice: Run examples from documentation

---

**Print this card for quick reference!**
