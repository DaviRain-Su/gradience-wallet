# Gradience Wallet - Local-first launcher (Windows)
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Definition
Set-Location $repoRoot

$env:DATABASE_URL = "sqlite:./gradience.db?mode=rwc"

Write-Host "[Gradience] Starting local API server..."
$apiJob = Start-Job -ScriptBlock {
    Set-Location $using:repoRoot
    cargo run -p gradience-api
}

Write-Host "[Gradience] Starting web frontend..."
$webJob = Start-Job -ScriptBlock {
    Set-Location (Join-Path $using:repoRoot "web")
    npm run dev
}

function Cleanup {
    Write-Host "[Gradience] Shutting down..."
    Stop-Job $webJob, $apiJob -ErrorAction SilentlyContinue
    Remove-Job $webJob, $apiJob -ErrorAction SilentlyContinue
}
trap { Cleanup; break }

Write-Host "[Gradience] Waiting for services to be ready..."
$ready = $false
for ($i = 0; $i -lt 60; $i++) {
    try {
        $web = Invoke-WebRequest -Uri http://localhost:3000 -UseBasicParsing -ErrorAction Stop
        $api = Invoke-WebRequest -Uri http://localhost:8080/health -UseBasicParsing -ErrorAction Stop
        $ready = $true
        break
    } catch {
        Start-Sleep -Seconds 1
    }
}

if ($ready) {
    Write-Host "[Gradience] Ready! Opening http://localhost:3000"
    Start-Process "http://localhost:3000"
} else {
    Write-Host "[Gradience] Timed out waiting for services. Check logs above."
}

while ($webJob.State -eq "Running" -or $apiJob.State -eq "Running") {
    Start-Sleep -Seconds 1
    Receive-Job $webJob
    Receive-Job $apiJob
}

Cleanup
