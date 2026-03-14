# Installing AgenticPDF CLI (mpdf)

## 📦 Installation Methods

### Global Installation (Recommended)

Install globally to use the `mpdf` command anywhere:

```bash
npm install -g agenticpdf
```

After installation, you can use the CLI from anywhere:

```bash
mpdf --version
mpdf help
mpdf info document.pdf
```

### Local Project Installation

Install as a project dependency:

```bash
npm install agenticpdf
```

Then use via npx or npm scripts:

```bash
# Using npx
npx mpdf info document.pdf

# Or add to package.json scripts
{
  "scripts": {
    "pdf-info": "mpdf info"
  }
}
```

### Direct from GitHub

Install the latest version from GitHub:

```bash
npm install -g nervosys/agenticpdf
```

## 🚀 Quick Start After Installation

### Global Installation

```bash
# Check installation
mpdf --version

# Show help
mpdf help

# Display PDF info
mpdf info document.pdf

# Extract text
mpdf extract -i document.pdf -o output.txt
```

### Local Installation

```bash
# Using npx
npx mpdf --version
npx mpdf help
npx mpdf info document.pdf

# Using npm scripts (add to package.json)
npm run pdf-info document.pdf
```

## 🎯 Command Aliases

Both commands work identically:

```bash
# Short command (recommended)
mpdf info document.pdf

# Full command (also works)
agenticpdf info document.pdf
```

## 📚 Verify Installation

Test your installation:

```bash
# 1. Check version
mpdf --version
# Expected: AgenticPDF CLI v1.0.0

# 2. Show help
mpdf help
# Should display full help menu

# 3. Test with a sample PDF (if you have one)
mpdf info /path/to/sample.pdf
```

## 🔧 Troubleshooting

### Command Not Found

If `mpdf` is not found after global installation:

1. **Check npm global bin path:**
   ```bash
   npm bin -g
   ```

2. **Add to PATH (if needed):**
   
   **Windows (PowerShell):**
   ```powershell
   $env:PATH += ";$(npm bin -g)"
   ```
   
   **Linux/macOS (Bash):**
   ```bash
   export PATH="$(npm bin -g):$PATH"
   ```

3. **Reinstall globally:**
   ```bash
   npm uninstall -g agenticpdf
   npm install -g agenticpdf
   ```

### Permission Errors (Linux/macOS)

If you get permission errors:

```bash
# Option 1: Use sudo (not recommended)
sudo npm install -g agenticpdf

# Option 2: Fix npm permissions (recommended)
mkdir ~/.npm-global
npm config set prefix '~/.npm-global'
echo 'export PATH=~/.npm-global/bin:$PATH' >> ~/.profile
source ~/.profile
npm install -g agenticpdf
```

### TypeScript/tsx Not Found

If you see "tsx not found" errors:

```bash
# tsx is included as a dependency, but you can install globally too
npm install -g tsx

# Then try mpdf again
mpdf --version
```

## 🌐 Platform-Specific Notes

### Windows

```powershell
# Install globally
npm install -g agenticpdf

# Use anywhere
mpdf info document.pdf

# If command not found, restart terminal
```

### macOS

```bash
# Install globally
npm install -g agenticpdf

# Use anywhere
mpdf info document.pdf

# May need to add to PATH in ~/.zshrc or ~/.bash_profile
```

### Linux

```bash
# Install globally
npm install -g agenticpdf

# Use anywhere
mpdf info document.pdf

# May need to fix npm permissions (see troubleshooting)
```

## 📦 What Gets Installed

When you install AgenticPDF, you get:

- **CLI executables:** `mpdf` and `agenticpdf` commands
- **TypeScript library:** Full AgenticPDF library for programmatic use
- **Dependencies:** tsx for running TypeScript files
- **Documentation:** Built-in help and online docs

## 🔄 Updating

### Update Global Installation

```bash
npm update -g agenticpdf
```

### Update Local Installation

```bash
npm update agenticpdf
```

### Check for Updates

```bash
npm outdated -g agenticpdf
```

## 🗑️ Uninstalling

### Remove Global Installation

```bash
npm uninstall -g agenticpdf
```

### Remove Local Installation

```bash
npm uninstall agenticpdf
```

## 💡 Usage Examples

### After Global Installation

```bash
# Display PDF information
mpdf info document.pdf

# Extract text to file
mpdf extract -i document.pdf -o output.txt

# Convert to JSON
mpdf convert -i document.pdf -f json --pretty -o output.json

# Generate RAG chunks
mpdf chunk -i document.pdf --chunk-size 1000 -o chunks.json

# Extract images
mpdf images -i document.pdf -o ./images/

# AI analysis
mpdf analyze -i document.pdf --ai --pretty
```

### After Local Installation

```bash
# Using npx prefix
npx mpdf info document.pdf
npx mpdf extract -i document.pdf -o output.txt
npx mpdf convert -i document.pdf -f json --pretty

# Or via package.json scripts
{
  "scripts": {
    "pdf:info": "mpdf info",
    "pdf:extract": "mpdf extract -i",
    "pdf:convert": "mpdf convert -f json"
  }
}
```

## 🎓 Next Steps

After installation:

1. **Read Quick Start:** `CLI_QUICKSTART.md`
2. **View Examples:** `mpdf help`
3. **Try Commands:** Start with `mpdf info` on a sample PDF
4. **Read Full Docs:** `CLI.md` for complete reference
5. **Check Reference Card:** `CLI_REFERENCE.md` for quick lookup

## 📞 Getting Help

- **CLI Help:** `mpdf help`
- **Command Help:** `mpdf <command> --help`
- **Documentation:** Check `CLI.md` and `CLI_QUICKSTART.md`
- **Issues:** Report at https://github.com/nervosys/agenticpdf/issues

## 🎯 Common Workflows

### Document Processing Pipeline

```bash
# Install globally
npm install -g agenticpdf

# Process documents
mpdf info document.pdf
mpdf extract -i document.pdf -o text.txt
mpdf analyze -i document.pdf --ai -o analysis.json
mpdf chunk -i document.pdf -o chunks.json
```

### Project Integration

```bash
# Install in project
npm install agenticpdf

# Add to package.json
{
  "scripts": {
    "process-pdf": "mpdf extract -i input.pdf -o output.txt",
    "analyze-pdf": "mpdf analyze -i input.pdf --ai -o analysis.json"
  }
}

# Run via npm
npm run process-pdf
npm run analyze-pdf
```

## ✅ Installation Checklist

- [ ] Install Node.js 18+ (check: `node --version`)
- [ ] Install npm (check: `npm --version`)
- [ ] Install AgenticPDF globally: `npm install -g agenticpdf`
- [ ] Verify installation: `mpdf --version`
- [ ] Test help command: `mpdf help`
- [ ] Try with a sample PDF: `mpdf info sample.pdf`
- [ ] Read quick start guide
- [ ] Start processing PDFs!

---

**Ready to process PDFs?** Try:

```bash
mpdf --version
mpdf help
```
