#!/usr/bin/env pwsh
<#
.SYNOPSIS
    ModernPDF CLI wrapper for PowerShell
    
.DESCRIPTION
    Convenient PowerShell wrapper for the ModernPDF CLI tool.
    Provides easy access to PDF processing features.
    
.PARAMETER Command
    The CLI command to execute (info, extract, convert, analyze, chunk, images, forms)
    
.PARAMETER Input
    Input PDF file path
    
.PARAMETER Output
    Output file or directory path
    
.PARAMETER Pages
    Page range (e.g., "1-5", "1,3,5")
    
.PARAMETER Format
    Output format (text, json, html, markdown)
    
.PARAMETER Verbose
    Enable verbose output
    
.PARAMETER Pretty
    Pretty-print JSON output
    
.PARAMETER Metadata
    Include metadata in output
    
.PARAMETER Tables
    Extract tables
    
.PARAMETER Images
    Extract images
    
.PARAMETER Forms
    Extract form fields
    
.PARAMETER Annotations
    Extract annotations
    
.PARAMETER AI
    Enable AI analysis features
    
.PARAMETER Chunk
    Generate semantic chunks
    
.PARAMETER ChunkSize
    Chunk size for semantic chunking
    
.PARAMETER Stream
    Use streaming mode
    
.EXAMPLE
    .\modernpdf-cli.ps1 -Command info -Input document.pdf
    
.EXAMPLE
    .\modernpdf-cli.ps1 -Command extract -Input document.pdf -Output output.txt -Verbose
    
.EXAMPLE
    .\modernpdf-cli.ps1 -Command convert -Input document.pdf -Format json -Pretty -Metadata
    
.EXAMPLE
    .\modernpdf-cli.ps1 -Command analyze -Input document.pdf -AI -Output analysis.json
    
.EXAMPLE
    .\modernpdf-cli.ps1 -Command chunk -Input document.pdf -ChunkSize 1000 -Output chunks.json
#>

param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet('info', 'extract', 'convert', 'analyze', 'chunk', 'images', 'forms', 'help', 'version')]
    [string]$Command,
    
    [Parameter(Mandatory = $false)]
    [Alias('i')]
    [string]$Input,
    
    [Parameter(Mandatory = $false)]
    [Alias('o')]
    [string]$Output,
    
    [Parameter(Mandatory = $false)]
    [Alias('p')]
    [string]$Pages,
    
    [Parameter(Mandatory = $false)]
    [Alias('f')]
    [string]$Format,
    
    [Parameter(Mandatory = $false)]
    [Alias('v')]
    [switch]$Verbose,
    
    [Parameter(Mandatory = $false)]
    [switch]$Pretty,
    
    [Parameter(Mandatory = $false)]
    [Alias('m')]
    [switch]$Metadata,
    
    [Parameter(Mandatory = $false)]
    [switch]$Tables,
    
    [Parameter(Mandatory = $false)]
    [switch]$Images,
    
    [Parameter(Mandatory = $false)]
    [switch]$Forms,
    
    [Parameter(Mandatory = $false)]
    [switch]$Annotations,
    
    [Parameter(Mandatory = $false)]
    [switch]$AI,
    
    [Parameter(Mandatory = $false)]
    [switch]$Chunk,
    
    [Parameter(Mandatory = $false)]
    [int]$ChunkSize,
    
    [Parameter(Mandatory = $false)]
    [switch]$Stream
)

# Build command arguments
$arguments = @($Command)

if ($Input) {
    $arguments += @('-i', $Input)
}

if ($Output) {
    $arguments += @('-o', $Output)
}

if ($Pages) {
    $arguments += @('-p', $Pages)
}

if ($Format) {
    $arguments += @('-f', $Format)
}

if ($Verbose) {
    $arguments += '-v'
}

if ($Pretty) {
    $arguments += '--pretty'
}

if ($Metadata) {
    $arguments += '--metadata'
}

if ($Tables) {
    $arguments += '--tables'
}

if ($Images) {
    $arguments += '--images'
}

if ($Forms) {
    $arguments += '--forms'
}

if ($Annotations) {
    $arguments += '--annotations'
}

if ($AI) {
    $arguments += '--ai'
}

if ($Chunk) {
    $arguments += '--chunk'
}

if ($ChunkSize -gt 0) {
    $arguments += @('--chunk-size', $ChunkSize)
}

if ($Stream) {
    $arguments += '--stream'
}

# Execute CLI
$scriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
$cliPath = Join-Path $scriptPath "cli.ts"

Write-Host "ModernPDF CLI" -ForegroundColor Cyan
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host ""

try {
    & npx tsx $cliPath @arguments
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "✓ Command completed successfully" -ForegroundColor Green
    }
    else {
        Write-Host ""
        Write-Host "✗ Command failed with exit code: $LASTEXITCODE" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}
catch {
    Write-Host ""
    Write-Host "✗ Error executing CLI: $_" -ForegroundColor Red
    exit 1
}
