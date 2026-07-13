param(
    [Parameter(Mandatory = $true)][string]$RunIndexPath,
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"
$RunIndexPath = (Resolve-Path -LiteralPath $RunIndexPath).Path
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir = Split-Path -Parent $RunIndexPath
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$index = Get-Content -Raw -Encoding UTF8 -LiteralPath $RunIndexPath | ConvertFrom-Json

function Get-Median {
    param([double[]]$Values)
    $sorted = @($Values | Sort-Object)
    if ($sorted.Count -eq 0) { return $null }
    $middle = [math]::Floor($sorted.Count / 2)
    if ($sorted.Count % 2 -eq 1) { return [double]$sorted[$middle] }
    ([double]$sorted[$middle - 1] + [double]$sorted[$middle]) / 2.0
}

function Get-Ratio {
    param($Numerator, $Denominator)
    if ($null -eq $Numerator -or $null -eq $Denominator -or [double]$Denominator -eq 0) { return $null }
    [math]::Round(([double]$Numerator / [double]$Denominator), 4)
}

$rows = [System.Collections.Generic.List[object]]::new()
foreach ($entry in @($index.results)) {
    if ([string]::IsNullOrWhiteSpace([string]$entry.metrics_path) -or -not (Test-Path -LiteralPath $entry.metrics_path)) {
        throw "metrics missing for $($entry.sample_class)/$($entry.repeat)/$($entry.arm)"
    }
    $metrics = Get-Content -Raw -Encoding UTF8 -LiteralPath $entry.metrics_path | ConvertFrom-Json
    $rows.Add([pscustomobject]@{
        sample_class = [string]$entry.sample_class
        scenario = [string]$entry.scenario
        repeat = [int]$entry.repeat
        arm = [string]$entry.arm
        logical_mode = [string]$entry.logical_mode
        business_success = [bool]$metrics.business_success
        agent_completion_status = [string]$metrics.agent_completion_status
        external_validation_status = [string]$metrics.external_validation_status
        requests = [double]$metrics.model_request_count
        input_tokens = [double]$metrics.input_tokens
        cached_input_tokens = [double]$metrics.cached_input_tokens
        output_tokens = [double]$metrics.output_tokens
        wall_time_ms = [double]$metrics.wall_time_ms
        projection_tokens_max = [double]$metrics.projection_tokens_max
        nodes = [double]$metrics.nodes
        edges = [double]$metrics.edges
        metrics_path = [string]$entry.metrics_path
    })
}

$aggregates = [System.Collections.Generic.List[object]]::new()
foreach ($sampleClass in @($rows.sample_class | Sort-Object -Unique)) {
    foreach ($arm in @($rows.arm | Sort-Object -Unique)) {
        $group = @($rows | Where-Object { $_.sample_class -eq $sampleClass -and $_.arm -eq $arm })
        if ($group.Count -eq 0) { continue }
        $aggregates.Add([pscustomobject]@{
            sample_class = $sampleClass
            arm = $arm
            runs = $group.Count
            success_count = @($group | Where-Object { $_.business_success }).Count
            requests_median = Get-Median @($group.requests)
            input_tokens_median = Get-Median @($group.input_tokens)
            cached_input_tokens_median = Get-Median @($group.cached_input_tokens)
            output_tokens_median = Get-Median @($group.output_tokens)
            wall_time_ms_median = Get-Median @($group.wall_time_ms)
            projection_tokens_max_median = Get-Median @($group.projection_tokens_max)
            nodes_median = Get-Median @($group.nodes)
            edges_median = Get-Median @($group.edges)
        })
    }
}

$comparisons = [System.Collections.Generic.List[object]]::new()
foreach ($sampleClass in @($rows.sample_class | Sort-Object -Unique)) {
    $candidate = $aggregates | Where-Object { $_.sample_class -eq $sampleClass -and $_.arm -eq "C" } | Select-Object -First 1
    foreach ($baselineArm in @("B0", "STD")) {
        $baseline = $aggregates | Where-Object { $_.sample_class -eq $sampleClass -and $_.arm -eq $baselineArm } | Select-Object -First 1
        if ($null -eq $candidate -or $null -eq $baseline) { continue }
        $comparisons.Add([pscustomobject]@{
            sample_class = $sampleClass
            candidate = "C"
            baseline = $baselineArm
            requests_ratio = Get-Ratio $candidate.requests_median $baseline.requests_median
            input_tokens_ratio = Get-Ratio $candidate.input_tokens_median $baseline.input_tokens_median
            cached_input_tokens_ratio = Get-Ratio $candidate.cached_input_tokens_median $baseline.cached_input_tokens_median
            wall_time_ratio = Get-Ratio $candidate.wall_time_ms_median $baseline.wall_time_ms_median
            projection_tokens_max_ratio = Get-Ratio $candidate.projection_tokens_max_median $baseline.projection_tokens_max_median
        })
    }
}

$observation = [ordered]@{
    schema_version = "taskspace-map-compression-observation-v1"
    phase = [string]$index.phase
    run_index_path = $RunIndexPath
    p0_alias_of = [string]$index.p0_alias_of
    rows = @($rows)
    aggregates = @($aggregates)
    comparisons = @($comparisons)
}
$jsonPath = Join-Path $OutputDir "map-compression-observation.json"
$observation | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $jsonPath -Encoding UTF8

$lines = [System.Collections.Generic.List[string]]::new()
$lines.Add("# Map Compression Observation")
$lines.Add("")
$lines.Add("- Phase: $($index.phase)")
$lines.Add("- P0: alias of B0")
$lines.Add("")
$lines.Add("| Sample | Arm | Success | Requests median | Input median | Cached median | Wall ms median | Projection max median |")
$lines.Add("|---|---|---:|---:|---:|---:|---:|---:|")
foreach ($row in $aggregates) {
    $lines.Add("| $($row.sample_class) | $($row.arm) | $($row.success_count)/$($row.runs) | $($row.requests_median) | $($row.input_tokens_median) | $($row.cached_input_tokens_median) | $($row.wall_time_ms_median) | $($row.projection_tokens_max_median) |")
}
$lines.Add("")
$lines.Add("| Sample | Comparison | Requests ratio | Input ratio | Cached ratio | Wall ratio | Projection ratio |")
$lines.Add("|---|---|---:|---:|---:|---:|---:|")
foreach ($row in $comparisons) {
    $lines.Add("| $($row.sample_class) | C/$($row.baseline) | $($row.requests_ratio) | $($row.input_tokens_ratio) | $($row.cached_input_tokens_ratio) | $($row.wall_time_ratio) | $($row.projection_tokens_max_ratio) |")
}
$mdPath = Join-Path $OutputDir "map-compression-observation.md"
$lines | Set-Content -LiteralPath $mdPath -Encoding UTF8
Write-Host "MapCompressionObservationJson: $jsonPath"
Write-Host "MapCompressionObservationMarkdown: $mdPath"
