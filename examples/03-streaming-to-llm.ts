/**
 * Streaming PDF to LLM Example
 * 
 * This example demonstrates how to stream PDF content to Large Language Models:
 * - Context-aware chunking for better LLM understanding
 * - Token management and context window optimization
 * - Progress tracking and error handling
 * - Multiple LLM provider integrations
 */

import AgenticPDF, { SemanticChunk } from '../agenticpdf';

interface LLMMessage {
    role: 'system' | 'user' | 'assistant';
    content: string;
}

interface LLMResponse {
    choices: Array<{
        message: {
            content: string;
        };
    }>;
    usage?: {
        prompt_tokens: number;
        completion_tokens: number;
        total_tokens: number;
    };
}

// Abstract LLM provider interface
abstract class LLMProvider {
    abstract name: string;
    abstract maxTokens: number;
    abstract contextWindow: number;

    abstract sendMessage(messages: LLMMessage[], options?: any): Promise<LLMResponse>;

    estimateTokens(text: string): number {
        // Rough estimation: ~4 characters per token
        return Math.ceil(text.length / 4);
    }
}

// OpenAI GPT provider
class OpenAIProvider extends LLMProvider {
    name = 'OpenAI GPT-4';
    maxTokens = 4000;
    contextWindow = 128000;

    constructor(private apiKey: string, private model = 'gpt-4o-mini') {
        super();
    }

