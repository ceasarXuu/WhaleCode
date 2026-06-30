param(
    [string]$PlanPath = "",
    [string]$ReportPath = "",
    [string]$EvidencePath = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if ([string]::IsNullOrWhiteSpace($PlanPath)) {
    $PlanPath = Join-Path $repoRoot "docs\v0.0.5\build-R4\r4-public-10-tool-stress-plan.json"
}
if ([string]::IsNullOrWhiteSpace($EvidencePath)) {
    $EvidencePath = Join-Path $repoRoot "target\r4-public-10-tool-stress\r4-public-10-tool-stress-evidence.json"
}

function Add-Failure {
    param([System.Collections.Generic.List[string]]$Failures, [string]$Message)
    [void]$Failures.Add($Message)
}

function Get-StringArray {
    param([object]$Value)
    @($Value | ForEach-Object { [string]$_ })
}

function Test-TruthValue {
    param([object]$Value)
    if ($null -eq $Value) { return $false }
    if ($Value -is [bool]) { return [bool]$Value }
    return [string]$Value -eq "true"
}

$knownRegistryTaskIds = @(
    "build-linux-kernel-qemu",
    "qemu-alpine-ssh",
    "qemu-startup",
    "git-multibranch",
    "git-workflow-hack",
    "sanitize-git-repo",
    "sqlite-with-gcov",
    "processing-pipeline",
    "csv-to-parquet",
    "tmux-advanced-workflow"
)

$failures = New-Object System.Collections.Generic.List[string]
$plan = $null
if (-not (Test-Path -LiteralPath $PlanPath -PathType Leaf)) {
    Add-Failure $failures "plan not found: $PlanPath"
} else {
    $plan = Get-Content -Raw -Encoding UTF8 -LiteralPath $PlanPath | ConvertFrom-Json
    if ([int]$plan.schema_version -ne 1) {
        Add-Failure $failures "schema_version must be 1"
    }
    if ([string]$plan.public_source.benchmark -ne "terminal-bench-core") {
        Add-Failure $failures "public_source.benchmark must be terminal-bench-core"
    }
    if ([string]$plan.public_source.version -ne "0.1.1") {
        Add-Failure $failures "public_source.version must be 0.1.1"
    }
    if ([string]$plan.public_source.commit -ne "91e10457b5410f16c44364da1a34cb6de8c488a5") {
        Add-Failure $failures "public_source.commit is not the pinned Terminal-Bench core commit"
    }
    foreach ($field in @("registry_url", "source_url", "branch", "dataset_path")) {
        if ([string]::IsNullOrWhiteSpace([string]$plan.public_source.$field)) {
            Add-Failure $failures "public_source.$field is required"
        }
    }

    $samples = @($plan.samples)
    if ($samples.Count -ne 10) {
        Add-Failure $failures "plan must contain exactly 10 public samples"
    }
    $seen = @{}
    foreach ($sample in $samples) {
        $taskId = [string]$sample.task_id
        if ([string]::IsNullOrWhiteSpace($taskId)) {
            Add-Failure $failures "sample task_id is required"
            continue
        }
        if ($seen.ContainsKey($taskId)) {
            Add-Failure $failures "duplicate task_id: $taskId"
        }
        $seen[$taskId] = $true
        if ($knownRegistryTaskIds -notcontains $taskId) {
            Add-Failure $failures "task_id is not in the pinned R4 Terminal-Bench public-10 registry subset: $taskId"
        }
        if ([string]::IsNullOrWhiteSpace([string]$sample.tool_stress_focus)) {
            Add-Failure $failures "tool_stress_focus missing for $taskId"
        }
        if ([string]::IsNullOrWhiteSpace([string]$sample.why_selected)) {
            Add-Failure $failures "why_selected missing for $taskId"
        }
    }

    $requiredFields = Get-StringArray $plan.required_report_fields
    foreach ($field in @(
        "task_id",
        "standard_outcome",
        "taskspace_outcome",
        "taskspace_wall_time_ratio",
        "taskspace_token_ratio",
        "request_2_plus_cache_hit_rate",
        "tool_feedback_loss_count",
        "tool_feedback_semantic_loss_count",
        "taskspace_map_attribution_missing_count",
        "tool_call_analysis_summary",
        "evidence_paths"
    )) {
        if ($requiredFields -notcontains $field) {
            Add-Failure $failures "required_report_fields missing: $field"
        }
    }
    if (@($plan.tool_analysis_questions).Count -lt 7) {
        Add-Failure $failures "tool_analysis_questions must cover the seven R4 analysis questions"
    }
}

$reportRows = @()
if (-not [string]::IsNullOrWhiteSpace($ReportPath)) {
    if (-not (Test-Path -LiteralPath $ReportPath -PathType Leaf)) {
        Add-Failure $failures "report not found: $ReportPath"
    } else {
        $report = Get-Content -Raw -Encoding UTF8 -LiteralPath $ReportPath | ConvertFrom-Json
        $reportRows = @($report.rows)
        if ($reportRows.Count -ne 10) {
            Add-Failure $failures "report must contain exactly 10 rows"
        }
        $requiredFields = if ($plan) { Get-StringArray $plan.required_report_fields } else { @() }
        foreach ($row in $reportRows) {
            $taskId = [string]$row.task_id
            foreach ($field in $requiredFields) {
                if (-not ($row.PSObject.Properties.Name -contains $field)) {
                    Add-Failure $failures "report row ${taskId} missing field: $field"
                }
            }
            if (-not (Test-TruthValue $row.task_id_registry_verified)) {
                Add-Failure $failures "report row ${taskId} must set task_id_registry_verified=true"
            }
        }
    }
}

$evidence = [ordered]@{
    schema_version = 1
    artifact = "r4-public-10-tool-stress-evidence"
    generated_at = (Get-Date).ToString("o")
    repo_root = $repoRoot
    plan_path = [System.IO.Path]::GetFullPath($PlanPath)
    plan_sha256 = if (Test-Path -LiteralPath $PlanPath -PathType Leaf) {
        (Get-FileHash -LiteralPath $PlanPath -Algorithm SHA256).Hash.ToLowerInvariant()
    } else { "" }
    report_path = if ([string]::IsNullOrWhiteSpace($ReportPath)) { "" } else { [System.IO.Path]::GetFullPath($ReportPath) }
    status = if ($failures.Count -eq 0) { "pass" } else { "fail" }
    sample_count = if ($plan) { @($plan.samples).Count } else { 0 }
    report_row_count = $reportRows.Count
    failure_count = $failures.Count
    failures = @($failures.ToArray())
}
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $EvidencePath) | Out-Null
[pscustomobject]$evidence | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $EvidencePath -Encoding UTF8

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Host "R4 public-10 tool-stress gate passed: $($evidence.sample_count) planned samples"
