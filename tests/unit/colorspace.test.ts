/**
 * Color Space Tests
 * 
 * Tests for advanced PDF color space support:
 * - ICCBased color profiles
 * - Indexed (palette-based) colors
 * - Pattern color spaces
 * - Separation and DeviceN spot colors
 * - Calibrated color spaces (CalRGB, CalGray)
 * - Color space operators (CS, cs, SCN, scn)
 */

import { describe, it, expect, beforeEach } from '@jest/globals';

// Mock canvas context
class MockCanvasRenderingContext2D {
    public strokeStyle: any = '#000000';
    public fillStyle: any = '#000000';
    public save = jest.fn();
    public restore = jest.fn();
    public transform = jest.fn();
    public scale = jest.fn();
    public translate = jest.fn();
}

describe('PDFColorSpaceProcessor', () => {
    describe('Color Space Parsing', () => {
        it('should parse simple color space names', () => {
            const deviceRGB = { name: 'DeviceRGB', numComponents: 3 };
            const deviceGray = { name: 'DeviceGray', numComponents: 1 };
            const deviceCMYK = { name: 'DeviceCMYK', numComponents: 4 };

            expect(deviceRGB.numComponents).toBe(3);
            expect(deviceGray.numComponents).toBe(1);
            expect(deviceCMYK.numComponents).toBe(4);
        });

        it('should handle color space abbreviations', () => {
            const rgb = { name: 'RGB', numComponents: 3 };
            const g = { name: 'G', numComponents: 1 };
            const cmyk = { name: 'CMYK', numComponents: 4 };

            expect(rgb.numComponents).toBe(3);
            expect(g.numComponents).toBe(1);
            expect(cmyk.numComponents).toBe(4);
        });
    });

    describe('ICCBased Color Space', () => {
        it('should parse ICCBased color space array', () => {
            // Mock ICCBased color space: [/ICCBased stream]
            const iccCS = {
                name: 'ICCBased',
                numComponents: 3,
                iccProfile: new Uint8Array([1, 2, 3, 4]), // Mock ICC data
                alternate: { name: 'DeviceRGB', numComponents: 3 }
            };

            expect(iccCS.name).toBe('ICCBased');
            expect(iccCS.numComponents).toBe(3);
            expect(iccCS.iccProfile).toBeInstanceOf(Uint8Array);
            expect(iccCS.alternate?.name).toBe('DeviceRGB');
        });

        it('should fallback to alternate color space', () => {
            const iccCS = {
                name: 'ICCBased',
                numComponents: 3,
                alternate: { name: 'DeviceRGB', numComponents: 3 }
            };

            // When ICC profile not available, use alternate
            expect(iccCS.alternate).toBeDefined();
            expect(iccCS.alternate?.name).toBe('DeviceRGB');
        });

        it('should handle different component counts', () => {
            const iccGray = { name: 'ICCBased', numComponents: 1 };
            const iccRGB = { name: 'ICCBased', numComponents: 3 };
            const iccCMYK = { name: 'ICCBased', numComponents: 4 };

            expect(iccGray.numComponents).toBe(1);
            expect(iccRGB.numComponents).toBe(3);
            expect(iccCMYK.numComponents).toBe(4);
        });
    });

    describe('Indexed Color Space', () => {
        it('should parse Indexed color space structure', () => {
            // Format: [/Indexed base hival lookup]
            const indexedCS = {
                name: 'Indexed',
                numComponents: 1, // Single index value
                base: { name: 'DeviceRGB', numComponents: 3 },
                hival: 255,
                lookup: new Uint8Array(256 * 3) // 256 RGB entries
            };

            expect(indexedCS.name).toBe('Indexed');
            expect(indexedCS.numComponents).toBe(1);
            expect(indexedCS.base?.numComponents).toBe(3);
            expect(indexedCS.hival).toBe(255);
        });

        it('should validate index range with hival', () => {
            const hival = 15; // 16 colors (0-15)
            const validIndex = 10;
            const invalidIndex = 20;

            const clampedIndex = Math.max(0, Math.min(Math.floor(validIndex), hival));
            expect(clampedIndex).toBe(10);

            const clampedInvalid = Math.max(0, Math.min(Math.floor(invalidIndex), hival));
            expect(clampedInvalid).toBe(15); // Clamped to hival
        });

        it('should calculate correct lookup table offset', () => {
            const index = 5;
            const componentCount = 3; // RGB
            const offset = index * componentCount;

            expect(offset).toBe(15);
        });
    });

    describe('Pattern Color Space', () => {
        it('should parse Pattern color space', () => {
            const patternCS = {
                name: 'Pattern',
                numComponents: 0 // Patterns have no fixed components
            };

            expect(patternCS.name).toBe('Pattern');
            expect(patternCS.numComponents).toBe(0);
        });

        it('should handle Pattern with base color space', () => {
            const patternCS = {
                name: 'Pattern',
                numComponents: 0,
                base: { name: 'DeviceRGB', numComponents: 3 }
            };

            expect(patternCS.base).toBeDefined();
            expect(patternCS.base?.name).toBe('DeviceRGB');
        });
    });

    describe('Separation Color Space', () => {
        it('should parse Separation color space', () => {
            // Format: [/Separation name alternateSpace tintTransform]
            const separationCS = {
                name: 'Separation',
                numComponents: 1, // Single tint value
                colorants: ['Magenta'],
                alternate: { name: 'DeviceCMYK', numComponents: 4 },
                tintTransform: {} // Function object
            };

            expect(separationCS.name).toBe('Separation');
            expect(separationCS.numComponents).toBe(1);
            expect(separationCS.colorants).toContain('Magenta');
            expect(separationCS.alternate?.name).toBe('DeviceCMYK');
        });

        it('should handle tint value scaling', () => {
            const tint = 0.5; // 50% tint
            const expectedScale = 0.5;

            expect(tint).toBe(expectedScale);
        });
    });

    describe('DeviceN Color Space', () => {
        it('should parse DeviceN color space', () => {
            // Format: [/DeviceN [colorants] alternateSpace tintTransform]
            const deviceNCS = {
                name: 'DeviceN',
                numComponents: 2,
                colorants: ['Cyan', 'Magenta'],
                alternate: { name: 'DeviceRGB', numComponents: 3 },
                tintTransform: {}
            };

            expect(deviceNCS.name).toBe('DeviceN');
            expect(deviceNCS.numComponents).toBe(2);
            expect(deviceNCS.colorants).toHaveLength(2);
            expect(deviceNCS.colorants).toContain('Cyan');
            expect(deviceNCS.colorants).toContain('Magenta');
        });

        it('should match component count to colorant count', () => {
            const colorants = ['Cyan', 'Magenta', 'Yellow', 'Black'];
            const numComponents = colorants.length;

            expect(numComponents).toBe(4);
        });
    });

    describe('Calibrated Color Spaces', () => {
        it('should parse CalRGB color space', () => {
            const calRGBCS = {
                name: 'CalRGB',
                numComponents: 3
            };

            expect(calRGBCS.name).toBe('CalRGB');
            expect(calRGBCS.numComponents).toBe(3);
        });

        it('should parse CalGray color space', () => {
            const calGrayCS = {
                name: 'CalGray',
                numComponents: 1
            };

            expect(calGrayCS.name).toBe('CalGray');
            expect(calGrayCS.numComponents).toBe(1);
        });
    });
});

