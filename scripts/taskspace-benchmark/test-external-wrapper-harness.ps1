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
    [string]$SandboxMode,
    [string[]]$ConfigOverride,
    [string]$AuditReviewRoot,
    [switch]$EnableAggregate,
    [switch]$AllowNonE2Result,
    [switch]$PlanOnly
)
Write-Host "validation_timeout=$ValidationTimeoutSeconds"
if ($PlanOnly) { exit 0 }
if ($AllowNonE2Result) { Write-Host "stub diagnostic allowed"; exit 0 }
Write-Host "stub target unsatisfied"
exit 1
'@ | Set-Content -LiteralPath $runnerStub -Encoding UTF8

$wrapperRunRoot = Join-Path $runDir "wrapper-runs"
$defaultOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-external-benchmark.ps1") -Benchmark terminal-bench -TaskDir $taskDir -SourceVersion "pinned" -RunRoot $wrapperRunRoot -RunnerPath $runnerStub 2>&1
Assert-True ($LASTEXITCODE -ne 0) "external benchmark wrapper hid unsatisfied E3 target by default"
$diagnosticOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-external-benchmark.ps1") -Benchmark terminal-bench -TaskDir $taskDir -SourceVersion "pinned" -RunRoot $wrapperRunRoot -RunnerPath $runnerStub -ValidationTimeoutSeconds 77 -AllowDiagnosticNonTargetResult 2>&1
Assert-True ($LASTEXITCODE -eq 0) "external benchmark wrapper did not allow explicit diagnostic non-target result"
Assert-True (($diagnosticOutput -join "`n") -match "DiagnosticNonTargetResultAllowed: True") "external benchmark wrapper did not print diagnostic opt-in marker"
Assert-True (($diagnosticOutput -join "`n") -match "validation_timeout=77") "external benchmark wrapper did not pass validation timeout separately"

if ($failures.Count -gt 0) {
    Write-Host "TaskSpace external wrapper self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "TaskSpace external wrapper self-test: PASS"
Write-Host "RunRoot: $runDir"
