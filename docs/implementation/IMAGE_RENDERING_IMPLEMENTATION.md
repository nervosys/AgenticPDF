# Image Rendering Implementation

## Overview

This document describes the implementation of PDF image rendering in AgenticPDF. The implementation handles the `Do` operator to display XObject images on the canvas with proper coordinate transformations.

## Problem Statement

**Issue**: PDFs contain images as XObjects that need to be rendered on the canvas.

**Challenges**:
- Images stored as compressed streams in PDF
- Multiple image formats (JPEG, PNG, raw bitmap data)
- Different color spaces (DeviceRGB, DeviceGray, DeviceCMYK, etc.)
- PDF coordinate system differs from Canvas (bottom-left vs top-left origin)
- Images positioned and scaled via Current Transformation Matrix (CTM)

## Implementation Architecture

### 1. XObject Parsing (parseResources)

**Location**: `agenticpdf.ts` ~line 1256-1373

**Key Components**:

```typescript
interface XObject {
  type: 'image' | 'form' | 'ps';
  data: Uint8Array;  // Raw image data
}

interface ImageResource {
  width: number;
  height: number;
  bitsPerComponent: number;
  colorSpace: string;
  filter?: string[];
  data: Uint8Array;
}

interface PDFResources {
  xObjects: Map<string, XObject>;      // For Do operator
  images: Map<string, ImageResource>;   // Detailed image info
  // ... other resources
}
```

**Parsing Process**:

```typescript
// Extract XObject dictionary from page resources
const xObjectRef = resourcesDict.entries.get('XObject');

// Iterate through each XObject
for (const [name, xObj] of xObjectDict.entries) {
  // Resolve reference to get stream object
  const stream = this.parseIndirectObject(...);
  
  // Determine XObject type from Subtype
  const subtype = stream.dictionary.entries.get('Subtype');
  // Types: Image, Form, PS (PostScript)
  
  // Create XObject entry
  resources.xObjects.set(name, {
    type: xObjectType,
    data: stream.data
  });
  
  // For images, also parse dimensions and color space
  if (isImage) {
    resources.images.set(name, {
      width: ...,
      height: ...,
      bitsPerComponent: ...,
      colorSpace: ...,
      data: stream.data
    });
  }
}
```

**PDF XObject Dictionary Example**:

```pdf
/XObject <<
  /Im1 10 0 R    % Reference to image stream
  /Im2 11 0 R
  /Fm1 12 0 R    % Reference to form XObject
>>

% Image stream object
10 0 obj
<<
  /Type /XObject
  /Subtype /Image
  /Width 640
  /Height 480
  /ColorSpace /DeviceRGB
  /BitsPerComponent 8
  /Filter /DCTDecode     % JPEG compression
  /Length 45678
>>
stream
...JPEG data...
endstream
endobj
```

### 2. Do Operator Handler

**Location**: `agenticpdf.ts` ~line 4549 (in executeOperator switch)

**PDF Syntax**:
```pdf
/Im1 Do   % Display XObject named "Im1"
```

**Implementation**:

```typescript
case 'Do': await this.displayXObject(operands); break;

private async displayXObject(operands: any[]): Promise<void> {
  // Extract XObject name from operands
  const xobjName = operands[0];  // e.g., "Im1"
  
  // Lookup in page resources
  const xobject = this.page.resources.xObjects.get(xobjName);
  
  // Dispatch based on type
  if (xobject.type === 'image') {
    await this.renderImage(xobject);
  } else if (xobject.type === 'form') {
    // TODO: Form XObjects (item #9)
  }
}
```

**Asynchronous Execution**:

The Do operator requires async/await because image loading is asynchronous:

```typescript
// Made executeOperator async
async executeOperator(operator: string, operands: any[]): Promise<void>

// Made rendering loop async
for (const op of operations) {
  await graphics.executeOperator(op.operator, op.operands);
}
```

