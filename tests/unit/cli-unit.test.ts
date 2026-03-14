/**
 * AgenticPDF CLI Unit Tests
 * 
 * Unit tests for CLI helper functions and argument parsing
 */

import { describe, it, expect } from '@jest/globals';

describe('CLI Argument Parsing - Unit Tests', () => {
    /**
     * Mock argument parser to test logic
     */
    function parseArgs(args: string[]): any {
        const options: any = {
            command: args[0] || 'help',
        };

        for (let i = 1; i < args.length; i++) {
            const arg = args[i];
            const nextArg = args[i + 1];

            switch (arg) {
                case '-i':
                case '--input':
                    options.input = nextArg;
                    i++;
                    break;
                case '-o':
                case '--output':
                    options.output = nextArg;
                    i++;
                    break;
                case '-p':
                case '--pages':
                    options.pages = nextArg;
                    i++;
                    break;
                case '-f':
                case '--format':
                    options.format = nextArg;
                    i++;
                    break;
                case '-v':
                case '--verbose':
                    options.verbose = true;
                    break;
                case '--pretty':
                    options.pretty = true;
                    break;
                case '-m':
                case '--metadata':
                    options.metadata = true;
                    break;
                case '--tables':
                    options.tables = true;
                    break;
                case '--images':
                    options.images = true;
                    break;
                case '--forms':
                    options.forms = true;
                    break;
                case '--annotations':
                    options.annotations = true;
                    break;
                case '--ai':
                    options.ai = true;
                    break;
                case '--chunk':
                    options.chunk = true;
                    break;
                case '--chunk-size':
                    options.chunkSize = parseInt(nextArg, 10);
                    i++;
                    break;
                case '--stream':
                    options.stream = true;
                    break;
                case '-h':
                case '--help':
                    options.help = true;
                    break;
                case '--version':
                    options.version = true;
                    break;
                default:
                    if (!options.input && !arg.startsWith('-')) {
                        options.input = arg;
                    }
            }
        }

        return options;
    }

    describe('Basic Command Parsing', () => {
        it('should parse command name', () => {
            const options = parseArgs(['info', 'test.pdf']);
            expect(options.command).toBe('info');
        });

        it('should default to help command', () => {
            const options = parseArgs([]);
            expect(options.command).toBe('help');
        });

        it('should parse multiple commands', () => {
            expect(parseArgs(['info']).command).toBe('info');
            expect(parseArgs(['extract']).command).toBe('extract');
            expect(parseArgs(['convert']).command).toBe('convert');
            expect(parseArgs(['analyze']).command).toBe('analyze');
            expect(parseArgs(['chunk']).command).toBe('chunk');
            expect(parseArgs(['images']).command).toBe('images');
            expect(parseArgs(['forms']).command).toBe('forms');
        });
    });

    describe('Input/Output Flags', () => {
        it('should parse short input flag', () => {
            const options = parseArgs(['info', '-i', 'test.pdf']);
            expect(options.input).toBe('test.pdf');
        });

        it('should parse long input flag', () => {
            const options = parseArgs(['info', '--input', 'test.pdf']);
            expect(options.input).toBe('test.pdf');
        });

        it('should parse short output flag', () => {
            const options = parseArgs(['extract', '-o', 'output.txt']);
            expect(options.output).toBe('output.txt');
        });

        it('should parse long output flag', () => {
            const options = parseArgs(['extract', '--output', 'output.txt']);
            expect(options.output).toBe('output.txt');
        });

        it('should parse input without flag', () => {
            const options = parseArgs(['info', 'test.pdf']);
            expect(options.input).toBe('test.pdf');
        });
    });

    describe('Boolean Flags', () => {
        it('should parse verbose flag (short)', () => {
            const options = parseArgs(['info', '-v']);
            expect(options.verbose).toBe(true);
        });

        it('should parse verbose flag (long)', () => {
            const options = parseArgs(['info', '--verbose']);
            expect(options.verbose).toBe(true);
        });

        it('should parse pretty flag', () => {
            const options = parseArgs(['convert', '--pretty']);
            expect(options.pretty).toBe(true);
        });

        it('should parse metadata flag', () => {
            const options = parseArgs(['extract', '-m']);
            expect(options.metadata).toBe(true);
        });

        it('should parse tables flag', () => {
            const options = parseArgs(['extract', '--tables']);
            expect(options.tables).toBe(true);
        });

        it('should parse images flag', () => {
            const options = parseArgs(['convert', '--images']);
            expect(options.images).toBe(true);
        });

        it('should parse forms flag', () => {
            const options = parseArgs(['convert', '--forms']);
            expect(options.forms).toBe(true);
        });

        it('should parse annotations flag', () => {
            const options = parseArgs(['convert', '--annotations']);
            expect(options.annotations).toBe(true);
        });

        it('should parse ai flag', () => {
            const options = parseArgs(['analyze', '--ai']);
            expect(options.ai).toBe(true);
        });

        it('should parse chunk flag', () => {
            const options = parseArgs(['chunk', '--chunk']);
            expect(options.chunk).toBe(true);
        });

        it('should parse stream flag', () => {
            const options = parseArgs(['extract', '--stream']);
            expect(options.stream).toBe(true);
        });

        it('should parse help flag', () => {
            const options = parseArgs(['info', '--help']);
            expect(options.help).toBe(true);
        });

        it('should parse version flag', () => {
            const options = parseArgs(['info', '--version']);
            expect(options.version).toBe(true);
        });
    });

    describe('Value Flags', () => {
        it('should parse pages flag', () => {
            const options = parseArgs(['extract', '-p', '1-5']);
            expect(options.pages).toBe('1-5');
        });

        it('should parse format flag', () => {
            const options = parseArgs(['convert', '-f', 'json']);
            expect(options.format).toBe('json');
        });

        it('should parse chunk-size flag', () => {
            const options = parseArgs(['chunk', '--chunk-size', '1000']);
            expect(options.chunkSize).toBe(1000);
        });
    });

    describe('Multiple Flags', () => {
        it('should parse multiple flags together', () => {
            const options = parseArgs([
                'extract',
                '-i', 'test.pdf',
                '-o', 'output.txt',
                '-v',
                '--metadata',
                '--tables'
            ]);
            expect(options.command).toBe('extract');
            expect(options.input).toBe('test.pdf');
            expect(options.output).toBe('output.txt');
            expect(options.verbose).toBe(true);
            expect(options.metadata).toBe(true);
            expect(options.tables).toBe(true);
        });

        it('should parse all flags for convert command', () => {
            const options = parseArgs([
                'convert',
                '-i', 'test.pdf',
                '-f', 'json',
                '--pretty',
                '--metadata',
                '--tables',
                '--annotations'
            ]);
            expect(options.command).toBe('convert');
            expect(options.format).toBe('json');
            expect(options.pretty).toBe(true);
            expect(options.metadata).toBe(true);
            expect(options.tables).toBe(true);
            expect(options.annotations).toBe(true);
        });
    });
});

