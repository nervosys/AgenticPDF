/**
 * Validate the complete AgenticPDF ontology.
 */
import { AgenticPDF } from '../agenticpdf.js';

console.log('=== AgenticPDF Ontology Validation ===\n');

// 1. Full ontology
const ontology = AgenticPDF.describe();
console.log(`Ontology: ${ontology.name} v${ontology.version}`);
console.log(`  @context: ${ontology['@context']}`);
console.log(`  @type: ${ontology['@type']}`);
console.log(`  Concepts: ${ontology.concepts.length}`);
console.log(`  Capabilities: ${ontology.capabilities.length}`);
console.log(`  Workflows: ${ontology.workflows.length}`);
console.log(`  Enums: ${Object.keys(ontology.enums).length}`);

// 2. List concepts
console.log('\n--- Concepts ---');
for (const c of ontology.concepts) {
  console.log(`  ${c.id}: ${c.properties.length} props, ${c.relationships.length} rels`);
}

// 3. List capabilities
console.log('\n--- Capabilities ---');
for (const cap of ontology.capabilities) {
  console.log(`  ${cap.id} [${cap.category}]: ${cap.methods.length} methods (streaming: ${cap.streaming})`);
}

// 4. List workflows
console.log('\n--- Workflows ---');
for (const w of ontology.workflows) {
  console.log(`  ${w.id}: ${w.steps.length} steps`);
}

// 5. Enums
console.log('\n--- Enums ---');
for (const [name, values] of Object.entries(ontology.enums)) {
  console.log(`  ${name}: ${values.length} values`);
}

// 6. Method signatures
const methods = AgenticPDF.getMethodSignatures();
console.log(`\nTotal method signatures: ${methods.length}`);

// 7. Tool schemas
const openaiTools = AgenticPDF.getToolSchemas('openai');
const anthropicTools = AgenticPDF.getToolSchemas('anthropic');
const genericTools = AgenticPDF.getToolSchemas('generic');
console.log(`Tool schemas: OpenAI=${openaiTools.length}, Anthropic=${anthropicTools.length}, Generic=${genericTools.length}`);

// 8. MCP manifest
const mcp = AgenticPDF.getMCPManifest();
console.log(`MCP manifest: ${mcp.tools.length} tools, ${mcp.resources.length} resources`);

// 9. JSON schemas
const schemas = AgenticPDF.getJSONSchemas();
console.log(`JSON schemas: ${Object.keys(schemas).length}`);
console.log(`  Types: ${Object.keys(schemas).join(', ')}`);

// 10. describeForAgent
const agentInfo = AgenticPDF.describeForAgent('openai');
console.log(`\ndescribeForAgent:`);
console.log(`  tools: ${agentInfo.tools.length}`);
console.log(`  schemas: ${Object.keys(agentInfo.schemas).length}`);
console.log(`  workflows: ${agentInfo.workflows.length}`);
console.log(`  bestPractices: ${agentInfo.agentGuidance.bestPractices.length}`);
console.log(`  apdfGuidance keys: ${Object.keys(agentInfo.agentGuidance.apdfGuidance).length}`);

// Assertions
const assert = (cond: boolean, msg: string) => {
  if (!cond) { console.error(`FAIL: ${msg}`); process.exit(1); }
  console.log(`PASS: ${msg}`);
};

console.log('\n=== Assertions ===');
assert(ontology.concepts.length >= 18, `concepts >= 18 (got ${ontology.concepts.length})`);
assert(ontology.capabilities.length >= 13, `capabilities >= 13 (got ${ontology.capabilities.length})`);
assert(ontology.workflows.length >= 13, `workflows >= 13 (got ${ontology.workflows.length})`);
assert(Object.keys(ontology.enums).length >= 14, `enums >= 14 (got ${Object.keys(ontology.enums).length})`);
assert(methods.length >= 30, `methods >= 30 (got ${methods.length})`);
assert(openaiTools.length >= 25, `openai tools >= 25 (got ${openaiTools.length})`);
assert(mcp.resources.length >= 6, `mcp resources >= 6 (got ${mcp.resources.length})`);
assert(Object.keys(schemas).length >= 30, `json schemas >= 30 (got ${Object.keys(schemas).length})`);
assert(agentInfo.agentGuidance.bestPractices.length >= 12, `bestPractices >= 12 (got ${agentInfo.agentGuidance.bestPractices.length})`);
assert(!!agentInfo.agentGuidance.apdfGuidance.encryption, 'apdfGuidance.encryption present');
assert(ontology.enums.ExportFormat.includes('apdf'), "ExportFormat includes 'apdf'");

// Check aPDF concepts exist
const conceptIds = new Set(ontology.concepts.map(c => c.id));
assert(conceptIds.has('APDFDocument'), 'APDFDocument concept exists');
assert(conceptIds.has('APDFBinaryContainer'), 'APDFBinaryContainer concept exists');
assert(conceptIds.has('APDFEncryption'), 'APDFEncryption concept exists');
assert(conceptIds.has('APDFArtifact'), 'APDFArtifact concept exists');
assert(conceptIds.has('APDFStructure'), 'APDFStructure concept exists');
assert(conceptIds.has('APDFAIContent'), 'APDFAIContent concept exists');
assert(conceptIds.has('APDFChunk'), 'APDFChunk concept exists');
assert(conceptIds.has('APDFDisplay'), 'APDFDisplay concept exists');
assert(conceptIds.has('APDFProvenance'), 'APDFProvenance concept exists');

// Check aPDF capabilities
const capIds = new Set(ontology.capabilities.map(c => c.id));
assert(capIds.has('apdf-format'), 'apdf-format capability exists');
assert(capIds.has('introspection'), 'introspection capability exists');

// Check aPDF workflows
const wfIds = new Set(ontology.workflows.map(w => w.id));
assert(wfIds.has('apdf-binary-generation'), 'apdf-binary-generation workflow exists');
assert(wfIds.has('apdf-encrypted-generation'), 'apdf-encrypted-generation workflow exists');
assert(wfIds.has('apdf-round-trip'), 'apdf-round-trip workflow exists');
assert(wfIds.has('apdf-streaming-index'), 'apdf-streaming-index workflow exists');
assert(wfIds.has('agent-discovery'), 'agent-discovery workflow exists');

// Check aPDF tool names
const toolNames = new Set(genericTools.map((t: any) => t.name));
assert(toolNames.has('generateAPDFMetadata'), 'generateAPDFMetadata tool exists');
assert(toolNames.has('generateAPDFBinary'), 'generateAPDFBinary tool exists');
assert(toolNames.has('readAPDF'), 'readAPDF tool exists');
assert(toolNames.has('readAPDFHeader'), 'readAPDFHeader tool exists');
assert(toolNames.has('readAPDFMetadata'), 'readAPDFMetadata tool exists');

// Check aPDF JSON schemas
assert(!!schemas.APDFDocument, 'APDFDocument schema exists');
assert(!!schemas.APDFHeader, 'APDFHeader schema exists');
assert(!!schemas.APDFBinaryOptions, 'APDFBinaryOptions schema exists');
assert(!!schemas.APDFChunk, 'APDFChunk schema exists');
assert(!!schemas.APDFArtifact, 'APDFArtifact schema exists');

console.log('\n=== All assertions passed! ===');
