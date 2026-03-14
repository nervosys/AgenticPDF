# Contributing to AgenticPDF

Thank you for your interest in contributing to AgenticPDF! This document provides guidelines and information about contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Commit Guidelines](#commit-guidelines)
- [Issue Reporting](#issue-reporting)

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Set up the development environment
4. Create a new branch for your feature or bug fix
5. Make your changes
6. Test your changes
7. Submit a pull request

## Development Setup

### Prerequisites

- Node.js 18.0.0 or higher
- npm 8.0.0 or higher

### Installation

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/agenticpdf.git
cd agenticpdf

# Install dependencies
npm install

# Run tests to verify setup
npm test
```

### Available Scripts

- `npm test` - Run the test suite
- `npm run test:watch` - Run tests in watch mode
- `npm run test:coverage` - Run tests with coverage report
- `npm run lint` - Run ESLint
- `npm run lint:fix` - Run ESLint with auto-fix
- `npm run typecheck` - Run TypeScript type checking
- `npm run build` - Build the project
- `npm run ci` - Run the full CI pipeline locally

## Making Changes

### Branch Naming

Use descriptive branch names with prefixes:

- `feature/` - New features
- `bugfix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test improvements

Examples:
- `feature/add-table-extraction`
- `bugfix/fix-memory-leak`
- `docs/update-api-examples`

### Architecture Considerations

AgenticPDF is designed as a single-file TypeScript library. When contributing:

1. **Single File Design**: All core functionality should remain in `agenticpdf.ts`
2. **No Runtime Dependencies**: The library should remain dependency-free
3. **Streaming-First**: New features should support streaming when applicable
4. **Memory Efficiency**: Consider memory usage for large PDF processing
5. **AI-Ready**: New features should integrate well with AI/ML workflows
6. **Browser & Server**: Ensure compatibility with both environments

## Testing

### Writing Tests

- Write tests for all new features and bug fixes
- Tests are located in the `tests/` directory
- Use descriptive test names that explain the behavior being tested
- Follow the existing test structure:
  - `tests/unit/` - Unit tests for individual components
  - `tests/integration/` - Integration tests for full workflows
  - `tests/mocks/` - Mock utilities and test fixtures

### Test Guidelines

```typescript
// Good test naming
test('should extract text with formatting when preserveFormatting is true', async () => {
  // Test implementation
});

// Include edge cases
test('should handle empty PDF gracefully', async () => {
  // Test implementation
});

// Test error conditions
test('should throw meaningful error for corrupted PDF', async () => {
  // Test implementation
});
```

### Running Tests

```bash
# Run all tests
npm test

# Run specific test files
npm run test:unit
npm run test:integration

# Run with coverage
npm run test:coverage

# Watch mode for development
npm run test:watch
```

## Pull Request Process

1. **Ensure CI passes**: All tests, linting, and type checking must pass
2. **Update documentation**: Include relevant documentation updates
3. **Add tests**: Include tests for new functionality
4. **Describe changes**: Provide a clear description of what and why
5. **Link issues**: Reference any related issues
6. **Request review**: Assign appropriate reviewers

### Pull Request Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Tests pass locally
- [ ] Added tests for new functionality
- [ ] Manual testing completed

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex logic
- [ ] Documentation updated
- [ ] No breaking changes (or clearly documented)
```

## Coding Standards

### TypeScript

- Use TypeScript strict mode
- Provide type annotations for public APIs
- Use interfaces for complex object types
- Prefer `const` over `let`, avoid `var`
- Use meaningful variable and function names

### Code Style

```typescript
// Good: Clear, descriptive names
class PDFTextExtractor {
  async extractTextWithFormatting(options: TextExtractionOptions): Promise<FormattedText> {
    // Implementation
  }
}

// Good: Use interfaces for complex types
interface StreamingOptions {
  chunkSize: number;
  progressCallback?: (progress: Progress) => void;
  abortSignal?: AbortSignal;
}

// Good: Handle errors appropriately
try {
  const result = await someOperation();
  return result;
} catch (error) {
  throw new Error(`Failed to process PDF: ${error.message}`);
}
```

### Documentation

- Use JSDoc comments for public APIs
- Include usage examples for complex features
- Document error conditions and edge cases
- Keep README and API docs up to date

```typescript
/**
 * Extracts text content from PDF with advanced formatting preservation.
 * 
 * @param options - Configuration options for text extraction
 * @returns Promise resolving to extracted text with formatting metadata
 * 
 * @example
 * ```typescript
 * const text = await pdf.extractText({
 *   preserveFormatting: true,
 *   extractTables: true
 * });
 * ```
 * 
 * @throws {Error} When PDF is corrupted or unreadable
 */
async extractText(options: TextExtractionOptions): Promise<TextContent> {
  // Implementation
}
```

## Commit Guidelines

Use conventional commits for clear commit history:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Types

- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation changes
- `style` - Code style changes (formatting, etc.)
- `refactor` - Code refactoring
- `test` - Adding or updating tests
- `chore` - Maintenance tasks

### Examples

```
feat(extraction): add table extraction with cell positioning
fix(streaming): resolve memory leak in large file processing
docs(api): update semantic chunking examples
test(integration): add tests for AI features workflow
```

## Issue Reporting

### Before Creating an Issue

1. Search existing issues to avoid duplicates
2. Try the latest version to see if the issue persists
3. Gather relevant information (Node.js version, environment, etc.)

### Issue Template

**Bug Report:**
- Description of the bug
- Steps to reproduce
- Expected behavior
- Actual behavior
- Environment details
- Code samples or test cases

**Feature Request:**
- Clear description of the feature
- Use case and motivation
- Proposed API (if applicable)
- Alternative solutions considered

## Getting Help

- Check the [documentation](README.md)
- Search existing [issues](https://github.com/nervosys/agenticpdf/issues)
- Join our [discussions](https://github.com/nervosys/agenticpdf/discussions)
- Reach out to maintainers for complex questions

## Recognition

Contributors will be recognized in:
- CHANGELOG.md for their contributions
- README.md contributors section
- GitHub's contributor graphs

Thank you for contributing to AgenticPDF! 🎉