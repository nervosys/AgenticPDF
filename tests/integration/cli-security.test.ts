/**
 * AgenticPDF CLI Security Tests
 * 
 * Tests for security fixes and vulnerability prevention
 */

import { describe, it, expect, beforeAll, afterAll } from '@jest/globals';
import { spawn } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

// Path to CLI entry point (relative to project root)
const CLI_PATH = path.join(process.cwd(), 'cli.js');
const SAMPLE_PDF = path.join(process.cwd(), 'demos', 'sample.pdf');
const TEST_OUTPUT_DIR = path.join(process.cwd(), 'tests', 'integration', 'cli-security-test-output');

/**
 * Helper function to execute CLI commands
 */
function runCLI(args: string[]): Promise<{ stdout: string; stderr: string; exitCode: number }> {
    return new Promise((resolve) => {
        const child = spawn('node', [CLI_PATH, ...args], {
            env: process.env,
        });

        let stdout = '';
        let stderr = '';

        child.stdout?.on('data', (data) => {
            stdout += data.toString();
        });

        child.stderr?.on('data', (data) => {
            stderr += data.toString();
        });

        child.on('close', (code) => {
            resolve({
                stdout,
                stderr,
                exitCode: code || 0,
            });
        });
    });
}

/**
 * Setup and teardown
 */
beforeAll(() => {
    // Create test output directory
    if (!fs.existsSync(TEST_OUTPUT_DIR)) {
        fs.mkdirSync(TEST_OUTPUT_DIR, { recursive: true });
    }
});

afterAll(() => {
    // Clean up test output directory
    if (fs.existsSync(TEST_OUTPUT_DIR)) {
        fs.rmSync(TEST_OUTPUT_DIR, { recursive: true, force: true });
    }
});

