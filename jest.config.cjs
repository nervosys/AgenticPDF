/** @type {import('jest').Config} */
module.exports = {
    preset: 'ts-jest',
    testEnvironment: 'node',
    setupFilesAfterEnv: ['<rootDir>/tests/setup.ts'],
    roots: ['<rootDir>/tests'],
    testMatch: [
        '**/__tests__/**/*.ts',
        '**/?(*.)+(spec|test).ts'
    ],
    testPathIgnorePatterns: [
        '/node_modules/',
        '/tests/visual/'
    ],
    transform: {
        '^.+\\.ts$': ['ts-jest', {
            useESM: false
        }]
    },
    collectCoverageFrom: [
        'agenticpdf.ts',
        '!**/*.d.ts',
        '!**/node_modules/**'
    ],
    coverageDirectory: 'coverage',
    coverageReporters: [
        'text',
        'lcov',
        'html'
    ],
    coverageThreshold: {
        global: {
            branches: 25,
            functions: 45,
            lines: 37,
            statements: 35
        }
    },
    testTimeout: 30000,
    maxWorkers: 4,
    verbose: true,
    moduleNameMapper: {
        '^@/(.*)$': '<rootDir>/$1'
    }
};
