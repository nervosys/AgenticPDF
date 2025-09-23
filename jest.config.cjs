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
    transform: {
        '^.+\\.ts$': ['ts-jest', {
            useESM: false
        }]
    },
    collectCoverageFrom: [
        'modernpdf.ts',
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
            branches: 11,  // Encourage slightly better branch coverage
            functions: 20, // Match current function coverage  
            lines: 17,     // Match current line coverage
            statements: 17 // Match current statement coverage
        }
    },
    testTimeout: 30000,
    maxWorkers: 4,
    verbose: true,
    moduleNameMapper: {
        '^@/(.*)$': '<rootDir>/$1'
    }
};