# Text Layer Implementation - PDF.js Approach

## Overview
The text layer implementation has been completely rewritten to match Mozilla PDF.js's proven approach for accurate text positioning and selection.

## Key Changes from Previous Implementation

### 1. **Coordinate System & Transform Matrix**
**PDF.js Approach:**
- Uses the raw transform matrix from PDF directly
- Applies transform: `[1, 0, 0, -1, -pageX, pageY + pageHeight]`
- Calculates position from transformed coordinates

**Implementation:**
```typescript
const tx = textItem.transform;
let angle = Math.atan2(tx[1], tx[0]);
const fontHeight = Math.hypot(tx[2], tx[3]);
const fontAscent = fontHeight * TextLayerBuilder.getAscent(fontFamily);

let left: number, top: number;
if (angle === 0) {
  left = tx[4];
  top = tx[5] - fontAscent;
} else {
  left = tx[4] + fontAscent * Math.sin(angle);
  top = tx[5] - fontAscent * Math.cos(angle);
}
```

### 2. **Percentage-Based Positioning**
**PDF.js Approach:**
- Uses percentage positioning relative to page dimensions
- More robust across different zoom levels
- Better handles dynamic resizing

**Implementation:**
```typescript
style.left = `${((100 * left) / this.pageWidth).toFixed(2)}%`;
style.top = `${((100 * top) / this.pageHeight).toFixed(2)}%`;
```

### 3. **Font Size with Min Font Size Multiplier**
**PDF.js Approach:**
- Multiplies font size by browser's minimum font size
- Scales down the rendered text to compensate
- Allows sub-pixel accuracy while respecting browser limits

**Implementation:**
```typescript
const minFontSize = TextLayerBuilder.minFontSize || 1;
style.fontSize = `${(minFontSize * fontHeight).toFixed(2)}px`;

// Then in layout:
if (minFontSize > 1) {
  transform = `scale(${1 / minFontSize})`;
}
```

### 4. **Canvas Width Scaling**
**PDF.js Approach:**
- Measures actual rendered text width using canvas context
- Scales horizontally to match PDF's specified width
- Only applies to multi-character text (optimization)

**Implementation:**
```typescript
if (properties.canvasWidth !== 0 && properties.hasText) {
  const ctx = TextLayerBuilder.getCanvasContext();
  ctx.font = `${fontSize * this.scale}px ${fontFamily}`;
  const { width } = ctx.measureText(textDiv.textContent || '');

  if (width > 0) {
    const scaleX = (canvasWidth * this.scale) / width;
    transform = `scaleX(${scaleX}) ${transform}`;
  }
}
```

### 5. **Smart Text Scaling Decision**
**PDF.js Logic:**
- Single-character text: Usually no scaling (performance)
- Multi-character text: Always scale
- Exception: Single-char with significant horizontal/vertical scale difference

**Implementation:**
```typescript
let shouldScaleText = false;
if (textItem.text.length > 1) {
  shouldScaleText = true;
} else if (textItem.text !== ' ' && tx[0] !== tx[3]) {
  const absScaleX = Math.abs(tx[0]);
  const absScaleY = Math.abs(tx[3]);
  if (absScaleX !== absScaleY && 
      Math.max(absScaleX, absScaleY) / Math.min(absScaleX, absScaleY) > 1.5) {
    shouldScaleText = true;
  }
}
```

### 6. **Font Ascent Calculation**
**PDF.js Approach:**
- Uses `fontBoundingBoxAscent` and `fontBoundingBoxDescent` from canvas metrics
- Calculates ratio: `ascent / (ascent + descent)`
- Falls back to 0.8 if metrics unavailable

**Implementation:**
```typescript
const ctx = this.getCanvasContext();
ctx.font = `${DEFAULT_FONT_SIZE}px ${fontFamily}`;
const metrics = ctx.measureText('');

const ascent = metrics.fontBoundingBoxAscent || 0;
const descent = Math.abs(metrics.fontBoundingBoxDescent || 0);

let ratio = 0.8; // Default fallback
if (ascent) {
  ratio = ascent / (ascent + descent);
}
```

### 7. **WeakMap for Properties**
**PDF.js Approach:**
- Uses WeakMap instead of Map for text div properties
- Better memory management (automatic garbage collection)
- No need for explicit cleanup

**Implementation:**
```typescript
private textDivProperties: WeakMap<HTMLElement, any> = new WeakMap();
```

## Benefits of This Approach

1. **Proven Accuracy**: PDF.js is used by millions of users daily
2. **Better Scaling**: Percentage-based positioning handles zoom better
3. **Performance**: Smart decisions about when to scale text
4. **Memory Efficient**: WeakMap allows automatic cleanup
5. **Cross-Browser Compatible**: Handles browser minimum font size restrictions
6. **Accurate Width Matching**: Canvas measurement ensures proper text width

## Testing

Open these demo files to test:
- `layout-comparison.html` - Side-by-side comparison
- `simple-text-viewer-fixed.html` - Full viewer with text selection
- `text-and-image-diagnostic.html` - Diagnostic information

## Debug Console Logs

The implementation includes debug logging for images:
- `[XObject]` - XObject display operations
- `[Image Rendering]` - Image loading and rendering
- Text operators are logged during extraction

Check browser console for detailed diagnostic information.

## Future Improvements

1. **Image Rendering**: Currently shows placeholders, need to handle:
   - Raw image data (not just JPEG/PNG/GIF)
   - DCTDecode, JPXDecode filters
   - ColorSpace transformations
   
2. **Font Matching**: Better font substitution for embedded fonts

3. **Ligatures**: Handle ligature detection and splitting

4. **Vertical Text**: Improve support for vertical writing modes