### 3. Image Rendering (renderImage)

**Location**: `agenticpdf.ts` ~line 5042-5097

**Implementation Flow**:

```
XObject Data → Detect Format → Create Blob → Load Image → Draw to Canvas
     |              |              |              |              |
  Uint8Array   Magic Bytes    URL.create     Image.onload   ctx.drawImage
```

**Code Walkthrough**:

```typescript
private async renderImage(xobject: XObject): Promise<void> {
  // 1. Convert Uint8Array to ArrayBuffer (for Blob constructor)
  const arrayBuffer = xobject.data.buffer.slice(
    xobject.data.byteOffset,
    xobject.data.byteOffset + xobject.data.byteLength
  ) as ArrayBuffer;
  
  // 2. Detect image format from magic bytes
  const bytes = new Uint8Array(arrayBuffer);
  let mimeType = 'image/jpeg'; // Default
  
  if (bytes[0] === 0xFF && bytes[1] === 0xD8 && bytes[2] === 0xFF) {
    mimeType = 'image/jpeg';  // JPEG: FF D8 FF
  } else if (bytes[0] === 0x89 && bytes[1] === 0x50 && 
             bytes[2] === 0x4E && bytes[3] === 0x47) {
    mimeType = 'image/png';   // PNG: 89 50 4E 47
  } else if (bytes[0] === 0x47 && bytes[1] === 0x49 && bytes[2] === 0x46) {
    mimeType = 'image/gif';   // GIF: 47 49 46
  }
  
  // 3. Create Blob with detected MIME type
  const blob = new Blob([arrayBuffer], { type: mimeType });
  const imageUrl = URL.createObjectURL(blob);
  
  // 4. Load image asynchronously
  const img = new Image();
  img.src = imageUrl;
  
  await new Promise<void>((resolve, reject) => {
    img.onload = () => {
      // 5. Draw to canvas with transformations
      this.ctx.save();
      
      // PDF images are drawn in 1x1 unit square
      // CTM scales and positions them
      // Flip vertically (PDF vs Canvas coordinates)
      this.ctx.scale(1, -1);
      this.ctx.translate(0, -1);
      
      // Draw image
      this.ctx.drawImage(img, 0, 0, 1, 1);
      
      this.ctx.restore();
      
      // Clean up
      URL.revokeObjectURL(imageUrl);
      resolve();
    };
    
    img.onerror = () => {
      URL.revokeObjectURL(imageUrl);
      reject(new Error('Failed to load image'));
    };
  });
}
```

## PDF Image Coordinate System

### Image Positioning

PDF images are **always drawn in a 1×1 unit square** at the origin (0,0):

```
(0,1) -------- (1,1)
  |              |
  |    Image     |
  |   (1×1)      |
  |              |
(0,0) -------- (1,0)
```

**Transformation via CTM**:

The Current Transformation Matrix (CTM) scales, rotates, and positions the image:

```pdf
q                    % Save graphics state
100 0 0 80 50 200 cm % CTM: Scale to 100×80, position at (50,200)
/Im1 Do              % Draw image (will be 100pt wide, 80pt tall, at x=50, y=200)
Q                    % Restore graphics state
```

**Matrix Decomposition**:
```
[a  b  0]   [sx  0  0]   [cos θ  -sin θ  0]   [1  0  0]
[c  d  0] = [0  sy  0] × [sin θ   cos θ  0] × [0  1  0]
[e  f  1]   [0   0  1]   [0       0      1]   [tx ty 1]

where:
  sx, sy = scale factors
  θ = rotation angle
  tx, ty = translation
```

**Example**:
```pdf
% Draw 200×150 pixel image at position (100, 400)
q
200 0 0 150 100 400 cm
/Im1 Do
Q

% Result: Image scaled to 200×150 units, positioned at (100, 400)
```

### Coordinate System Flip

**PDF**: Origin at **bottom-left**, Y increases **upward**
**Canvas**: Origin at **top-left**, Y increases **downward**

