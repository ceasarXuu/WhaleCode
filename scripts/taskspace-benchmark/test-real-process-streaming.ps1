param()

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $repoRoot "scripts\action-map-real-user-e2e-lib.ps1")

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function New-TestDir {
    $dir = Join-Path ([System.IO.Path]::GetTempPath()) ("whale-real-process-streaming-" + [guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $dir
}

$normalDir = New-TestDir
$normalOut = Join-Path $normalDir "stdout.log"
$normalErr = Join-Path $normalDir "stderr.log"
$normalTiming = Join-Path $normalDir "timing.json"
$normalExit = Invoke-RealProcess "powershell" @(
    "-NoProfile",
    "-Command",
    "[Console]::Out.WriteLine('normal-out'); [Console]::Error.WriteLine('normal-err')"
) $normalDir $normalOut $normalErr 10 "" $normalTiming

Assert-True ($normalExit -eq 0) "normal process exit code should be 0"
Assert-True ((Get-Content -Raw -LiteralPath $normalOut) -match "normal-out") "normal stdout was not streamed"
Assert-True ((Get-Content -Raw -LiteralPath $normalErr) -match "normal-err") "normal stderr was not streamed"
$normalTimingJson = Get-Content -Raw -LiteralPath $normalTiming | ConvertFrom-Json
Assert-True (-not [bool]$normalTimingJson.timed_out) "normal timing should not be timed_out"
Assert-True ([int]$normalTimingJson.exit_code -eq 0) "normal timing exit_code should be 0"

$timeoutDir = New-TestDir
$timeoutOut = Join-Path $timeoutDir "stdout.log"
$timeoutErr = Join-Path $timeoutDir "stderr.log"
$timeoutTiming = Join-Path $timeoutDir "timing.json"
$timedOut = $false
try {
    Invoke-RealProcess "powershell" @(
        "-NoProfile",
        "-Command",
        "[Console]::Out.WriteLine('timeout-out-before'); [Console]::Error.WriteLine('timeout-err-before'); Start-Sleep -Seconds 5; [Console]::Out.WriteLine('timeout-out-after')"
    ) $timeoutDir $timeoutOut $timeoutErr 1 "" $timeoutTiming | Out-Null
} catch {
    $timedOut = ([string]$_.Exception.Message -match "Process timed out after 1 seconds")
}

Assert-True $timedOut "timeout process should throw timeout error"
Assert-True ((Get-Content -Raw -LiteralPath $timeoutOut) -match "timeout-out-before") "timeout stdout before kill was not preserved"
Assert-True ((Get-Content -Raw -LiteralPath $timeoutOut) -notmatch "timeout-out-after") "timeout stdout after kill should not be present"
$timeoutErrText = Get-Content -Raw -LiteralPath $timeoutErr
Assert-True ($timeoutErrText -match "timeout-err-before") "timeout stderr before kill was not preserved"
Assert-True ($timeoutErrText -match "Process timed out after 1 seconds") "timeout marker was not written to stderr"
$timeoutTimingJson = Get-Content -Raw -LiteralPath $timeoutTiming | ConvertFrom-Json
Assert-True ([bool]$timeoutTimingJson.timed_out) "timeout timing should be timed_out"
Assert-True (-not [bool]$timeoutTimingJson.completed) "timeout timing should not be completed"

Write-Host "PASS: Invoke-RealProcess streams stdout/stderr and preserves timeout diagnostics"
foreach ($path in @($normalDir, $timeoutDir)) {
    if (Test-Path -LiteralPath $path) { Remove-Item -Force -Recurse -LiteralPath $path }
}
