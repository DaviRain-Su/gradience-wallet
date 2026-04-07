# Gradience Examples — One-click launcher (Windows)
# Starts: API + Web UI + Embedded wallet demo server

$ROOT = Split-Path -Parent $PSScriptRoot

Write-Host "[Examples] Starting Gradience API + Web UI..."
$localJob = Start-Job -ScriptBlock {
    param($root)
    Set-Location $root
    .\start-local.ps1
} -ArgumentList $ROOT

Write-Host "[Examples] Starting embedded-wallet demo on :3001..."
$embedJob = Start-Job -ScriptBlock {
    param($examples)
    Set-Location $examples
    npx serve -p 3001 embedded-wallet
} -ArgumentList $PSScriptRoot

function Cleanup {
    Write-Host "[Examples] Shutting down all demos..."
    Stop-Job $localJob, $embedJob -ErrorAction SilentlyContinue
    Remove-Job $localJob, $embedJob -ErrorAction SilentlyContinue
    exit
}

trap { Cleanup }

Write-Host ""
Write-Host "=========================================="
Write-Host "  Gradience Demo Matrix Ready"
Write-Host "=========================================="
Write-Host "  Web UI      -> http://localhost:3000"
Write-Host "  API         -> http://localhost:8080"
Write-Host "  Embedded    -> http://localhost:3001"
Write-Host ""
Write-Host "  MCP Client  -> cd examples\mcp-client"
Write-Host "                  `$env:WALLET_ID='id'; node index.js"
Write-Host "=========================================="
Write-Host ""
Write-Host "Press Ctrl+C to stop all demos."

while ($true) {
    Start-Sleep -Seconds 1
}