    async sendMessage(messages: LLMMessage[], options: any = {}): Promise<LLMResponse> {
        const response = await fetch('https://api.openai.com/v1/chat/completions', {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${this.apiKey}`,
                'Content-Type': 'application/json'
            },
            body: JSON.stringify({
                model: this.model,
                messages,
                max_tokens: options.maxTokens || this.maxTokens,
                temperature: options.temperature || 0.1,
                stream: false
            })
        });

        if (!response.ok) {
            throw new Error(`OpenAI API error: ${response.statusText}`);
        }

        return response.json();
    }
}

// Mock LLM provider for demonstration
class MockLLMProvider extends LLMProvider {
    name = 'Mock LLM';
    maxTokens = 2000;
    contextWindow = 32000;

    async sendMessage(messages: LLMMessage[]): Promise<LLMResponse> {
        // Simulate API delay
        await new Promise(resolve => setTimeout(resolve, 500 + Math.random() * 1000));

        const lastMessage = messages[messages.length - 1];
        const content = lastMessage.content;

        // Generate mock response based on content
        let response = '';

        if (content.includes('summary') || content.includes('summarize')) {
            response = `This appears to be a document chunk containing information about ${this.extractTopics(content).join(', ')}. The content discusses key concepts and provides detailed information on the subject matter.`;
        } else if (content.includes('question') || content.includes('?')) {
            response = `Based on the provided document chunk, I can see references to ${this.extractTopics(content).slice(0, 2).join(' and ')}. The document provides relevant information to address the query.`;
        } else {
            response = `I've analyzed this document chunk (${this.estimateTokens(content)} tokens). Key topics include: ${this.extractTopics(content).slice(0, 3).join(', ')}. The content appears to be well-structured and informative.`;
        }

        return {
            choices: [{
                message: {
                    content: response
                }
            }],
            usage: {
                prompt_tokens: this.estimateTokens(messages.map(m => m.content).join('')),
                completion_tokens: this.estimateTokens(response),
                total_tokens: this.estimateTokens(messages.map(m => m.content).join('') + response)
            }
        };
    }

    private extractTopics(text: string): string[] {
        // Simple topic extraction for mock responses
        const words = text.toLowerCase().match(/\b[a-z]{4,}\b/g) || [];
        const topics = [...new Set(words)]
            .filter(word => !['this', 'that', 'with', 'from', 'they', 'have', 'been', 'will', 'such', 'more', 'some', 'time', 'very', 'when', 'much', 'work', 'also', 'than', 'only', 'even', 'well', 'back', 'good', 'each', 'come', 'call', 'make', 'take', 'find', 'give', 'know', 'part'].includes(word))
            .slice(0, 5);

        return topics.length > 0 ? topics : ['general content', 'document information'];
    }
}

export async function streamToLLMExample(file: File, apiKey?: string) {
    console.log('=== Streaming PDF to LLM Example ===\n');

    let pdf: AgenticPDF | null = null;

    try {
        // Load PDF with streaming optimizations
        pdf = await AgenticPDF.fromFile(file, {
            lazyLoad: true,
            maxMemoryUsage: 50 * 1024 * 1024, // 50MB limit
            streamOptions: {
                chunkSize: 1024 * 1024, // 1MB chunks
                backpressureThreshold: 5 * 1024 * 1024, // 5MB threshold
                progressCallback: (progress) => {
                    if (progress.bytesRead % (5 * 1024 * 1024) === 0) { // Every 5MB
                        console.log(`📥 Loading progress: ${Math.round(progress.bytesRead / progress.totalBytes * 100)}%`);
                    }
                }
            }
        });

        console.log(`📄 Loaded: ${file.name} (${pdf.getPageCount()} pages)`);

        // Initialize LLM provider
        const llmProvider = apiKey
            ? new OpenAIProvider(apiKey)
            : new MockLLMProvider();

        console.log(`🤖 Using LLM: ${llmProvider.name}`);
        console.log(`📏 Context window: ${llmProvider.contextWindow.toLocaleString()} tokens\n`);

        // Demonstrate different streaming strategies
        await demonstrateBasicStreaming(pdf, llmProvider);
        await demonstrateContextAwareStreaming(pdf, llmProvider);
        await demonstrateInteractiveAnalysis(pdf, llmProvider);

    } catch (error) {
        console.error('Streaming to LLM failed:', error);
    } finally {
        pdf?.close();
        console.log('\n✅ Streaming completed and resources cleaned up');
    }
}

async function demonstrateBasicStreaming(pdf: AgenticPDF, llmProvider: LLMProvider) {
    console.log('--- Basic Streaming to LLM ---');

    try {
        let chunkCount = 0;
        let totalTokens = 0;
        const responses: string[] = [];

        console.log('🌊 Starting basic streaming...');

        // Stream chunks and send to LLM
        for await (const chunk of pdf.streamSemanticChunks({
            strategy: 'semantic',
            maxChunkSize: 1500, // Optimized for LLM context
            preserveParagraphs: true
        })) {
            chunkCount++;

            // Prepare context for LLM
            const contextualContent = [
                `[Document Chunk ${chunkCount}]`,
                `[Pages: ${chunk.pageNumbers.join('-')}]`,
                `[Type: ${chunk.type}]`,
                `[Content Length: ${chunk.content.length} characters]`,
                '',
                chunk.content
            ].join('\n');

            const messages: LLMMessage[] = [
                {
                    role: 'system',
                    content: 'You are analyzing document chunks. Provide a brief summary and identify key topics.'
                },
                {
                    role: 'user',
                    content: `Please analyze this document chunk:\n\n${contextualContent}`
                }
            ];

            try {
                const response = await llmProvider.sendMessage(messages);
                const analysis = response.choices[0].message.content;

                console.log(`\n📊 Chunk ${chunkCount} Analysis (Pages ${chunk.pageNumbers.join('-')}):`);
                console.log(`   ${analysis}`);

                if (response.usage) {
                    totalTokens += response.usage.total_tokens;
                    console.log(`   Tokens used: ${response.usage.total_tokens}`);
                }

                responses.push(analysis);

                // Stop after 5 chunks for demo
                if (chunkCount >= 5) {
                    console.log('\n   (Stopping demo after 5 chunks)');
                    break;
                }

            } catch (error) {
                console.error(`   Error analyzing chunk ${chunkCount}:`, (error as Error).message);
            }

            // Small delay to avoid rate limits
            await new Promise(resolve => setTimeout(resolve, 100));
        }

        console.log(`\n📈 Basic streaming summary:`);
        console.log(`   Processed chunks: ${chunkCount}`);
        console.log(`   Total tokens used: ${totalTokens.toLocaleString()}`);
        console.log(`   Average tokens per chunk: ${Math.round(totalTokens / chunkCount)}`);

    } catch (error) {
        console.error('Basic streaming failed:', error);
    }
}

async function demonstrateContextAwareStreaming(pdf: AgenticPDF, llmProvider: LLMProvider) {
    console.log('\n--- Context-Aware Streaming ---');

    try {
        const conversationHistory: LLMMessage[] = [
            {
                role: 'system',
                content: 'You are analyzing a document progressively. Maintain context between chunks and build understanding as you read through the document. Identify patterns, themes, and connections between different sections.'
            }
        ];

        let chunkCount = 0;
        let totalTokensUsed = 0;
        const maxContextTokens = Math.floor(llmProvider.contextWindow * 0.7); // Use 70% of context window

        console.log('🧠 Starting context-aware streaming...');
        console.log(`📏 Max context tokens: ${maxContextTokens.toLocaleString()}`);

        for await (const chunk of pdf.streamSemanticChunks({
            strategy: 'semantic',
            maxChunkSize: 1200,
            preserveParagraphs: true
        })) {
            chunkCount++;

            // Create contextual prompt
            const chunkPrompt = [
                `Document chunk ${chunkCount} from pages ${chunk.pageNumbers.join('-')}:`,
                `Type: ${chunk.type}`,
                `Content: ${chunk.content}`,
                '',
                'Please analyze this chunk in the context of the previous chunks. What new information does it provide? How does it connect to what we\'ve seen before?'
            ].join('\n');

            conversationHistory.push({
                role: 'user',
                content: chunkPrompt
            });

            // Estimate tokens and manage context window
            const totalHistoryTokens = conversationHistory
                .map(msg => llmProvider.estimateTokens(msg.content))
                .reduce((a, b) => a + b, 0);

            // Trim history if approaching context limit
            while (totalHistoryTokens > maxContextTokens && conversationHistory.length > 2) {
                // Keep system message and remove oldest user/assistant pairs
                conversationHistory.splice(1, 2);
            }

            try {
                const response = await llmProvider.sendMessage(conversationHistory);
                const analysis = response.choices[0].message.content;

                // Add response to conversation history
                conversationHistory.push({
                    role: 'assistant',
                    content: analysis
                });

                console.log(`\n🔗 Contextual Analysis ${chunkCount}:`);
                console.log(`   ${analysis}`);

                if (response.usage) {
                    totalTokensUsed += response.usage.total_tokens;
                    console.log(`   Context tokens: ${response.usage.prompt_tokens}, Response: ${response.usage.completion_tokens}`);
                }

                // Stop after 4 chunks for demo
                if (chunkCount >= 4) {
                    console.log('\n   (Stopping demo after 4 chunks)');
                    break;
                }

            } catch (error) {
                console.error(`   Error in contextual analysis ${chunkCount}:`, (error as Error).message);
                // Remove the failed user message
                conversationHistory.pop();
            }

            await new Promise(resolve => setTimeout(resolve, 200));
        }

        console.log(`\n📈 Context-aware streaming summary:`);
        console.log(`   Processed chunks: ${chunkCount}`);
        console.log(`   Total tokens used: ${totalTokensUsed.toLocaleString()}`);
        console.log(`   Conversation history length: ${conversationHistory.length} messages`);

    } catch (error) {
        console.error('Context-aware streaming failed:', error);
    }
}

async function demonstrateInteractiveAnalysis(pdf: AgenticPDF, llmProvider: LLMProvider) {
    console.log('\n--- Interactive Document Analysis ---');

    try {
        // Collect chunks first for interactive querying
        const chunks: SemanticChunk[] = [];
        console.log('📚 Collecting document chunks...');

        let chunkCount = 0;
        for await (const chunk of pdf.streamSemanticChunks({
            strategy: 'semantic',
            maxChunkSize: 1000,
            includeMetadata: true
        })) {
            chunks.push(chunk);
            chunkCount++;

            if (chunkCount % 10 === 0) {
                console.log(`   Collected ${chunkCount} chunks...`);
            }

            // Limit for demo
            if (chunkCount >= 20) {
                console.log('   (Limited to 20 chunks for demo)');
                break;
            }
        }

        console.log(`✅ Collected ${chunks.length} chunks for analysis`);

        // Demonstrate different types of queries
        const queries = [
            'What are the main topics discussed in this document?',
            'Are there any statistics or numerical data mentioned?',
            'What conclusions or recommendations are made?'
        ];

        for (const query of queries) {
            console.log(`\n❓ Query: "${query}"`);
            await answerQueryFromChunks(llmProvider, chunks, query);
        }

    } catch (error) {
        console.error('Interactive analysis failed:', error);
    }
}

async function answerQueryFromChunks(
    llmProvider: LLMProvider,
    chunks: SemanticChunk[],
    query: string
) {
    try {
        // Find most relevant chunks (simplified relevance scoring)
        const scoredChunks = chunks.map(chunk => ({
            chunk,
            score: calculateRelevanceScore(chunk, query)
        })).sort((a, b) => b.score - a.score);

        // Select top chunks that fit in context
        const selectedChunks: SemanticChunk[] = [];
        let totalTokens = llmProvider.estimateTokens(query) + 500; // Buffer for system prompt and response

        for (const scored of scoredChunks) {
            const chunkTokens = llmProvider.estimateTokens(scored.chunk.content);
            if (totalTokens + chunkTokens < llmProvider.contextWindow * 0.8) {
                selectedChunks.push(scored.chunk);
                totalTokens += chunkTokens;
            }

            if (selectedChunks.length >= 5) break; // Limit number of chunks
        }

        console.log(`   Using ${selectedChunks.length} most relevant chunks (${totalTokens} estimated tokens)`);

        // Create context from selected chunks
        const context = selectedChunks.map((chunk, index) =>
            `[Chunk ${index + 1} - Pages ${chunk.pageNumbers.join('-')} - Type: ${chunk.type}]\n${chunk.content}`
        ).join('\n\n---\n\n');

        const messages: LLMMessage[] = [
            {
                role: 'system',
                content: 'You are an expert document analyst. Answer questions based on the provided document chunks. Be specific and cite which chunks contain relevant information.'
            },
            {
                role: 'user',
                content: `Based on the following document chunks, please answer this question: "${query}"\n\nDocument chunks:\n\n${context}`
            }
        ];

        const response = await llmProvider.sendMessage(messages);
        const answer = response.choices[0].message.content;

        console.log(`   💡 Answer: ${answer}`);

        if (response.usage) {
            console.log(`   📊 Tokens used: ${response.usage.total_tokens}`);
        }

    } catch (error) {
        console.error(`   Error answering query:`, (error as Error).message);
    }
}

function calculateRelevanceScore(chunk: SemanticChunk, query: string): number {
    const queryWords = query.toLowerCase().split(/\s+/);
    const chunkText = chunk.content.toLowerCase();

    // Simple relevance scoring
    let score = 0;

    // Exact word matches
    for (const word of queryWords) {
        const matches = (chunkText.match(new RegExp(word, 'g')) || []).length;
        score += matches * 2;
    }

    // Keyword matches
    if (chunk.metadata.keywords) {
        for (const keyword of chunk.metadata.keywords) {
            if (queryWords.some(word => keyword.toLowerCase().includes(word))) {
                score += 3;
            }
        }
    }

    // Boost important chunks
    score *= (chunk.metadata.importance || 1);

    // Prefer certain chunk types for different queries
    if (query.includes('conclusion') || query.includes('recommendation')) {
        if (chunk.type === 'Paragraph' && chunk.pageNumbers[0] > 1) {
            score *= 1.5; // Later paragraphs more likely to have conclusions
        }
    }

    return score;
}

// Browser integration
if (typeof window !== 'undefined') {
    document.addEventListener('DOMContentLoaded', () => {
        const container = document.createElement('div');
        container.innerHTML = `
      <h2>Stream PDF to LLM Example</h2>
      <div>
        <input type="file" id="pdfFile" accept=".pdf" />
        <br><br>
        <label>
          LLM API Key (optional - uses mock LLM if not provided):
          <input type="password" id="apiKey" placeholder="sk-..." />
        </label>
        <br><br>
        <button id="streamBtn" disabled>Stream to LLM</button>
      </div>
      <div id="output" style="white-space: pre-wrap; font-family: monospace; background: #f5f5f5; padding: 10px; margin-top: 10px; max-height: 400px; overflow-y: auto;"></div>
    `;

        document.body.appendChild(container);

        const fileInput = document.getElementById('pdfFile') as HTMLInputElement;
        const apiKeyInput = document.getElementById('apiKey') as HTMLInputElement;
        const streamBtn = document.getElementById('streamBtn') as HTMLButtonElement;
        const output = document.getElementById('output') as HTMLDivElement;

        fileInput.onchange = () => {
            streamBtn.disabled = !fileInput.files?.[0];
        };

        streamBtn.onclick = async () => {
            const file = fileInput.files?.[0];
            if (!file) return;

            const apiKey = apiKeyInput.value.trim() || undefined;

            // Redirect console.log to output
            const originalLog = console.log;
            console.log = (...args) => {
                output.textContent += args.join(' ') + '\n';
                output.scrollTop = output.scrollHeight;
                originalLog(...args);
            };

            try {
                streamBtn.disabled = true;
                output.textContent = '';
                await streamToLLMExample(file, apiKey);
            } finally {
                streamBtn.disabled = false;
                console.log = originalLog;
            }
        };
    });
}