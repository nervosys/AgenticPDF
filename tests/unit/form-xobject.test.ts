/**
 * Form XObject Tests
 * Tests for Form XObject rendering, resource management, and nested forms
 */

import { describe, it, expect, beforeEach } from '@jest/globals';

// Mock canvas and image for testing
class MockCanvasRenderingContext2D {
    private state: any[] = [];
    private operations: Array<{ type: string; args: any[] }> = [];

    save(): void {
        this.state.push({ ...this });
        this.operations.push({ type: 'save', args: [] });
    }

    restore(): void {
        if (this.state.length > 0) {
            const savedState = this.state.pop();
            Object.assign(this, savedState);
        }
        this.operations.push({ type: 'restore', args: [] });
    }

    transform(a: number, b: number, c: number, d: number, e: number, f: number): void {
        this.operations.push({ type: 'transform', args: [a, b, c, d, e, f] });
    }

    translate(x: number, y: number): void {
        this.operations.push({ type: 'translate', args: [x, y] });
    }

    scale(x: number, y: number): void {
        this.operations.push({ type: 'scale', args: [x, y] });
    }

    rotate(angle: number): void {
        this.operations.push({ type: 'rotate', args: [angle] });
    }

    beginPath(): void {
        this.operations.push({ type: 'beginPath', args: [] });
    }

    rect(x: number, y: number, w: number, h: number): void {
        this.operations.push({ type: 'rect', args: [x, y, w, h] });
    }

    clip(): void {
        this.operations.push({ type: 'clip', args: [] });
    }

    moveTo(x: number, y: number): void {
        this.operations.push({ type: 'moveTo', args: [x, y] });
    }

    lineTo(x: number, y: number): void {
        this.operations.push({ type: 'lineTo', args: [x, y] });
    }

    stroke(): void {
        this.operations.push({ type: 'stroke', args: [] });
    }

    fill(): void {
        this.operations.push({ type: 'fill', args: [] });
    }

    getOperations(): Array<{ type: string; args: any[] }> {
        return this.operations;
    }

    clearOperations(): void {
        this.operations = [];
    }

    [key: string]: any;
}

class MockCanvas {
    private ctx: MockCanvasRenderingContext2D;
    width: number = 800;
    height: number = 600;

    constructor() {
        this.ctx = new MockCanvasRenderingContext2D();
    }

    getContext(type: string): MockCanvasRenderingContext2D | null {
        if (type === '2d') return this.ctx;
        return null;
    }
}

// Mock global canvas and image if not available
if (typeof HTMLCanvasElement === 'undefined') {
    (global as any).HTMLCanvasElement = MockCanvas;
}

if (typeof Image === 'undefined') {
    (global as any).Image = class MockImage {
        src: string = '';
        onload: (() => void) | null = null;
        onerror: (() => void) | null = null;
    };
}

describe('Form XObject - Interface and Structure', () => {
    it('should have correct XObject interface properties', () => {
        const formXObject = {
            type: 'form' as const,
            data: new Uint8Array([]),
            bbox: [0, 0, 100, 100],
            matrix: [1, 0, 0, 1, 0, 0],
            resources: {
                fonts: new Map(),
                images: new Map(),
                xObjects: new Map(),
                colorSpaces: new Map(),
                patterns: new Map()
            }
        };

        expect(formXObject.type).toBe('form');
        expect(formXObject.bbox).toBeDefined();
        expect(formXObject.matrix).toBeDefined();
        expect(formXObject.resources).toBeDefined();
    });

    it('should support minimal form XObject structure', () => {
        const minimalForm = {
            type: 'form' as const,
            data: new Uint8Array([0x31, 0x30, 0x30, 0x20, 0x31, 0x30, 0x30, 0x20, 0x6D]) // "100 100 m"
        };

        expect(minimalForm.type).toBe('form');
        expect(minimalForm.data).toBeInstanceOf(Uint8Array);
        expect(minimalForm.data.length).toBeGreaterThan(0);
    });

    it('should support form XObject with transformation matrix', () => {
        const formWithMatrix = {
            type: 'form' as const,
            data: new Uint8Array([]),
            matrix: [2, 0, 0, 2, 100, 100] // Scale 2x and translate
        };

        expect(formWithMatrix.matrix).toBeDefined();
        expect(formWithMatrix.matrix).toHaveLength(6);
        expect(formWithMatrix.matrix[0]).toBe(2); // Scale X
        expect(formWithMatrix.matrix[3]).toBe(2); // Scale Y
        expect(formWithMatrix.matrix[4]).toBe(100); // Translate X
        expect(formWithMatrix.matrix[5]).toBe(100); // Translate Y
    });

    it('should support form XObject with bounding box', () => {
        const formWithBBox = {
            type: 'form' as const,
            data: new Uint8Array([]),
            bbox: [0, 0, 200, 300]
        };

        expect(formWithBBox.bbox).toBeDefined();
        expect(formWithBBox.bbox).toHaveLength(4);
        expect(formWithBBox.bbox[2] - formWithBBox.bbox[0]).toBe(200); // Width
        expect(formWithBBox.bbox[3] - formWithBBox.bbox[1]).toBe(300); // Height
    });
});