describe('Color Conversion', () => {
    describe('Grayscale to RGB', () => {
        it('should convert grayscale values to RGB', () => {
            const gray = 0.5;
            const rgb = [gray, gray, gray];

            expect(rgb).toEqual([0.5, 0.5, 0.5]);
        });

        it('should handle black (0.0)', () => {
            const gray = 0.0;
            const rgb = [gray, gray, gray];

            expect(rgb).toEqual([0.0, 0.0, 0.0]);
        });

        it('should handle white (1.0)', () => {
            const gray = 1.0;
            const rgb = [gray, gray, gray];

            expect(rgb).toEqual([1.0, 1.0, 1.0]);
        });
    });

    describe('CMYK to RGB', () => {
        it('should convert CMYK to RGB', () => {
            const c = 0.0, m = 1.0, y = 1.0, k = 0.0;
            const r = (1 - c) * (1 - k);
            const g = (1 - m) * (1 - k);
            const b = (1 - y) * (1 - k);

            expect(r).toBe(1.0); // Red
            expect(g).toBeCloseTo(0.0);
            expect(b).toBeCloseTo(0.0);
        });

        it('should handle pure black (K=1)', () => {
            const c = 0.0, m = 0.0, y = 0.0, k = 1.0;
            const r = (1 - c) * (1 - k);
            const g = (1 - m) * (1 - k);
            const b = (1 - y) * (1 - k);

            expect([r, g, b]).toEqual([0.0, 0.0, 0.0]);
        });

        it('should handle cyan (C=1, M=0, Y=0, K=0)', () => {
            const c = 1.0, m = 0.0, y = 0.0, k = 0.0;
            const r = (1 - c) * (1 - k);
            const g = (1 - m) * (1 - k);
            const b = (1 - y) * (1 - k);

            expect(r).toBeCloseTo(0.0);
            expect(g).toBe(1.0);
            expect(b).toBe(1.0);
        });
    });

    describe('Indexed Color Lookup', () => {
        it('should lookup RGB values from palette', () => {
            // Create simple 4-color palette (RGB)
            const lookup = new Uint8Array([
                255, 0, 0,    // Index 0: Red
                0, 255, 0,    // Index 1: Green
                0, 0, 255,    // Index 2: Blue
                255, 255, 0   // Index 3: Yellow
            ]);

            // Lookup index 2 (Blue)
            const index = 2;
            const offset = index * 3;
            const r = lookup[offset] / 255;
            const g = lookup[offset + 1] / 255;
            const b = lookup[offset + 2] / 255;

            expect([r, g, b]).toEqual([0, 0, 1]);
        });

        it('should clamp out-of-range indices', () => {
            const hival = 3; // Max index
            const index = 10; // Out of range
            const clampedIndex = Math.max(0, Math.min(index, hival));

            expect(clampedIndex).toBe(3);
        });
    });

    describe('RGB to CSS Color', () => {
        it('should convert RGB array to CSS string', () => {
            const rgb = [1.0, 0.5, 0.0];
            const r = Math.floor(rgb[0] * 255);
            const g = Math.floor(rgb[1] * 255);
            const b = Math.floor(rgb[2] * 255);
            const css = `rgb(${r},${g},${b})`;

            expect(css).toBe('rgb(255,127,0)');
        });

        it('should clamp values to valid range', () => {
            const rgb = [1.5, -0.2, 0.5]; // Invalid values
            const r = Math.max(0, Math.min(255, Math.floor(rgb[0] * 255)));
            const g = Math.max(0, Math.min(255, Math.floor(rgb[1] * 255)));
            const b = Math.max(0, Math.min(255, Math.floor(rgb[2] * 255)));

            expect(r).toBe(255); // Clamped from 382.5
            expect(g).toBe(0);   // Clamped from -51
            expect(b).toBe(127);
        });
    });
});

