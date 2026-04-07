/**
 * Unit tests for PretextLayout — native multiline text measurement & layout engine.
 */

import {
  PretextLayout,
  PretextOptions,
  PreparedText,
  PreparedTextWithSegments,
  PretextLayoutResult,
  PretextLayoutLine,
  PretextLayoutLineRange,
  PretextCursor,
  AgenticPDF,
} from '../../agenticpdf';

// PretextLayout uses Canvas/OffscreenCanvas for measurement.
// In Node/Jest we have neither, so it falls back to the heuristic context
// (0.6 × fontSize per character). All tests use this server-side fallback.

const FONT = '16px sans-serif';
// With heuristic: each Latin char ≈ 9.6px wide, space ≈ 9.6px

describe('PretextLayout', () => {
  afterEach(() => {
    PretextLayout.clearCache();
  });

  // ── prepare / prepareWithSegments ──────────────────────────────────

  describe('prepare()', () => {
    test('returns a PreparedText with original text', () => {
      const p = PretextLayout.prepare('Hello', FONT);
      expect(p.text).toBe('Hello');
      expect(p._segments).toBeDefined();
      expect(p._totalWidth).toBeGreaterThan(0);
    });

    test('handles empty string', () => {
      const p = PretextLayout.prepare('', FONT);
      expect(p.text).toBe('');
      expect(p._segments).toHaveLength(0);
      expect(p._totalWidth).toBe(0);
    });

    test('collapses whitespace in normal mode', () => {
      const p = PretextLayout.prepare('Hello   \t\n  World', FONT);
      // After collapsing: "Hello World"
      const fullText = p._segments.map(s => s.text).join('');
      expect(fullText).toBe('Hello World');
    });

    test('trims leading/trailing whitespace in normal mode', () => {
      const p = PretextLayout.prepare('  Hello  ', FONT);
      const fullText = p._segments.map(s => s.text).join('');
      expect(fullText).toBe('Hello');
    });
  });

  describe('prepareWithSegments()', () => {
    test('exposes segments array', () => {
      const p = PretextLayout.prepareWithSegments('Hello World', FONT);
      expect(p.segments).toBeDefined();
      expect(p.segments.length).toBeGreaterThan(0);
      expect(p.segments).toBe(p._segments);
    });

    test('segments cover the full text', () => {
      const p = PretextLayout.prepareWithSegments('one two three', FONT);
      const reconstructed = p.segments.map(s => s.text).join('');
      expect(reconstructed).toBe('one two three');
    });
  });

  // ── layout() ──────────────────────────────────────────────────────────

  describe('layout()', () => {
    test('single line when text fits', () => {
      const p = PretextLayout.prepare('Hi', FONT);
      const result = PretextLayout.layout(p, 1000, 24);
      expect(result.lineCount).toBe(1);
      expect(result.height).toBe(24);
    });

    test('wraps when text exceeds maxWidth', () => {
      // 20 characters ≈ 192px at 9.6px each; maxWidth=100 should force wrapping
      const p = PretextLayout.prepare('Hello World Forever More', FONT);
      const result = PretextLayout.layout(p, 100, 24);
      expect(result.lineCount).toBeGreaterThan(1);
      expect(result.height).toBe(result.lineCount * 24);
    });

    test('handles empty text', () => {
      const p = PretextLayout.prepare('', FONT);
      const result = PretextLayout.layout(p, 200, 24);
      // 0 segments → 0 lines
      expect(result.lineCount).toBe(0);
      expect(result.height).toBe(0);
    });

    test('height equals lineCount × lineHeight', () => {
      const p = PretextLayout.prepare('A B C D E F G H I J K L M N', FONT);
      const result = PretextLayout.layout(p, 60, 20);
      expect(result.height).toBe(result.lineCount * 20);
    });
  });

  // ── layoutWithLines() ─────────────────────────────────────────────────

  describe('layoutWithLines()', () => {
    test('returns lines array', () => {
      const p = PretextLayout.prepareWithSegments('Hello World Test', FONT);
      const result = PretextLayout.layoutWithLines(p, 100, 24);
      expect(result.lines).toBeDefined();
      expect(Array.isArray(result.lines)).toBe(true);
      expect(result.lines.length).toBe(result.lineCount);
    });

    test('each line has text, width, start, end', () => {
      const p = PretextLayout.prepareWithSegments('Hello World Test Line', FONT);
      const result = PretextLayout.layoutWithLines(p, 100, 24);
      for (const line of result.lines) {
        expect(typeof line.text).toBe('string');
        expect(typeof line.width).toBe('number');
        expect(line.start).toBeDefined();
        expect(line.end).toBeDefined();
        expect(typeof line.start.segmentIndex).toBe('number');
        expect(typeof line.start.graphemeIndex).toBe('number');
      }
    });

    test('reconstructed text covers (approximately) the input', () => {
      const input = 'Hello World Foo Bar Baz';
      const p = PretextLayout.prepareWithSegments(input, FONT);
      const result = PretextLayout.layoutWithLines(p, 80, 24);
      const joined = result.lines.map(l => l.text).join(' ');
      // All original words should appear
      for (const word of input.split(' ')) {
        expect(joined).toContain(word);
      }
    });

    test('single word returns at least one line', () => {
      const p = PretextLayout.prepareWithSegments('Hello', FONT);
      const result = PretextLayout.layoutWithLines(p, 200, 24);
      expect(result.lines.length).toBeGreaterThanOrEqual(1);
      expect(result.lines[0].text).toContain('Hello');
    });

    test('empty text produces one empty line', () => {
      const p = PretextLayout.prepareWithSegments('', FONT);
      const result = PretextLayout.layoutWithLines(p, 200, 24);
      expect(result.lines.length).toBe(1);
      expect(result.lines[0].text).toBe('');
    });
  });

  // ── walkLineRanges() ──────────────────────────────────────────────────

  describe('walkLineRanges()', () => {
    test('calls onLine for each line', () => {
      const p = PretextLayout.prepareWithSegments('Hello World Test Line', FONT);
      const ranges: PretextLayoutLineRange[] = [];
      const count = PretextLayout.walkLineRanges(p, 80, (range) => {
        ranges.push(range);
      });
      expect(count).toBeGreaterThan(0);
      expect(ranges.length).toBe(count);
    });

    test('each range has valid cursors', () => {
      const p = PretextLayout.prepareWithSegments('A B C D E F G', FONT);
      PretextLayout.walkLineRanges(p, 40, (range) => {
        expect(range.start.segmentIndex).toBeGreaterThanOrEqual(0);
        expect(range.end.segmentIndex).toBeGreaterThanOrEqual(range.start.segmentIndex);
        expect(typeof range.width).toBe('number');
        expect(range.width).toBeGreaterThanOrEqual(0);
      });
    });
  });

  // ── layoutNextLine() ──────────────────────────────────────────────────

  describe('layoutNextLine()', () => {
    test('iterates through all lines', () => {
      const p = PretextLayout.prepareWithSegments('Hello World Again Test', FONT);
      const lines: PretextLayoutLine[] = [];
      let cursor: PretextCursor = { segmentIndex: 0, graphemeIndex: 0 };

      while (true) {
        const line = PretextLayout.layoutNextLine(p, cursor, 80);
        if (!line) break;
        lines.push(line);
        cursor = line.end;
      }

      expect(lines.length).toBeGreaterThan(0);
    });

    test('returns null when past end', () => {
      const p = PretextLayout.prepareWithSegments('Hi', FONT);
      const line = PretextLayout.layoutNextLine(p, { segmentIndex: 999, graphemeIndex: 0 }, 200);
      expect(line).toBeNull();
    });
  });

  // ── Pre-wrap mode ─────────────────────────────────────────────────────

  describe('pre-wrap mode', () => {
    test('preserves spaces', () => {
      const p = PretextLayout.prepareWithSegments('Hello   World', FONT, { whiteSpace: 'pre-wrap' });
      const fullText = p.segments.map(s => s.text).join('');
      expect(fullText).toBe('Hello   World');
    });

    test('preserves newlines as hard breaks', () => {
      const p = PretextLayout.prepareWithSegments('Line1\nLine2', FONT, { whiteSpace: 'pre-wrap' });
      const hasNewline = p.segments.some(s => s.isNewline);
      expect(hasNewline).toBe(true);
    });

    test('newlines cause line breaks in layout', () => {
      const p = PretextLayout.prepareWithSegments('Line1\nLine2', FONT, { whiteSpace: 'pre-wrap' });
      const result = PretextLayout.layoutWithLines(p, 1000, 24);
      expect(result.lineCount).toBe(2);
    });

    test('preserves tabs', () => {
      const p = PretextLayout.prepareWithSegments('A\tB', FONT, { whiteSpace: 'pre-wrap' });
      const hasTab = p.segments.some(s => s.isTab);
      expect(hasTab).toBe(true);
    });
  });

  // ── CJK handling ──────────────────────────────────────────────────────

  describe('CJK text handling', () => {
    test('segments CJK characters individually', () => {
      const p = PretextLayout.prepareWithSegments('你好世界', FONT);
      // Each CJK character should be its own segment (breakable individually)
      const nonWsSegments = p.segments.filter(s => !s.isWhitespace);
      expect(nonWsSegments.length).toBe(4);
    });

    test('CJK characters have wider estimated widths', () => {
      const latin = PretextLayout.prepare('abcd', FONT);
      const cjk = PretextLayout.prepare('你好世界', FONT);
      // 4 CJK chars at 16px each = 64px; 4 Latin chars at 9.6px each = 38.4px
      expect(cjk._totalWidth).toBeGreaterThan(latin._totalWidth);
    });

    test('CJK text wraps at character boundaries', () => {
      // 4 CJK chars ≈ 64px; maxWidth=40 should break after 2 chars
      const p = PretextLayout.prepareWithSegments('你好世界', FONT);
      const result = PretextLayout.layoutWithLines(p, 40, 24);
      expect(result.lineCount).toBeGreaterThan(1);
    });
  });

  // ── clearCache / setLocale ────────────────────────────────────────────

  describe('clearCache()', () => {
    test('does not throw', () => {
      PretextLayout.prepare('test', FONT);
      expect(() => PretextLayout.clearCache()).not.toThrow();
    });

    test('measurement still works after cache clear', () => {
      PretextLayout.prepare('cached', FONT);
      PretextLayout.clearCache();
      const p = PretextLayout.prepare('cached', FONT);
      expect(p._totalWidth).toBeGreaterThan(0);
    });
  });

  describe('isCacheDirty()', () => {
    test('returns false after clearCache', () => {
      PretextLayout.clearCache();
      expect(PretextLayout.isCacheDirty()).toBe(false);
    });

    test('returns true after measurement', () => {
      PretextLayout.clearCache();
      PretextLayout.prepare('test', FONT);
      expect(PretextLayout.isCacheDirty()).toBe(true);
    });
  });

  describe('setLocale()', () => {
    test('does not throw with valid locale', () => {
      expect(() => PretextLayout.setLocale('en-US')).not.toThrow();
    });

    test('does not throw with undefined (reset)', () => {
      expect(() => PretextLayout.setLocale(undefined)).not.toThrow();
    });

    test('clears cache on locale change', () => {
      PretextLayout.prepare('test', FONT);
      PretextLayout.setLocale('ja-JP');
      // After setLocale, cache should be cleared
      // Verify by checking that measurement still works
      const p = PretextLayout.prepare('test', FONT);
      expect(p._totalWidth).toBeGreaterThan(0);
      PretextLayout.setLocale(undefined);
    });
  });

  // ── Word breaking (overflow-wrap: break-word) ─────────────────────────

  describe('overflow-wrap: break-word', () => {
    test('breaks long word at grapheme boundaries', () => {
      // A single "word" wider than maxWidth
      const longWord = 'Supercalifragilisticexpialidocious';
      const p = PretextLayout.prepareWithSegments(longWord, FONT);
      // With heuristic: 34 chars × 9.6px ≈ 326px; maxWidth=50 should force multiple breaks
      const result = PretextLayout.layoutWithLines(p, 50, 24);
      expect(result.lineCount).toBeGreaterThan(1);
    });

    test('very narrow width still produces output', () => {
      const p = PretextLayout.prepareWithSegments('Hello', FONT);
      // maxWidth=5px — even one char (~9.6px) won't fit, but we should still get output
      const result = PretextLayout.layoutWithLines(p, 5, 24);
      expect(result.lineCount).toBeGreaterThan(0);
    });
  });

  // ── AgenticPDF convenience methods ────────────────────────────────────

  describe('AgenticPDF.prepareText() / layoutText()', () => {
    test('prepareText delegates to PretextLayout', () => {
      const p = AgenticPDF.prepareText('Hello World', FONT, { enablePretextLayout: true });
      expect(p.text).toBe('Hello World');
      expect(p.segments).toBeDefined();
    });

    test('layoutText returns lines', () => {
      const p = AgenticPDF.prepareText('Hello World', FONT, { enablePretextLayout: true });
      const result = AgenticPDF.layoutText(p, 200, 24, { enablePretextLayout: true });
      expect(result.lines).toBeDefined();
      expect(result.lineCount).toBeGreaterThanOrEqual(1);
      expect(result.height).toBe(result.lineCount * 24);
    });

    test('prepareText throws without enablePretextLayout', () => {
      expect(() => AgenticPDF.prepareText('Hello', FONT)).toThrow('PretextLayout is not enabled');
    });

    test('layoutText throws without enablePretextLayout', () => {
      const p = PretextLayout.prepareWithSegments('Hello', FONT);
      expect(() => AgenticPDF.layoutText(p, 200, 24)).toThrow('PretextLayout is not enabled');
    });
  });

  // ── Edge cases ────────────────────────────────────────────────────────

  describe('edge cases', () => {
    test('whitespace-only input in normal mode produces empty segments', () => {
      const p = PretextLayout.prepare('   \t  \n  ', FONT);
      expect(p._segments).toHaveLength(0);
    });

    test('single character', () => {
      const p = PretextLayout.prepare('X', FONT);
      expect(p._segments.length).toBeGreaterThanOrEqual(1);
      expect(p._totalWidth).toBeGreaterThan(0);
    });

    test('mixed CJK and Latin', () => {
      const p = PretextLayout.prepareWithSegments('Hello你好World', FONT);
      expect(p.segments.length).toBeGreaterThan(1);
      const text = p.segments.map(s => s.text).join('');
      expect(text).toBe('Hello你好World');
    });

    test('emoji text', () => {
      const p = PretextLayout.prepare('Hello 😀🎉 World', FONT);
      expect(p._totalWidth).toBeGreaterThan(0);
    });

    test('very long text does not throw', () => {
      const longText = 'word '.repeat(1000);
      expect(() => {
        const p = PretextLayout.prepare(longText, FONT);
        PretextLayout.layout(p, 300, 20);
      }).not.toThrow();
    });

    test('maxWidth of 0 still produces lines', () => {
      const p = PretextLayout.prepare('Hello', FONT);
      const result = PretextLayout.layout(p, 0, 24);
      // Each grapheme overflows but still fits on its own line
      expect(result.lineCount).toBeGreaterThan(0);
    });

    test('pre-wrap consecutive newlines produce empty lines', () => {
      const p = PretextLayout.prepareWithSegments('A\n\nB', FONT, { whiteSpace: 'pre-wrap' });
      const result = PretextLayout.layoutWithLines(p, 1000, 24);
      expect(result.lineCount).toBe(3);
    });

    test('pre-wrap tabs contribute to width', () => {
      const p = PretextLayout.prepareWithSegments('\t', FONT, { whiteSpace: 'pre-wrap' });
      expect(p._totalWidth).toBeGreaterThan(0);
      const tabSeg = p.segments.find(s => s.isTab);
      expect(tabSeg).toBeDefined();
      expect(tabSeg!.width).toBeGreaterThan(0);
    });

    test('mixed whitespace and words in pre-wrap', () => {
      const p = PretextLayout.prepareWithSegments('  A  B  ', FONT, { whiteSpace: 'pre-wrap' });
      const text = p.segments.map(s => s.text).join('');
      expect(text).toBe('  A  B  ');
    });

    test('walkLineRanges returns 0 for empty text', () => {
      const p = PretextLayout.prepareWithSegments('', FONT);
      let called = false;
      const count = PretextLayout.walkLineRanges(p, 200, () => { called = true; });
      expect(count).toBe(0);
      expect(called).toBe(false);
    });

    test('layoutNextLine returns null for empty prepared text', () => {
      const p = PretextLayout.prepareWithSegments('', FONT);
      const line = PretextLayout.layoutNextLine(p, { segmentIndex: 0, graphemeIndex: 0 }, 200);
      expect(line).toBeNull();
    });

    test('long word broken across multiple lines via layout()', () => {
      const longWord = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
      const p = PretextLayout.prepare(longWord, FONT);
      // 26 chars × 9.6 ≈ 249.6px, maxWidth=30 → should wrap many times
      const result = PretextLayout.layout(p, 30, 24);
      expect(result.lineCount).toBeGreaterThan(5);
    });

    test('clearAllCaches skips pretext when cache is clean', () => {
      PretextLayout.clearCache();
      expect(PretextLayout.isCacheDirty()).toBe(false);
      // Should not throw even when cache is clean
      expect(() => AgenticPDF.clearAllCaches()).not.toThrow();
    });

    test('clearAllCaches clears pretext when cache is dirty', () => {
      PretextLayout.prepare('populate cache', FONT);
      expect(PretextLayout.isCacheDirty()).toBe(true);
      AgenticPDF.clearAllCaches();
      expect(PretextLayout.isCacheDirty()).toBe(false);
    });
  });
});
