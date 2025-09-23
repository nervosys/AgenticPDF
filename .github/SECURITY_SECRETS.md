# Security Workflow Configuration Guide

This guide explains the different security workflow options and the secrets they use.

## 🔐 GitHub Secrets Used

### Required Secrets
- `GITHUB_TOKEN` - **Automatically provided by GitHub** - No setup needed

### Optional Secrets
- `SEMGREP_APP_TOKEN` - For enhanced Semgrep features (optional)
- `GITLEAKS_LICENSE` - For GitLeaks Pro features (optional)

## 📋 Workflow Options

### Option 1: Full Security Workflow (`security.yml`)
**Includes:**
- ✅ CodeQL Analysis (GitHub native)
- ✅ Trivy Vulnerability Scanner
- ✅ Semgrep Static Analysis (requires token for full features)
- ✅ GitLeaks Secret Scanning (works without license)
- ✅ TruffleHog Secret Detection
- ✅ npm audit
- ✅ License checking
- ✅ Dependency review

**Pros:**
- Most comprehensive security coverage
- Professional-grade scanning tools
- Enhanced rule sets with tokens

**Cons:**
- May require optional secrets for full functionality
- Some tools may fail without tokens (but workflow continues)

### Option 2: Simplified Security Workflow (`security-simple.yml`)
**Includes:**
- ✅ CodeQL Analysis (GitHub native)
- ✅ Trivy Vulnerability Scanner
- ✅ npm audit
- ✅ Basic secret pattern detection
- ✅ License checking
- ✅ Dependency review

**Pros:**
- No external secrets required
- All tools work out of the box
- Covers most security needs

**Cons:**
- Less comprehensive secret detection
- Basic static analysis only

## 🛠️ Setting Up Optional Secrets

### If you want to use `SEMGREP_APP_TOKEN`:

1. **Sign up for Semgrep:**
   - Go to [semgrep.dev](https://semgrep.dev)
   - Create a free account

2. **Get your token:**
   - Navigate to Settings → Tokens
   - Create a new token for your repository

3. **Add to GitHub:**
   - Go to your GitHub repository
   - Settings → Secrets and variables → Actions
   - Click "New repository secret"
   - Name: `SEMGREP_APP_TOKEN`
   - Value: Your token from Semgrep

### If you want to use `GITLEAKS_LICENSE`:

1. **Purchase GitLeaks Pro:**
   - Visit [gitleaks.io](https://gitleaks.io)
   - Purchase a license (only if you need pro features)

2. **Add to GitHub:**
   - Go to your GitHub repository
   - Settings → Secrets and variables → Actions
   - Click "New repository secret"
   - Name: `GITLEAKS_LICENSE`
   - Value: Your license key

## 🚀 Recommended Setup

### For Most Projects:
**Use `security-simple.yml`** - rename it to `security.yml`
- No secrets needed
- Comprehensive coverage
- Easy to maintain

### For Enterprise/High-Security Projects:
**Use the full `security.yml`** and set up the optional secrets
- Maximum security coverage
- Professional tooling
- Enhanced reporting

## 🔄 Switching Between Workflows

### To use the simplified version:
```bash
# Rename the current file
mv .github/workflows/security.yml .github/workflows/security-full.yml

# Use the simplified version
mv .github/workflows/security-simple.yml .github/workflows/security.yml
```

### To use the full version:
```bash
# Set up the optional secrets in GitHub (see above)
# The current security.yml will work with continue-on-error for missing secrets
```

## ⚠️ Important Notes

1. **GitHub Token:** The `GITHUB_TOKEN` is automatically provided by GitHub Actions - no setup needed

2. **Optional Secrets:** The workflow will continue even if optional secrets are missing (thanks to `continue-on-error: true`)

3. **Free Tools:** CodeQL, Trivy, npm audit, and basic GitLeaks work without any secrets

4. **Security Tab:** All SARIF results are uploaded to GitHub's Security tab regardless of secrets

## 🧪 Testing Your Setup

Run this command to validate your workflows:
```bash
npm run validate:workflows
```

Or test security locally:
```bash
npm run security:audit
```

## 📊 What You Get Without Secrets

Even without any optional secrets, you still get:
- ✅ GitHub CodeQL analysis
- ✅ Trivy vulnerability scanning  
- ✅ npm dependency auditing
- ✅ License compliance checking
- ✅ Basic secret pattern detection
- ✅ GitHub Security tab integration
- ✅ Pull request dependency reviews

This provides excellent security coverage for most projects!