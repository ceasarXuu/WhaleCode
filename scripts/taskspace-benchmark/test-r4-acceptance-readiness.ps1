param(
    [string]$OutputPath = "",
    [string]$SnapshotReportPath = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $repoRoot "target\r4-acceptance-readiness\r4-acceptance-readiness.json"
}
if ([string]::IsNullOrWhiteSpace($SnapshotReportPath)) {
    $SnapshotReportPath = Join-Path $repoRoot "docs\v0.0.5\build-R4\r4-public-10-tool-stress-report.snapshot.json"
}

function Invoke-R4ReadinessGate {
    param(
        [string]$Name,
        [string[]]$Arguments
    )
    $started = Get-Date
    $output = & powershell -NoProfile -ExecutionPolicy Bypass @Arguments 2>&1
    $exitCode = $LASTEXITCODE
    [pscustomobject]@{
        name = $Name
        status = if ($exitCode -eq 0) { "pass" } else { "fail" }
        exit_code = [int]$exitCode
        duration_ms = [int64](((Get-Date) - $started).TotalMilliseconds)
        command = "powershell -NoProfile -ExecutionPolicy Bypass $($Arguments -join ' ')"
        output_tail = @($output | Select-Object -Last 20 | ForEach-Object { [string]$_ })
    }
}

$gates = @(
    (Invoke-R4ReadinessGate "tool_path_coverage" @("-File", (Join-Path $PSScriptRoot "test-r4-tool-path-coverage.ps1"))),
    (Invoke-R4ReadinessGate "sample_ledger" @("-File", (Join-Path $PSScriptRoot "test-r4-sample-ledger.ps1"))),
    (Invoke-R4ReadinessGate "public_10_snapshot" @("-File", (Join-Path $PSScriptRoot "test-r4-public-10-tool-stress-plan.ps1"), "-ReportPath", $SnapshotReportPath)),
    (Invoke-R4ReadinessGate "usage_accounting" @("-File", (Join-Path $PSScriptRoot "test-r4-public-10-usage-accounting-gate.ps1"))),
    (Invoke-R4ReadinessGate "external_wrapper" @("-File", (Join-Path $PSScriptRoot "test-external-wrapper-harness.ps1")))
)

$failures = @($gates | Where-Object { [string]$_.status -ne "pass" })
$providerCredentialPresent = -not [string]::IsNullOrWhiteSpace($env:DEEPSEEK_API_KEY)
$blockers = New-Object System.Collections.Generic.List[object]
if (-not $providerCredentialPresent) {
    $blockers.Add([pscustomobject]@{
            stable_code = "provider_credential_missing"
            message = "DEEPSEEK_API_KEY is required before R4 can produce real DeepSeek utility evidence."
            required_next_step = "Set DEEPSEEK_API_KEY and rerun organization-json-generator or the public-10 negative sample subset."
        })
}

$status = if ($failures.Count -gt 0) {
    "fail"
} elseif ($blockers.Count -gt 0) {
    "blocked"
} else {
    "ready_for_real_utility_rerun"
}

$report = [ordered]@{
    schema_version = 1
    artifact = "r4-acceptance-readiness"
    generated_at = (Get-Date).ToString("o")
    repo_root = $repoRoot
    head = (& git -C $repoRoot rev-parse --short HEAD)
    r4_phase = "R4-H/post-closeout"
    status = $status
    engineering_gates_status = if ($failures.Count -eq 0) { "pass" } else { "fail" }
    provider_credential_status = if ($providerCredentialPresent) { "present" } else { "missing" }
    e3_readiness = "not_ready_until_real_utility_evidence_passes"
    gate_count = [int]$gates.Count
    failed_gate_count = [int]$failures.Count
    blockers = @($blockers.ToArray())
    gates = @($gates)
}

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutputPath) | Out-Null
[pscustomobject]$report | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $OutputPath -Encoding UTF8

if ($failures.Count -gt 0) {
    Write-Host "R4 acceptance readiness failed: $($failures.Count) gate(s) failed"
    Write-Host "ReadinessReport: $OutputPath"
    exit 1
}
if ($blockers.Count -gt 0) {
    Write-Host "R4 acceptance readiness blocked: provider_credential_missing"
    Write-Host "ReadinessReport: $OutputPath"
    exit 3
}
Write-Host "R4 acceptance readiness passed for real utility rerun"
Write-Host "ReadinessReport: $OutputPath"
