# Installing IronDocuments CLI (apdf)

## 📦 Installation Methods

### Global Installation (Recommended)

Install globally to use the `apdf` command anywhere:

```bash
npm install -g irondocuments
```

After installation, you can use the CLI from anywhere:

```bash
irondoc --version
irondoc help
irondoc info document.pdf
```

### Local Project Installation

Install as a project dependency:

```bash
npm install irondocuments
```

Then use via npx or npm scripts:

```bash
# Using npx
npx irondoc info document.pdf

# Or add to package.json scripts
{
  "scripts": {
    "pdf-info": "irondoc info"
  }
}
```

### Direct from GitHub

Install the latest version from GitHub:

```bash
npm install -g nervosys/irondocuments
```

## 🚀 Quick Start After Installation

### Global Installation

```bash
# Check installation
irondoc --version

# Show help
irondoc help

# Display PDF info
irondoc info document.pdf

# Extract text
irondoc extract -i document.pdf -o output.txt
```

### Local Installation

```bash
# Using npx
npx irondoc --version
npx irondoc help
npx irondoc info document.pdf

# Using npm scripts (add to package.json)
npm run pdf-info document.pdf
```

## 🎯 Command Aliases

Both commands work identically:

```bash
# Short command (recommended)
irondoc info document.pdf

# Full command (also works)
idoc info document.pdf
```

## 📚 Verify Installation

Test your installation:

```bash
# 1. Check version
irondoc --version
# Expected: IronDocuments CLI v1.0.0

# 2. Show help
irondoc help
# Should display full help menu

# 3. Test with a sample PDF (if you have one)
irondoc info /path/to/sample.pdf
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
   npm uninstall -g irondocuments
   npm install -g irondocuments
   ```

### Permission Errors (Linux/macOS)

If you get permission errors:

```bash
# Option 1: Use sudo (not recommended)
sudo npm install -g irondocuments

# Option 2: Fix npm permissions (recommended)
mkdir ~/.npm-global
npm config set prefix '~/.npm-global'
echo 'export PATH=~/.npm-global/bin:$PATH' >> ~/.profile
source ~/.profile
npm install -g irondocuments
```

### TypeScript/tsx Not Found

If you see "tsx not found" errors:

```bash
# tsx is included as a dependency, but you can install globally too
npm install -g tsx

# Then try irondoc again
irondoc --version
```

## 🌐 Platform-Specific Notes

### Windows

```powershell
# Install globally
npm install -g irondocuments

# Use anywhere
irondoc info document.pdf

# If command not found, restart terminal
```

### macOS

```bash
# Install globally
npm install -g irondocuments

# Use anywhere
irondoc info document.pdf

# May need to add to PATH in ~/.zshrc or ~/.bash_profile
```

### Linux

```bash
# Install globally
npm install -g irondocuments

# Use anywhere
irondoc info document.pdf

# May need to fix npm permissions (see troubleshooting)
```

## 📦 What Gets Installed

When you install IronDocuments, you get:

- **CLI executables:** `apdf` and `irondocuments` commands
- **TypeScript library:** Full IronDocuments library for programmatic use
- **Dependencies:** tsx for running TypeScript files
- **Documentation:** Built-in help and online docs

## 🔄 Updating

### Update Global Installation

```bash
npm update -g irondocuments
```

### Update Local Installation

```bash
npm update irondocuments
```

### Check for Updates

```bash
npm outdated -g irondocuments
```

## 🗑️ Uninstalling

### Remove Global Installation

```bash
npm uninstall -g irondocuments
```

### Remove Local Installation

```bash
npm uninstall irondocuments
```

## 💡 Usage Examples

### After Global Installation

```bash
# Display PDF information
irondoc info document.pdf

# Extract text to file
irondoc extract -i document.pdf -o output.txt

# Convert to JSON
irondoc convert -i document.pdf -f json --pretty -o output.json

# Generate RAG chunks
irondoc chunk -i document.pdf --chunk-size 1000 -o chunks.json

# Extract images
irondoc images -i document.pdf -o ./images/

# AI analysis
irondoc analyze -i document.pdf --ai --pretty
```

### After Local Installation

```bash
# Using npx prefix
npx irondoc info document.pdf
npx irondoc extract -i document.pdf -o output.txt
npx irondoc convert -i document.pdf -f json --pretty

# Or via package.json scripts
{
  "scripts": {
    "pdf:info": "irondoc info",
    "pdf:extract": "irondoc extract -i",
    "pdf:convert": "irondoc convert -f json"
  }
}
```

## 🎓 Next Steps

After installation:

1. **Read Quick Start:** `CLI_QUICKSTART.md`
2. **View Examples:** `irondoc help`
3. **Try Commands:** Start with `irondoc info` on a sample PDF
4. **Read Full Docs:** `CLI.md` for complete reference
5. **Check Reference Card:** `CLI_REFERENCE.md` for quick lookup

## 📞 Getting Help

- **CLI Help:** `irondoc help`
- **Command Help:** `irondoc <command> --help`
- **Documentation:** Check `CLI.md` and `CLI_QUICKSTART.md`
- **Issues:** Report at https://github.com/nervosys/IronDocuments/issues

## 🎯 Common Workflows

### Document Processing Pipeline

```bash
# Install globally
npm install -g irondocuments

# Process documents
irondoc info document.pdf
irondoc extract -i document.pdf -o text.txt
irondoc analyze -i document.pdf --ai -o analysis.json
irondoc chunk -i document.pdf -o chunks.json
```

### Project Integration

```bash
# Install in project
npm install irondocuments

# Add to package.json
{
  "scripts": {
    "process-pdf": "irondoc extract -i input.pdf -o output.txt",
    "analyze-pdf": "irondoc analyze -i input.pdf --ai -o analysis.json"
  }
}

# Run via npm
npm run process-pdf
npm run analyze-pdf
```

## ✅ Installation Checklist

- [ ] Install Node.js 18+ (check: `node --version`)
- [ ] Install npm (check: `npm --version`)
- [ ] Install IronDocuments globally: `npm install -g irondocuments`
- [ ] Verify installation: `irondoc --version`
- [ ] Test help command: `irondoc help`
- [ ] Try with a sample PDF: `irondoc info sample.pdf`
- [ ] Read quick start guide
- [ ] Start processing PDFs!

---

**Ready to process PDFs?** Try:

```bash
irondoc --version
irondoc help
```
