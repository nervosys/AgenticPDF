/**
 * Jest test setup file
 * Configures global test environment and mocks
 */

import * as Mocks from './mocks';

// Create global MockFetch instance that tests can configure
export const globalMockFetch = new Mocks.MockFetch();

// Mock global APIs that may not be available in Node.js
const mockFn = () => () => { };

Object.assign(globalThis, {
    TextEncoder,
    TextDecoder,
    fetch: globalMockFetch.fetch,
});

// Mock DOM APIs for browser-specific code
Object.defineProperty(globalThis, 'document', {
    value: {
        createElement: mockFn(),
    },
});

// Mock Web Workers
Object.defineProperty(globalThis, 'Worker', {
    value: function Worker() {
        return {
            postMessage: mockFn(),
            terminate: mockFn(),
            addEventListener: mockFn(),
            removeEventListener: mockFn(),
        };
    },
});

export { }; // Make this a module
