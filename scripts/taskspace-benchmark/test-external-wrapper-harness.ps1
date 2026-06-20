param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\external-wrapper-selftest" }
$runDir = Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff")
New-Item -ItemType Directory -Path $runDir -Force | Out-Null
$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { $script:failures.Add($Message) } }

$taskDir = Join-Path $runDir "terminal-bench-no-env"
New-Item -ItemType Directory -Path $taskDir | Out-Null
@'
instruction: |-
  Create a file called hello.txt.
category: file-operations
'@ | Set-Content -LiteralPath (Join-Path $taskDir "task.yaml") -Encoding UTF8
"FROM scratch" | Set-Content -LiteralPath (Join-Path $taskDir "Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $taskDir "run-tests.sh") -Encoding UTF8

$runnerStub = Join-Path $runDir "external-wrapper-stub.ps1"
@'
param(
    [string]$ScenarioPath,
    [int]$Repeats,
    [string]$WhaleBin,
    [string]$Model,
    [string]$RunRoot,
    [int]$TimeoutSeconds,
    [int]$ValidationTimeoutSeconds,
    [string]$SourceVersion,
    [string]$SandboxMode,
    [string[]]$ConfigOverride,
    [string]$AuditReviewRoot,
    [switch]$EnableAggregate,
    [switch]$AllowNonE2Result,
    [switch]$PlanOnly
)
Write-Host "validation_timeout=$ValidationTimeoutSeconds"
Write-Host "source_version=$SourceVersion"
if ($PlanOnly) { exit 0 }
if ($AllowNonE2Result) { Write-Host "stub diagnostic allowed"; exit 0 }
Write-Host "stub target unsatisfied"
exit 1
'@ | Set-Content -LiteralPath $runnerStub -Encoding UTF8
$freshWhaleBin = Join-Path $runDir "fresh-whale.exe"
"fake fresh whale" | Set-Content -LiteralPath $freshWhaleBin -Encoding UTF8
(Get-Item -LiteralPath $freshWhaleBin).LastWriteTimeUtc = (Get-Date).ToUniversalTime().AddMinutes(1)

$wrapperRunRoot = Join-Path $runDir "wrapper-runs"
$defaultOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-external-benchmark.ps1") -Benchmark terminal-bench -TaskDir $taskDir -SourceVersion "pinned" -RunRoot $wrapperRunRoot -RunnerPath $runnerStub -WhaleBin $freshWhaleBin 2>&1
Assert-True ($LASTEXITCODE -ne 0) "external benchmark wrapper hid unsatisfied E3 target by default"
$diagnosticOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-external-benchmark.ps1") -Benchmark terminal-bench -TaskDir $taskDir -SourceVersion "pinned" -RunRoot $wrapperRunRoot -RunnerPath $runnerStub -WhaleBin $freshWhaleBin -ValidationTimeoutSeconds 77 -AllowDiagnosticNonTargetResult 2>&1
Assert-True ($LASTEXITCODE -eq 0) "external benchmark wrapper did not allow explicit diagnostic non-target result"
Assert-True (($diagnosticOutput -join "`n") -match "DiagnosticNonTargetResultAllowed: True") "external benchmark wrapper did not print diagnostic opt-in marker"
Assert-True (($diagnosticOutput -join "`n") -match "validation_timeout=77") "external benchmark wrapper did not pass validation timeout separately"
Assert-True (($diagnosticOutput -join "`n") -match "source_version=pinned") "external benchmark wrapper did not pass source version to runner"

$staleWhaleBin = Join-Path $runDir "stale-whale.exe"
"fake stale whale" | Set-Content -LiteralPath $staleWhaleBin -Encoding UTF8
(Get-Item -LiteralPath $staleWhaleBin).LastWriteTimeUtc = ([DateTimeOffset]::FromUnixTimeSeconds(0)).UtcDateTime
$staleRunRoot = Join-Path $runDir "stale-wrapper-run"
$staleOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-external-benchmark.ps1") -Benchmark terminal-bench -TaskDir $taskDir -SourceVersion "pinned" -RunRoot $staleRunRoot -RunnerPath $runnerStub -WhaleBin $staleWhaleBin 2>&1
Assert-True ($LASTEXITCODE -eq 3) "external benchmark wrapper did not reject stale whale binary"
Assert-True (($staleOutput -join "`n") -match "WhaleBinaryHealth:") "external benchmark wrapper did not print stale binary health path"
$staleSampleStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $staleRunRoot "sample-status.json") | ConvertFrom-Json
Assert-True ([string]$staleSampleStatus.run_validity -eq "invalid_harness") "stale whale binary was not classified as invalid_harness"
Assert-True ([string]$staleSampleStatus.abort_reason -eq "whale_binary_stale_for_codex_source") "stale whale binary abort reason was not stable"

$directRunRoot = Join-Path $runDir "direct-stale-run"
$directOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-benchmark.ps1") -Scenario single-file-fast-fix -RunRoot $directRunRoot -WhaleBin $staleWhaleBin 2>&1
Assert-True ($LASTEXITCODE -eq 3) "direct benchmark runner did not reject stale whale binary"
Assert-True (($directOutput -join "`n") -match "WhaleBinaryHealth:") "direct benchmark runner did not print stale binary health path"
$directStatusPath = Get-ChildItem -LiteralPath $directRunRoot -Filter "sample-status.json" -Recurse | Select-Object -First 1
Assert-True ($null -ne $directStatusPath) "direct benchmark runner did not write sample-status for stale whale binary"
if ($directStatusPath) {
    $directSampleStatus = Get-Content -Raw -Encoding UTF8 -LiteralPath $directStatusPath.FullName | ConvertFrom-Json
    Assert-True ([string]$directSampleStatus.abort_phase -eq "whale_binary_preflight") "direct stale whale binary abort phase was not stable"
    Assert-True ([string]$directSampleStatus.abort_reason -eq "whale_binary_stale_for_codex_source") "direct stale whale binary abort reason was not stable"
}

if ($failures.Count -gt 0) {
    Write-Host "TaskSpace external wrapper self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "TaskSpace external wrapper self-test: PASS"
Write-Host "RunRoot: $runDir"
