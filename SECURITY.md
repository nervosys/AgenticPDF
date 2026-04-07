# Security Policy

## Supported Versions

We actively support the following versions of AgenticPDF with security updates:

| Version | Supported          |
| ------- | ------------------ |
| 1.x.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

The AgenticPDF team takes security seriously. If you discover a security vulnerability, please follow these steps:

### 🔒 Private Disclosure

**DO NOT** create a public GitHub issue for security vulnerabilities.

Instead, please report security issues privately by:

1. **Email**: Send details to [security@nervosys.ai](mailto:security@nervosys.ai)
2. **GitHub Security**: Use [GitHub's private vulnerability reporting](https://github.com/nervosys/AgenticPDF/security/advisories/new)

### 📝 What to Include

When reporting a vulnerability, please include:

- **Description**: Clear description of the vulnerability
- **Impact**: Potential impact and severity assessment
- **Reproduction**: Step-by-step instructions to reproduce
- **Environment**: Version, platform, and configuration details
- **Proof of Concept**: Code or screenshots demonstrating the issue
- **Suggested Fix**: If you have ideas for remediation

### ⏱️ Response Timeline

We commit to the following response times:

- **Initial Response**: Within 48 hours
- **Assessment**: Within 5 business days
- **Fix Timeline**: Based on severity (see below)
- **Disclosure**: Coordinated disclosure after fix is available

### 🚨 Severity Levels

| Severity     | Description                                   | Response Time |
| ------------ | --------------------------------------------- | ------------- |
| **Critical** | Remote code execution, data breach            | 24-48 hours   |
| **High**     | Privilege escalation, sensitive data exposure | 3-7 days      |
| **Medium**   | Information disclosure, DoS                   | 1-2 weeks     |
| **Low**      | Minor security improvements                   | Next release  |

## 🛡️ Security Best Practices

When using AgenticPDF:

### Input Validation

```typescript
// Always validate PDF sources
const trustedSources = ['https://trusted-domain.com', 'https://internal.company.com'];

function validatePDFSource(url: string): boolean {
  try {
    const parsed = new URL(url);
    return trustedSources.some(trusted => parsed.origin === trusted);
  } catch {
    return false;
  }
}

// Use validation before processing
if (validatePDFSource(pdfUrl)) {
  const pdf = await AgenticPDF.fromUrl(pdfUrl);
  // Process safely...
}
```

### Memory Limits

```typescript
// Set memory limits to prevent DoS
const pdf = await AgenticPDF.fromFile(file, {
  maxMemoryUsage: 50 * 1024 * 1024, // 50MB limit
  maxProcessingTime: 30000 // 30 second timeout
});
```

### Content Sanitization

```typescript
// Sanitize extracted content before displaying
function sanitizeText(text: string): string {
  return text
    .replace(/<script[^>]*>.*?<\/script>/gi, '')
    .replace(/javascript:/gi, '')
    .replace(/on\w+\s*=/gi, '');
}

const text = await pdf.extractText();
const safeText = sanitizeText(text);
```

## 🔍 Security Features

AgenticPDF includes several built-in security features:

### Safe PDF Processing

- **No Script Execution**: PDFs are processed as data, never executed
- **Memory Protection**: Configurable limits prevent memory exhaustion
- **Input Validation**: Built-in checks for malformed PDF structures
- **Sandboxed Workers**: Web Workers provide isolation for processing

### Data Protection

- **No Data Persistence**: No automatic caching or storage of PDF content
- **Cleanup Methods**: Explicit memory cleanup via `pdf.close()`
- **Stream Processing**: Large files processed in chunks, not loaded entirely

### Network Security

- **HTTPS Only**: Encourage secure connections for URL-based loading
- **No External Requests**: Library doesn't make unexpected network calls
- **Configurable Timeouts**: Prevent hanging network requests

## 🚫 Known Limitations

Please be aware of these security considerations:

### PDF Complexity

- Very large or complex PDFs may cause high memory usage
- Malformed PDFs might cause processing errors (but not security issues)
- Embedded files in PDFs are extracted as data only

### Browser Environment

- File system access limited by browser security model
- Web Workers may have different security contexts
- Content Security Policy may affect functionality

## 📋 Security Checklist

For applications using AgenticPDF:

- [ ] Validate all PDF sources and inputs
- [ ] Set appropriate memory and processing limits
- [ ] Sanitize extracted content before display
- [ ] Use HTTPS for URL-based PDF loading
- [ ] Implement proper error handling
- [ ] Keep AgenticPDF updated to latest version
- [ ] Monitor for security advisories
- [ ] Test with malicious/malformed PDF samples

## 🔄 Security Updates

### Notification

- Security updates are announced via GitHub Security Advisories
- Critical updates may include immediate notifications
- Subscribe to releases for security update notifications

### Update Process

```bash
# Check for updates
npm outdated AgenticPDF

# Update to latest secure version
npm update AgenticPDF

# Verify update
npm ls AgenticPDF
```

## 📞 Contact

For security-related questions or concerns:

- **Security Email**: [security@nervosys.ai](mailto:security@nervosys.ai)
- **General Issues**: [GitHub Issues](https://github.com/nervosys/AgenticPDF/issues)
- **Discussions**: [GitHub Discussions](https://github.com/nervosys/AgenticPDF/discussions)

## 🏆 Responsible Disclosure

We appreciate security researchers who:

- Follow responsible disclosure practices
- Provide clear reproduction steps
- Allow reasonable time for fixes before public disclosure
- Work with us to minimize user impact

### Hall of Fame

We maintain a list of security researchers who have helped improve AgenticPDF security. Contributors will be acknowledged (with permission) in:

- Security advisory credits
- Repository contributors list
- Annual security report

---

**Thank you for helping keep AgenticPDF and its users safe!** 🛡️
