$ErrorActionPreference = "Stop"
if (-not (Get-Command Get-TaskspaceFileSha256 -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "e3-identity.ps1")
}

function Read-TaskspaceCalibrationTaskList {
    param([Parameter(Mandatory = $true)][string]$TaskListPath)
    $raw = Get-Content -Raw -Encoding UTF8 -LiteralPath $TaskListPath
    if ($raw.TrimStart().StartsWith("[")) { return @($raw | ConvertFrom-Json) }
    @(Get-Content -Encoding UTF8 -LiteralPath $TaskListPath |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
        ForEach-Object { $_ | ConvertFrom-Json })
}

function Get-TaskspaceCalibrationTaskFamily {
    param($Task, [string]$Benchmark = "")
    foreach ($field in @("task_family", "family", "category", "scenario_family")) {
        if ($Task.PSObject.Properties.Name -contains $field -and
            -not [string]::IsNullOrWhiteSpace([string]$Task.$field)) {
            return [string]$Task.$field
        }
    }
    if ($Task.PSObject.Properties.Name -contains "task_dir" -and
        -not [string]::IsNullOrWhiteSpace([string]$Task.task_dir)) {
        $parent = Split-Path -Parent ([string]$Task.task_dir)
        if (-not [string]::IsNullOrWhiteSpace($parent)) { return Split-Path -Leaf $parent }
    }
    if (-not [string]::IsNullOrWhiteSpace($Benchmark)) { return $Benchmark }
    "unknown"
}

function Get-TaskspaceCalibrationTaskId {
    param($Task, [int]$Index)
    foreach ($field in @("sample_id", "task_id", "id", "name")) {
        if ($Task.PSObject.Properties.Name -contains $field -and
            -not [string]::IsNullOrWhiteSpace([string]$Task.$field)) {
            return [string]$Task.$field
        }
    }
    "task-{0:000}" -f ($Index + 1)
}

function New-TaskspaceCalibrationSelection {
    param(
        [Parameter(Mandatory = $true)][string]$TaskListPath,
        [Parameter(Mandatory = $true)][string]$OutputPath,
        [string]$Benchmark = "",
        [int]$SelectionCount = 3
    )
    if ($SelectionCount -lt 1) { throw "SelectionCount must be >= 1." }
    $tasks = @(Read-TaskspaceCalibrationTaskList $TaskListPath)
    if ($tasks.Count -eq 0) { throw "TaskListPath contains no samples." }
    $rows = for ($i = 0; $i -lt $tasks.Count; $i++) {
        $task = $tasks[$i]
        [pscustomobject]@{
            index = $i
            sample_id = Get-TaskspaceCalibrationTaskId $task $i
            task_dir = if ($task.PSObject.Properties.Name -contains "task_dir") { [string]$task.task_dir } else { "" }
            source_version = if ($task.PSObject.Properties.Name -contains "source_version") { [string]$task.source_version } else { "" }
            task_family = Get-TaskspaceCalibrationTaskFamily $task $Benchmark
        }
    }
    $selected = New-Object System.Collections.Generic.List[object]
    foreach ($family in @($rows | Group-Object task_family | Sort-Object Name)) {
        if ($selected.Count -ge $SelectionCount) { break }
        $selected.Add(@($family.Group | Sort-Object index | Select-Object -First 1)[0])
    }
    foreach ($row in @($rows | Sort-Object index)) {
        if ($selected.Count -ge $SelectionCount) { break }
        if (@($selected | Where-Object { [int]$_.index -eq [int]$row.index }).Count -eq 0) {
            $selected.Add($row)
        }
    }
    $selectedRows = @($selected.ToArray())
    $selectedIndexes = @($selectedRows | ForEach-Object { [int]$_.index })
    $excluded = @($rows | Where-Object { $selectedIndexes -notcontains [int]$_.index } | ForEach-Object {
            [pscustomobject]@{
                sample_id = [string]$_.sample_id
                task_family = [string]$_.task_family
                reason = "not_selected_after_family_coverage_limit"
            }
        })
    $subsetHash = Get-TaskspaceStableJsonHash @($selectedRows | ForEach-Object {
            [ordered]@{
                sample_id = [string]$_.sample_id
                task_dir = [string]$_.task_dir
                source_version = [string]$_.source_version
                task_family = [string]$_.task_family
            }
        })
    $artifact = [ordered]@{
        schema_version = 1
        source_task_list_path = [System.IO.Path]::GetFullPath($TaskListPath)
        source_task_list_hash = Get-TaskspaceFileSha256 $TaskListPath
        subset_task_list_hash = $subsetHash
        selection_count = $selectedRows.Count
        requested_selection_count = $SelectionCount
        deterministic_rule = "one_per_task_family_sorted_by_family_then_fill_by_task_order"
        selected_task_ids = @($selectedRows | ForEach-Object { [string]$_.sample_id })
        selected_task_families = @($selectedRows | ForEach-Object { [string]$_.task_family })
        selected_tasks = @($selectedRows)
        excluded_tasks = @($excluded)
        insufficient_family_count = (@($rows | Select-Object -ExpandProperty task_family -Unique).Count -lt $SelectionCount)
        generated_at = (Get-Date).ToString("o")
    }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $OutputPath) | Out-Null
    $artifact | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
    [pscustomobject]$artifact
}
