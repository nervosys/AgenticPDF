/**
 * AgenticPDF Examples Index
 * 
 * This file provides easy access to all example demonstrations
 * of the AgenticPDF library's capabilities.
 */

// Import all examples
export { basicPDFProcessing, processPDFFile } from './01-basic-processing';
export { aiIntegrationExample } from './02-ai-integration';
export { streamToLLMExample } from './03-streaming-to-llm';
export { BatchProcessor, batchProcessingExample } from './04-batch-processing';
export { PDFWebSocketProcessor, MockWebSocketServer, realTimeProcessingExample } from './05-realtime-websocket';
export { apdfExamples } from './06-apdf-metadata';
export { apdfUseCases } from './07-apdf-use-cases';
export { typesettingWebDisplayExamples } from './08-typesetting-web-display';

// Example runner interface
export interface ExampleRunner {
    name: string;
    description: string;
    requiredInputs: string[];
    run: (inputs: { [key: string]: any }) => Promise<void>;
}

// Available examples
export const examples: ExampleRunner[] = [
    {
        name: 'Basic Processing',
        description: 'Fundamental PDF operations: loading, text extraction, metadata, and basic search',
        requiredInputs: ['file: File'],
        run: async (inputs) => {
            const { processPDFFile } = await import('./01-basic-processing');
            await processPDFFile(inputs.file);
        }
    },
    {
        name: 'AI Integration',
        description: 'AI-powered features: semantic chunking, embedding providers, document analysis',
        requiredInputs: ['file: File', 'apiKey?: string'],
        run: async (inputs) => {
            const { aiIntegrationExample } = await import('./02-ai-integration');
            await aiIntegrationExample(inputs.file, inputs.apiKey);
        }
    },
    {
        name: 'Streaming to LLM',
        description: 'Stream PDF content to Large Language Models with context management',
        requiredInputs: ['file: File', 'apiKey?: string'],
        run: async (inputs) => {
            const { streamToLLMExample } = await import('./03-streaming-to-llm');
            await streamToLLMExample(inputs.file, inputs.apiKey);
        }
    },
    {
        name: 'Batch Processing',
        description: 'Efficient processing of multiple PDFs with memory management and progress tracking',
        requiredInputs: ['files: File[]'],
        run: async (inputs) => {
            const { batchProcessingExample } = await import('./04-batch-processing');
            await batchProcessingExample(inputs.files);
        }
    },
    {
        name: 'Real-time WebSocket',
        description: 'Real-time PDF processing with live progress updates via WebSocket',
        requiredInputs: ['file: File', 'wsUrl?: string'],
        run: async (inputs) => {
            const { realTimeProcessingExample } = await import('./05-realtime-websocket');
            await realTimeProcessingExample(inputs.file, inputs.wsUrl);
        }
    },
    {
        name: 'aPDF Metadata',
        description: 'Generate aPDF metadata envelopes for AI workflows, research linking, and web display',
        requiredInputs: ['file: File'],
        run: async (inputs) => {
            const { apdfExamples } = await import('./06-apdf-metadata');
            await apdfExamples(inputs.file);
        }
    },
    {
        name: 'aPDF Use Cases',
        description: 'Real-world aPDF workflows: multi-agent research, compliance audit, knowledge graphs, document routing, revision diff, literature survey, Markdown publishing, and embedding cache',
        requiredInputs: ['file: File', 'additionalFiles?: File[]'],
        run: async (inputs) => {
            const { apdfUseCases } = await import('./07-apdf-use-cases');
            await apdfUseCases(inputs.file, inputs.additionalFiles);
        }
    },
    {
        name: 'Typesetting & Web Display',
        description: 'aPDF-driven typesetting: CSS generation, responsive HTML, font audit, accessible reading view, print stylesheet, social meta tags, canvas rendering, and multi-format export',
        requiredInputs: ['file: File', 'pageUrl?: string'],
        run: async (inputs) => {
            const { typesettingWebDisplayExamples } = await import('./08-typesetting-web-display');
            await typesettingWebDisplayExamples(inputs.file, inputs.pageUrl);
        }
    }
];