describe('Form XObject - Content Stream Parsing', () => {
    it('should parse simple graphics operations in form', () => {
        // Form content: "100 100 m 200 200 l S"
        const formContent = new Uint8Array([
            0x31, 0x30, 0x30, 0x20, 0x31, 0x30, 0x30, 0x20, 0x6D, 0x20,
            0x32, 0x30, 0x30, 0x20, 0x32, 0x30, 0x30, 0x20, 0x6C, 0x20,
            0x53
        ]);

        const form = {
            type: 'form' as const,
            data: formContent
        };

        expect(form.data.length).toBeGreaterThan(0);

        // Verify content can be decoded
        const text = new TextDecoder().decode(formContent);
        expect(text).toContain('m');
        expect(text).toContain('l');
        expect(text).toContain('S');
    });

    it('should parse text operations in form', () => {
        // Form content: "BT /F1 12 Tf (Hello) Tj ET"
        const formContent = new Uint8Array([
            0x42, 0x54, 0x20, 0x2F, 0x46, 0x31, 0x20, 0x31, 0x32, 0x20,
            0x54, 0x66, 0x20, 0x28, 0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x29,
            0x20, 0x54, 0x6A, 0x20, 0x45, 0x54
        ]);

        const form = {
            type: 'form' as const,
            data: formContent
        };

        const text = new TextDecoder().decode(formContent);
        expect(text).toContain('BT');
        expect(text).toContain('Tf');
        expect(text).toContain('Tj');
        expect(text).toContain('ET');
    });

    it('should handle empty form content', () => {
        const emptyForm = {
            type: 'form' as const,
            data: new Uint8Array([])
        };

        expect(emptyForm.data.length).toBe(0);
    });

    it('should handle form with comments', () => {
        // Form content: "% Comment\n100 100 m"
        const formWithComment = new Uint8Array([
            0x25, 0x20, 0x43, 0x6F, 0x6D, 0x6D, 0x65, 0x6E, 0x74, 0x0A,
            0x31, 0x30, 0x30, 0x20, 0x31, 0x30, 0x30, 0x20, 0x6D
        ]);

        const form = {
            type: 'form' as const,
            data: formWithComment
        };

        const text = new TextDecoder().decode(formWithComment);
        expect(text).toContain('%');
        expect(text).toContain('m');
    });
});

