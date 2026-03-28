/**
 * Validation script for AgenticPDF Agent Skills & Tools Runtime.
 * Tests: skill registration, tool dispatch, security policies, middleware, context lifecycle.
 *
 * Usage: npx tsx scripts/test-agent-skills.ts
 */

import AgenticPDF, {
  AgentContext,
  type AgentSkill,
  type AgentTool,
  type AgentToolCall,
  type AgentToolResult,
  type AgentMiddleware,
  type AgentSecurityPolicy,
  type AgentContextOptions,
} from '../agenticpdf.ts';

let passed = 0;
let failed = 0;

function assert(condition: boolean, message: string): void {
  if (condition) {
    passed++;
    console.log(`  ✅ ${message}`);
  } else {
    failed++;
    console.error(`  ❌ ${message}`);
  }
}

// ── 1. Built-in Skill Registration ─────────────────────────────────────────

console.log('\n🔹 1. Built-in Skill Registration');

const skills = AgenticPDF.listSkills();
assert(skills.length >= 6, `At least 6 built-in skills registered (got ${skills.length})`);

const skillIds = skills.map(s => s.id);
assert(skillIds.includes('pdf-extraction'), 'pdf-extraction skill exists');
assert(skillIds.includes('pdf-analysis'), 'pdf-analysis skill exists');
assert(skillIds.includes('pdf-forms'), 'pdf-forms skill exists');
assert(skillIds.includes('pdf-export'), 'pdf-export skill exists');
assert(skillIds.includes('apdf-format'), 'apdf-format skill exists');
assert(skillIds.includes('introspection'), 'introspection skill exists');

const extraction = AgenticPDF.getSkill('pdf-extraction');
assert(extraction !== undefined, 'getSkill() returns pdf-extraction');
assert(extraction!.tools.length === 6, `pdf-extraction has 6 tools (got ${extraction!.tools.length})`);

// ── 2. Tool Listing & Filtering ────────────────────────────────────────────

console.log('\n🔹 2. Tool Listing & Filtering');

const allTools = AgenticPDF.listTools();
assert(allTools.length >= 24, `At least 24 tools total (got ${allTools.length})`);

const extractionTools = AgenticPDF.listTools('extraction');
assert(extractionTools.length === 5, `5 extraction tools (got ${extractionTools.length})`);
assert(extractionTools.every(t => t.category === 'extraction'), 'All filtered tools have correct category');

const introspectionTools = AgenticPDF.listTools('introspection');
assert(introspectionTools.length >= 4, `At least 4 introspection tools (got ${introspectionTools.length})`);

// ── 3. Custom Skill Registration ───────────────────────────────────────────

console.log('\n🔹 3. Custom Skill Registration');

const customTool: AgentTool = {
  name: 'test_custom_tool',
  description: 'A test custom tool',
  parameters: [{ name: 'input', type: 'string', description: 'Input value', required: true }],
  category: 'custom',
  requiresDocument: false,
  handler: async (args) => ({ echo: args.input }),
};

const customSkill: AgentSkill = {
  id: 'test-custom-skill',
  name: 'Test Custom Skill',
  description: 'A test skill for validation',
  version: '1.0.0',
  tools: [customTool],
};

AgenticPDF.registerSkill(customSkill);
assert(AgenticPDF.getSkill('test-custom-skill') !== undefined, 'Custom skill registered successfully');

let duplicateError = false;
try {
  AgenticPDF.registerSkill(customSkill);
} catch {
  duplicateError = true;
}
assert(duplicateError, 'Duplicate skill registration throws error');

const removed = AgenticPDF.unregisterSkill('test-custom-skill');
assert(removed === true, 'unregisterSkill() returns true');
assert(AgenticPDF.getSkill('test-custom-skill') === undefined, 'Skill removed after unregister');

const removedAgain = AgenticPDF.unregisterSkill('test-custom-skill');
assert(removedAgain === false, 'unregisterSkill() returns false for missing skill');

// Re-register for later tests
AgenticPDF.registerSkill(customSkill);

// ── 4. AgentContext with Introspection Tools ───────────────────────────────

console.log('\n🔹 4. AgentContext with Introspection Tools');

// Use a minimal "document" by constructing directly — introspection tools don't need a real PDF
const dummyPdf = Object.create(AgenticPDF.prototype);
dummyPdf._pages = [];
dummyPdf._metadata = { pageCount: 0, pdfVersion: '1.7' };
dummyPdf._closed = false;
dummyPdf.createAgentSession = AgenticPDF.prototype.createAgentSession;

const ctx = new AgentContext(dummyPdf);
assert(ctx.session.sessionId.startsWith('session_'), 'Context created with valid session ID');

const available = ctx.getAvailableTools();
assert(available.length >= 24, `At least 24 tools available (got ${available.length})`);

