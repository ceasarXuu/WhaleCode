param(
    [string]$LedgerPath = "",
    [string]$EvidencePath = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if ([string]::IsNullOrWhiteSpace($LedgerPath)) {
    $LedgerPath = Join-Path $repoRoot "docs\v0.0.5\build-R4\r4-sample-evidence-ledger.json"
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
    $EvidencePath = Join-Path $repoRoot "target\r4-sample-ledger\r4-sample-ledger-evidence.json"
}

function Add-Failure {
    param([System.Collections.Generic.List[string]]$Failures, [string]$Message)
    [void]$Failures.Add($Message)
}

function Test-RepoPath {
    param([string]$RelativePath)
    if ([string]::IsNullOrWhiteSpace($RelativePath)) { return $false }
    $fullPath = Join-Path $repoRoot $RelativePath
    Test-Path -LiteralPath $fullPath -PathType Leaf
}

$failures = New-Object System.Collections.Generic.List[string]
$ledger = $null
if (-not (Test-Path -LiteralPath $LedgerPath -PathType Leaf)) {
    Add-Failure $failures "ledger not found: $LedgerPath"
} else {
    $ledger = Get-Content -Raw -Encoding UTF8 -LiteralPath $LedgerPath | ConvertFrom-Json
    if ([int]$ledger.schema_version -ne 1) {
        Add-Failure $failures "schema_version must be 1"
    }
    $samples = @($ledger.samples)
    if ($samples.Count -lt 6) {
        Add-Failure $failures "sample_count must be >= 6"
    }

    $ids = @{}
    foreach ($sample in $samples) {
        $id = [string]$sample.id
        if ([string]::IsNullOrWhiteSpace($id)) {
            Add-Failure $failures "sample id is required"
            continue
        }
        if ($ids.ContainsKey($id)) {
            Add-Failure $failures "duplicate sample id: $id"
        }
        $ids[$id] = $true

        if ([string]::IsNullOrWhiteSpace([string]$sample.failure_class)) {
            Add-Failure $failures "failure_class missing for $id"
        }
        if ([string]$sample.owner_phase -notmatch "^R4-[A-H]$") {
            Add-Failure $failures "owner_phase must be R4-A..R4-H for $id"
        }
        if (-not (Test-RepoPath ([string]$sample.primary_evidence))) {
            Add-Failure $failures "primary_evidence missing for ${id}: $($sample.primary_evidence)"
        }
        foreach ($secondary in @($sample.secondary_evidence)) {
            if ([string]::IsNullOrWhiteSpace([string]$secondary)) { continue }
            if (-not (Test-RepoPath ([string]$secondary))) {
                Add-Failure $failures "secondary_evidence missing for ${id}: $secondary"
            }
        }
        if ([string]::IsNullOrWhiteSpace([string]$sample.required_followup)) {
            Add-Failure $failures "required_followup missing for $id"
        }
    }

    $presentClasses = @($samples | ForEach-Object { [string]$_.failure_class } | Select-Object -Unique)
    foreach ($requiredClass in @($ledger.required_failure_classes)) {
        if ($presentClasses -notcontains [string]$requiredClass) {
            Add-Failure $failures "required failure class missing: $requiredClass"
        }
    }
}

$evidence = [ordered]@{
    schema_version = 1
    artifact = "r4-sample-ledger-evidence"
    generated_at = (Get-Date).ToString("o")
    repo_root = $repoRoot
    ledger_path = [System.IO.Path]::GetFullPath($LedgerPath)
    ledger_sha256 = if (Test-Path -LiteralPath $LedgerPath -PathType Leaf) {
        (Get-FileHash -LiteralPath $LedgerPath -Algorithm SHA256).Hash.ToLowerInvariant()
    } else { "" }
    status = if ($failures.Count -eq 0) { "pass" } else { "fail" }
    sample_count = if ($ledger) { @($ledger.samples).Count } else { 0 }
    failure_count = $failures.Count
    failures = @($failures.ToArray())
}
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $EvidencePath) | Out-Null
[pscustomobject]$evidence | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $EvidencePath -Encoding UTF8

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Host "R4 sample ledger gate passed: $($evidence.sample_count) samples"
