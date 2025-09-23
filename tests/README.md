# ModernPDF Testing Suite

## Overview

This directory contains a comprehensive testing suite for the ModernPDF library, designed to ensure reliability, performance, and correctness across all features and edge cases.

## Test Structure

```
tests/
├── setup.ts              # Jest test environment setup
├── mocks/                 # Mock implementations
│   └── index.ts          # Comprehensive mock utilities
├── fixtures/              # Test data and samples
│   └── index.ts          # Sample content and metadata
├── unit/                  # Unit tests
│   ├── pdf-parser.test.ts # Core PDF parsing tests
│   ├── extraction.test.ts # Content extraction tests
│   ├── ai-features.test.ts # AI and semantic analysis tests
│   ├── streaming.test.ts  # Streaming operations tests
│   └── error-handling.test.ts # Error handling and edge cases
└── integration/           # Integration tests
    └── modernpdf.test.ts  # Main class integration tests
```

## Running Tests

### Prerequisites

Make sure you have all dependencies installed:

```bash
npm install
```

### Basic Test Execution

Run all tests:
```bash
npm test
```

Run tests in watch mode:
```bash
npm run test:watch
```

Run specific test file:
```bash
npm test pdf-parser.test.ts
```

Run tests with coverage:
```bash
npm run test:coverage
```

### Test Categories

#### Unit Tests
Test individual components in isolation:

```bash
# Core PDF parsing functionality
npm test unit/pdf-parser.test.ts

# Content extraction (text, images, forms, annotations)
npm test unit/extraction.test.ts

# AI features and semantic analysis
npm test unit/ai-features.test.ts

# Streaming operations and progress tracking
npm test unit/streaming.test.ts

# Error handling and edge cases
npm test unit/error-handling.test.ts
```

#### Integration Tests
Test high-level workflows and component interactions:

```bash
# Main ModernPDF class integration
npm test integration/modernpdf.test.ts
```

### Test Configuration

The test suite uses Jest with TypeScript support. Key configuration files:

- **`jest.config.js`** - Main Jest configuration
- **`tsconfig.json`** - TypeScript configuration (includes Jest types)
- **`tests/setup.ts`** - Test environment setup and global mocks

## Test Coverage

### Coverage Reports

Generate detailed coverage reports:

```bash
npm run test:coverage
```

This creates coverage reports in multiple formats:
- **Terminal output** - Summary in console
- **HTML report** - `coverage/index.html` for detailed analysis
- **LCOV format** - `coverage/lcov.info` for CI integration

### Coverage Targets

The test suite aims for:
- **Statements**: 95%+
- **Branches**: 90%+
- **Functions**: 95%+
- **Lines**: 95%+

### Coverage Areas

#### Core Functionality (100% target)
- PDF parsing and validation
- Content extraction algorithms
- Stream management
- Memory management

#### AI Features (95% target)
- Embedding generation
- Semantic chunking
- Document analysis
- Structural recognition

#### Error Handling (90% target)
- Malformed PDF handling
- Network failure recovery
- Memory limit enforcement
- Input validation

## Mock Architecture

The test suite includes comprehensive mocks for external dependencies:

### MockEmbeddingProvider
- Deterministic embedding generation
- Configurable delays and errors
- Batch processing simulation

### MockPDFGenerator
- Various PDF structures (simple, multi-page, corrupted)
- Invalid formats for error testing
- Large documents for memory testing

### MockReadableStream
- Streaming simulation with backpressure
- Configurable read delays
- Cancellation support

### MockProgressTracker
- Progress event simulation
- Callback testing
- Performance metrics

### MockFileSystem
- File operation simulation
- Error injection capabilities
- Path management

### MockFetch
- Network request simulation
- HTTP error scenarios
- Timeout and delay simulation

### TestUtils
- Common test data generation
- Stream utilities
- Mock data validation

## Test Data and Fixtures

### Sample Content
- **TEXTS**: Various text samples (short, medium, long, technical, multilingual)
- **METADATA**: PDF metadata examples (simple, complex, encrypted)
- **CHUNKS**: Semantic chunks with embeddings and metadata

### PDF Structures
- **PAGES**: Page definitions with different layouts
- **FONTS**: Font configurations and mappings
- **IMAGES**: Image metadata and specifications
- **ANNOTATIONS**: Various annotation types
- **FORM_FIELDS**: Form field definitions

### Progress and Error States
- **PROGRESS_STATES**: Various progress scenarios
- **ERRORS**: Common error conditions and messages

## Writing Tests

### Best Practices

1. **Use descriptive test names**:
   ```typescript
   test('should extract text with preserved formatting', async () => {
     // Test implementation
   });
   ```