**Solution**: Flip Y-axis when drawing:

```typescript
this.ctx.scale(1, -1);     // Flip Y axis
this.ctx.translate(0, -1);  // Move to correct position
this.ctx.drawImage(img, 0, 0, 1, 1);
```

**Without flip**: Image appears upside-down
**With flip**: Image appears correctly

## Image Format Handling

### Supported Formats

| Format   | Filter      | MIME Type  | Magic Bytes | Status      |
| -------- | ----------- | ---------- | ----------- | ----------- |
| JPEG     | DCTDecode   | image/jpeg | FF D8 FF    | ✅ Supported |
| PNG      | FlateDecode | image/png  | 89 50 4E 47 | ✅ Supported |
| GIF      | -           | image/gif  | 47 49 46    | ✅ Supported |
| JPEG2000 | JPXDecode   | image/jp2  | 00 00 00 0C | ❌ Not yet   |
| JBIG2    | JBIG2Decode | -          | -           | ❌ Not yet   |
| Raw      | None        | -          | -           | ❌ Not yet   |

### Format Detection

**Magic Byte Detection**:

```typescript
// JPEG: First 3 bytes are FF D8 FF
if (bytes[0] === 0xFF && bytes[1] === 0xD8 && bytes[2] === 0xFF) {
  return 'image/jpeg';
}

// PNG: First 4 bytes are 89 50 4E 47 (PNG signature)
if (bytes[0] === 0x89 && bytes[1] === 0x50 && 
    bytes[2] === 0x4E && bytes[3] === 0x47) {
  return 'image/png';
}

// GIF: First 3 bytes are 47 49 46 ("GIF")
if (bytes[0] === 0x47 && bytes[1] === 0x49 && bytes[2] === 0x46) {
  return 'image/gif';
}
```

**Filter-Based Detection** (Future Enhancement):

```typescript
// Check PDF Filter entry
const filter = stream.dictionary.entries.get('Filter');

switch (filter) {
  case 'DCTDecode': return 'image/jpeg';
  case 'FlateDecode': return 'image/png';  // If already compressed as PNG
  case 'JPXDecode': return 'image/jp2';
  case 'JBIG2Decode': return 'image/jbig2';
  // ... etc
}
```

### Raw Image Data

PDFs can contain **uncompressed** image data that needs to be rendered pixel-by-pixel:

```typescript
// Future implementation for raw images
private renderRawImage(imageRes: ImageResource): Promise<void> {
  const canvas = document.createElement('canvas');
  canvas.width = imageRes.width;
  canvas.height = imageRes.height;
  const ctx = canvas.getContext('2d')!;
  
  // Create ImageData from raw pixels
  const imageData = ctx.createImageData(imageRes.width, imageRes.height);
  
  // Convert color space and bit depth
  this.convertPixels(imageRes.data, imageData.data, 
                     imageRes.colorSpace, imageRes.bitsPerComponent);
  
  ctx.putImageData(imageData, 0, 0);
  
  // Draw to main canvas
  this.ctx.drawImage(canvas, 0, 0, 1, 1);
}
```

## Color Space Support

### Current Implementation

**Supported Color Spaces**:
- **DeviceRGB**: RGB (red, green, blue) - most common
- **DeviceGray**: Grayscale
- **DeviceCMYK**: CMYK (cyan, magenta, yellow, black)

**Implementation Status**:
- ✅ Color space parsed from image dictionary
- ❌ Color conversion not yet implemented
- ❌ Relies on browser's native image decoder

### Advanced Color Spaces (TODO #7)

**Not Yet Supported**:
- **CalRGB/CalGray**: Calibrated color spaces
- **ICCBased**: ICC color profiles
- **Indexed**: Palette-based images (lookup tables)
- **Separation**: Spot colors
- **DeviceN**: Multiple colorants
- **Pattern**: Pattern-based fills