describe('Page Range Parsing - Unit Tests', () => {
    /**
     * Mock page range parser
     */
    function parsePageRange(rangeStr: string): { start?: number; end?: number } | number[] {
        if (rangeStr.includes('-')) {
            const [start, end] = rangeStr.split('-').map(n => parseInt(n.trim(), 10));
            return { start, end };
        } else if (rangeStr.includes(',')) {
            return rangeStr.split(',').map(n => parseInt(n.trim(), 10));
        } else {
            const page = parseInt(rangeStr, 10);
            return { start: page, end: page };
        }
    }

    it('should parse single page', () => {
        const result = parsePageRange('5');
        expect(result).toEqual({ start: 5, end: 5 });
    });

    it('should parse page range', () => {
        const result = parsePageRange('1-10');
        expect(result).toEqual({ start: 1, end: 10 });
    });

    it('should parse comma-separated pages', () => {
        const result = parsePageRange('1,3,5,7');
        expect(result).toEqual([1, 3, 5, 7]);
    });

    it('should handle spaces in range', () => {
        const result = parsePageRange('1 - 10');
        expect(result).toEqual({ start: 1, end: 10 });
    });

    it('should handle spaces in comma list', () => {
        const result = parsePageRange('1, 3, 5');
        expect(result).toEqual([1, 3, 5]);
    });
});

describe('Output Format Validation - Unit Tests', () => {
    const validFormats = ['text', 'json', 'html', 'markdown'];

    function isValidFormat(format: string): boolean {
        return validFormats.includes(format.toLowerCase());
    }

    it('should validate text format', () => {
        expect(isValidFormat('text')).toBe(true);
    });

    it('should validate json format', () => {
        expect(isValidFormat('json')).toBe(true);
    });

    it('should validate html format', () => {
        expect(isValidFormat('html')).toBe(true);
    });

    it('should validate markdown format', () => {
        expect(isValidFormat('markdown')).toBe(true);
    });

    it('should reject invalid format', () => {
        expect(isValidFormat('invalid')).toBe(false);
    });

    it('should be case insensitive', () => {
        expect(isValidFormat('JSON')).toBe(true);
        expect(isValidFormat('HTML')).toBe(true);
    });
});

