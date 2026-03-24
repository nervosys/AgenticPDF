# Installing AgenticPDF CLI (apdf)

## 📦 Installation Methods

### Global Installation (Recommended)

Install globally to use the `apdf` command anywhere:

```bash
npm install -g agenticpdf
```

After installation, you can use the CLI from anywhere:

```bash
apdf --version
apdf help
apdf info document.pdf
```

### Local Project Installation

Install as a project dependency:

```bash
npm install agenticpdf
```

Then use via npx or npm scripts:

```bash
# Using npx
npx apdf info document.pdf

# Or add to package.json scripts
{
  "scripts": {
    "pdf-info": "apdf info"
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
apdf --version

# Show help
apdf help

# Display PDF info
apdf info document.pdf

# Extract text
apdf extract -i document.pdf -o output.txt
```

### Local Installation

```bash
# Using npx
npx apdf --version
npx apdf help
npx apdf info document.pdf

# Using npm scripts (add to package.json)
npm run pdf-info document.pdf
```

## 🎯 Command Aliases

Both commands work identically:

```bash
# Short command (recommended)
apdf info document.pdf

# Full command (also works)
agenticpdf info document.pdf
```

## 📚 Verify Installation

Test your installation:

```bash
# 1. Check version
apdf --version
# Expected: AgenticPDF CLI v1.0.0

# 2. Show help
apdf help
# Should display full help menu

# 3. Test with a sample PDF (if you have one)
apdf info /path/to/sample.pdf
```

## 🔧 Troubleshooting

### Command Not Found

If `apdf` is not found after global installation:

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

# Then try apdf again
apdf --version
```

## 🌐 Platform-Specific Notes

### Windows

```powershell
# Install globally
npm install -g agenticpdf

# Use anywhere
apdf info document.pdf

# If command not found, restart terminal
```

### macOS

```bash
# Install globally
npm install -g agenticpdf

# Use anywhere
apdf info document.pdf

# May need to add to PATH in ~/.zshrc or ~/.bash_profile
```

### Linux

```bash
# Install globally
npm install -g agenticpdf

# Use anywhere
apdf info document.pdf

# May need to fix npm permissions (see troubleshooting)
```

## 📦 What Gets Installed

When you install AgenticPDF, you get:

- **CLI executables:** `apdf` and `agenticpdf` commands
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
apdf info document.pdf

# Extract text to file
apdf extract -i document.pdf -o output.txt

# Convert to JSON
apdf convert -i document.pdf -f json --pretty -o output.json

# Generate RAG chunks
apdf chunk -i document.pdf --chunk-size 1000 -o chunks.json

# Extract images
apdf images -i document.pdf -o ./images/

# AI analysis
apdf analyze -i document.pdf --ai --pretty
```

### After Local Installation

```bash
# Using npx prefix
npx apdf info document.pdf
npx apdf extract -i document.pdf -o output.txt
npx apdf convert -i document.pdf -f json --pretty

# Or via package.json scripts
{
  "scripts": {
    "pdf:info": "apdf info",
    "pdf:extract": "apdf extract -i",
    "pdf:convert": "apdf convert -f json"
  }
}
```

## 🎓 Next Steps

After installation:

1. **Read Quick Start:** `CLI_QUICKSTART.md`
2. **View Examples:** `apdf help`
3. **Try Commands:** Start with `apdf info` on a sample PDF
4. **Read Full Docs:** `CLI.md` for complete reference
5. **Check Reference Card:** `CLI_REFERENCE.md` for quick lookup

## 📞 Getting Help

- **CLI Help:** `apdf help`
- **Command Help:** `apdf <command> --help`
- **Documentation:** Check `CLI.md` and `CLI_QUICKSTART.md`
- **Issues:** Report at https://github.com/nervosys/agenticpdf/issues

## 🎯 Common Workflows

### Document Processing Pipeline

```bash
# Install globally
npm install -g agenticpdf

# Process documents
apdf info document.pdf
apdf extract -i document.pdf -o text.txt
apdf analyze -i document.pdf --ai -o analysis.json
apdf chunk -i document.pdf -o chunks.json
```

### Project Integration

```bash
# Install in project
npm install agenticpdf

# Add to package.json
{
  "scripts": {
    "process-pdf": "apdf extract -i input.pdf -o output.txt",
    "analyze-pdf": "apdf analyze -i input.pdf --ai -o analysis.json"
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
- [ ] Verify installation: `apdf --version`
- [ ] Test help command: `apdf help`
- [ ] Try with a sample PDF: `apdf info sample.pdf`
- [ ] Read quick start guide
- [ ] Start processing PDFs!

---

**Ready to process PDFs?** Try:

```bash
apdf --version
apdf help
```
