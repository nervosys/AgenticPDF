#!/usr/bin/env node

/**
 * Simple YAML validation script for GitHub workflows
 * Checks basic syntax and structure of workflow files
 */

const fs = require('fs');
const path = require('path');

function validateWorkflow(filePath) {
    try {
        const content = fs.readFileSync(filePath, 'utf8');
        
        // Basic YAML structure checks
        const lines = content.split('\n');
        let indentationValid = true;
        let hasName = false;
        let hasOn = false;
        let hasJobs = false;
        
        for (let i = 0; i < lines.length; i++) {
            const line = lines[i];
            const lineNum = i + 1;
            
            // Check for required top-level keys
            if (line.startsWith('name:')) hasName = true;
            if (line.startsWith('on:')) hasOn = true;
            if (line.startsWith('jobs:')) hasJobs = true;
            
            // Check for consistent indentation (spaces, not tabs)
            if (line.includes('\t')) {
                console.warn(`⚠️  Line ${lineNum}: Contains tabs, should use spaces`);
                indentationValid = false;
            }
        }
        
        // Validate required sections
        const issues = [];
        if (!hasName) issues.push('Missing "name" field');
        if (!hasOn) issues.push('Missing "on" trigger field');
        if (!hasJobs) issues.push('Missing "jobs" section');
        
        if (issues.length === 0 && indentationValid) {
            console.log(`✅ ${path.basename(filePath)}: Valid`);
            return true;
        } else {
            console.log(`❌ ${path.basename(filePath)}: Issues found`);
            issues.forEach(issue => console.log(`   - ${issue}`));
            return false;
        }
        
    } catch (error) {
        console.log(`❌ ${path.basename(filePath)}: Error reading file - ${error.message}`);
        return false;
    }
}

// Validate all workflow files
const workflowDir = path.join(__dirname, '.github', 'workflows');
const workflowFiles = fs.readdirSync(workflowDir)
    .filter(file => file.endsWith('.yml') || file.endsWith('.yaml'))
    .map(file => path.join(workflowDir, file));

console.log('🔍 Validating GitHub workflow files...\n');

let allValid = true;
workflowFiles.forEach(file => {
    const isValid = validateWorkflow(file);
    if (!isValid) allValid = false;
});

console.log('\n' + (allValid ? '✅ All workflows are valid!' : '❌ Some workflows have issues'));
process.exit(allValid ? 0 : 1);