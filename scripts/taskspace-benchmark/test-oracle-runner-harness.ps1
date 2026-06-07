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

$fakeBin = New-Dir (Join-Path $runDir "fake-bin")
$fakeDockerLog = Join-Path $artifactDir "fake-docker.log"
$fakeDocker = Join-Path $fakeBin "docker.cmd"
@'
@echo off
echo %*>>"%FAKE_DOCKER_LOG%"
if "%1"=="inspect" (
  echo [{^"Id^":^"fake-container-id^",^"Config^":{^"Labels^":{^"whale.taskspace.terminal_bench^":^"true^",^"whale.taskspace.repo_hash^":^"0123456789abcdef^",^"whale.taskspace.proof_nonce^":^"0123456789abcdef0123456789abcdef^",^"whale.taskspace.proof_dir_hash^":^"fedcba9876543210^"}}}]
  exit /b 0
)
if "%1"=="rm" exit /b 0
if "%1"=="image" exit /b 1
exit /b 0
'@ | Set-Content -LiteralPath $fakeDocker -Encoding ASCII
$cleanupProofDir = New-Dir (Join-Path $artifactDir "cleanup-proof")
@{
    proof_nonce = "0123456789abcdef0123456789abcdef"
    docker_backend = "native"
    image = "whale-taskspace-terminal-bench:0123456789abcdef-01234567"
    container_name = "whale-tbench-0123456789abcdef-01234567"
    repo_hash = "0123456789abcdef"
    proof_dir_hash = "fedcba9876543210"
    validator_command = "bash /tests/run-tests.sh"
} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $cleanupProofDir "terminal-bench-runtime-manifest.json") -Encoding UTF8
$cleanupStdout = Join-Path $artifactDir "validation-cleanup-timeout.stdout.log"
$cleanupStderr = Join-Path $artifactDir "validation-cleanup-timeout.stderr.log"
$oldPath = $env:PATH
$oldFakeDockerLog = $env:FAKE_DOCKER_LOG
try {
    $env:PATH = "$fakeBin;$oldPath"
    $env:FAKE_DOCKER_LOG = $fakeDockerLog
    $cleanupTimeoutExit = Invoke-TaskspaceValidationCommand $repoDir ([pscustomobject]@{
        command = "powershell"
        args = @("-NoProfile", "-Command", "Start-Sleep -Seconds 3")
    }) $cleanupStdout $cleanupStderr 1 $cleanupProofDir
} finally {
    $env:PATH = $oldPath
    if ($null -eq $oldFakeDockerLog) {
        Remove-Item Env:\FAKE_DOCKER_LOG -ErrorAction SilentlyContinue
    } else {
        $env:FAKE_DOCKER_LOG = $oldFakeDockerLog
    }
}
if ($cleanupTimeoutExit -ne 124) {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- validation cleanup timeout path returned $cleanupTimeoutExit instead of 124"
    exit 1
}
$cleanupResultPath = Join-Path $cleanupProofDir "validation-cleanup-result.json"
if (-not (Test-Path -LiteralPath $cleanupResultPath)) {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- timeout cleanup result was not written"
    exit 1
}
$cleanupResult = Get-Content -Raw -Encoding UTF8 -LiteralPath $cleanupResultPath | ConvertFrom-Json
if ([string]$cleanupResult.classification -ne "ok" -or [string]$cleanupResult.reason -ne "timeout") {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- timeout cleanup result was not classified as ok timeout"
    exit 1
}
$fakeDockerText = if (Test-Path -LiteralPath $fakeDockerLog) { Get-Content -Raw -Encoding UTF8 -LiteralPath $fakeDockerLog } else { "" }
if ($fakeDockerText -notmatch "inspect whale-tbench-0123456789abcdef-01234567" -or $fakeDockerText -notmatch "rm -f whale-tbench-0123456789abcdef-01234567") {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- timeout cleanup did not inspect and remove the exact validator container"
    exit 1
}
if ($fakeDockerText -match "whale\.taskspace\.terminal_bench") {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- timeout cleanup used a broad label selector"
    exit 1
}
$cleanupStderrText = Get-Content -Raw -Encoding UTF8 -LiteralPath $cleanupStderr
if ($cleanupStderrText -notmatch "(?m)^validation_cleanup_result_path=") {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- timeout cleanup result marker was not written as a standalone stderr line"
    exit 1
}