describe('Command Validation - Unit Tests', () => {
    const validCommands = [
        'info', 'extract', 'render', 'convert', 'analyze',
        'chunk', 'forms', 'images', 'merge', 'split',
        'help', 'version'
    ];

    function isValidCommand(command: string): boolean {
        return validCommands.includes(command.toLowerCase());
    }

    it('should validate all commands', () => {
        validCommands.forEach(cmd => {
            expect(isValidCommand(cmd)).toBe(true);
        });
    });

    it('should reject invalid command', () => {
        expect(isValidCommand('invalid')).toBe(false);
        expect(isValidCommand('unknown')).toBe(false);
    });

    it('should be case insensitive', () => {
        expect(isValidCommand('INFO')).toBe(true);
        expect(isValidCommand('Extract')).toBe(true);
    });
});

describe('File Path Validation - Unit Tests', () => {
    function validateFilePath(filePath: string): boolean {
        if (!filePath) return false;
        if (filePath.length === 0) return false;
        return true;
    }

    it('should accept valid paths', () => {
        expect(validateFilePath('test.pdf')).toBe(true);
        expect(validateFilePath('./test.pdf')).toBe(true);
        expect(validateFilePath('/absolute/path/test.pdf')).toBe(true);
        expect(validateFilePath('C:\\Windows\\test.pdf')).toBe(true);
    });

    it('should reject empty paths', () => {
        expect(validateFilePath('')).toBe(false);
    });

    it('should reject undefined/null', () => {
        expect(validateFilePath(undefined as any)).toBe(false);
        expect(validateFilePath(null as any)).toBe(false);
    });
});

describe('Chunk Size Validation - Unit Tests', () => {
    function validateChunkSize(size: number): boolean {
        if (isNaN(size)) return false;
        if (size <= 0) return false;
        if (size > 10000) return false; // reasonable upper limit
        return true;
    }

    it('should accept valid chunk sizes', () => {
        expect(validateChunkSize(500)).toBe(true);
        expect(validateChunkSize(1000)).toBe(true);
        expect(validateChunkSize(2000)).toBe(true);
    });

    it('should reject invalid sizes', () => {
        expect(validateChunkSize(0)).toBe(false);
        expect(validateChunkSize(-1)).toBe(false);
        expect(validateChunkSize(NaN)).toBe(false);
    });

    it('should reject excessively large sizes', () => {
        expect(validateChunkSize(20000)).toBe(false);
    });
});

describe('ANSI Color Codes - Unit Tests', () => {
    const colors = {
        reset: '\x1b[0m',
        red: '\x1b[31m',
        green: '\x1b[32m',
        yellow: '\x1b[33m',
        blue: '\x1b[34m',
        cyan: '\x1b[36m',
    };

    it('should have valid color codes', () => {
        expect(colors.reset).toMatch(/\x1b\[\d+m/);
        expect(colors.red).toMatch(/\x1b\[\d+m/);
        expect(colors.green).toMatch(/\x1b\[\d+m/);
        expect(colors.yellow).toMatch(/\x1b\[\d+m/);
        expect(colors.blue).toMatch(/\x1b\[\d+m/);
        expect(colors.cyan).toMatch(/\x1b\[\d+m/);
    });

    it('should format colored strings', () => {
        const coloredText = `${colors.green}Success${colors.reset}`;
        expect(coloredText).toContain(colors.green);
        expect(coloredText).toContain(colors.reset);
        expect(coloredText).toContain('Success');
    });
});

describe('Error Message Formatting - Unit Tests', () => {
    function formatErrorMessage(message: string): string {
        return `\x1b[31m✗\x1b[0m ${message}`;
    }

    function formatSuccessMessage(message: string): string {
        return `\x1b[32m✓\x1b[0m ${message}`;
    }

    function formatInfoMessage(message: string): string {
        return `\x1b[34mℹ\x1b[0m ${message}`;
    }

    it('should format error messages', () => {
        const msg = formatErrorMessage('File not found');
        expect(msg).toContain('✗');
        expect(msg).toContain('File not found');
    });

    it('should format success messages', () => {
        const msg = formatSuccessMessage('Operation completed');
        expect(msg).toContain('✓');
        expect(msg).toContain('Operation completed');
    });

    it('should format info messages', () => {
        const msg = formatInfoMessage('Processing...');
        expect(msg).toContain('ℹ');
        expect(msg).toContain('Processing...');
    });
});
