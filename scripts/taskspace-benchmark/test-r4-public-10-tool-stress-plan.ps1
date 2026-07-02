param(
    [string]$PlanPath = "",
    [string]$ReportPath = "",
    [string]$EvidencePath = "",
    [int]$RegistryTimeoutSeconds = 30,
    [switch]$SkipLiveRegistryCheck
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

function Get-Sha256String {
    param([string]$Value)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Value)
        $hash = $sha.ComputeHash($bytes)
        return (($hash | ForEach-Object { $_.ToString("x2") }) -join "")
    } finally {
        $sha.Dispose()
    }
}

$failures = New-Object System.Collections.Generic.List[string]
$plan = $null
$registryTaskIds = @()
$registryVerified = $false
$registryEntryCommit = ""
$registryEntryTaskCount = 0
$registryTaskSubsetSha256 = ""
$registryError = ""
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

    if ($SkipLiveRegistryCheck) {
        Add-Failure $failures "live public registry verification is required for R4 public-10 gate"
    } else {
        try {
            $registry = Invoke-RestMethod `
                -Uri ([string]$plan.public_source.registry_url) `
                -TimeoutSec $RegistryTimeoutSeconds
            $entry = @($registry | Where-Object {
                    [string]$_.name -eq [string]$plan.public_source.benchmark -and
                    [string]$_.version -eq [string]$plan.public_source.version
                })[0]
            if ($null -eq $entry) {
                Add-Failure $failures "public registry entry not found for $($plan.public_source.benchmark) $($plan.public_source.version)"
            } else {
                $registryEntryCommit = [string]$entry.commit_hash
                $registryTaskIds = Get-StringArray $entry.task_id_subset
                $registryEntryTaskCount = @($registryTaskIds).Count
                $registryTaskSubsetSha256 = Get-Sha256String (($registryTaskIds | Sort-Object) -join "`n")
                $registryVerified = $true
                if ([string]$entry.github_url -ne [string]$plan.public_source.source_url) {
                    Add-Failure $failures "public_source.source_url does not match registry github_url"
                }
                if ([string]$entry.branch -ne [string]$plan.public_source.branch) {
                    Add-Failure $failures "public_source.branch does not match registry branch"
                }
                if ([string]$entry.dataset_path -ne [string]$plan.public_source.dataset_path) {
                    Add-Failure $failures "public_source.dataset_path does not match registry dataset_path"
                }
                if ($registryEntryCommit -ne [string]$plan.public_source.commit) {
                    Add-Failure $failures "public_source.commit does not match registry commit_hash"
                }
                if ($registryEntryTaskCount -lt 10) {
                    Add-Failure $failures "public registry task_id_subset must contain at least 10 tasks"
                }
                if ($null -ne $plan.public_source.registry_subset_count -and
                    [int]$plan.public_source.registry_subset_count -ne $registryEntryTaskCount) {
                    Add-Failure $failures "public_source.registry_subset_count does not match live registry"
                }
                if ($null -ne $plan.public_source.registry_task_id_subset_sha256 -and
                    [string]$plan.public_source.registry_task_id_subset_sha256 -ne $registryTaskSubsetSha256) {
                    Add-Failure $failures "public_source.registry_task_id_subset_sha256 does not match live registry"
                }
            }
        } catch {
            $registryError = $_.Exception.Message
            Add-Failure $failures "failed to verify live public registry: $registryError"
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
        if ($registryTaskIds -notcontains $taskId) {
            Add-Failure $failures "task_id is not in the live Terminal-Bench public registry subset: $taskId"
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
        "standard_token_summary_availability",
        "taskspace_token_summary_availability",
        "standard_usage_accounting_status",
        "taskspace_usage_accounting_status",
        "token_ratio_availability",
        "request_2_plus_cache_hit_rate",
        "request_2_plus_cache_hit_rate_availability",
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
        $plannedTaskIds = @($plan.samples | ForEach-Object { [string]$_.task_id })
        $requiredFields = if ($plan) { Get-StringArray $plan.required_report_fields } else { @() }
        foreach ($row in $reportRows) {
            $taskId = [string]$row.task_id
            if ($plannedTaskIds -notcontains $taskId) {
                Add-Failure $failures "report row task_id was not in planned public-10 sample set: $taskId"
            }
            foreach ($field in $requiredFields) {
                if (-not ($row.PSObject.Properties.Name -contains $field)) {
                    Add-Failure $failures "report row ${taskId} missing field: $field"
                }
            }
            if (-not (Test-TruthValue $row.task_id_registry_verified)) {
                Add-Failure $failures "report row ${taskId} must set task_id_registry_verified=true"
            }
            $tokenAvailability = [string]$row.token_ratio_availability
            $tokenRatioMissing = ($null -eq $row.taskspace_token_ratio -or [string]::IsNullOrWhiteSpace([string]$row.taskspace_token_ratio))
            if ($tokenAvailability -eq "measured" -and $tokenRatioMissing) {
                Add-Failure $failures "report row ${taskId} token_ratio_availability=measured but taskspace_token_ratio is missing"
            }
            if ($tokenAvailability -ne "measured" -and -not $tokenRatioMissing) {
                Add-Failure $failures "report row ${taskId} has taskspace_token_ratio but token_ratio_availability is not measured"
            }
            foreach ($mode in @("standard", "taskspace")) {
                $statusName = "${mode}_usage_accounting_status"
                $availabilityName = "${mode}_token_summary_availability"
                $inputName = "${mode}_input_tokens"
                $outputName = "${mode}_output_tokens"
                $status = [string]$row.$statusName
                $availability = [string]$row.$availabilityName
                if ([string]::IsNullOrWhiteSpace($status)) {
                    Add-Failure $failures "report row ${taskId} missing ${statusName}"
                }
                if ([string]::IsNullOrWhiteSpace($availability)) {
                    Add-Failure $failures "report row ${taskId} missing ${availabilityName}"
                }
                $inputMissing = ($null -eq $row.$inputName -or [string]::IsNullOrWhiteSpace([string]$row.$inputName))
                $outputMissing = ($null -eq $row.$outputName -or [string]::IsNullOrWhiteSpace([string]$row.$outputName))
                if (($inputMissing -or $outputMissing) -and $status -eq "measured") {
                    Add-Failure $failures "report row ${taskId} ${mode} usage_accounting_status=measured but token fields are missing"
                }
            }
            $cacheAvailability = [string]$row.request_2_plus_cache_hit_rate_availability
            $cacheMissing = ($null -eq $row.request_2_plus_cache_hit_rate -or [string]::IsNullOrWhiteSpace([string]$row.request_2_plus_cache_hit_rate))
            if ($cacheAvailability -in @("measured", "derived_from_token_summary") -and $cacheMissing) {
                Add-Failure $failures "report row ${taskId} cache availability is $cacheAvailability but request_2_plus_cache_hit_rate is missing"
            }
            if ($cacheAvailability -notin @("measured", "derived_from_token_summary", "cache_trace_unavailable", "source_missing", "missing_run")) {
                Add-Failure $failures "report row ${taskId} has unknown request_2_plus_cache_hit_rate_availability: $cacheAvailability"
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
    registry_url = if ($plan) { [string]$plan.public_source.registry_url } else { "" }
    live_registry_checked = -not [bool]$SkipLiveRegistryCheck
    registry_verified = $registryVerified
    registry_entry_commit_hash = $registryEntryCommit
    registry_task_count = $registryEntryTaskCount
    registry_task_id_subset_sha256 = $registryTaskSubsetSha256
    registry_error = $registryError
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