**Future Implementation**:

```typescript
interface ImageResource {
  colorSpace: string | ColorSpace;  // Can be complex
  // ...
}

// Handle indexed color space
if (colorSpace.name === 'Indexed') {
  const palette = colorSpace.lookup;  // Color lookup table
  const pixelValue = imageData[i];
  const rgb = palette[pixelValue];    // Map to RGB
  // ... render
}

// Handle ICC profiles
if (colorSpace.name === 'ICCBased') {
  const profile = colorSpace.iccProfile;
  const convertedRGB = applyICCProfile(imageData, profile);
  // ... render
}
```

## Graphics State Integration

### Transformation Matrix

Images are affected by the **Current Transformation Matrix** (CTM):

```pdf
q                      % Save state
1 0 0 1 100 200 cm    % Translate to (100, 200)
2 0 0 2 0 0 cm        % Scale by 2×2
/Im1 Do               % Draw image (scaled and translated)
Q                     % Restore state
```

**Canvas Implementation**:

```typescript
// CTM is automatically maintained by canvas
// When Do is called, the current transform is already applied
case 'cm': this.concatMatrix(operands); break;  // Updates CTM

// Image draws in current coordinate system
this.ctx.drawImage(img, 0, 0, 1, 1);  // Already transformed!
```

### Graphics State Stack

Images rendered within saved graphics states:

```pdf
q                % Save state
  q              % Save state again
    /Im1 Do      % Draw image 1
  Q              % Restore to first save
  /Im2 Do        % Draw image 2
Q                % Restore to original
```

**Implementation**:

```typescript
case 'q': this.saveGraphicsState(); break;  // this.ctx.save()
case 'Q': this.restoreGraphicsState(); break;  // this.ctx.restore()
case 'Do': await this.displayXObject(operands); break;
```

## Error Handling

### Graceful Degradation

```typescript
private async renderImage(xobject: XObject): Promise<void> {
  try {
    // ... image loading and rendering
  } catch (error) {
    // Log error but don't crash rendering
    console.warn('Error rendering image:', error);
    
    // Could draw placeholder rectangle
    this.ctx.save();
    this.ctx.fillStyle = '#ddd';
    this.ctx.fillRect(0, 0, 1, 1);
    this.ctx.strokeStyle = '#999';
    this.ctx.strokeRect(0, 0, 1, 1);
    this.ctx.restore();
  }
}
```

### Common Error Cases

**1. Missing XObject**:
```typescript
const xobject = this.page.resources.xObjects.get(xobjName);
if (!xobject) return;  // Silently skip
```

**2. Invalid Image Data**:
```typescript
img.onerror = () => {
  URL.revokeObjectURL(imageUrl);
  reject(new Error('Failed to load image'));
};
```

**3. Unsupported Format**:
```typescript
// Detected format, but browser can't decode
// Fallback to default JPEG assumption
let mimeType = 'image/jpeg';
```

## Performance Considerations

### Memory Management

**Blob URL Cleanup**:
```typescript
const imageUrl = URL.createObjectURL(blob);
// ... use image
URL.revokeObjectURL(imageUrl);  // Critical! Prevents memory leak
```

**Image Caching** (Future):
```typescript
private imageCache: Map<string, HTMLImageElement> = new Map();

private async renderImage(xobject: XObject): Promise<void> {
  const cacheKey = this.getImageHash(xobject.data);
  
  if (this.imageCache.has(cacheKey)) {
    const cachedImg = this.imageCache.get(cacheKey)!;
    this.ctx.drawImage(cachedImg, 0, 0, 1, 1);
    return;
  }
  
  // Load and cache...
  this.imageCache.set(cacheKey, img);
}
```

### Async Performance

**Sequential Loading**:
```typescript
// Current: Images load one at a time
for (const op of operations) {
  await graphics.executeOperator(op.operator, op.operands);
}
```

