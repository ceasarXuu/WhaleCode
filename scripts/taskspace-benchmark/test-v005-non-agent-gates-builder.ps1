param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\v005-non-agent-gates-builder-selftest" }
$runDir = Join-Path ([System.IO.Path]::GetFullPath($RunRoot)) (Get-Date -Format "yyyyMMdd-HHmmss-fff")
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { [void]$script:failures.Add($Message) } }

$taskListHash = "task-list-builder-selftest"
$profileHash = "profile-builder-selftest"
$sourceVersion = "terminal-bench@builder-selftest"
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "build-v005-non-agent-gates.ps1") `
    -RunRoot $runDir `
    -TaskListHash $taskListHash `
    -ProfileHash $profileHash `
    -SourceVersion $sourceVersion `
    -FixtureMode *> $null
Assert-True ($LASTEXITCODE -eq 0) "builder fixture mode did not exit 0"

$artifactPath = Join-Path $runDir "v005-non-agent-gates.json"
Assert-True (Test-Path -LiteralPath $artifactPath) "builder did not write v005-non-agent-gates.json"
$artifact = Get-Content -Raw -Encoding UTF8 -LiteralPath $artifactPath | ConvertFrom-Json
$head = (& git -C $repoRoot rev-parse HEAD).Trim()
Assert-True ([int]$artifact.schema_version -eq 1 -and [string]$artifact.status -eq "pass") "builder artifact did not pass schema/status"
Assert-True ([string]$artifact.git_commit -eq $head -and [string]$artifact.task_list_hash -eq $taskListHash -and [string]$artifact.profile_hash -eq $profileHash -and [string]$artifact.source_version -eq $sourceVersion) "builder artifact identity did not bind to current inputs"

$requiredGates = @(
    "provider_request_hook",
    "runtime_budget_response",
    "budget_quality_impact",
    "active_context_replacement",
    "state_commit_displacement",
    "spawn_node_budget",
    "request_phase_attribution",
    "release_decision_fixture",
    "start_gate_fixture",
    "external_wrapper_fixture",
    "marker_writer_fixture",
    "r4_tool_path_coverage",
    "r4_sample_ledger",
    "cache_regression_surface"
)
foreach ($gateName in $requiredGates) {
    $gate = $artifact.gates.$gateName
    Assert-True ($null -ne $gate) "missing gate: $gateName"
    if ($gate) {
        Assert-True ([string]$gate.status -eq "pass" -and [int]$gate.exit_code -eq 0) "gate did not pass: $gateName"
        Assert-True ([string]$gate.git_commit -eq $head -and [string]$gate.task_list_hash -eq $taskListHash -and [string]$gate.profile_hash -eq $profileHash -and [string]$gate.source_version -eq $sourceVersion) "gate identity mismatch: $gateName"
        Assert-True (-not [string]::IsNullOrWhiteSpace([string]$gate.command)) "gate command missing: $gateName"
        Assert-True ((Test-Path -LiteralPath ([string]$gate.evidence_path) -PathType Leaf)) "gate evidence missing: $gateName"
        $sha = if (Test-Path -LiteralPath ([string]$gate.evidence_path) -PathType Leaf) { (Get-FileHash -LiteralPath ([string]$gate.evidence_path) -Algorithm SHA256).Hash.ToLowerInvariant() } else { "" }
        Assert-True ($sha -eq [string]$gate.evidence_sha256) "gate evidence sha mismatch: $gateName"
    }
}

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Host "v005 non-agent gates builder selftest passed"
