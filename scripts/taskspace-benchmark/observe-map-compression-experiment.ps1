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

function Get-Sum {
    param([double[]]$Values)
    if ($Values.Count -eq 0) { return $null }
    [double](($Values | Measure-Object -Sum).Sum)
}

function Get-Mean {
    param([double[]]$Values)
    if ($Values.Count -eq 0) { return $null }
    [math]::Round([double](($Values | Measure-Object -Average).Average), 2)
}

function Get-Ratio {
    param($Numerator, $Denominator)
    if ($null -eq $Numerator -or $null -eq $Denominator -or [double]$Denominator -eq 0) { return $null }
    [math]::Round(([double]$Numerator / [double]$Denominator), 4)
}

function Convert-TraceTags {
    param($Tags)
    $result = @{}
    foreach ($tag in @($Tags)) {
        $text = [string]$tag
        $separator = $text.IndexOf(":")
        if ($separator -lt 1) { continue }
        $result[$text.Substring(0, $separator)] = $text.Substring($separator + 1)
    }
    $result
}

function Get-OptionalTagDouble {
    param($Tags, [string]$Name)
    if (-not $Tags.ContainsKey($Name)) { return $null }
    [double]$Tags[$Name]
}

function Get-CompressionTraceMetrics {
    param([string]$RolloutPath)
    if ([string]::IsNullOrWhiteSpace($RolloutPath) -or -not (Test-Path -LiteralPath $RolloutPath)) {
        return [pscustomobject]@{ availability = "unavailable"; strategy_ids = @(); evaluation_count = $null; activation_count = $null; b0_median = $null; before_median = $null; after_median = $null; activated_ratio = $null; folded_node_count_median = $null; expanded_node_count_median = $null; eligible_node_count_median = $null; recoverable_hidden_event_count_median = $null; folded_hidden_event_count_median = $null; node_detail_before_median = $null; node_detail_after_median = $null; skeleton_before_median = $null; skeleton_after_median = $null; covered_node_count_median = $null; archive_payload_bytes_median = $null }
    }
    $events = [System.Collections.Generic.List[object]]::new()
    foreach ($line in [System.IO.File]::ReadLines($RolloutPath)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $record = $line | ConvertFrom-Json -Depth 30 } catch { continue }
        $payload = $record.payload
        if ([string]$record.type -ne "event_msg" -or [string]$payload.type -ne "map_runtime" -or
            [string]$payload.map_event_type -ne "taskspace_trace_event_recorded" -or
            [string]$payload.kind -ne "projection_budget") { continue }
        $tags = Convert-TraceTags $payload.tags
        $strategyId = [string]$tags.strategy_id
        if ([string]::IsNullOrWhiteSpace($strategyId) -or $strategyId -eq "none") { continue }
        $events.Add([pscustomobject]@{
            strategy_id = $strategyId
            activation = Get-OptionalTagDouble $tags "strategy_activation_count"
            b0 = Get-OptionalTagDouble $tags "b0_projection_bytes"
            before = Get-OptionalTagDouble $tags "projection_bytes_before_strategy"
            after = Get-OptionalTagDouble $tags "projection_bytes_after_strategy"
            folded = Get-OptionalTagDouble $tags "folded_node_count"
            expanded = Get-OptionalTagDouble $tags "expanded_node_count"
            eligible = Get-OptionalTagDouble $tags "fold_eligible_node_count"
            recoverable_hidden = Get-OptionalTagDouble $tags "recoverable_hidden_event_count"
            folded_hidden = Get-OptionalTagDouble $tags "folded_hidden_event_count"
            node_detail_before = Get-OptionalTagDouble $tags "node_detail_bytes_before_strategy"
            node_detail_after = Get-OptionalTagDouble $tags "node_detail_bytes_after_strategy"
            skeleton_before = Get-OptionalTagDouble $tags "skeleton_bytes_before_strategy"
            skeleton_after = Get-OptionalTagDouble $tags "skeleton_bytes_after_strategy"
            covered = Get-OptionalTagDouble $tags "covered_node_count"
            archive_payload_bytes = Get-OptionalTagDouble $tags "archive_payload_bytes"
        })
    }
    if ($events.Count -eq 0) {
        return [pscustomobject]@{ availability = "trace_absent"; strategy_ids = @(); evaluation_count = 0; activation_count = $null; b0_median = $null; before_median = $null; after_median = $null; activated_ratio = $null; folded_node_count_median = $null; expanded_node_count_median = $null; eligible_node_count_median = $null; recoverable_hidden_event_count_median = $null; folded_hidden_event_count_median = $null; node_detail_before_median = $null; node_detail_after_median = $null; skeleton_before_median = $null; skeleton_after_median = $null; covered_node_count_median = $null; archive_payload_bytes_median = $null }
    }
    $activated = @($events | Where-Object { $_.activation -gt 0 })
    $before = Get-Median @($activated.before | Where-Object { $null -ne $_ })
    $after = Get-Median @($activated.after | Where-Object { $null -ne $_ })
    [pscustomobject]@{
        availability = "rollout_trace"
        strategy_ids = @($events.strategy_id | Sort-Object -Unique)
        evaluation_count = $events.Count
        activation_count = [double](($events | Measure-Object activation -Sum).Sum)
        b0_median = Get-Median @($activated.b0 | Where-Object { $null -ne $_ })
        before_median = $before
        after_median = $after
        activated_ratio = Get-Ratio $after $before
        folded_node_count_median = Get-Median @($activated.folded | Where-Object { $null -ne $_ })
        expanded_node_count_median = Get-Median @($activated.expanded | Where-Object { $null -ne $_ })
        eligible_node_count_median = Get-Median @($activated.eligible | Where-Object { $null -ne $_ })
        recoverable_hidden_event_count_median = Get-Median @($activated.recoverable_hidden | Where-Object { $null -ne $_ })
        folded_hidden_event_count_median = Get-Median @($activated.folded_hidden | Where-Object { $null -ne $_ })
        node_detail_before_median = Get-Median @($activated.node_detail_before | Where-Object { $null -ne $_ })
        node_detail_after_median = Get-Median @($activated.node_detail_after | Where-Object { $null -ne $_ })
        skeleton_before_median = Get-Median @($activated.skeleton_before | Where-Object { $null -ne $_ })
        skeleton_after_median = Get-Median @($activated.skeleton_after | Where-Object { $null -ne $_ })
        covered_node_count_median = Get-Median @($activated.covered | Where-Object { $null -ne $_ })
        archive_payload_bytes_median = Get-Median @($activated.archive_payload_bytes | Where-Object { $null -ne $_ })
    }
}

