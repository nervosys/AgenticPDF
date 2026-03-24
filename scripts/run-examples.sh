#!/bin/bash
# AgenticPDF Examples Runner for Unix/Linux/macOS
# This shell script provides easy access to run AgenticPDF examples

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Function to print colored output
print_colored() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Function to show header
show_header() {
    echo
    print_colored $CYAN "===================================="
    print_colored $CYAN "    AgenticPDF Examples Runner"
    print_colored $CYAN "===================================="
    echo
}

# Function to show help
show_help() {
    print_colored $BLUE "Usage: ./run-examples.sh [options]"
    echo
    print_colored $YELLOW "Options:"
    echo "  --file <path>    Specify PDF file to process"
    echo "  --all            Run all examples with the specified file"
    echo "  --help, -h       Show this help message"
    echo
    print_colored $YELLOW "Examples:"
    echo "  ./run-examples.sh                    (Interactive mode)"
    echo "  ./run-examples.sh --file sample.pdf"
    echo "  ./run-examples.sh --file test.pdf --all"
    echo
    print_colored $YELLOW "Environment Variables:"
    echo "  API_KEY          Set API key for AI features"
    echo "  DEBUG            Set to 1 for detailed error info"
    echo
}

# Function to check dependencies
check_dependencies() {
    # Check if Node.js is installed
    if ! command -v node &> /dev/null; then
        print_colored $RED "ERROR: Node.js is not installed or not in PATH"
        print_colored $YELLOW "Please install Node.js from https://nodejs.org/"
        exit 1
    fi

    # Check if npm packages are installed
    if [ ! -d "node_modules" ]; then
        print_colored $YELLOW "Installing dependencies..."
        npm install
        if [ $? -ne 0 ]; then
            print_colored $RED "ERROR: Failed to install dependencies"
            exit 1
        fi
        echo
    fi
}

# Function to run interactive mode
run_interactive() {
    print_colored $CYAN "Starting interactive example runner..."
    echo
    print_colored $YELLOW "Available options:"
    echo "1. Interactive CLI mode (default)"
    echo "2. Open browser demo"
    echo "3. Exit"
    echo
    read -p "Choose an option (1-3): " choice
    
    case $choice in
        1|"")
            npx tsx run-examples-simple.ts
            ;;
        2)
            echo
            print_colored $CYAN "Opening browser demo..."
            print_colored $YELLOW "Please open examples-demo.html in your web browser"
            
            # Try to open browser on different systems
            if command -v xdg-open &> /dev/null; then
                xdg-open examples-demo.html
            elif command -v open &> /dev/null; then
                open examples-demo.html
            else
                print_colored $YELLOW "Please manually open examples-demo.html in your browser"
            fi
            ;;
        3)
            print_colored $GREEN "Goodbye!"
            exit 0
            ;;
        *)
            print_colored $YELLOW "Invalid choice. Starting interactive mode..."
            npx tsx run-examples-simple.ts
            ;;
    esac
}

# Parse command line arguments
PDF_FILE=""
RUN_ALL=""
SHOW_HELP=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --file)
            PDF_FILE="$2"
            shift 2
            ;;
        --all)
            RUN_ALL="1"
            shift
            ;;
        --help|-h)
            SHOW_HELP="1"
            shift
            ;;
        *)
            print_colored $RED "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Main execution
show_header

# Show help if requested
if [ "$SHOW_HELP" = "1" ]; then
    show_help
    exit 0
fi

# Check dependencies
check_dependencies

# Run examples based on arguments
if [ -n "$PDF_FILE" ]; then
    if [ "$RUN_ALL" = "1" ]; then
        print_colored $GREEN "Running all examples with file: $PDF_FILE"
        echo
        npx tsx run-examples-simple.ts --file="$PDF_FILE" --all
    else
        print_colored $GREEN "Running examples with file: $PDF_FILE"
        echo
        npx tsx run-examples-simple.ts --file="$PDF_FILE"
    fi
else
    run_interactive
fi

# Check exit code and provide feedback
exit_code=$?
echo

if [ $exit_code -ne 0 ]; then
    print_colored $RED "================================"
    print_colored $RED "Example execution completed with errors"
    print_colored $RED "================================"
    echo
    print_colored $YELLOW "Troubleshooting tips:"
    echo "- Ensure your PDF file exists and is readable"
    echo "- Check that you have a valid API key for AI features"
    echo "- Try running with DEBUG=1 for more detailed error information"
    echo "- Verify that all dependencies are properly installed"
    echo
else
    print_colored $GREEN "================================"
    print_colored $GREEN "Examples completed successfully!"
    print_colored $GREEN "================================"
    echo
    print_colored $CYAN "🎉 Great job! You've successfully run the AgenticPDF examples."
    print_colored $YELLOW "💡 Try different examples with various PDF files to explore all features."
    echo
fi

exit $exit_code