**Parallel Loading** (Future):
```typescript
// Collect all Do operators
const imageOps = operations.filter(op => op.operator === 'Do');

// Pre-load all images in parallel
await Promise.all(imageOps.map(op => 
  this.preloadImage(op.operands[0])
));

// Then render synchronously
for (const op of operations) {
  graphics.executeOperator(op.operator, op.operands);
}
```

## Testing & Validation

### Test Cases

**Test 1: Basic JPEG Image**
```pdf
q
200 0 0 150 100 400 cm
/Im1 Do
Q
```
Expected: JPEG image at (100, 400), scaled to 200×150

**Test 2: Multiple Images**
```pdf
/Im1 Do
100 0 0 100 200 300 cm
/Im2 Do
```
Expected: Two images, second transformed

**Test 3: Nested Graphics State**
```pdf
q
  1 0 0 1 50 50 cm
  q
    2 0 0 2 0 0 cm
    /Im1 Do
  Q
Q
```
Expected: Image scaled 2×2, translated (50, 50)

**Test 4: Missing Image**
```pdf
/NonExistent Do
```
Expected: Silently skip, no error

### Validation Methods

**Visual Comparison**:
1. Render PDF in Adobe Acrobat
2. Screenshot page
3. Render in AgenticPDF
4. Compare pixel-by-pixel or visually

**Automated Testing**:
```typescript
describe('Image Rendering', () => {
  it('should render JPEG images', async () => {
    const pdf = await AgenticPDF.fromFile('test-image.pdf');
    const page = await pdf.getPage(1);
    
    expect(page.resources?.xObjects.size).toBeGreaterThan(0);
    
    const canvas = document.createElement('canvas');
    await renderer.renderToCanvas(1, canvas);
    
    // Check that pixels were drawn
    const ctx = canvas.getContext('2d')!;
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
    const hasPixels = imageData.data.some(v => v !== 255);
    expect(hasPixels).toBe(true);
  });
});
```

## Known Limitations

### 1. Raw Image Data

**Issue**: Uncompressed images not yet supported.

**Impact**: Some PDFs with raw pixel data won't render images.

**Workaround**: Most PDFs use compressed images (JPEG, FlateDecode).

**Status**: Not implemented (requires pixel conversion).

### 2. Advanced Filters

**Issue**: JPEG2000, JBIG2 not supported.

**Impact**: Some modern PDFs may have missing images.

**Workaround**: Browser may support JP2 natively.

**Status**: Low priority (uncommon).

### 3. Color Space Conversion

**Issue**: ICC profiles, indexed colors not converted.

**Impact**: Colors may appear incorrect.

**Workaround**: Relies on browser's native decoder.

**Status**: Planned for TODO #7.

### 4. Image Masks

**Issue**: Soft masks, stencil masks not implemented.

**Impact**: Transparency effects missing.

**Example**:
```pdf
/Im1 Do          % Base image
/Mask /Im2       % Apply Im2 as mask
```

**Status**: Future enhancement.

### 5. Inline Images

**Issue**: BI/ID/EI operators not supported.

**Impact**: Inline images not rendered.

**Example**:
```pdf
BI               % Begin inline image
  /W 100
  /H 100
  /CS /RGB
ID               % Image data follows
...binary data...
EI               % End inline image
```

**Status**: Planned for TODO #8 (Parser Robustness).

## Integration Examples

### Basic Image Rendering

```typescript
const pdf = await AgenticPDF.fromFile(file);
const page = await pdf.getPage(1);

// Images automatically extracted
console.log(`Page has ${page.resources?.xObjects.size} XObjects`);

// Render page (images included)
const canvas = document.createElement('canvas');
const renderer = new PDFRenderer(pdf);
await renderer.renderToCanvas(1, canvas);
```

### Custom Image Handler

