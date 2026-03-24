@echo off
REM AgenticPDF Examples Runner for Windows
REM This batch file provides easy access to run AgenticPDF examples

echo.
echo ====================================
echo    AgenticPDF Examples Runner
echo ====================================
echo.

REM Check if Node.js is installed
node --version >nul 2>&1
if errorlevel 1 (
    echo ERROR: Node.js is not installed or not in PATH
    echo Please install Node.js from https://nodejs.org/
    pause
    exit /b 1
)

REM Check if npm packages are installed
if not exist "node_modules" (
    echo Installing dependencies...
    npm install
    if errorlevel 1 (
        echo ERROR: Failed to install dependencies
        pause
        exit /b 1
    )
    echo.
)

REM Parse command line arguments
set "PDF_FILE="
set "RUN_ALL="
set "SHOW_HELP="

:parse_args
if "%~1"=="" goto :done_parsing
if "%~1"=="--file" (
    set "PDF_FILE=%~2"
    shift
    shift
    goto :parse_args
)
if "%~1"=="--all" (
    set "RUN_ALL=1"
    shift
    goto :parse_args
)
if "%~1"=="--help" (
    set "SHOW_HELP=1"
    shift
    goto :parse_args
)
if "%~1"=="-h" (
    set "SHOW_HELP=1"
    shift
    goto :parse_args
)
shift
goto :parse_args

:done_parsing

REM Show help if requested
if defined SHOW_HELP (
    echo Usage: run-examples.bat [options]
    echo.
    echo Options:
    echo   --file ^<path^>    Specify PDF file to process
    echo   --all            Run all examples with the specified file
    echo   --help, -h       Show this help message
    echo.
    echo Examples:
    echo   run-examples.bat                    ^(Interactive mode^)
    echo   run-examples.bat --file sample.pdf
    echo   run-examples.bat --file test.pdf --all
    echo.
    echo Environment Variables:
    echo   API_KEY          Set API key for AI features
    echo   DEBUG            Set to 1 for detailed error info
    echo.
    pause
    exit /b 0
)

REM Run examples based on arguments
if defined PDF_FILE (
    if defined RUN_ALL (
        echo Running all examples with file: %PDF_FILE%
        echo.
        npx tsx run-examples-simple.ts --file="%PDF_FILE%" --all
    ) else (
        echo Running examples with file: %PDF_FILE%
        echo.
        npx tsx run-examples-simple.ts --file="%PDF_FILE%"
    )
) else (
    echo Starting interactive example runner...
    echo.
    echo Available options:
    echo 1. Interactive mode ^(default^)
    echo 2. Open browser demo
    echo 3. Exit
    echo.
    set /p "choice=Choose an option (1-3): "
    
    if "%choice%"=="1" (
        npx tsx run-examples-simple.ts
    ) else if "%choice%"=="2" (
        echo.
        echo Opening browser demo...
        echo Please open examples-demo.html in your web browser
        start examples-demo.html
    ) else if "%choice%"=="3" (
        exit /b 0
    ) else (
        echo Invalid choice. Starting interactive mode...
        npx tsx run-examples-simple.ts
    )
)

REM Check exit code
if errorlevel 1 (
    echo.
    echo ================================
    echo Example execution completed with errors
    echo ================================
    echo.
    echo Troubleshooting tips:
    echo - Ensure your PDF file exists and is readable
    echo - Check that you have a valid API key for AI features
    echo - Try running with DEBUG=1 for more detailed error information
    echo.
    pause
) else (
    echo.
    echo ================================
    echo Examples completed successfully!
    echo ================================
    echo.
)

pause