// Utility function to run all examples with a single file
export async function runAllExamples(file: File, apiKey?: string): Promise<void> {
    console.log('🚀 Running all AgenticPDF examples...\n');

    for (const example of examples) {
        console.log(`\n${'='.repeat(60)}`);
        console.log(`🧪 Running: ${example.name}`);
        console.log(`📝 Description: ${example.description}`);
        console.log(`${'='.repeat(60)}\n`);

        try {
            const inputs: { [key: string]: any } = { file };
            if (apiKey) inputs.apiKey = apiKey;
            if (example.name === 'Batch Processing') {
                inputs.files = [file]; // Convert single file to array for batch processing
            }

            await example.run(inputs);
            console.log(`\n✅ ${example.name} completed successfully`);

        } catch (error) {
            console.error(`\n❌ ${example.name} failed:`, (error as Error).message);
        }

        // Pause between examples
        await new Promise(resolve => setTimeout(resolve, 1000));
    }

    console.log('\n🎉 All examples completed!');
}

// Browser integration - create a comprehensive example runner
if (typeof window !== 'undefined') {
    document.addEventListener('DOMContentLoaded', () => {
        // Check if examples UI already exists
        if (document.getElementById('agenticpdf-examples')) return;

        const container = document.createElement('div');
        container.id = 'agenticpdf-examples';
        container.innerHTML = `
      <div style="max-width: 800px; margin: 20px auto; padding: 20px; font-family: Arial, sans-serif;">
        <h1>🚀 AgenticPDF Examples</h1>
        <p>Explore the capabilities of AgenticPDF with these interactive examples.</p>
        
        <div style="margin: 20px 0; padding: 15px; background: #f0f8ff; border-radius: 5px;">
          <h3>📁 File Selection</h3>
          <input type="file" id="exampleFile" accept=".pdf" style="margin-bottom: 10px;" />
          <input type="file" id="batchFiles" accept=".pdf" multiple style="margin-bottom: 10px;" />
          <br>
          <label>
            🔑 API Key (optional for AI features):
            <input type="password" id="apiKey" placeholder="Enter your API key..." style="width: 300px;" />
          </label>
        </div>
        
        <div id="exampleButtons" style="display: flex; flex-wrap: wrap; gap: 10px; margin: 20px 0;">
          <!-- Example buttons will be generated here -->
        </div>
        
        <div style="margin: 20px 0;">
          <button id="runAllBtn" disabled style="background: #4CAF50; color: white; padding: 10px 20px; border: none; border-radius: 5px; cursor: pointer; font-size: 16px;">
            🎯 Run All Examples
          </button>
        </div>
        
        <div id="exampleOutput" style="background: #f5f5f5; border: 1px solid #ddd; border-radius: 5px; padding: 15px; margin: 20px 0; max-height: 500px; overflow-y: auto; font-family: 'Courier New', monospace; white-space: pre-wrap;">
          Select a PDF file and choose an example to run...
        </div>
        
        <div id="exampleStatus" style="margin: 10px 0; font-weight: bold;"></div>
      </div>
    `;

        document.body.appendChild(container);

        // Get UI elements
        const fileInput = document.getElementById('exampleFile') as HTMLInputElement;
        const batchInput = document.getElementById('batchFiles') as HTMLInputElement;
        const apiKeyInput = document.getElementById('apiKey') as HTMLInputElement;
        const buttonsContainer = document.getElementById('exampleButtons') as HTMLDivElement;
        const runAllBtn = document.getElementById('runAllBtn') as HTMLButtonElement;
        const output = document.getElementById('exampleOutput') as HTMLDivElement;
        const status = document.getElementById('exampleStatus') as HTMLDivElement;

        // Generate example buttons
        examples.forEach((example, index) => {
            const button = document.createElement('button');
            button.textContent = `${index + 1}. ${example.name}`;
            button.title = example.description;
            button.style.cssText = 'padding: 8px 15px; margin: 5px; border: 1px solid #ddd; border-radius: 3px; background: #fff; cursor: pointer;';
            button.disabled = true;

            button.onclick = async () => {
                const file = fileInput.files?.[0];
                const files = Array.from(batchInput.files || []);
                const apiKey = apiKeyInput.value.trim() || undefined;

                if (example.name === 'Batch Processing' && files.length === 0) {
                    alert('Please select multiple files for batch processing');
                    return;
                } else if (example.name !== 'Batch Processing' && !file) {
                    alert('Please select a PDF file');
                    return;
                }

                await runExample(example, { file, files, apiKey });
            };

            buttonsContainer.appendChild(button);
        });

        // Update button states when files are selected
        const updateButtonStates = () => {
            const hasFile = !!fileInput.files?.[0];
            const hasBatchFiles = !!batchInput.files?.length;

            Array.from(buttonsContainer.children).forEach((button, index) => {
                const example = examples[index];
                (button as HTMLButtonElement).disabled = example.name === 'Batch Processing' ? !hasBatchFiles : !hasFile;
            });

            runAllBtn.disabled = !hasFile;
        };

        fileInput.onchange = updateButtonStates;
        batchInput.onchange = updateButtonStates;

        // Run all examples
        runAllBtn.onclick = async () => {
            const file = fileInput.files?.[0];
            if (!file) {
                alert('Please select a PDF file');
                return;
            }

            const apiKey = apiKeyInput.value.trim() || undefined;
            await runAllExamples(file, apiKey);
        };

        // Example runner function
        async function runExample(example: ExampleRunner, inputs: any) {
            status.textContent = `Running: ${example.name}...`;
            output.textContent = '';

            // Disable all buttons during execution
            Array.from(buttonsContainer.children).forEach(btn => {
                (btn as HTMLButtonElement).disabled = true;
            });
            runAllBtn.disabled = true;

            // Redirect console output
            const originalLog = console.log;
            const originalError = console.error;

            console.log = (...args) => {
                output.textContent += args.join(' ') + '\\n';
                output.scrollTop = output.scrollHeight;
                originalLog(...args);
            };

            console.error = (...args) => {
                output.textContent += '[ERROR] ' + args.join(' ') + '\\n';
                output.scrollTop = output.scrollHeight;
                originalError(...args);
            };

            try {
                await example.run(inputs);
                status.textContent = `✅ ${example.name} completed successfully`;
                status.style.color = 'green';
            } catch (error) {
                status.textContent = `❌ ${example.name} failed: ${(error as Error).message}`;
                status.style.color = 'red';
            } finally {
                // Restore console
                console.log = originalLog;
                console.error = originalError;

                // Re-enable buttons
                updateButtonStates();
            }
        }
    });
}

// Node.js usage example
export function showUsageExamples(): void {
    console.log(`
AgenticPDF Examples Usage:

1. Import specific examples:
   import { basicPDFProcessing } from './examples/01-basic-processing';
   import { aiIntegrationExample } from './examples/02-ai-integration';

2. Use the example runner:
   import { examples, runAllExamples } from './examples';
   
   // Run specific example
   await examples[0].run({ file: myPDFFile });
   
   // Run all examples
   await runAllExamples(myPDFFile, 'your-api-key');

3. Use batch processor:
   import { BatchProcessor } from './examples/04-batch-processing';
   
   const processor = new BatchProcessor({
     maxConcurrent: 3,
     enableAI: true
   });
   
   const results = await processor.processBatch(files);

4. Real-time processing:
   import { PDFWebSocketProcessor } from './examples/05-realtime-websocket';
   
   const processor = new PDFWebSocketProcessor('ws://localhost:8080');
   await processor.connect();
   await processor.processWithWebSocket(file);

For more details, see the individual example files.
  `);
}

export default examples;