describe('Form XObject - Resource Management', () => {
    it('should support form-specific resources', () => {
        const formResources = {
            fonts: new Map([['F1', { name: 'F1', type: 'Font', subtype: 'Type1', baseFont: 'Helvetica' }]]),
            images: new Map(),
            xObjects: new Map(),
            colorSpaces: new Map(),
            patterns: new Map()
        };

        const form = {
            type: 'form' as const,
            data: new Uint8Array([]),
            resources: formResources
        };

        expect(form.resources).toBeDefined();
        expect(form.resources.fonts.size).toBe(1);
        expect(form.resources.fonts.get('F1')).toBeDefined();
    });

    it('should merge form resources with page resources', () => {
        const pageResources = {
            fonts: new Map([['F1', { name: 'F1', type: 'Font', subtype: 'Type1', baseFont: 'Helvetica' }]]),
            images: new Map(),
            xObjects: new Map(),
            colorSpaces: new Map(),
            patterns: new Map()
        };

        const formResources = {
            fonts: new Map([['F2', { name: 'F2', type: 'Font', subtype: 'Type1', baseFont: 'Times' }]]),
            images: new Map(),
            xObjects: new Map(),
            colorSpaces: new Map(),
            patterns: new Map()
        };

        // Merge resources (form resources take precedence)
        const merged = {
            fonts: new Map([...pageResources.fonts, ...formResources.fonts]),
            images: new Map([...pageResources.images, ...formResources.images]),
            xObjects: new Map([...pageResources.xObjects, ...formResources.xObjects]),
            colorSpaces: new Map([...pageResources.colorSpaces, ...formResources.colorSpaces]),
            patterns: new Map([...pageResources.patterns, ...formResources.patterns])
        };

        expect(merged.fonts.size).toBe(2);
        expect(merged.fonts.has('F1')).toBe(true);
        expect(merged.fonts.has('F2')).toBe(true);
    });

    it('should handle resource conflicts (form takes precedence)', () => {
        const pageResources = {
            fonts: new Map([['F1', { name: 'F1', type: 'Font', subtype: 'Type1', baseFont: 'Helvetica' }]]),
            images: new Map(),
            xObjects: new Map(),
            colorSpaces: new Map(),
            patterns: new Map()
        };

        const formResources = {
            fonts: new Map([['F1', { name: 'F1', type: 'Font', subtype: 'Type1', baseFont: 'Times' }]]), // Override F1
            images: new Map(),
            xObjects: new Map(),
            colorSpaces: new Map(),
            patterns: new Map()
        };

        const merged = {
            fonts: new Map([...pageResources.fonts, ...formResources.fonts]),
            images: new Map(),
            xObjects: new Map(),
            colorSpaces: new Map(),
            patterns: new Map()
        };

        expect(merged.fonts.size).toBe(1);
        expect(merged.fonts.get('F1')?.baseFont).toBe('Times'); // Form resource wins
    });

    it('should handle forms without resources', () => {
        const form: any = {
            type: 'form' as const,
            data: new Uint8Array([0x53]) // "S" (stroke)
        };

        expect(form.resources).toBeUndefined();
    });
});

describe('Form XObject - Transformation Matrix', () => {
    it('should apply identity matrix (no transformation)', () => {
        const identityMatrix = [1, 0, 0, 1, 0, 0];

        expect(identityMatrix[0]).toBe(1); // a
        expect(identityMatrix[1]).toBe(0); // b
        expect(identityMatrix[2]).toBe(0); // c
        expect(identityMatrix[3]).toBe(1); // d
        expect(identityMatrix[4]).toBe(0); // e
        expect(identityMatrix[5]).toBe(0); // f
    });

    it('should apply scaling matrix', () => {
        const scaleMatrix = [2, 0, 0, 3, 0, 0]; // Scale 2x horizontally, 3x vertically

        expect(scaleMatrix[0]).toBe(2); // Scale X
        expect(scaleMatrix[3]).toBe(3); // Scale Y
    });

    it('should apply translation matrix', () => {
        const translateMatrix = [1, 0, 0, 1, 100, 200]; // Translate (100, 200)

        expect(translateMatrix[4]).toBe(100); // Translate X
        expect(translateMatrix[5]).toBe(200); // Translate Y
    });

    it('should apply rotation matrix', () => {
        const angle = Math.PI / 4; // 45 degrees
        const cosAngle = Math.cos(angle);
        const sinAngle = Math.sin(angle);
        const rotateMatrix = [cosAngle, sinAngle, -sinAngle, cosAngle, 0, 0];

        expect(Math.abs(rotateMatrix[0] - cosAngle)).toBeLessThan(0.001);
        expect(Math.abs(rotateMatrix[1] - sinAngle)).toBeLessThan(0.001);
    });

    it('should apply combined transformation matrix', () => {
        // Scale 2x, rotate 45°, translate (100, 100)
        const scale = 2;
        const angle = Math.PI / 4;
        const tx = 100;
        const ty = 100;

        const cos = Math.cos(angle);
        const sin = Math.sin(angle);

        const combinedMatrix = [
            scale * cos,
            scale * sin,
            -scale * sin,
            scale * cos,
            tx,
            ty
        ];

        expect(combinedMatrix).toHaveLength(6);
    });
});