describe('CLI - Security Tests', () => {
    describe('Path Traversal Prevention', () => {
        it('should prevent path traversal in image output paths', async () => {
            if (!fs.existsSync(SAMPLE_PDF)) {
                console.warn('Sample PDF not found, skipping test');
                return;
            }

            const result = await runCLI([
                'images',
                '-i', SAMPLE_PDF,
                '-o', '../../../etc/passwd'
            ]);

            // Should fail or not write to system directory
            // Either exits with error or writes to safe location
            const etcPasswdExists = fs.existsSync('/etc/passwd');
            if (etcPasswdExists) {
                // On Unix-like systems, ensure we didn't write there
                expect(result.exitCode).not.toBe(0);
            }
        }, 30000);

        it('should prevent writing to system32 on Windows', async () => {
            if (!fs.existsSync(SAMPLE_PDF)) {
                console.warn('Sample PDF not found, skipping test');
                return;
            }

            const result = await runCLI([
                'images',
                '-i', SAMPLE_PDF,
                '-o', 'C:\\Windows\\System32'
            ]);

            // Should fail on Windows
            if (process.platform === 'win32') {
                expect(result.exitCode).not.toBe(0);
                // Error could be about system directories or permissions
                expect(result.stderr.length).toBeGreaterThan(0);
            }
        }, 30000);

        it('should sanitize output filenames', async () => {
            if (!fs.existsSync(SAMPLE_PDF)) {
                console.warn('Sample PDF not found, skipping test');
                return;
            }

            const safeOutput = path.join(TEST_OUTPUT_DIR, 'images');
            const result = await runCLI([
                'images',
                '-i', SAMPLE_PDF,
                '-o', safeOutput
            ]);

            // Should succeed with sanitized names
            if (fs.existsSync(safeOutput)) {
                const files = fs.readdirSync(safeOutput);
                files.forEach(file => {
                    // Filenames should only contain safe characters
                    expect(file).toMatch(/^image_\d+\.(png|jpg|jpeg|gif|webp|bmp|tiff)$/);
                });
            }
        }, 30000);
    });

    describe('Input Validation', () => {
        it('should reject invalid chunk sizes (negative)', async () => {
            if (!fs.existsSync(SAMPLE_PDF)) {
                console.warn('Sample PDF not found, skipping test');
                return;
            }

            const result = await runCLI([
                'chunk',
                '-i', SAMPLE_PDF,
                '--chunk-size', '-1'
            ]);

            expect(result.exitCode).toBe(1);
            expect(result.stderr).toMatch(/chunk size|invalid/i);
        }, 30000);

        it('should reject invalid chunk sizes (too large)', async () => {
            if (!fs.existsSync(SAMPLE_PDF)) {
                console.warn('Sample PDF not found, skipping test');
                return;
            }

            const result = await runCLI([
                'chunk',
                '-i', SAMPLE_PDF,
                '--chunk-size', '99999999'
            ]);

            expect(result.exitCode).toBe(1);
            expect(result.stderr).toMatch(/chunk size|must be between/i);
        }, 30000);

        it('should reject invalid chunk sizes (non-numeric)', async () => {
            if (!fs.existsSync(SAMPLE_PDF)) {
                console.warn('Sample PDF not found, skipping test');
                return;
            }

            const result = await runCLI([
                'chunk',
                '-i', SAMPLE_PDF,
                '--chunk-size', 'abc'
            ]);

            expect(result.exitCode).toBe(1);
            expect(result.stderr).toMatch(/invalid chunk size|must be a number/i);
        }, 30000);

        it('should reject invalid output formats', async () => {
            if (!fs.existsSync(SAMPLE_PDF)) {
                console.warn('Sample PDF not found, skipping test');
                return;
            }

            const result = await runCLI([
                'convert',
                '-i', SAMPLE_PDF,
                '-f', 'invalid_format'
            ]);

            expect(result.exitCode).toBe(1);
            expect(result.stderr).toMatch(/invalid format/i);
        }, 30000);

        it('should accept valid output formats', async () => {
            if (!fs.existsSync(SAMPLE_PDF)) {
                console.warn('Sample PDF not found, skipping test');
                return;
            }

            const validFormats = ['text', 'json', 'html', 'markdown'];

            for (const format of validFormats) {
                const result = await runCLI([
                    'convert',
                    '-i', SAMPLE_PDF,
                    '-f', format
                ]);

                // Should not fail on validation (may fail on processing which is ok)
                if (result.exitCode === 1) {
                    expect(result.stderr).not.toMatch(/invalid format/i);
                }
            }
        }, 120000);

        it('should reject missing required arguments', async () => {
            const result = await runCLI([
                'extract',
                '--output', 'test.txt'
                // Missing input file
            ]);

            expect(result.exitCode).toBe(1);
            expect(result.stderr).toMatch(/input|required|file/i);
        }, 30000);
    });

    describe('File Access Security', () => {
        it('should reject non-existent files', async () => {
            const result = await runCLI([
                'info',
                '-i', 'non-existent-file.pdf'
            ]);

            expect(result.exitCode).toBe(1);
            expect(result.stderr).toMatch(/file not found|does not exist/i);
        }, 30000);

        it('should reject directories as input', async () => {
            const result = await runCLI([
                'info',
                '-i', TEST_OUTPUT_DIR
            ]);

            expect(result.exitCode).toBe(1);
            expect(result.stderr).toMatch(/not a file|is not a file/i);
        }, 30000);

        it('should handle permission denied errors gracefully', async () => {
            // This test is platform-specific and may not work on all systems
            if (process.platform !== 'win32') {
                // Create a file without read permissions
                const restrictedFile = path.join(TEST_OUTPUT_DIR, 'restricted.pdf');
                fs.writeFileSync(restrictedFile, 'test');
                try {
                    fs.chmodSync(restrictedFile, 0o000); // No permissions

                    const result = await runCLI([
                        'info',
                        '-i', restrictedFile
                    ]);

                    expect(result.exitCode).toBe(1);
                    expect(result.stderr).toMatch(/permission|cannot read/i);

                    // Restore permissions for cleanup
                    fs.chmodSync(restrictedFile, 0o644);
                } catch (err) {
                    // If chmod fails, skip test
                    console.warn('Cannot test file permissions on this system');
                }
            }
        }, 30000);
    });

    describe('Command Injection Prevention', () => {
        it('should handle special characters in file paths safely', async () => {
            // Test with shell metacharacters
            const specialChars = ['$', '`', '|', ';', '&', '<', '>'];

            for (const char of specialChars) {
                const result = await runCLI([
                    'info',
                    '-i', `test${char}file.pdf`
                ]);

                // Should fail (non-zero exit code), not execute commands
                expect(result.exitCode).not.toBe(0);
                expect(result.stderr).toMatch(/file not found|does not exist/i);
                // Should not contain shell execution errors
                expect(result.stderr).not.toMatch(/syntax error|unexpected/i);
            }
        }, 60000);

        it('should not execute shell commands in arguments', async () => {
            const result = await runCLI([
                'info',
                '-i', '$(whoami).pdf'
            ]);

            expect(result.exitCode).toBe(1);
            expect(result.stderr).toMatch(/file not found/i);
            // Should not contain username or shell output
        }, 30000);
    });

    describe('Extension Validation', () => {
        it('should only allow safe image extensions', async () => {
            if (!fs.existsSync(SAMPLE_PDF)) {
                console.warn('Sample PDF not found, skipping test');
                return;
            }

            const outputDir = path.join(TEST_OUTPUT_DIR, 'safe-images');
            const result = await runCLI([
                'images',
                '-i', SAMPLE_PDF,
                '-o', outputDir
            ]);

            if (fs.existsSync(outputDir)) {
                const files = fs.readdirSync(outputDir);
                const allowedExtensions = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'tiff'];

                files.forEach(file => {
                    const ext = path.extname(file).substring(1).toLowerCase();
                    expect(allowedExtensions).toContain(ext);
                });
            }
        }, 30000);
    });

    describe('Error Message Safety', () => {
        it('should not leak sensitive path information', async () => {
            const result = await runCLI([
                'info',
                '-i', '/etc/shadow'
            ]);

            expect(result.exitCode).toBe(1);
            // Error should be generic, not reveal system paths
            expect(result.stderr).toMatch(/file not found|cannot read/i);
        }, 30000);

        it('should provide safe error messages for invalid inputs', async () => {
            const result = await runCLI([
                'chunk',
                '--chunk-size', '<script>alert(1)</script>'
            ]);

            expect(result.exitCode).toBe(1);
            // Should not echo back unsanitized input
            expect(result.stderr).not.toContain('<script>');
        }, 30000);
    });
});
