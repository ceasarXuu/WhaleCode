param(
    [string]$RunRoot = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $repoRoot "scripts\action-map-real-user-e2e-lib.ps1")
. (Join-Path $PSScriptRoot "lib\oracle-runner.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\oracle-runner-selftest" }
$stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
$runDir = New-Dir (Join-Path $RunRoot $stamp)
$repoDir = New-Dir (Join-Path $runDir "repo")
$artifactDir = New-Dir (Join-Path $runDir "artifacts")
$hiddenOraclePath = Join-Path $runDir "reviewer-only\private-oracle\oracle.py"
New-Item -ItemType Directory -Path (Split-Path -Parent $hiddenOraclePath) -Force | Out-Null
Write-Text $hiddenOraclePath "oracle"

New-Item -ItemType File -Path (Join-Path $artifactDir "empty.log") -Force | Out-Null
$clean = Test-TaskspaceOracleLeak $repoDir $artifactDir $hiddenOraclePath
if ($clean.leaked) {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- empty artifact file was treated as oracle leak"
    exit 1
}

Write-Text (Join-Path $artifactDir "leak.log") "leaked path: $hiddenOraclePath"
$leaky = Test-TaskspaceOracleLeak $repoDir $artifactDir $hiddenOraclePath
if (-not $leaky.leaked) {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- oracle path leak was not detected"
    exit 1
}

$timeoutStdout = Join-Path $artifactDir "validation-timeout.stdout.log"
$timeoutStderr = Join-Path $artifactDir "validation-timeout.stderr.log"
$timeoutExit = Invoke-TaskspaceValidationCommand $repoDir ([pscustomobject]@{
    command = "powershell"
    args = @("-NoProfile", "-Command", "Start-Sleep -Seconds 3")
}) $timeoutStdout $timeoutStderr 1
if ($timeoutExit -ne 124) {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- validation timeout aborted or returned $timeoutExit instead of 124"
    exit 1
}

Write-Host "TaskSpace oracle-runner self-test: PASS"
Write-Host "RunRoot: $runDir"