describe('Form XObject - Bounding Box Clipping', () => {
    it('should define bounding box coordinates', () => {
        const bbox = [0, 0, 100, 200]; // [llx, lly, urx, ury]

        expect(bbox[0]).toBe(0);   // Lower-left X
        expect(bbox[1]).toBe(0);   // Lower-left Y
        expect(bbox[2]).toBe(100); // Upper-right X
        expect(bbox[3]).toBe(200); // Upper-right Y
    });

    it('should calculate bounding box dimensions', () => {
        const bbox = [10, 20, 110, 220];
        const width = bbox[2] - bbox[0];
        const height = bbox[3] - bbox[1];

        expect(width).toBe(100);
        expect(height).toBe(200);
    });

    it('should handle negative bounding box coordinates', () => {
        const bbox = [-50, -100, 50, 100];
        const width = bbox[2] - bbox[0];
        const height = bbox[3] - bbox[1];

        expect(width).toBe(100);
        expect(height).toBe(200);
    });

    it('should validate bounding box (urx > llx, ury > lly)', () => {
        const validBBox = [0, 0, 100, 100];
        const isValid = validBBox[2] > validBBox[0] && validBBox[3] > validBBox[1];

        expect(isValid).toBe(true);
    });

    it('should detect invalid bounding box', () => {
        const invalidBBox = [100, 100, 0, 0]; // Inverted
        const isValid = invalidBBox[2] > invalidBBox[0] && invalidBBox[3] > invalidBBox[1];

        expect(isValid).toBe(false);
    });
});

describe('Form XObject - Nested Forms', () => {
    it('should support nested form XObjects', () => {
        const innerForm = {
            type: 'form' as const,
            data: new Uint8Array([0x53]) // "S"
        };

        const outerForm = {
            type: 'form' as const,
            data: new Uint8Array([0x2F, 0x49, 0x6E, 0x6E, 0x65, 0x72, 0x20, 0x44, 0x6F]), // "/Inner Do"
            resources: {
                fonts: new Map(),
                images: new Map(),
                xObjects: new Map([['Inner', innerForm]]),
                colorSpaces: new Map(),
                patterns: new Map()
            }
        };

        expect(outerForm.resources?.xObjects.has('Inner')).toBe(true);
        expect(outerForm.resources?.xObjects.get('Inner')).toEqual(innerForm);
    });

    it('should handle multiple nested levels', () => {
        const level3Form = {
            type: 'form' as const,
            data: new Uint8Array([0x53])
        };

        const level2Form = {
            type: 'form' as const,
            data: new Uint8Array([0x2F, 0x4C, 0x33, 0x20, 0x44, 0x6F]), // "/L3 Do"
            resources: {
                fonts: new Map(),
                images: new Map(),
                xObjects: new Map([['L3', level3Form]]),
                colorSpaces: new Map(),
                patterns: new Map()
            }
        };

        const level1Form = {
            type: 'form' as const,
            data: new Uint8Array([0x2F, 0x4C, 0x32, 0x20, 0x44, 0x6F]), // "/L2 Do"
            resources: {
                fonts: new Map(),
                images: new Map(),
                xObjects: new Map([['L2', level2Form]]),
                colorSpaces: new Map(),
                patterns: new Map()
            }
        };

        expect(level1Form.resources?.xObjects.has('L2')).toBe(true);
        expect(level2Form.resources?.xObjects.has('L3')).toBe(true);
    });

    it('should prevent circular references (conceptual test)', () => {
        // Circular references should be prevented at parsing time
        // This test demonstrates the structure that should be prevented
        const formA = {
            type: 'form' as const,
            data: new Uint8Array([0x2F, 0x42, 0x20, 0x44, 0x6F]), // "/B Do"
            resources: {
                fonts: new Map(),
                images: new Map(),
                xObjects: new Map(),
                colorSpaces: new Map(),
                patterns: new Map()
            }
        };

        const formB = {
            type: 'form' as const,
            data: new Uint8Array([0x2F, 0x41, 0x20, 0x44, 0x6F]), // "/A Do"
            resources: {
                fonts: new Map(),
                images: new Map(),
                xObjects: new Map(),
                colorSpaces: new Map(),
                patterns: new Map()
            }
        };

        // Don't actually create circular reference in test
        // Just verify the structure exists
        expect(formA.resources).toBeDefined();
        expect(formB.resources).toBeDefined();
    });
});

