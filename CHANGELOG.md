# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-09-23

### Added

#### 🎨 Modern UI Features
- **Theme Toggle Support**: Built-in dark/light mode functionality for PDF viewers
- **Optimal Viewer Configuration**: Pre-configured viewer settings with theme management
- **Theme Persistence**: User theme preferences saved across sessions
- **Responsive Design**: Auto-fitting viewers that maintain aspect ratios

#### 🚀 Core Features
- **Streaming-First Architecture**: Memory-efficient processing of large PDF files
- **AI-Native Design**: Built-in semantic chunking and structural analysis
- **Zero Dependencies**: Complete TypeScript implementation in a single file
- **Complete PDF Support**: Text extraction, images, forms, annotations, and rendering
- **Web Worker Support**: CPU-intensive operations offloaded to workers
- **Memory Management**: Configurable limits and lazy loading capabilities

#### 🤖 AI Integration
- **Semantic Chunking**: Intelligent content segmentation for RAG systems
- **Embedding Provider Interface**: Support for custom embedding models
- **Structural Analysis**: Automatic document structure detection
- **NLP-Ready Processing**: Text processing optimized for language models

#### 🌐 Cross-Platform Support
- **Browser Compatibility**: Works in modern browsers with CDN support
- **Node.js Support**: Full server-side functionality
- **TypeScript Native**: Complete type safety and IntelliSense support
- **Progressive Enhancement**: Graceful degradation for older environments

#### 📦 Developer Experience
- **Interactive Demos**: Comprehensive HTML demo suite
- **CLI Examples**: TypeScript examples for common use cases
- **Comprehensive Testing**: 177+ tests with full coverage
- **Documentation**: Complete API documentation and guides

#### 🔧 Distribution Options
- **NPM Package**: Traditional package installation
- **Single File**: Direct file inclusion for maximum portability
- **CDN Support**: Browser-ready distribution via unpkg
- **Type Definitions**: Full TypeScript declarations included

### Features Breakdown

#### Theme Toggle System
- Automatic theme detection based on system preferences
- Manual theme switching with smooth transitions
- Persistent theme storage using localStorage
- Customizable color schemes and styling
- Event-driven theme change notifications

#### Optimal Configuration
- Pre-tuned settings for best performance and user experience
- Automatic viewport fitting and aspect ratio maintenance
- Optimized rendering settings for different document types
- Built-in error handling and recovery mechanisms

#### Interactive Demo Suite
- **PDF Viewer Demo**: Full-featured viewer with all capabilities
- **Simple Demo**: Basic integration example for quick start
- **Theme Toggle Demo**: Focused demonstration of theme functionality
- **Configuration Test**: Validation of optimal settings
- **API Examples**: Interactive exploration of library features

#### Streaming Capabilities
- **Memory-Efficient Processing**: Handle large files without memory overflow
- **Progress Tracking**: Real-time processing feedback
- **Abort Signals**: Cancellable operations for better UX
- **Backpressure Handling**: Automatic flow control for smooth streaming

#### AI and Machine Learning
- **Semantic Analysis**: Content understanding and classification
- **Document Structure**: Automatic detection of sections, tables, figures
- **Entity Recognition**: Extraction of key information and entities
- **Similarity Matching**: Content comparison and clustering
- **Embedding Generation**: Vector representations for semantic search

### Technical Specifications

- **Minimum Node.js Version**: 18.0.0
- **TypeScript Version**: 5.9.3
- **Test Coverage**: 871 tests across 23 suites
- **License**: AGPL-3.0-or-later
- **Architecture**: Single file (`agenticpdf.ts`), optional `otel.ts` module
- **Browser Support**: Modern browsers with ES2022+ support

### Documentation

- Complete README with quick start guide
- Interactive demo documentation
- CLI examples documentation
- API reference and type definitions
- Contributing guidelines and code of conduct
- Security policy and vulnerability reporting
- Comprehensive test suite documentation

### Breaking Changes

This is the initial 1.0.0 release, so no breaking changes apply.

### Migration Guide

This is the first stable release. For users upgrading from pre-release versions:

1. Update import statements to use the stable API
2. Review configuration options for optimal viewer settings
3. Update theme-related code to use the new theme toggle system
4. Test integration with the updated demo examples

### Contributors

- AgenticPDF Team
- Community contributors

### Security

Two comprehensive security audit passes (25 total fixes):

#### Pass 1 (12 fixes)
- SSRF protocol validation on `fromUrl()`
- Path traversal prevention in file operations
- Replaced `Math.random` with cryptographic PRNG
- XSS sanitization in HTML export
- ReDoS-safe regex patterns
- Prototype pollution protection in object merging
- Bounded streaming (max buffer sizes)
- Recursion depth limits in PDF object parsing
- aPDF metadata size limits
- Error message information disclosure prevention
- Fixed duplicate TypeScript exports (TS2484)
- Fixed `Uint8Array` type handling

#### Pass 2 (13 findings, 10 code fixes)
- SSRF private IP blocking (RFC 1918, link-local, loopback)
- HTTP redirect validation (limit count, block protocol downgrade)
- Telemetry endpoint exfiltration prevention (hardcoded endpoint removed)
- YAML frontmatter injection sanitization
- CSV formula injection prevention in exports
- PKCS#7 padding oracle mitigation
- aPDF v1.0 entry size limits
- `JSON.parse` safety wrappers
- Demo DOM XSS fixes (input sanitization)
- Worker URL validation (same-origin, blob/data only)
- Digital signature verification documentation

### OpenTelemetry Integration

- Added `@opentelemetry/api` integration in `Telemetry` class
- Lazy resolution — OTEL activated only when packages are present
- Span emission for all tracked operations
- Counter and histogram metrics for events and durations
- Standalone `otel.ts` module for full SDK bootstrap
- `.env` / `.env.example` for OTEL configuration
- Graceful degradation to no-ops when OTEL is unavailable

### aPDF Binary Format (v1.1)

- Custom binary serialization with LZ77 compression
- Full metadata round-trip fidelity
- Configurable entry size limits for security
- `.apdf` file extension support

---

## [Unreleased]

Future planned features:
- Additional theme customization options
- Enhanced AI analysis capabilities
- Performance optimizations
- Extended format support

---

For more details about any release, please see the [GitHub releases page](https://github.com/nervosys/AgenticPDF/releases).