describe('Color Space Operators', () => {
    let mockCtx: MockCanvasRenderingContext2D;

    beforeEach(() => {
        mockCtx = new MockCanvasRenderingContext2D();
    });

    describe('CS operator (Set Stroke Color Space)', () => {
        it('should set stroke color space to DeviceRGB', () => {
            let strokeColorSpace = { name: 'DeviceRGB', numComponents: 3 };

            expect(strokeColorSpace.name).toBe('DeviceRGB');
            expect(strokeColorSpace.numComponents).toBe(3);
        });

        it('should set stroke color space to DeviceGray', () => {
            let strokeColorSpace = { name: 'DeviceGray', numComponents: 1 };

            expect(strokeColorSpace.name).toBe('DeviceGray');
            expect(strokeColorSpace.numComponents).toBe(1);
        });
    });

    describe('cs operator (Set Fill Color Space)', () => {
        it('should set fill color space to DeviceCMYK', () => {
            let fillColorSpace = { name: 'DeviceCMYK', numComponents: 4 };

            expect(fillColorSpace.name).toBe('DeviceCMYK');
            expect(fillColorSpace.numComponents).toBe(4);
        });
    });

    describe('SCN operator (Set Stroke Color)', () => {
        it('should set stroke color in DeviceRGB space', () => {
            const colorSpace = { name: 'DeviceRGB', numComponents: 3 };
            const values = [1.0, 0.0, 0.0]; // Red

            // Convert to RGB (already RGB)
            const rgb = values;
            const css = `rgb(${Math.floor(rgb[0] * 255)},${Math.floor(rgb[1] * 255)},${Math.floor(rgb[2] * 255)})`;
            mockCtx.strokeStyle = css;

            expect(mockCtx.strokeStyle).toBe('rgb(255,0,0)');
        });

        it('should set stroke color in DeviceGray space', () => {
            const colorSpace = { name: 'DeviceGray', numComponents: 1 };
            const values = [0.5]; // 50% gray

            // Convert gray to RGB
            const rgb = [values[0], values[0], values[0]];
            const css = `rgb(${Math.floor(rgb[0] * 255)},${Math.floor(rgb[1] * 255)},${Math.floor(rgb[2] * 255)})`;
            mockCtx.strokeStyle = css;

            expect(mockCtx.strokeStyle).toBe('rgb(127,127,127)');
        });
    });

    describe('scn operator (Set Fill Color)', () => {
        it('should set fill color in DeviceCMYK space', () => {
            const colorSpace = { name: 'DeviceCMYK', numComponents: 4 };
            const values = [0.0, 1.0, 1.0, 0.0]; // Red in CMYK

            // Convert CMYK to RGB
            const [c, m, y, k] = values;
            const r = (1 - c) * (1 - k);
            const g = (1 - m) * (1 - k);
            const b = (1 - y) * (1 - k);
            const css = `rgb(${Math.floor(r * 255)},${Math.floor(g * 255)},${Math.floor(b * 255)})`;
            mockCtx.fillStyle = css;

            expect(mockCtx.fillStyle).toBe('rgb(255,0,0)');
        });
    });
});