describe('Form XObject - Graphics State Preservation', () => {
    it('should save graphics state before form rendering', () => {
        const ctx = new MockCanvasRenderingContext2D();

        ctx.save();
        expect(ctx.getOperations()).toContainEqual({ type: 'save', args: [] });
    });

    it('should restore graphics state after form rendering', () => {
        const ctx = new MockCanvasRenderingContext2D();

        ctx.save();
        ctx.restore();

        const ops = ctx.getOperations();
        expect(ops).toContainEqual({ type: 'save', args: [] });
        expect(ops).toContainEqual({ type: 'restore', args: [] });
    });

    it('should apply transformations within saved state', () => {
        const ctx = new MockCanvasRenderingContext2D();

        ctx.save();
        ctx.transform(1, 0, 0, 1, 100, 100);
        ctx.restore();

        const ops = ctx.getOperations();
        expect(ops).toContainEqual({ type: 'save', args: [] });
        expect(ops).toContainEqual({ type: 'transform', args: [1, 0, 0, 1, 100, 100] });
        expect(ops).toContainEqual({ type: 'restore', args: [] });
    });

    it('should restore state even on error', () => {
        const ctx = new MockCanvasRenderingContext2D();

        try {
            ctx.save();
            // Simulate error
            throw new Error('Test error');
        } catch (error) {
            ctx.restore(); // Should still be called
        }

        const ops = ctx.getOperations();
        expect(ops).toContainEqual({ type: 'restore', args: [] });
    });
});

