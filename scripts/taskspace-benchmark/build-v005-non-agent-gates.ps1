param(
    [Parameter(Mandatory = $true)][string]$RunRoot,
    [Parameter(Mandatory = $true)][string]$TaskListHash,
    [Parameter(Mandatory = $true)][string]$ProfileHash,
    [Parameter(Mandatory = $true)][string]$SourceVersion,
    [string]$OutputPath = "",
    [switch]$FixtureMode
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\e3-start-gate.ps1")

$runRootFull = [System.IO.Path]::GetFullPath($RunRoot)
New-Item -ItemType Directory -Force -Path $runRootFull | Out-Null
$evidenceRoot = Join-Path $runRootFull "non-agent-evidence"
New-Item -ItemType Directory -Force -Path $evidenceRoot | Out-Null
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $runRootFull "v005-non-agent-gates.json"
}
$head = (& git -C $repoRoot rev-parse HEAD).Trim()
$generatedAt = (Get-Date).ToString("o")

function New-V005GateSpec {
    param([string]$Name, [string]$Command, [int]$TimeoutSeconds)
    [pscustomobject]@{ name = $Name; command = $Command; timeout_seconds = $TimeoutSeconds }
}

if ($FixtureMode) {
    $pass = "Write-Output 'fixture gate pass'; exit 0"
    $specs = @(
        New-V005GateSpec "provider_request_hook" $pass 30
        New-V005GateSpec "runtime_budget_response" $pass 30
        New-V005GateSpec "budget_quality_impact" $pass 30
        New-V005GateSpec "active_context_replacement" $pass 30
        New-V005GateSpec "state_commit_displacement" $pass 30
        New-V005GateSpec "spawn_node_budget" $pass 30
        New-V005GateSpec "request_phase_attribution" $pass 30
        New-V005GateSpec "release_decision_fixture" $pass 30
        New-V005GateSpec "start_gate_fixture" $pass 30
        New-V005GateSpec "external_wrapper_fixture" $pass 30
        New-V005GateSpec "marker_writer_fixture" $pass 30
        New-V005GateSpec "r4_tool_path_coverage" $pass 30
        New-V005GateSpec "r4_sample_ledger" $pass 30
        New-V005GateSpec "r4_public_10_tool_stress_plan" $pass 30
        New-V005GateSpec "cache_regression_surface" $pass 30
    )
} else {
    $rustWorkspace = "Set-Location third_party\codex-cli\codex-rs;"
    $specs = @(
        New-V005GateSpec "provider_request_hook" "$rustWorkspace cargo test -p codex-core provider_request_budget --lib; exit `$LASTEXITCODE" 600
        New-V005GateSpec "runtime_budget_response" "$rustWorkspace cargo test -p codex-core provider_request_budget --lib; exit `$LASTEXITCODE" 600
        New-V005GateSpec "budget_quality_impact" "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1" 180
        New-V005GateSpec "active_context_replacement" "$rustWorkspace cargo test -p codex-core active_context_replacement --lib; exit `$LASTEXITCODE" 600
        New-V005GateSpec "state_commit_displacement" "$rustWorkspace cargo test -p codex-core state_commit --lib; exit `$LASTEXITCODE" 600
        New-V005GateSpec "spawn_node_budget" "$rustWorkspace cargo test -p codex-core budget --lib; exit `$LASTEXITCODE" 600
        New-V005GateSpec "request_phase_attribution" "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-cost-instrumentation.ps1" 180
        New-V005GateSpec "release_decision_fixture" "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-release-decision.ps1" 240
        New-V005GateSpec "start_gate_fixture" "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-e3-start-gate.ps1" 420
        New-V005GateSpec "external_wrapper_fixture" "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-external-wrapper-harness.ps1" 240
        New-V005GateSpec "marker_writer_fixture" "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-v005-marker-writer.ps1" 120
        New-V005GateSpec "r4_tool_path_coverage" "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-r4-tool-path-coverage.ps1" 120
        New-V005GateSpec "r4_sample_ledger" "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-r4-sample-ledger.ps1" 120
        New-V005GateSpec "r4_public_10_tool_stress_plan" "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\taskspace-benchmark\test-r4-public-10-tool-stress-plan.ps1" 120
        New-V005GateSpec "cache_regression_surface" "python3 scripts\cache-regression\check_cache_regression_gate.py --source head --require-live-baseline --require-clean-subject" 900
    )
}

$resultsByCommand = @{}
$gates = [ordered]@{}
foreach ($spec in @($specs)) {
    $command = [string]$spec.command
    if (-not $resultsByCommand.ContainsKey($command)) {
        $resultsByCommand[$command] = Invoke-TaskspaceGateCommand $repoRoot $command ([int]$spec.timeout_seconds)
    }
    $result = $resultsByCommand[$command]
    $evidencePath = Join-Path $evidenceRoot "$($spec.name).txt"
    $lines = New-Object System.Collections.Generic.List[string]
    [void]$lines.Add("gate: $($spec.name)")
    [void]$lines.Add("command: $command")
    [void]$lines.Add("exit_code: $($result.exit_code)")
    [void]$lines.Add("timed_out: $($result.timed_out)")
    [void]$lines.Add("duration_ms: $($result.duration_ms)")
    [void]$lines.Add("git_commit: $head")
    [void]$lines.Add("generated_at: $generatedAt")
    [void]$lines.Add("")
    [void]$lines.Add("output_tail:")
    foreach ($line in @($result.output_tail)) { [void]$lines.Add([string]$line) }
    $lines.ToArray() | Set-Content -LiteralPath $evidencePath -Encoding UTF8
    $sha = (Get-FileHash -LiteralPath $evidencePath -Algorithm SHA256).Hash.ToLowerInvariant()
    $gates[[string]$spec.name] = [pscustomobject]@{
        status = if ([int]$result.exit_code -eq 0 -and -not [bool]$result.timed_out) { "pass" } else { "fail" }
        producer = "build-v005-non-agent-gates.ps1"
        command = $command
        exit_code = [int]$result.exit_code
        timed_out = [bool]$result.timed_out
        duration_ms = [int64]$result.duration_ms
        generated_at = $generatedAt
        git_commit = $head
        profile_hash = $ProfileHash
        task_list_hash = $TaskListHash
        source_version = $SourceVersion
        evidence_path = $evidencePath
        evidence_sha256 = $sha
    }
}

$failed = @($gates.Values | Where-Object { [string]$_.status -ne "pass" })
$artifact = [pscustomobject]@{
    schema_version = 1
    status = if ($failed.Count -eq 0) { "pass" } else { "fail" }
    producer = "build-v005-non-agent-gates.ps1"
    git_commit = $head
    profile_hash = $ProfileHash
    task_list_hash = $TaskListHash
    source_version = $SourceVersion
    generated_at = $generatedAt
    gates = [pscustomobject]$gates
}
$artifact | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
if ($failed.Count -eq 0) { exit 0 }
exit 1