```typescript
class CustomImageRenderer extends PDFGraphicsExecutor {
  private async displayXObject(operands: any[]): Promise<void> {
    const xobjName = operands[0];
    const xobject = this.page.resources.xObjects.get(xobjName);
    
    if (xobject && xobject.type === 'image') {
      // Custom processing
      await this.processImage(xobject);
    }
    
    // Call base implementation
    await super.displayXObject(operands);
  }
  
  private async processImage(xobject: XObject): Promise<void> {
    // Extract image metadata, apply filters, etc.
    console.log('Processing image:', xobject.data.length, 'bytes');
  }
}
```

### Image Extraction

```typescript
async function extractImages(pdf: AgenticPDF): Promise<Blob[]> {
  const images: Blob[] = [];
  
  for (let i = 1; i <= pdf.metadata.pageCount; i++) {
    const page = await pdf.getPage(i);
    
    if (page.resources?.xObjects) {
      for (const [name, xobject] of page.resources.xObjects) {
        if (xobject.type === 'image') {
          // Detect format and create blob
          const mimeType = detectImageFormat(xobject.data);
          const blob = new Blob([xobject.data], { type: mimeType });
          images.push(blob);
        }
      }
    }
  }
  
  return images;
}
```

## Future Enhancements

### 1. Raw Image Rendering

**Goal**: Support uncompressed images.

**Implementation**:
```typescript
private renderRawImage(imageRes: ImageResource): Promise<void> {
  // Create canvas
  const tempCanvas = document.createElement('canvas');
  tempCanvas.width = imageRes.width;
  tempCanvas.height = imageRes.height;
  const tempCtx = tempCanvas.getContext('2d')!;
  
  // Convert raw data to RGBA
  const rgba = this.convertToRGBA(
    imageRes.data,
    imageRes.colorSpace,
    imageRes.bitsPerComponent,
    imageRes.width,
    imageRes.height
  );
  
  // Create ImageData
  const imageData = tempCtx.createImageData(imageRes.width, imageRes.height);
  imageData.data.set(rgba);
  tempCtx.putImageData(imageData, 0, 0);
  
  // Draw to main canvas
  this.ctx.drawImage(tempCanvas, 0, 0, 1, 1);
}
```

### 2. Image Caching

**Goal**: Reuse decoded images across pages.

**Benefits**: 
- Faster re-rendering
- Reduced memory usage
- Better performance

### 3. Progressive Loading

**Goal**: Load images as they come into view.

**Implementation**:
```typescript
class LazyImageRenderer {
  private visibleBounds: Rectangle;
  
  private async displayXObject(operands: any[]): Promise<void> {
    // Check if image is in visible area
    if (!this.isVisible(this.currentTransform)) {
      return;  // Skip loading
    }
    
    await super.displayXObject(operands);
  }
}
```

### 4. WebP/AVIF Support

**Goal**: Modern image formats for better compression.

**Implementation**: Detect format, use native browser support.

## References

**PDF Specification**:
- Section 8.9: Images
- Section 8.9.5: Image Dictionaries
- Section 8.9.6: Inline Images
- Section 4.8: XObjects

**Related Implementations**:
- GLYPH_WIDTH_IMPLEMENTATION.md - Text metrics
- TEXT_DECODING_IMPLEMENTATION.md - String decoding
- demos/pdf-viewer.html - Visual testing

## Conclusion

Image rendering is a critical feature for PDF viewers. This implementation provides:

✅ **XObject parsing and storage**
✅ **Do operator support**
✅ **Async image loading**
✅ **Format detection (JPEG, PNG, GIF)**
✅ **Coordinate system transformations**
✅ **Error handling and graceful degradation**

**Remaining Work** (Future TODOs):
❌ Raw image data conversion
❌ Advanced color spaces (TODO #7)
❌ Image masks and transparency
❌ Inline images (TODO #8)
❌ JPEG2000/JBIG2 support

This implementation provides a solid foundation for displaying images in PDF documents, with room for future enhancements.