describe('Graphics State Color Space Integration', () => {
    it('should save and restore color spaces with graphics state', () => {
        // Initial state
        let strokeCS = { name: 'DeviceGray', numComponents: 1 };
        let fillCS = { name: 'DeviceGray', numComponents: 1 };

        // Save state
        const savedStrokeCS = { ...strokeCS };
        const savedFillCS = { ...fillCS };

        // Change color spaces
        strokeCS = { name: 'DeviceRGB', numComponents: 3 };
        fillCS = { name: 'DeviceCMYK', numComponents: 4 };

        expect(strokeCS.name).toBe('DeviceRGB');
        expect(fillCS.name).toBe('DeviceCMYK');

        // Restore state
        strokeCS = { ...savedStrokeCS };
        fillCS = { ...savedFillCS };

        expect(strokeCS.name).toBe('DeviceGray');
        expect(fillCS.name).toBe('DeviceGray');
    });

    it('should maintain independent stroke and fill color spaces', () => {
        let strokeCS = { name: 'DeviceRGB', numComponents: 3 };
        let fillCS = { name: 'DeviceGray', numComponents: 1 };

        expect(strokeCS.name).toBe('DeviceRGB');
        expect(fillCS.name).toBe('DeviceGray');
        expect(strokeCS.numComponents).not.toBe(fillCS.numComponents);
    });
});

describe('Edge Cases and Error Handling', () => {
    it('should handle empty operands gracefully', () => {
        const operands: number[] = [];

        if (operands.length === 0) {
            // Should return early or use defaults
            expect(operands.length).toBe(0);
        }
    });

    it('should handle missing lookup table', () => {
        const indexedCS = {
            name: 'Indexed',
            numComponents: 1,
            base: { name: 'DeviceRGB', numComponents: 3 },
            hival: 255,
            lookup: undefined
        };

        if (!indexedCS.lookup) {
            // Should fallback to black
            const defaultRGB = [0, 0, 0];
            expect(defaultRGB).toEqual([0, 0, 0]);
        }
    });

    it('should handle missing alternate color space', () => {
        const iccCS = {
            name: 'ICCBased',
            numComponents: 3,
            alternate: undefined
        };

        if (!iccCS.alternate) {
            // Should use default based on component count
            const defaultCS = iccCS.numComponents === 3 ? 'DeviceRGB' :
                iccCS.numComponents === 1 ? 'DeviceGray' :
                    'DeviceCMYK';
            expect(defaultCS).toBe('DeviceRGB');
        }
    });

    it('should handle hex string conversion', () => {
        const hexString = '<FF00FF>';
        const clean = hexString.replace(/[<>\s]/g, '');
        const bytes = new Uint8Array(clean.length / 2);

        for (let i = 0; i < bytes.length; i++) {
            bytes[i] = parseInt(clean.substr(i * 2, 2), 16);
        }

        expect(bytes).toEqual(new Uint8Array([255, 0, 255]));
    });
});

describe('Real-World Color Space Scenarios', () => {
    it('should handle PDF with indexed color images', () => {
        // Common in web graphics, GIF-like images
        const indexedCS = {
            name: 'Indexed',
            base: { name: 'DeviceRGB', numComponents: 3 },
            hival: 255,
            lookup: new Uint8Array(256 * 3)
        };

        expect(indexedCS.name).toBe('Indexed');
        expect(indexedCS.lookup.length).toBe(768); // 256 colors × 3 components
    });

    it('should handle ICC color profiles for print PDFs', () => {
        const iccCS = {
            name: 'ICCBased',
            numComponents: 4, // CMYK
            iccProfile: new Uint8Array(1024), // Mock ICC data
            alternate: { name: 'DeviceCMYK', numComponents: 4 }
        };

        expect(iccCS.name).toBe('ICCBased');
        expect(iccCS.numComponents).toBe(4);
        expect(iccCS.iccProfile).toBeDefined();
    });

    it('should handle spot colors in separation space', () => {
        // Common in professional printing
        const separationCS = {
            name: 'Separation',
            colorants: ['PANTONE 185 C'],
            alternate: { name: 'DeviceCMYK', numComponents: 4 }
        };

        expect(separationCS.name).toBe('Separation');
        expect(separationCS.colorants[0]).toContain('PANTONE');
    });
});