$invalidProofDir = New-Dir (Join-Path $artifactDir "invalid-cleanup-proof")
Set-Content -LiteralPath (Join-Path $invalidProofDir "terminal-bench-runtime-manifest.json") -Encoding UTF8 -Value "{not-json"
$invalidStdout = Join-Path $artifactDir "validation-invalid-cleanup.stdout.log"
$invalidStderr = Join-Path $artifactDir "validation-invalid-cleanup.stderr.log"
$invalidCleanupExit = Invoke-TaskspaceValidationCommand $repoDir ([pscustomobject]@{
    command = "powershell"
    args = @("-NoProfile", "-Command", "Start-Sleep -Seconds 3")
}) $invalidStdout $invalidStderr 1 $invalidProofDir
if ($invalidCleanupExit -ne 124) {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- invalid cleanup manifest changed validation timeout exit code to $invalidCleanupExit"
    exit 1
}
$invalidCleanupResult = Join-Path $invalidProofDir "validation-cleanup-result.json"
if (-not (Test-Path -LiteralPath $invalidCleanupResult)) {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- invalid cleanup manifest did not write a cleanup result"
    exit 1
}
$invalidCleanup = Get-Content -Raw -Encoding UTF8 -LiteralPath $invalidCleanupResult | ConvertFrom-Json
if ([string]$invalidCleanup.classification -ne "docker_cleanup_manifest_invalid") {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- invalid cleanup manifest was not fail-closed"
    exit 1
}

$fakeFailBin = New-Dir (Join-Path $runDir "fake-fail-bin")
$fakeFailDockerLog = Join-Path $artifactDir "fake-fail-docker.log"
$fakeFailDocker = Join-Path $fakeFailBin "docker.cmd"
@'
@echo off
echo %*>>"%FAKE_DOCKER_LOG%"
if "%1"=="inspect" (
  echo [{^"Id^":^"fake-container-id^",^"Config^":{^"Labels^":{^"whale.taskspace.terminal_bench^":^"true^",^"whale.taskspace.repo_hash^":^"0123456789abcdef^",^"whale.taskspace.proof_nonce^":^"0123456789abcdef0123456789abcdef^",^"whale.taskspace.proof_dir_hash^":^"fedcba9876543210^"}}}]
  exit /b 0
)
if "%1"=="rm" exit /b 7
if "%1"=="image" exit /b 1
exit /b 0
'@ | Set-Content -LiteralPath $fakeFailDocker -Encoding ASCII
$cleanupFailProofDir = New-Dir (Join-Path $artifactDir "cleanup-failure-proof")
@{
    proof_nonce = "0123456789abcdef0123456789abcdef"
    docker_backend = "native"
    image = "whale-taskspace-terminal-bench:0123456789abcdef-01234567"
    container_name = "whale-tbench-0123456789abcdef-01234567"
    repo_hash = "0123456789abcdef"
    proof_dir_hash = "fedcba9876543210"
    validator_command = "bash /tests/run-tests.sh"
} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $cleanupFailProofDir "terminal-bench-runtime-manifest.json") -Encoding UTF8
$cleanupFailStdout = Join-Path $artifactDir "validation-cleanup-failure.stdout.log"
$cleanupFailStderr = Join-Path $artifactDir "validation-cleanup-failure.stderr.log"
$oldPath = $env:PATH
$oldFakeDockerLog = $env:FAKE_DOCKER_LOG
try {
    $env:PATH = "$fakeFailBin;$oldPath"
    $env:FAKE_DOCKER_LOG = $fakeFailDockerLog
    $cleanupFailureExit = Invoke-TaskspaceValidationCommand $repoDir ([pscustomobject]@{
        command = "powershell"
        args = @("-NoProfile", "-Command", "Start-Sleep -Seconds 3")
    }) $cleanupFailStdout $cleanupFailStderr 1 $cleanupFailProofDir
} finally {
    $env:PATH = $oldPath
    if ($null -eq $oldFakeDockerLog) {
        Remove-Item Env:\FAKE_DOCKER_LOG -ErrorAction SilentlyContinue
    } else {
        $env:FAKE_DOCKER_LOG = $oldFakeDockerLog
    }
}
if ($cleanupFailureExit -ne 124) {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- cleanup rm failure changed validation timeout exit code to $cleanupFailureExit"
    exit 1
}
$cleanupFailure = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $cleanupFailProofDir "validation-cleanup-result.json") | ConvertFrom-Json
if ([string]$cleanupFailure.classification -ne "docker_cleanup_container_failure" -or [int]$cleanupFailure.container_rm_exit_code -ne 7) {
    Write-Host "TaskSpace oracle-runner self-test: FAIL"
    Write-Host "- cleanup rm failure was not preserved as cleanup classification"
    exit 1
}

Write-Host "TaskSpace oracle-runner self-test: PASS"
Write-Host "RunRoot: $runDir"