// Test tool schemas generation
const openaiSchemas = ctx.getToolSchemas('openai');
assert(openaiSchemas.length > 0, `Tool schemas generated (got ${openaiSchemas.length})`);
assert(openaiSchemas[0].type === 'function', 'OpenAI schemas have type: function');

// ── 5. Tool Execution via Context ──────────────────────────────────────────

console.log('\n🔹 5. Tool Execution via Context');

const describeResult = await ctx.executeTool({ name: 'describe' });
assert(describeResult.success === true, 'describe tool executed successfully');
assert(describeResult.toolName === 'describe', 'Result has correct toolName');
assert(describeResult.durationMs >= 0, 'Result has durationMs');
assert(describeResult.result !== undefined, 'Result has return value');

const listSkillsResult = await ctx.executeTool({ name: 'listSkills' });
assert(listSkillsResult.success === true, 'listSkills tool executed successfully');
assert(Array.isArray(listSkillsResult.result), 'listSkills returns an array');

const listToolsResult = await ctx.executeTool({ name: 'listTools', arguments: { category: 'extraction' } });
assert(listToolsResult.success === true, 'listTools with category filter executed');
assert(Array.isArray(listToolsResult.result), 'listTools returns an array');

// Test custom tool execution
const customResult = await ctx.executeTool({ name: 'test_custom_tool', arguments: { input: 'hello' } });
assert(customResult.success === true, 'Custom tool executed successfully');
assert((customResult.result as any).echo === 'hello', 'Custom tool returns correct result');

// Unknown tool
const unknownResult = await ctx.executeTool({ name: 'nonexistent_tool' });
assert(unknownResult.success === false, 'Unknown tool returns failure');
assert(unknownResult.error !== undefined, 'Unknown tool has error message');

// ── 6. Batch Execution ─────────────────────────────────────────────────────

console.log('\n🔹 6. Batch Execution');

const batchResults = await ctx.executeToolBatch([
  { name: 'describe' },
  { name: 'listSkills' },
  { name: 'test_custom_tool', arguments: { input: 'batch' } },
]);
assert(batchResults.length === 3, `Batch returned 3 results (got ${batchResults.length})`);
assert(batchResults.every(r => r.success), 'All batch results succeeded');

// ── 7. Security Policy ─────────────────────────────────────────────────────

console.log('\n🔹 7. Security Policy');

const secureCtx = new AgentContext(dummyPdf, {
  securityPolicy: {
    allowedTools: ['describe', 'listSkills'],
    blockedTools: [],
    maxCallsPerSession: 2,
    maxExecutionTimeMs: 5000,
    allowMutations: false,
  },
});

const secureAvailable = secureCtx.getAvailableTools();
assert(secureAvailable.length === 2, `Only 2 allowed tools (got ${secureAvailable.length})`);

const allowedResult = await secureCtx.executeTool({ name: 'describe' });
assert(allowedResult.success === true, 'Allowed tool executes');

const blockedResult = await secureCtx.executeTool({ name: 'listTools' });
assert(blockedResult.success === false, 'Non-allowed tool is blocked');

// Second successful call fills the limit of 2
await secureCtx.executeTool({ name: 'listSkills' });
const overLimitResult = await secureCtx.executeTool({ name: 'describe' });
assert(overLimitResult.success === false, 'Call over maxCallsPerSession is rejected');
assert(overLimitResult.error?.includes('exceeded'), 'Over-limit error mentions exceeded');

// Blocked tools list
const blockedCtx = new AgentContext(dummyPdf, {
  securityPolicy: {
    blockedTools: ['describe'],
  },
});

const blockedDescribe = await blockedCtx.executeTool({ name: 'describe' });
assert(blockedDescribe.success === false, 'Blocked tool is rejected');
const allowedList = await blockedCtx.executeTool({ name: 'listSkills' });
assert(allowedList.success === true, 'Non-blocked tool is allowed');

// ── 8. Middleware ───────────────────────────────────────────────────────────

console.log('\n🔹 8. Middleware');

const middlewareLog: string[] = [];

const loggingMiddleware: AgentMiddleware = {
  name: 'test-logger',
  before: async (call) => { middlewareLog.push(`before:${call.name}`); return call; },
  after: async (call, result) => { middlewareLog.push(`after:${result.toolName}:${result.success}`); return result; },
  onError: async (call, error) => { middlewareLog.push(`error:${call.name}`); return null; },
};

const mwCtx = new AgentContext(dummyPdf, { middleware: [loggingMiddleware] });
await mwCtx.executeTool({ name: 'describe' });
assert(middlewareLog.includes('before:describe'), 'before middleware called');
assert(middlewareLog.some(l => l.startsWith('after:describe:true')), 'after middleware called');

