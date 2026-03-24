# AgenticPDF Scripts

Utility scripts for development, testing, and demonstration.

## 📜 Available Scripts

### CLI Testing Scripts

#### `agenticpdf-cli.ps1`
PowerShell wrapper for testing the CLI on Windows.

**Usage:**
```powershell
.\scripts\agenticpdf-cli.ps1 info -i demos\sample.pdf
```

#### `agenticpdf-cli.sh`
Bash wrapper for testing the CLI on Unix/Linux/macOS.

**Usage:**
```bash
./scripts/agenticpdf-cli.sh info -i demos/sample.pdf
```

#### `test-cli.js`
Node.js script for automated CLI testing.

**Usage:**
```bash
node scripts/test-cli.js
```

### Example Scripts

#### `run-examples.ts`
TypeScript script to run all library examples.

**Usage:**
```bash
npx tsx scripts/run-examples.ts
```

#### `run-examples-simple.ts`
Simplified version of the examples runner.

**Usage:**
```bash
npx tsx scripts/run-examples-simple.ts
```

#### `run-examples.sh`
Bash script to run examples on Unix/Linux/macOS.

**Usage:**
```bash
./scripts/run-examples.sh
```

#### `run-examples.bat`
Batch script to run examples on Windows.

**Usage:**
```cmd
scripts\run-examples.bat
```

### Development Scripts

#### `validate-workflows.cjs`
Validates GitHub Actions workflow files for syntax and best practices.

**Usage:**
```bash
node scripts/validate-workflows.cjs
```

## 🚀 Quick Start

### Run All Examples
```bash
# TypeScript (recommended)
npx tsx scripts/run-examples.ts

# Or use shell scripts
./scripts/run-examples.sh          # Unix/Linux/macOS
scripts\run-examples.bat           # Windows
```

### Test CLI Locally
```bash
# PowerShell (Windows)
.\scripts\agenticpdf-cli.ps1 help

# Bash (Unix/Linux/macOS)
./scripts/agenticpdf-cli.sh help
```

### Validate Workflows
```bash
node scripts/validate-workflows.cjs
```

## 📝 Script Descriptions

| Script                   | Language   | Purpose                       |
| ------------------------ | ---------- | ----------------------------- |
| `agenticpdf-cli.ps1`     | PowerShell | CLI testing wrapper (Windows) |
| `agenticpdf-cli.sh`      | Bash       | CLI testing wrapper (Unix)    |
| `test-cli.js`            | Node.js    | Automated CLI tests           |
| `run-examples.ts`        | TypeScript | Run all library examples      |
| `run-examples-simple.ts` | TypeScript | Simple examples runner        |
| `run-examples.sh`        | Bash       | Examples runner (Unix)        |
| `run-examples.bat`       | Batch      | Examples runner (Windows)     |
| `validate-workflows.cjs` | Node.js    | GitHub Actions validator      |

## 🛠️ Development Workflow

### Testing the CLI During Development
```bash
# 1. Make changes to cli.ts
# 2. Test locally without installing
.\scripts\agenticpdf-cli.ps1 info -i demos\sample.pdf

# 3. Run full test suite
npm test
```

### Running Examples
```bash
# Quick test of all examples
npx tsx scripts/run-examples-simple.ts

# Full examples with detailed output
npx tsx scripts/run-examples.ts
```

### Validating CI/CD
```bash
# Before committing workflow changes
node scripts/validate-workflows.cjs
```

## 📦 NPM Scripts

These scripts are also integrated into `package.json`:

```json
{
  "scripts": {
    "examples": "tsx scripts/run-examples.ts",
    "examples:simple": "tsx scripts/run-examples-simple.ts",
    "validate:workflows": "node scripts/validate-workflows.cjs",
    "test:cli": "node scripts/test-cli.js"
  }
}
```

**Usage:**
```bash
npm run examples
npm run examples:simple
npm run validate:workflows
npm run test:cli
```

## 🔧 Requirements

- **Node.js** 18+ for all scripts
- **tsx** for TypeScript scripts (`npm install -g tsx`)
- **PowerShell** 5.1+ for `.ps1` scripts (Windows)
- **Bash** for `.sh` scripts (Unix/Linux/macOS)

## 🆘 Troubleshooting

### Permission Denied (Unix/Linux/macOS)
```bash
chmod +x scripts/*.sh
```

### Execution Policy (Windows PowerShell)
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### TypeScript Not Found
```bash
npm install -g tsx
```

## 📚 Related Documentation

- [CLI Documentation](../docs/cli/)
- [Examples](../examples/)
- [Contributing Guide](../CONTRIBUTING.md)
