# ModernPDF CLI Examples
# Practical examples for common PDF processing tasks

Write-Host "ModernPDF CLI - Usage Examples" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor DarkGray
Write-Host ""

# Example 1: Process a single PDF
Write-Host "Example 1: Extract text from a PDF" -ForegroundColor Yellow
Write-Host "Command:" -ForegroundColor Gray
Write-Host '  npm run cli -- extract -i demos/sample.pdf -o output.txt' -ForegroundColor White
Write-Host ""

# Example 2: Batch processing
Write-Host "Example 2: Batch process all PDFs in a directory" -ForegroundColor Yellow
Write-Host "Command:" -ForegroundColor Gray
Write-Host @'
  Get-ChildItem *.pdf | ForEach-Object {
    npm run cli -- extract -i $_.Name -o "$($_.BaseName).txt"
  }
'@ -ForegroundColor White
Write-Host ""

# Example 3: AI Analysis
Write-Host "Example 3: AI-powered document analysis" -ForegroundColor Yellow
Write-Host "Command:" -ForegroundColor Gray
Write-Host '  npm run cli -- analyze -i demos/sample.pdf --ai --pretty -o analysis.json' -ForegroundColor White
Write-Host ""

# Example 4: Generate RAG chunks
Write-Host "Example 4: Generate semantic chunks for RAG" -ForegroundColor Yellow
Write-Host "Command:" -ForegroundColor Gray
Write-Host '  npm run cli -- chunk -i demos/sample.pdf --chunk-size 1000 --pretty -o chunks.json' -ForegroundColor White
Write-Host ""

# Example 5: Convert to multiple formats
Write-Host "Example 5: Convert to multiple formats" -ForegroundColor Yellow
Write-Host "Command:" -ForegroundColor Gray
Write-Host @'
  npm run cli -- convert -i demos/sample.pdf -f text -o output.txt
  npm run cli -- convert -i demos/sample.pdf -f json --pretty -o output.json
  npm run cli -- convert -i demos/sample.pdf -f html -o output.html
  npm run cli -- convert -i demos/sample.pdf -f markdown -o output.md
'@ -ForegroundColor White
Write-Host ""

# Example 6: Extract images
Write-Host "Example 6: Extract all images" -ForegroundColor Yellow
Write-Host "Command:" -ForegroundColor Gray
Write-Host '  npm run cli -- images -i demos/sample.pdf -o ./extracted-images/ -v' -ForegroundColor White
Write-Host ""

# Example 7: Extract specific pages
Write-Host "Example 7: Extract specific pages" -ForegroundColor Yellow
Write-Host "Command:" -ForegroundColor Gray
Write-Host '  npm run cli -- extract -i demos/sample.pdf -p 1-5 -o first-pages.txt' -ForegroundColor White
Write-Host ""

# Example 8: Streaming large files
Write-Host "Example 8: Stream large PDFs" -ForegroundColor Yellow
Write-Host "Command:" -ForegroundColor Gray
Write-Host '  npm run cli -- extract -i large-document.pdf --stream -v -o output.txt' -ForegroundColor White
Write-Host ""

# Example 9: Get PDF info
Write-Host "Example 9: Display PDF information" -ForegroundColor Yellow
Write-Host "Command:" -ForegroundColor Gray
Write-Host '  npm run cli -- info demos/sample.pdf' -ForegroundColor White
Write-Host ""

# Example 10: Extract form fields
Write-Host "Example 10: Extract form fields" -ForegroundColor Yellow
Write-Host "Command:" -ForegroundColor Gray
Write-Host '  npm run cli -- forms -i form.pdf --pretty -o fields.json' -ForegroundColor White
Write-Host ""

Write-Host "═══════════════════════════════════════════════════════" -ForegroundColor DarkGray
Write-Host ""
Write-Host "Try running any of these commands!" -ForegroundColor Green
Write-Host "For more details, see CLI.md" -ForegroundColor Gray
Write-Host ""

# Interactive menu
Write-Host "Would you like to run a demo? (Y/N)" -ForegroundColor Cyan
$response = Read-Host

if ($response -eq 'Y' -or $response -eq 'y') {
    Write-Host ""
    Write-Host "Running demo: Extract text from sample.pdf" -ForegroundColor Green
    Write-Host ""
    
    if (Test-Path "demos/sample.pdf") {
        npm run cli -- extract -i demos/sample.pdf -v
    }
    else {
        Write-Host "Sample PDF not found at demos/sample.pdf" -ForegroundColor Red
        Write-Host "Please ensure the demos directory contains sample.pdf" -ForegroundColor Yellow
    }
}
else {
    Write-Host "You can run any example by copying and pasting the command." -ForegroundColor Gray
}

Write-Host ""
