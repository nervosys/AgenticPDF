#!/usr/bin/env bash

# ModernPDF CLI wrapper for Bash/Unix
# Convenient shell wrapper for the ModernPDF CLI tool

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLI_PATH="${SCRIPT_DIR}/cli.ts"

# Colors
CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
GRAY='\033[0;90m'
RESET='\033[0m'

# Display usage
usage() {
    echo -e "${CYAN}ModernPDF CLI${RESET}"
    echo -e "${GRAY}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    echo ""
    echo "Usage: $0 <command> [options]"
    echo ""
    echo "Commands:"
    echo "  info       Display PDF information"
    echo "  extract    Extract text content"
    echo "  convert    Convert to different formats"
    echo "  analyze    AI-powered analysis"
    echo "  chunk      Generate semantic chunks"
    echo "  images     Extract images"
    echo "  forms      Extract form fields"
    echo "  help       Display help"
    echo "  version    Show version"
    echo ""
    echo "Options:"
    echo "  -i, --input <file>       Input PDF file"
    echo "  -o, --output <file>      Output file"
    echo "  -p, --pages <range>      Page range"
    echo "  -f, --format <format>    Output format"
    echo "  -v, --verbose            Verbose output"
    echo "  --pretty                 Pretty-print JSON"
    echo "  -m, --metadata           Include metadata"
    echo "  --tables                 Extract tables"
    echo "  --images                 Extract images"
    echo "  --forms                  Extract forms"
    echo "  --annotations            Extract annotations"
    echo "  --ai                     Enable AI features"
    echo "  --chunk                  Generate chunks"
    echo "  --chunk-size <size>      Chunk size"
    echo "  --stream                 Use streaming"
    echo ""
    echo "Examples:"
    echo "  $0 info document.pdf"
    echo "  $0 extract -i document.pdf -o output.txt"
    echo "  $0 convert -i document.pdf -f json --pretty"
    echo "  $0 analyze -i document.pdf --ai"
    echo "  $0 chunk -i document.pdf --chunk-size 1000"
    echo ""
    echo "For detailed documentation, see CLI.md"
}

# Check if command provided
if [ $# -eq 0 ]; then
    usage
    exit 1
fi

# Show header
echo -e "${CYAN}ModernPDF CLI${RESET}"
echo -e "${GRAY}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
echo ""

# Execute CLI
if npx tsx "$CLI_PATH" "$@"; then
    echo ""
    echo -e "${GREEN}✓ Command completed successfully${RESET}"
    exit 0
else
    exit_code=$?
    echo ""
    echo -e "${RED}✗ Command failed with exit code: ${exit_code}${RESET}"
    exit $exit_code
fi
