# AgenticPDF Demos

This directory contains interactive demonstrations and examples showcasing the features and capabilities of AgenticPDF.

## 🚀 Quick Start

To run the demos, simply open any HTML file in your web browser. All demos are self-contained and don't require a server.

## 📋 Available Demos

### 1. 🎯 PDF Viewer (`pdf-viewer.html`)
**Full-featured PDF viewer using native AgenticPDF implementation**

- **Features:**
  - **Native AgenticPDF**: Uses `agenticpdf-browser.js` (no external dependencies)
  - Dark/Light mode toggle with persistence
  - Fit-to-width with aspect ratio preservation
  - High DPI rendering for sharp text
  - Continuous scrolling
  - Zoom controls with keyboard shortcuts
  - Page navigation
  - Professional dark theme by default

- **Usage:** Open `pdf-viewer.html` in your browser
- **Sample File:** Uses `sample.pdf` for demonstration
- **Key Innovations:** 
  - This viewer demonstrates the optimal configuration built into AgenticPDF
  - Native AgenticPDF implementation
  - Showcases real AgenticPDF capabilities and API patterns

### 2. 🌐 Simple Demo (`simple-demo.html`)
**Basic AgenticPDF integration example**

- **Features:**
  - File upload interface
  - Drag and drop support
  - Mock PDF processing demonstration
  - Educational content about AgenticPDF features
  - Interactive feedback system

- **Usage:** Open `simple-demo.html` in your browser
- **Purpose:** Great starting point for understanding AgenticPDF basics

### 3. 🧪 Configuration Test (`test-optimal-config.html`)
**Test suite for optimal viewer configuration**

- **Features:**
  - Automated testing of default configuration
  - Verification of theme toggle functionality
  - Canvas configuration testing
  - Fit-to-width calculation validation
  - Interactive test runners

- **Usage:** Open `test-optimal-config.html` and run the tests
- **Purpose:** Validates that the optimal configuration works correctly

### 4. 🎨 Theme Toggle Demo (`theme-toggle-demo.html`)
**Comprehensive theme management demonstration**

- **Features:**
  - Live theme switching demonstration
  - Theme persistence testing
  - CSS integration examples
  - API usage documentation
  - Best practices guide

- **Usage:** Open `theme-toggle-demo.html` to explore theme features
- **Purpose:** Shows the new theme toggle functionality in detail

### 5. 🔧 Examples Demo (`examples-demo.html`)
**Interactive API examples**

- **Features:**
  - Live code examples
  - API method demonstrations
  - Interactive parameter testing
  - Real-time results
  - Copy-paste ready code snippets

- **Usage:** Open `examples-demo.html` for hands-on API exploration
- **Purpose:** Learn AgenticPDF API through interactive examples

## 🛠️ Development Tools

### Running Examples via npm

```bash
# Run basic examples
npm run examples

# Interactive examples
npm run examples:interactive

# Open demo in browser
npm run examples:demo

# Open simple demo
npm run examples:simple

# Get help
npm run examples:help
```

### Local Development

1. **Clone the repository**
   ```bash
   git clone [repository-url]
   cd AgenticPDF
   ```

2. **Install dependencies**
   ```bash
   npm install
   ```

3. **Open demos**
   ```bash
   # Open any demo file in your browser
   open demos/pdf-viewer.html
   ```

## 🎯 Demo Features Showcase

### Theme Toggle Integration
All demos demonstrate the new theme toggle functionality that's now built into AgenticPDF:

- **Dark Mode (Default):** Professional dark theme with optimal contrast
- **Light Mode:** Clean light theme for bright environments
- **Persistence:** Theme choice saved across browser sessions
- **System Detection:** Automatic theme based on system preferences
- **Smooth Transitions:** CSS animations for theme changes

### Optimal Configuration
The demos showcase the optimal viewer configuration that's now the default for AgenticPDF:

```typescript
{
  scale: 1.0,
  renderScale: 2.0,              // High DPI for sharp text
  fitToWidth: true,              // Auto-fit to container width
  maintainAspectRatio: true,     // Preserve PDF proportions
  autoFitOnLoad: true,           // Auto-fit when loading
  darkMode: true,                // Dark theme by default
  continuousScrolling: true,     // Smooth page transitions
  enableThemeToggle: true,       // Theme switching enabled
  persistTheme: true,            // Save theme preference
  defaultTheme: 'dark',          // Start with dark mode
  themeStorageKey: 'agenticpdf-theme'
}
```

### Performance Features
- **High DPI Rendering:** 2.0 render scale for crisp text on modern displays
- **Memory Management:** Efficient resource usage and cleanup
- **Streaming Support:** Handle large PDFs without blocking
- **Progressive Loading:** Load content as needed

## 📚 Implementation Documentation

### Configuration Implementation
- **`OPTIMAL_CONFIG_IMPLEMENTATION.md`** - Details on how the optimal configuration was integrated into AgenticPDF
- **`THEME_TOGGLE_IMPLEMENTATION.md`** - Complete guide to the theme toggle system

### API Reference
Each demo includes inline documentation and comments explaining:
- AgenticPDF API usage
- Configuration options
- Event handling
- Error management
- Best practices

## 🔧 Customization

### Creating Custom Demos

1. **Copy an existing demo** as a starting point
2. **Modify the configuration** to test specific features
3. **Add your own UI elements** and styling
4. **Update the documentation** to explain your demo

### Example Custom Configuration
```typescript
const pdf = await AgenticPDF.fromFile(file, {
  renderOptions: {
    // Override defaults
    defaultTheme: 'light',
    enableThemeToggle: false,
    renderScale: 1.5,
    // Add custom options
    customProperty: 'value'
  }
});
```

## 🎨 Styling

All demos include responsive CSS that works with the theme toggle system:

```css
/* Dark mode (default) */
body {
  background: #1a1a1a;
  color: white;
}

/* Light mode */
body.light-mode {
  background: #f5f5f5;
  color: #333;
}

/* Component styling with theme support */
.pdf-viewer {
  background: var(--viewer-bg, #111);
  transition: all 0.3s ease;
}

body.light-mode .pdf-viewer {
  --viewer-bg: #e9ecef;
}
```

## 🚀 Production Usage

The demos show how AgenticPDF works out of the box with optimal settings. For production use:

1. **Use the default configuration** - it includes all optimal settings
2. **Add theme CSS** for light mode support
3. **Customize as needed** - override specific options
4. **Test thoroughly** - use the test demo to validate your setup

## ⚡ Performance Tips

Based on the demo implementations:

1. **Enable lazy loading** for large documents
2. **Use streaming APIs** for real-time processing
3. **Leverage Web Workers** for CPU-intensive operations
4. **Implement proper cleanup** to prevent memory leaks
5. **Cache frequently accessed content**

## 🤝 Contributing

To add new demos or improve existing ones:

1. Create your demo in the `demos/` directory
2. Update this README with demo information
3. Test the demo in multiple browsers
4. Submit a pull request with your changes

## 📄 Sample Content

The `sample.pdf` file is included for testing purposes. It contains:
- Multiple pages with varied content
- Text in different fonts and sizes
- Images and graphics
- Form fields and annotations
- Various PDF features for comprehensive testing

## 🎯 Next Steps

After exploring the demos:

1. **Read the main README** for full API documentation
2. **Check the implementation docs** for detailed technical information
3. **Run the test suite** to understand the codebase
4. **Start building** with AgenticPDF in your own projects

---

**Note:** All demos are designed to work offline and don't require any external dependencies beyond the included AgenticPDF library.