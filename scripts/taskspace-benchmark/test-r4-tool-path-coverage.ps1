param(
    [string]$ManifestPath = "",
    [string]$EvidencePath = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if ([string]::IsNullOrWhiteSpace($ManifestPath)) {
    $ManifestPath = Join-Path $repoRoot "docs\v0.0.5\build-R4\r4-tool-path-coverage.json"
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
    $EvidencePath = Join-Path $repoRoot "target\r4-tool-path-coverage\r4-tool-path-coverage-evidence.json"
}

function Add-Failure {
    param([System.Collections.Generic.List[string]]$Failures, [string]$Message)
    [void]$Failures.Add($Message)
}

function Test-FileContains {
    param([string]$Path, [string]$Pattern)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $false }
    $text = Get-Content -Raw -Encoding UTF8 -LiteralPath $Path
    return $text.Contains($Pattern)
}

$failures = New-Object System.Collections.Generic.List[string]
if (-not (Test-Path -LiteralPath $ManifestPath -PathType Leaf)) {
    Add-Failure $failures "manifest not found: $ManifestPath"
} else {
    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $ManifestPath | ConvertFrom-Json
    if ([int]$manifest.schema_version -ne 1) {
        Add-Failure $failures "schema_version must be 1"
    }
    $allowedStatuses = @($manifest.allowed_statuses | ForEach-Object { [string]$_ })
    $paths = @($manifest.paths)
    if ($paths.Count -eq 0) {
        Add-Failure $failures "manifest contains no paths"
    }

    $ids = @{}
    foreach ($path in $paths) {
        $id = [string]$path.id
        if ([string]::IsNullOrWhiteSpace($id)) {
            Add-Failure $failures "path id is required"
            continue
        }
        if ($ids.ContainsKey($id)) {
            Add-Failure $failures "duplicate path id: $id"
        }
        $ids[$id] = $true

        $status = [string]$path.status
        if ($allowedStatuses -notcontains $status) {
            Add-Failure $failures "invalid status for ${id}: $status"
        }
        if ($status -eq "unknown" -or [string]::IsNullOrWhiteSpace($status)) {
            Add-Failure $failures "unknown status for $id"
        }

        $ownerPhase = [string]$path.owner_phase
        if ([string]::IsNullOrWhiteSpace($ownerPhase) -or $ownerPhase -eq "unowned") {
            Add-Failure $failures "missing owner_phase for $id"
        }
        if (@("P0", "P1") -contains [string]$path.priority -and $ownerPhase -notmatch "^R4-[A-H]$") {
            Add-Failure $failures "P0/P1 path must have R4 phase owner: $id"
        }

        $anchors = @($path.source_anchors)
        if ($anchors.Count -eq 0) {
            Add-Failure $failures "missing source_anchors for $id"
        }
        foreach ($anchor in $anchors) {
            $sourceFile = Join-Path $repoRoot ([string]$anchor.file)
            $pattern = [string]$anchor.pattern
            if ([string]::IsNullOrWhiteSpace($pattern)) {
                Add-Failure $failures "missing source anchor pattern for $id"
                continue
            }
            if (-not (Test-FileContains $sourceFile $pattern)) {
                Add-Failure $failures "source anchor not found for ${id}: $($anchor.file) :: $pattern"
            }
        }

        if (@($path.required_semantics).Count -eq 0) {
            Add-Failure $failures "required_semantics missing for $id"
        }
        if ([string]::IsNullOrWhiteSpace([string]$path.required_evidence)) {
            Add-Failure $failures "required_evidence missing for $id"
        }
        if ($status -eq "canonical" -and [string]::IsNullOrWhiteSpace([string]$path.coverage_test)) {
            Add-Failure $failures "canonical path missing coverage_test: $id"
        }
        if ($status -eq "intentionally-excluded") {
            if ([string]::IsNullOrWhiteSpace([string]$path.exclusion_rationale)) {
                Add-Failure $failures "intentionally-excluded path missing exclusion_rationale: $id"
            }
            if ([string]::IsNullOrWhiteSpace([string]$path.exclusion_test)) {
                Add-Failure $failures "intentionally-excluded path missing exclusion_test: $id"
            }
        }
    }
}

$evidence = [ordered]@{
    schema_version = 1
    artifact = "r4-tool-path-coverage-evidence"
    generated_at = (Get-Date).ToString("o")
    repo_root = $repoRoot
    manifest_path = [System.IO.Path]::GetFullPath($ManifestPath)
    manifest_sha256 = if (Test-Path -LiteralPath $ManifestPath -PathType Leaf) {
        (Get-FileHash -LiteralPath $ManifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
    } else { "" }
    status = if ($failures.Count -eq 0) { "pass" } else { "fail" }
    path_count = if ($manifest) { @($manifest.paths).Count } else { 0 }
    canonical_count = if ($manifest) { @($manifest.paths | Where-Object { [string]$_.status -eq "canonical" }).Count } else { 0 }
    needs_fix_count = if ($manifest) { @($manifest.paths | Where-Object { [string]$_.status -eq "needs-fix" }).Count } else { 0 }
    failure_count = $failures.Count
    failures = @($failures.ToArray())
}
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $EvidencePath) | Out-Null
[pscustomobject]$evidence | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $EvidencePath -Encoding UTF8

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Host "R4 tool path coverage gate passed: $($evidence.path_count) paths"