2. **Arrange-Act-Assert pattern**:
   ```typescript
   test('should generate semantic chunks', async () => {
     // Arrange
     const text = TestFixtures.TEXTS.TECHNICAL;
     const options = { strategy: 'semantic', maxChunkSize: 1000 };
     
     // Act
     const chunks = await generateSemanticChunks(text, options);
     
     // Assert
     expect(chunks.length).toBeGreaterThan(0);
     expect(chunks[0].content.length).toBeLessThanOrEqual(1000);
   });
   ```

3. **Use test fixtures for consistency**:
   ```typescript
   const mockProvider = new Mocks.MockEmbeddingProvider();
   const samplePDF = Mocks.MockPDFGenerator.createSimplePDF();
   const testText = TestFixtures.TEXTS.SHORT;
   ```

4. **Test error conditions**:
   ```typescript
   test('should handle invalid PDF header', async () => {
     const invalidPDF = new Uint8Array([0x25, 0x50, 0x44, 0x46]);
     
     await expect(ModernPDF.fromBuffer(invalidPDF.buffer))
       .rejects.toThrow(/invalid.*pdf.*header/i);
   });
   ```

5. **Clean up resources**:
   ```typescript
   afterEach(() => {
     mockFetch.clear();
     // Other cleanup
   });
   ```

### Adding New Tests

1. **Identify the appropriate test file** (unit vs integration)
2. **Use existing mocks and fixtures** when possible
3. **Follow the established patterns** for consistency
4. **Add comprehensive error cases** for new functionality
5. **Update this documentation** if adding new test categories

### Test Organization

#### Unit Test Structure
```typescript
describe('FeatureName', () => {
  describe('method or operation', () => {
    test('should handle normal case', () => {});
    test('should handle edge case', () => {});
    test('should handle error case', () => {});
  });
});
```

#### Integration Test Structure
```typescript
describe('FeatureName Integration', () => {
  describe('end-to-end workflow', () => {
    test('should complete full workflow', () => {});
    test('should handle workflow errors', () => {});
  });
});
```

## Debugging Tests

### Running Single Tests
```bash
npm test -- --testNamePattern="should extract text"
```

### Debug Mode
```bash
npm test -- --runInBand --detectOpenHandles
```

### Verbose Output
```bash
npm test -- --verbose
```

### Memory Usage
```bash
npm test -- --logHeapUsage
```

## Continuous Integration

### GitHub Actions Integration

The test suite is designed to work with CI/CD pipelines:

```yaml
- name: Run tests
  run: npm test

- name: Generate coverage
  run: npm run test:coverage

- name: Upload coverage
  uses: codecov/codecov-action@v3
  with:
    file: ./coverage/lcov.info
```

### Performance Monitoring

Monitor test performance to catch regressions:
- Test execution time
- Memory usage patterns
- Coverage trends

## Troubleshooting

### Common Issues

#### Tests Timing Out
- Increase Jest timeout: `jest.setTimeout(30000)`
- Check for infinite loops in mock implementations
- Verify async operations are properly awaited

#### Memory Issues
- Ensure PDF instances are properly closed
- Clear mock data between tests
- Use `--logHeapUsage` to identify leaks

#### Type Errors
- Verify Jest types are properly configured
- Check mock implementations match interfaces
- Ensure proper imports in test files

#### Mock Failures
- Verify mock setup in `beforeEach` hooks
- Check mock data validity
- Clear mock state between tests

### Getting Help

1. **Check test output** for specific error messages
2. **Review mock implementations** in `tests/mocks/`
3. **Verify test fixtures** in `tests/fixtures/`
4. **Run tests in isolation** to identify specific failures
5. **Check TypeScript compilation** for type issues

## Performance Benchmarks

The test suite includes performance monitoring for:

- **PDF parsing speed** (pages per second)
- **Text extraction throughput** (characters per second)
- **Embedding generation rate** (embeddings per second)
- **Memory usage patterns** (peak and sustained usage)
- **Stream processing rates** (bytes per second)

These benchmarks help identify performance regressions and optimization opportunities.

## Maintenance

### Regular Tasks

1. **Update test fixtures** when adding new features
2. **Review mock implementations** for accuracy
3. **Monitor test execution times** for performance issues
4. **Update documentation** when adding new test categories
5. **Review coverage reports** to identify gaps

### Version Updates

When updating dependencies:
1. **Run full test suite** to identify breaking changes
2. **Update mock implementations** if needed
3. **Verify TypeScript compatibility**
4. **Update Jest configuration** if required

This comprehensive testing suite ensures ModernPDF maintains high quality and reliability across all supported features and use cases.