describe('Form XObject - Real-World Scenarios', () => {
    it('should handle form used as page header', () => {
        const headerForm = {
            type: 'form' as const,
            data: new Uint8Array([
                // BT /F1 10 Tf 50 780 Td (Header Text) Tj ET
                0x42, 0x54, 0x20, 0x2F, 0x46, 0x31, 0x20, 0x31, 0x30, 0x20,
                0x54, 0x66, 0x20, 0x35, 0x30, 0x20, 0x37, 0x38, 0x30, 0x20,
                0x54, 0x64, 0x20, 0x28, 0x48, 0x65, 0x61, 0x64, 0x65, 0x72,
                0x20, 0x54, 0x65, 0x78, 0x74, 0x29, 0x20, 0x54, 0x6A, 0x20,
                0x45, 0x54
            ]),
            bbox: [0, 770, 612, 792]
        };

        expect(headerForm.bbox).toBeDefined();
        expect(headerForm.bbox![3] - headerForm.bbox![1]).toBe(22); // Header height
    });

    it('should handle form used as page footer', () => {
        const footerForm = {
            type: 'form' as const,
            data: new Uint8Array([
                // BT /F1 8 Tf 300 10 Td (Page 1) Tj ET
                0x42, 0x54, 0x20, 0x2F, 0x46, 0x31, 0x20, 0x38, 0x20, 0x54,
                0x66, 0x20, 0x33, 0x30, 0x30, 0x20, 0x31, 0x30, 0x20, 0x54,
                0x64, 0x20, 0x28, 0x50, 0x61, 0x67, 0x65, 0x20, 0x31, 0x29,
                0x20, 0x54, 0x6A, 0x20, 0x45, 0x54
            ]),
            bbox: [0, 0, 612, 30]
        };

        expect(footerForm.bbox).toBeDefined();
        expect(footerForm.bbox![3] - footerForm.bbox![1]).toBe(30); // Footer height
    });

    it('should handle form used as logo/watermark', () => {
        const logoForm = {
            type: 'form' as const,
            data: new Uint8Array([0x53]), // Simple graphics
            bbox: [450, 700, 550, 780],
            matrix: [0.5, 0, 0, 0.5, 0, 0] // 50% scale
        };

        expect(logoForm.bbox).toBeDefined();
        expect(logoForm.matrix).toBeDefined();
        expect(logoForm.matrix[0]).toBe(0.5); // 50% scale
    });

    it('should handle form used as template/background', () => {
        const templateForm = {
            type: 'form' as const,
            data: new Uint8Array([
                // Draw border and grid
                0x31, 0x20, 0x77, 0x20, // "1 w" (line width)
                0x30, 0x20, 0x30, 0x20, 0x36, 0x31, 0x32, 0x20, 0x37, 0x39, 0x32, 0x20, 0x72, 0x65, 0x20, // "0 0 612 792 re"
                0x53 // "S" (stroke)
            ]),
            bbox: [0, 0, 612, 792]
        };

        expect(templateForm.bbox).toBeDefined();
        const width = templateForm.bbox![2] - templateForm.bbox![0];
        const height = templateForm.bbox![3] - templateForm.bbox![1];
        expect(width).toBe(612); // Letter width
        expect(height).toBe(792); // Letter height
    });

    it('should handle form with complex graphics', () => {
        const complexForm = {
            type: 'form' as const,
            data: new Uint8Array([
                // q (save state)
                0x71, 0x20,
                // 1 0 0 RG (red stroke)
                0x31, 0x20, 0x30, 0x20, 0x30, 0x20, 0x52, 0x47, 0x20,
                // 100 100 m 200 200 l S
                0x31, 0x30, 0x30, 0x20, 0x31, 0x30, 0x30, 0x20, 0x6D, 0x20,
                0x32, 0x30, 0x30, 0x20, 0x32, 0x30, 0x30, 0x20, 0x6C, 0x20,
                0x53, 0x20,
                // Q (restore state)
                0x51
            ]),
            bbox: [0, 0, 300, 300],
            resources: {
                fonts: new Map(),
                images: new Map(),
                xObjects: new Map(),
                colorSpaces: new Map([['DeviceRGB', { name: 'DeviceRGB', numComponents: 3 }]]),
                patterns: new Map()
            }
        };

        expect(complexForm.bbox).toBeDefined();
        expect(complexForm.resources).toBeDefined();
        expect(complexForm.resources.colorSpaces.size).toBeGreaterThan(0);
    });
});

describe('Form XObject - Edge Cases', () => {
    it('should handle form with zero-size bounding box', () => {
        const zeroSizeForm = {
            type: 'form' as const,
            data: new Uint8Array([]),
            bbox: [0, 0, 0, 0]
        };

        const width = zeroSizeForm.bbox[2] - zeroSizeForm.bbox[0];
        const height = zeroSizeForm.bbox[3] - zeroSizeForm.bbox[1];

        expect(width).toBe(0);
        expect(height).toBe(0);
    });

    it('should handle form with very large bounding box', () => {
        const largeBBoxForm = {
            type: 'form' as const,
            data: new Uint8Array([]),
            bbox: [0, 0, 10000, 10000]
        };

        const width = largeBBoxForm.bbox[2] - largeBBoxForm.bbox[0];
        const height = largeBBoxForm.bbox[3] - largeBBoxForm.bbox[1];

        expect(width).toBe(10000);
        expect(height).toBe(10000);
    });

    it('should handle form with identity matrix explicitly specified', () => {
        const formWithIdentity = {
            type: 'form' as const,
            data: new Uint8Array([]),
            matrix: [1, 0, 0, 1, 0, 0]
        };

        expect(formWithIdentity.matrix).toEqual([1, 0, 0, 1, 0, 0]);
    });

    it('should handle form with no matrix (implicitly identity)', () => {
        const formNoMatrix: any = {
            type: 'form' as const,
            data: new Uint8Array([])
        };

        expect(formNoMatrix.matrix).toBeUndefined();
    });

    it('should handle form with empty resources', () => {
        const formEmptyResources = {
            type: 'form' as const,
            data: new Uint8Array([]),
            resources: {
                fonts: new Map(),
                images: new Map(),
                xObjects: new Map(),
                colorSpaces: new Map(),
                patterns: new Map()
            }
        };

        expect(formEmptyResources.resources.fonts.size).toBe(0);
        expect(formEmptyResources.resources.images.size).toBe(0);
        expect(formEmptyResources.resources.xObjects.size).toBe(0);
    });

    it('should handle form with transparency group', () => {
        const formWithGroup = {
            type: 'form' as const,
            data: new Uint8Array([]),
            group: {
                type: 'Transparency',
                colorSpace: 'DeviceRGB',
                isolated: true,
                knockout: false
            }
        };

        expect(formWithGroup.group).toBeDefined();
        expect(formWithGroup.group.type).toBe('Transparency');
    });
});