// Error middleware — register a tool that throws to trigger onError
const throwingSkill: AgentSkill = {
  id: 'test-throwing-skill',
  name: 'Throwing Skill',
  description: 'A skill with a tool that throws',
  version: '1.0.0',
  tools: [{
    name: 'test_throw',
    description: 'A tool that always throws',
    parameters: [],
    category: 'test',
    requiresDocument: false,
    handler: async () => { throw new Error('intentional error'); },
  }],
};
AgenticPDF.registerSkill(throwingSkill);
middlewareLog.length = 0;
const mwCtx2 = new AgentContext(dummyPdf, { middleware: [loggingMiddleware] });
await mwCtx2.executeTool({ name: 'test_throw' });
assert(middlewareLog.includes('error:test_throw'), 'onError middleware called on handler exception');
AgenticPDF.unregisterSkill('test-throwing-skill');

// ── 9. Skill Activation ────────────────────────────────────────────────────

console.log('\n🔹 9. Skill Activation');

const activCtx = new AgentContext(dummyPdf);
activCtx.activateSkills(['introspection', 'test-custom-skill']);
const activeSkills = activCtx.getActiveSkills();
assert(activeSkills.length === 2, `2 active skills (got ${activeSkills.length})`);

const activTools = activCtx.getAvailableTools();
assert(activTools.length <= 5, `Limited tools with 2 skills (got ${activTools.length})`);

const inScopeResult = await activCtx.executeTool({ name: 'describe' });
assert(inScopeResult.success === true, 'In-scope tool executes');

// ── 10. History & Stats ────────────────────────────────────────────────────

console.log('\n🔹 10. History & Stats');

const statsCtx = new AgentContext(dummyPdf);
await statsCtx.executeTool({ name: 'describe' });
await statsCtx.executeTool({ name: 'listSkills' });
await statsCtx.executeTool({ name: 'nonexistent' });

const history = statsCtx.getHistory();
assert(history.length === 2, `2 history entries — unknown tools not recorded (got ${history.length})`);

const stats = statsCtx.getStats();
assert(stats.callCount === 2, `callCount = 2 (got ${stats.callCount})`);
assert(stats.successCount === 2, `successCount = 2 (got ${stats.successCount})`);
assert(stats.errorCount === 0, `errorCount = 0 — rejected calls not tracked (got ${stats.errorCount})`);
assert(stats.totalDurationMs >= 0, 'totalDurationMs >= 0');
assert(stats.toolsUsed.length >= 2, `toolsUsed has entries (got ${stats.toolsUsed.length})`);

// ── 11. Context Close ──────────────────────────────────────────────────────

console.log('\n🔹 11. Context Close');

const closeCtx = new AgentContext(dummyPdf);
closeCtx.close();
const closedResult = await closeCtx.executeTool({ name: 'describe' });
assert(closedResult.success === false, 'Closed context rejects tool calls');
assert(closedResult.error?.includes('closed'), 'Error mentions closed');

// ── 12. Ontology Integration ───────────────────────────────────────────────

console.log('\n🔹 12. Ontology Integration');

const ontology = AgenticPDF.describe();
const concepts = ontology.concepts.map(c => c.id);
assert(concepts.includes('AgentSkill'), 'AgentSkill concept in ontology');
assert(concepts.includes('AgentTool'), 'AgentTool concept in ontology');
assert(concepts.includes('AgentContext'), 'AgentContext concept in ontology');

const capabilities = AgenticPDF.getCapabilities();
const capIds = capabilities.map(c => c.id);
assert(capIds.includes('agent-skills'), 'agent-skills capability exists');

const workflows = AgenticPDF.getWorkflows();
const wfIds = workflows.map(w => w.id);
assert(wfIds.includes('agent-tool-execution'), 'agent-tool-execution workflow exists');

const schemas = AgenticPDF.getJSONSchemas();
assert(schemas.AgentTool !== undefined, 'AgentTool JSON schema exists');
assert(schemas.AgentSkill !== undefined, 'AgentSkill JSON schema exists');
assert(schemas.AgentToolCall !== undefined, 'AgentToolCall JSON schema exists');
assert(schemas.AgentToolResult !== undefined, 'AgentToolResult JSON schema exists');
assert(schemas.AgentSecurityPolicy !== undefined, 'AgentSecurityPolicy JSON schema exists');
assert(schemas.AgentMiddleware !== undefined, 'AgentMiddleware JSON schema exists');
assert(schemas.AgentContextOptions !== undefined, 'AgentContextOptions JSON schema exists');

const agentInfo = AgenticPDF.describeForAgent('openai');
assert(agentInfo.agentGuidance.agentSkillsGuidance !== undefined, 'Agent guidance includes agentSkillsGuidance');

// ── Cleanup ────────────────────────────────────────────────────────────────

AgenticPDF.unregisterSkill('test-custom-skill');

// ── Results ────────────────────────────────────────────────────────────────

console.log(`\n${'='.repeat(60)}`);
console.log(`Agent Skills & Tools Runtime — ${passed} passed, ${failed} failed`);
console.log('='.repeat(60));

if (failed > 0) {
  process.exit(1);
}