$rows = [System.Collections.Generic.List[object]]::new()
foreach ($entry in @($index.results)) {
    if ([string]::IsNullOrWhiteSpace([string]$entry.metrics_path) -or -not (Test-Path -LiteralPath $entry.metrics_path)) {
        throw "metrics missing for $($entry.sample_class)/$($entry.repeat)/$($entry.arm)"
    }
    $metrics = Get-Content -Raw -Encoding UTF8 -LiteralPath $entry.metrics_path | ConvertFrom-Json
    $compression = Get-CompressionTraceMetrics ([string]$metrics.rollout_path)
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
        uncached_input_tokens = [double]$metrics.uncached_input_tokens
        cache_input_ratio = Get-Ratio ([double]$metrics.cached_input_tokens) ([double]$metrics.input_tokens)
        output_tokens = [double]$metrics.output_tokens
        wall_time_ms = [double]$metrics.wall_time_ms
        projection_tokens_max = [double]$metrics.projection_tokens_max
        nodes = [double]$metrics.nodes
        edges = [double]$metrics.edges
        compression_trace_availability = $compression.availability
        strategy_ids = @($compression.strategy_ids)
        strategy_evaluation_count = $compression.evaluation_count
        strategy_activation_count = $compression.activation_count
        b0_projection_bytes_median = $compression.b0_median
        activated_projection_before_median = $compression.before_median
        activated_projection_after_median = $compression.after_median
        activated_projection_ratio = $compression.activated_ratio
        folded_node_count_median = $compression.folded_node_count_median
        expanded_node_count_median = $compression.expanded_node_count_median
        eligible_node_count_median = $compression.eligible_node_count_median
        recoverable_hidden_event_count_median = $compression.recoverable_hidden_event_count_median
        folded_hidden_event_count_median = $compression.folded_hidden_event_count_median
        node_detail_before_median = $compression.node_detail_before_median
        node_detail_after_median = $compression.node_detail_after_median
        skeleton_before_median = $compression.skeleton_before_median
        skeleton_after_median = $compression.skeleton_after_median
        covered_node_count_median = $compression.covered_node_count_median
        archive_payload_bytes_median = $compression.archive_payload_bytes_median
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
            requests_sum = Get-Sum @($group.requests)
            requests_mean = Get-Mean @($group.requests)
            requests_median = Get-Median @($group.requests)
            input_tokens_sum = Get-Sum @($group.input_tokens)
            input_tokens_mean = Get-Mean @($group.input_tokens)
            input_tokens_median = Get-Median @($group.input_tokens)
            cached_input_tokens_sum = Get-Sum @($group.cached_input_tokens)
            cached_input_tokens_mean = Get-Mean @($group.cached_input_tokens)
            cached_input_tokens_median = Get-Median @($group.cached_input_tokens)
            uncached_input_tokens_sum = Get-Sum @($group.uncached_input_tokens)
            uncached_input_tokens_mean = Get-Mean @($group.uncached_input_tokens)
            uncached_input_tokens_median = Get-Median @($group.uncached_input_tokens)
            cache_input_ratio_total = Get-Ratio (Get-Sum @($group.cached_input_tokens)) (Get-Sum @($group.input_tokens))
            cache_input_ratio_mean = Get-Mean @($group.cache_input_ratio)
            cache_input_ratio_median = Get-Median @($group.cache_input_ratio)
            output_tokens_sum = Get-Sum @($group.output_tokens)
            output_tokens_mean = Get-Mean @($group.output_tokens)
            output_tokens_median = Get-Median @($group.output_tokens)
            wall_time_ms_sum = Get-Sum @($group.wall_time_ms)
            wall_time_ms_mean = Get-Mean @($group.wall_time_ms)
            wall_time_ms_median = Get-Median @($group.wall_time_ms)
            projection_tokens_max_median = Get-Median @($group.projection_tokens_max)
            nodes_median = Get-Median @($group.nodes)
            edges_median = Get-Median @($group.edges)
            strategy_activation_count_median = Get-Median @($group.strategy_activation_count | Where-Object { $null -ne $_ })
            b0_projection_bytes_median = Get-Median @($group.b0_projection_bytes_median | Where-Object { $null -ne $_ })
            activated_projection_before_median = Get-Median @($group.activated_projection_before_median | Where-Object { $null -ne $_ })
            activated_projection_after_median = Get-Median @($group.activated_projection_after_median | Where-Object { $null -ne $_ })
            activated_projection_ratio_median = Get-Median @($group.activated_projection_ratio | Where-Object { $null -ne $_ })
            folded_node_count_median = Get-Median @($group.folded_node_count_median | Where-Object { $null -ne $_ })
            expanded_node_count_median = Get-Median @($group.expanded_node_count_median | Where-Object { $null -ne $_ })
            eligible_node_count_median = Get-Median @($group.eligible_node_count_median | Where-Object { $null -ne $_ })
            recoverable_hidden_event_count_median = Get-Median @($group.recoverable_hidden_event_count_median | Where-Object { $null -ne $_ })
            folded_hidden_event_count_median = Get-Median @($group.folded_hidden_event_count_median | Where-Object { $null -ne $_ })
            node_detail_before_median = Get-Median @($group.node_detail_before_median | Where-Object { $null -ne $_ })
            node_detail_after_median = Get-Median @($group.node_detail_after_median | Where-Object { $null -ne $_ })
            skeleton_before_median = Get-Median @($group.skeleton_before_median | Where-Object { $null -ne $_ })
            skeleton_after_median = Get-Median @($group.skeleton_after_median | Where-Object { $null -ne $_ })
            covered_node_count_median = Get-Median @($group.covered_node_count_median | Where-Object { $null -ne $_ })
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
    schema_version = "taskspace-map-compression-observation-v2"
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
$lines.Add("| Sample | Arm | Success | Requests sum/mean/P50 | Input sum/mean/P50 | Cached sum/mean/P50 | Cache total/mean/P50 | Wall ms sum/mean/P50 | Projection max P50 |")
$lines.Add("|---|---|---:|---:|---:|---:|---:|---:|---:|")
foreach ($row in $aggregates) {
    $lines.Add("| $($row.sample_class) | $($row.arm) | $($row.success_count)/$($row.runs) | $($row.requests_sum)/$($row.requests_mean)/$($row.requests_median) | $($row.input_tokens_sum)/$($row.input_tokens_mean)/$($row.input_tokens_median) | $($row.cached_input_tokens_sum)/$($row.cached_input_tokens_mean)/$($row.cached_input_tokens_median) | $($row.cache_input_ratio_total)/$($row.cache_input_ratio_mean)/$($row.cache_input_ratio_median) | $($row.wall_time_ms_sum)/$($row.wall_time_ms_mean)/$($row.wall_time_ms_median) | $($row.projection_tokens_max_median) |")
}
$lines.Add("")
$lines.Add("| Sample | Arm | Activation P50 | Projection B0/full/after/ratio | Folded/expanded/eligible nodes P50 | Hidden recoverable/folded | Detail before/after | Skeleton before/after |")
$lines.Add("|---|---|---:|---:|---:|---:|---:|---:|")
foreach ($row in $aggregates) {
    $lines.Add("| $($row.sample_class) | $($row.arm) | $($row.strategy_activation_count_median) | $($row.b0_projection_bytes_median)/$($row.activated_projection_before_median)/$($row.activated_projection_after_median)/$($row.activated_projection_ratio_median) | $($row.folded_node_count_median)/$($row.expanded_node_count_median)/$($row.eligible_node_count_median) | $($row.recoverable_hidden_event_count_median)/$($row.folded_hidden_event_count_median) | $($row.node_detail_before_median)/$($row.node_detail_after_median) | $($row.skeleton_before_median)/$($row.skeleton_after_median) |")
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