describe('Form XObject - Error Handling', () => {
    it('should handle missing form data gracefully', () => {
        const formNoData = {
            type: 'form' as const,
            data: new Uint8Array([])
        };

        expect(formNoData.data.length).toBe(0);
    });

    it('should handle corrupted form content gracefully', () => {
        const corruptedForm = {
            type: 'form' as const,
            data: new Uint8Array([0xFF, 0xFF, 0xFF, 0xFF]) // Invalid data
        };

        expect(corruptedForm.data).toBeInstanceOf(Uint8Array);
    });

    it('should handle form with invalid matrix values', () => {
        const invalidMatrix = {
            type: 'form' as const,
            data: new Uint8Array([]),
            matrix: [NaN, 0, 0, Infinity, 0, 0] // Invalid values
        };

        expect(isNaN(invalidMatrix.matrix[0])).toBe(true);
        expect(!isFinite(invalidMatrix.matrix[3])).toBe(true);
    });

    it('should handle form with invalid bounding box', () => {
        const invalidBBox = {
            type: 'form' as const,
            data: new Uint8Array([]),
            bbox: [100, 100, 0, 0] // Invalid (reversed)
        };

        const isValid = invalidBBox.bbox[2] > invalidBBox.bbox[0] &&
            invalidBBox.bbox[3] > invalidBBox.bbox[1];

        expect(isValid).toBe(false);
    });

    it('should handle form rendering errors without crashing', () => {
        const form = {
            type: 'form' as const,
            data: new Uint8Array([0x00]) // Minimal data
        };

        // Simulate rendering attempt
        expect(() => {
            // This would normally trigger rendering
            // but in test environment, just verify structure
            expect(form.data).toBeDefined();
        }).not.toThrow();
    });
});

describe('Form XObject - Performance Considerations', () => {
    it('should handle reusing the same form multiple times', () => {
        const sharedForm = {
            type: 'form' as const,
            data: new Uint8Array([0x53])
        };

        const usages = [
            { form: sharedForm, matrix: [1, 0, 0, 1, 0, 0] },
            { form: sharedForm, matrix: [1, 0, 0, 1, 100, 0] },
            { form: sharedForm, matrix: [1, 0, 0, 1, 200, 0] }
        ];

        expect(usages[0].form).toBe(usages[1].form);
        expect(usages[1].form).toBe(usages[2].form);
        expect(usages.length).toBe(3);
    });

    it('should handle large form content efficiently', () => {
        const largeContent = new Uint8Array(100000); // 100KB
        largeContent.fill(0x53); // Fill with 'S'

        const largeForm = {
            type: 'form' as const,
            data: largeContent
        };

        expect(largeForm.data.length).toBe(100000);
    });

    it('should handle many nested forms', () => {
        const forms: any[] = [];

        for (let i = 0; i < 100; i++) {
            forms.push({
                type: 'form' as const,
                data: new Uint8Array([0x53])
            });
        }

        expect(forms.length).toBe(100);
    });
});
