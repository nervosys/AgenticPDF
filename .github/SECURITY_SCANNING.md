# Security Scanning and CI/CD

This document outlines the security scanning and continuous integration measures implemented for AgenticPDF.

## 🛡️ Security Workflows

### Main Security Scan Workflow (`.github/workflows/security.yml`)

**Triggers:**
- Daily scheduled runs at 2 AM UTC
- On push to master/main branches
- On pull requests to master/main branches
- Manual workflow dispatch

**Security Scans Performed:**

1. **Dependency Security**
   - `npm audit` - Checks for known vulnerabilities in dependencies
   - Dependency review for pull requests
   - License compatibility checks

2. **Code Analysis**
   - **CodeQL Analysis** - GitHub's semantic code analysis engine
   - **Trivy Scanner** - Comprehensive vulnerability scanning
   - **Semgrep** - Static analysis for security bugs and code quality
   - **ESLint Security** - JavaScript/TypeScript security linting

3. **Secret Detection**
   - **TruffleHog OSS** - Git repository secret scanner
   - **GitLeaks** - Detects secrets in git repos, commits, and more
   - File pattern analysis for sensitive files

4. **License Compliance**
   - Automated license checking for all dependencies
   - Ensures only approved licenses are used

### Dependency Security Workflow (`.github/workflows/dependency-security.yml`)

**Triggers:**
- Weekly schedule (Mondays at 9 AM UTC)
- Manual workflow dispatch

**Features:**
- Checks for outdated dependencies
- Runs security audits
- Generates dependency reports
- Stores reports as artifacts for review

## 🔧 Configuration Files

### CodeQL Configuration (`.github/codeql/codeql-config.yml`)
- Configures GitHub's CodeQL analysis
- Defines query suites and scan paths
- Excludes test files and dependencies from scanning

### Security Configuration (`.github/security-config.yml`)
- Centralized security policy configuration
- Defines severity levels and exclusions
- Lists allowed licenses and security policies

## 🎯 Security Scan Coverage

### Code Security
- ✅ Static code analysis (CodeQL, Semgrep)
- ✅ Vulnerability scanning (Trivy)
- ✅ Security linting (ESLint)
- ✅ Dependency vulnerability checks (npm audit)

### Secret Detection
- ✅ Git history scanning (TruffleHog)
- ✅ Secret pattern detection (GitLeaks)
- ✅ File pattern analysis for sensitive data

### Compliance
- ✅ License compatibility verification
- ✅ Dependency security reviews
- ✅ SARIF report generation for GitHub Security tab

## 📊 Security Reporting

### Automated Reports
- **SARIF Files** - Uploaded to GitHub Security tab
- **Dependency Reports** - Weekly artifact generation
- **Audit Logs** - Complete scan history in workflow logs

### Manual Review Points
- Pull request dependency reviews
- Security issue triage in GitHub Security tab
- Weekly dependency report analysis

## 🚨 Security Response

### Vulnerability Discovery
1. Automated scans detect potential issues
2. SARIF reports uploaded to GitHub Security tab
3. Security alerts generated for critical findings
4. Team notification through GitHub notifications

### Incident Response
1. Review security alerts in GitHub Security tab
2. Assess severity and impact
3. Apply fixes or mitigations
4. Re-run security scans to verify fixes

## 🔍 Manual Security Checks

### Before Release
- [ ] Run full security scan manually
- [ ] Review dependency audit results
- [ ] Check for secrets in commit history
- [ ] Verify license compliance
- [ ] Review CodeQL analysis results

### Regular Maintenance
- [ ] Weekly dependency report review
- [ ] Monthly security workflow validation
- [ ] Quarterly security configuration updates
- [ ] Annual security policy review

## 🛠️ Security Tools Used

| Tool       | Purpose                    | Frequency     |
| ---------- | -------------------------- | ------------- |
| CodeQL     | Static code analysis       | Every push/PR |
| Trivy      | Vulnerability scanning     | Every push/PR |
| Semgrep    | Security bug detection     | Every push/PR |
| TruffleHog | Secret detection           | Every push/PR |
| GitLeaks   | Git secret scanning        | Every push/PR |
| npm audit  | Dependency vulnerabilities | Every push/PR |
| ESLint     | Code quality & security    | Every push/PR |

## 🔐 Best Practices

### Development
- Run `npm audit` before committing
- Use `npm audit fix` to resolve vulnerabilities
- Review security warnings in IDE
- Avoid committing sensitive information

### CI/CD
- All security scans must pass before merge
- Regular security workflow updates
- Monitor GitHub Security tab for alerts
- Keep security tools up to date

### Maintenance
- Regular dependency updates
- Security patch prioritization
- Continuous monitoring of security advisories
- Documentation updates for new threats

## 📞 Security Contact

For security issues and vulnerabilities:
- **Private disclosure**: Use GitHub Security Advisories
- **Email**: security@nervosys.com
- **Public issues**: Only for non-security bugs

See [SECURITY.md](SECURITY.md) for complete security policy and reporting guidelines.
