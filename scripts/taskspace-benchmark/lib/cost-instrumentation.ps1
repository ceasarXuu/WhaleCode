function Add-TaskspaceCostCount {
    param([hashtable]$Table, [string]$Key)
    if ([string]::IsNullOrWhiteSpace($Key)) { $Key = "unknown" }
    if (-not $Table.ContainsKey($Key)) { $Table[$Key] = 0 }
    $Table[$Key]++
}

function Convert-TaskspaceCostTable {
    param([hashtable]$Table)
    $ordered = [ordered]@{}
    foreach ($key in @($Table.Keys | Sort-Object)) { $ordered[$key] = $Table[$key] }
    [pscustomobject]$ordered
}

function Get-TaskspaceCostJsonlRows {
    param([string]$Path)
    $rows = New-Object System.Collections.Generic.List[object]
    $parseErrors = 0
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path)) {
        return [pscustomobject]@{ rows = @(); parse_errors = 0; source_status = "missing" }
    }
    foreach ($line in @(Get-Content -Encoding UTF8 -LiteralPath $Path)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try {
            $rows.Add(($line | ConvertFrom-Json))
        } catch {
            $parseErrors++
        }
    }
    [pscustomobject]@{ rows = @($rows.ToArray()); parse_errors = $parseErrors; source_status = "read" }
}

function Get-TaskspaceCostProperty {
    param($Value, [string[]]$Names)
    if ($null -eq $Value) { return $null }
    foreach ($name in $Names) {
        if ($Value.PSObject.Properties.Name -contains $name) { return $Value.$name }
    }
    $null
}

function Get-TaskspaceTokenUsageObjects {
    param($Value)
    $found = New-Object System.Collections.Generic.List[object]
    function Visit-UsageValue($Current) {
        if ($null -eq $Current) { return }
        if ($Current -is [string] -or $Current -is [ValueType]) { return }
        $names = @($Current.PSObject.Properties.Name)
        $hasInput = @("input_tokens", "prompt_tokens") | Where-Object { $names -contains $_ }
        $hasOutput = @("output_tokens", "completion_tokens") | Where-Object { $names -contains $_ }
        if ($names -contains "usage" -and $null -ne $Current.usage) {
            Visit-UsageValue $Current.usage
        }
        if ($hasInput.Count -gt 0 -or $hasOutput.Count -gt 0) {
            $found.Add($Current)
            return
        }
        foreach ($prop in @($Current.PSObject.Properties)) {
            if ($prop.Name -eq "usage") { continue }
            if ($prop.Value -is [System.Collections.IEnumerable] -and -not ($prop.Value -is [string])) {
                foreach ($item in @($prop.Value)) { Visit-UsageValue $item }
            } else {
                Visit-UsageValue $prop.Value
            }
        }
    }
    Visit-UsageValue $Value
    @($found.ToArray())
}

function Get-TaskspaceUsageNumber {
    param($Usage, [string[]]$Names)
    $value = Get-TaskspaceCostProperty $Usage $Names
    if ($null -eq $value -or [string]::IsNullOrWhiteSpace([string]$value)) { return $null }
    try { return [int64]$value } catch { return $null }
}

function Get-TaskspaceCachedInputTokens {
    param($Usage)
    $direct = Get-TaskspaceUsageNumber $Usage @("cached_input_tokens", "cached_prompt_tokens")
    if ($null -ne $direct) { return $direct }
    $details = Get-TaskspaceCostProperty $Usage @("input_tokens_details", "prompt_tokens_details")
    if ($details) { return Get-TaskspaceUsageNumber $details @("cached_tokens") }
    $null
}

function New-TaskspaceTokenSummary {
    param([string]$JsonlPath)
    $parsed = Get-TaskspaceCostJsonlRows $JsonlPath
    $inputTotal = [int64]0
    $outputTotal = [int64]0
    $cachedTotal = [int64]0
    $usageCount = 0
    $missingInput = 0
    $missingOutput = 0
    $missingCached = 0
    foreach ($row in @($parsed.rows)) {
        foreach ($usage in @(Get-TaskspaceTokenUsageObjects $row)) {
            $usageCount++
            $input = Get-TaskspaceUsageNumber $usage @("input_tokens", "prompt_tokens")
            $output = Get-TaskspaceUsageNumber $usage @("output_tokens", "completion_tokens")
            $cached = Get-TaskspaceCachedInputTokens $usage
            if ($null -eq $input) { $missingInput++ } else { $inputTotal += $input }
            if ($null -eq $output) { $missingOutput++ } else { $outputTotal += $output }
            if ($null -eq $cached) { $missingCached++ } else { $cachedTotal += $cached }
        }
    }
    $status = if ($parsed.source_status -ne "read") { "source_missing" } elseif ($usageCount -eq 0) { "usage_unavailable" } elseif ($missingInput -gt 0 -or $missingOutput -gt 0) { "partial" } else { "measured" }
    [pscustomobject]@{
        schema_version = "taskspace-token-summary-v1"
        source_path = $JsonlPath
        source_status = [string]$parsed.source_status
        parse_errors = [int]$parsed.parse_errors
        parse_status = if ($parsed.parse_errors -gt 0) { "partial" } else { "ok" }
        availability = $status
        model_request_count = if ($usageCount -gt 0) { [int]$usageCount } else { $null }
        input_tokens = if ($usageCount -gt 0 -and $missingInput -lt $usageCount) { $inputTotal } else { $null }
        output_tokens = if ($usageCount -gt 0 -and $missingOutput -lt $usageCount) { $outputTotal } else { $null }
        cached_input_tokens = if ($usageCount -gt 0 -and $missingCached -lt $usageCount) { $cachedTotal } else { $null }
        uncached_input_tokens = if ($usageCount -gt 0 -and $missingInput -lt $usageCount -and $missingCached -lt $usageCount) { [Math]::Max(0, $inputTotal - $cachedTotal) } else { $null }
        missing_usage_fields = [pscustomobject]@{
            input_tokens = [int]$missingInput
            output_tokens = [int]$missingOutput
            cached_input_tokens = [int]$missingCached
        }
    }
}

function New-TaskspaceRequestSummary {
    param([string]$JsonlPath, $TokenSummary)
    [pscustomobject]@{
        schema_version = "taskspace-request-summary-v1"
        source_path = $JsonlPath
        availability = [string]$TokenSummary.availability
        model_request_count = $TokenSummary.model_request_count
        avg_input_tokens_per_request = if ($TokenSummary.model_request_count -and $TokenSummary.input_tokens -ne $null) { [Math]::Round([double]$TokenSummary.input_tokens / [double]$TokenSummary.model_request_count, 4) } else { $null }
        avg_output_tokens_per_request = if ($TokenSummary.model_request_count -and $TokenSummary.output_tokens -ne $null) { [Math]::Round([double]$TokenSummary.output_tokens / [double]$TokenSummary.model_request_count, 4) } else { $null }
        parse_status = [string]$TokenSummary.parse_status
        parse_errors = [int]$TokenSummary.parse_errors
    }
}

function New-TaskspaceControlUsageSummary {
    param([string]$JsonlPath, [string]$ObservabilityJsonPath = "")
    $parsed = Get-TaskspaceCostJsonlRows $JsonlPath
    $actions = @{}
    $total = 0
    $stateCommit = 0
    function Visit-ControlValue($Current) {
        if ($null -eq $Current -or $Current -is [string] -or $Current -is [ValueType]) { return }
        $names = @($Current.PSObject.Properties.Name)
        $nameValue = Get-TaskspaceCostProperty $Current @("name", "tool")
        if ([string]$nameValue -eq "taskspace_control") {
            $action = ""
            $arguments = Get-TaskspaceCostProperty $Current @("arguments", "args")
            if ($arguments) {
                if ($arguments -is [string]) {
                    try {
                        $parsedArgs = $arguments | ConvertFrom-Json
                        $action = [string](Get-TaskspaceCostProperty $parsedArgs @("action"))
                    } catch {}
                } else {
                    $action = [string](Get-TaskspaceCostProperty $arguments @("action"))
                }
            }
            if ([string]::IsNullOrWhiteSpace($action)) {
                $action = [string](Get-TaskspaceCostProperty $Current @("action"))
            }
            $script:taskspaceCostControlTotal++
            if ($action -eq "state_commit") { $script:taskspaceCostStateCommit++ }
            Add-TaskspaceCostCount $script:taskspaceCostActions $action
        }
        foreach ($prop in @($Current.PSObject.Properties)) {
            if ($prop.Value -is [System.Collections.IEnumerable] -and -not ($prop.Value -is [string])) {
                foreach ($item in @($prop.Value)) { Visit-ControlValue $item }
            } else {
                Visit-ControlValue $prop.Value
            }
        }
    }
    $script:taskspaceCostActions = $actions
    $script:taskspaceCostControlTotal = 0
    $script:taskspaceCostStateCommit = 0
    foreach ($row in @($parsed.rows)) { Visit-ControlValue $row }
    $total = $script:taskspaceCostControlTotal
    $stateCommit = $script:taskspaceCostStateCommit
    Remove-Variable -Name taskspaceCostActions -Scope Script -ErrorAction SilentlyContinue
    Remove-Variable -Name taskspaceCostControlTotal -Scope Script -ErrorAction SilentlyContinue
    Remove-Variable -Name taskspaceCostStateCommit -Scope Script -ErrorAction SilentlyContinue
    $runtimeEventCounts = @{}
    $runtimeEventTotal = 0
    $runtimeStateCommit = 0
    $runtimeSourceStatus = "missing"
    if (-not [string]::IsNullOrWhiteSpace($ObservabilityJsonPath) -and (Test-Path -LiteralPath $ObservabilityJsonPath)) {
        try {
            $obs = Get-Content -Raw -Encoding UTF8 -LiteralPath $ObservabilityJsonPath | ConvertFrom-Json
            $runtimeSourceStatus = "read"
            foreach ($event in @($obs.timeline | Where-Object { [string]$_.kind -notlike "tool:*" })) {
                $kind = [string]$event.kind
                Add-TaskspaceCostCount $runtimeEventCounts $kind
                $runtimeEventTotal++
                $updateKind = [string](Get-TaskspaceCostProperty $event @("updateKind"))
                if ([string]::IsNullOrWhiteSpace($updateKind) -and $null -ne $event.details) {
                    $updateKind = [string](Get-TaskspaceCostProperty $event.details @("updateKind"))
                }
                if ($updateKind -like "state_commit*") { $runtimeStateCommit++ }
            }
        } catch {
            $runtimeSourceStatus = "parse_error"
        }
    }
    [pscustomobject]@{
        schema_version = "taskspace-control-usage-v1"
        source_path = $JsonlPath
        observability_source_path = $ObservabilityJsonPath
        source_status = [string]$parsed.source_status
        observability_source_status = $runtimeSourceStatus
        parse_errors = [int]$parsed.parse_errors
        availability = if ($parsed.source_status -eq "read") { "measured" } else { "source_missing" }
        taskspace_control_count = [int]$total
        state_commit_count = [int]$stateCommit
        runtime_state_commit_count = [int]$runtimeStateCommit
        taskspace_runtime_event_count = [int]$runtimeEventTotal
        action_counts = Convert-TaskspaceCostTable $actions
        runtime_event_counts = Convert-TaskspaceCostTable $runtimeEventCounts
    }
}

function New-TaskspaceReplaySummary {
    param([string]$ArtifactDir)
    $largest = [int64]0
    $largeReplay = 0
    $checked = 0
    foreach ($name in @("taskspace.graph.final.json", "taskspace.graph.timeout.json", "graph-health.json")) {
        $path = Join-Path $ArtifactDir $name
        if (-not (Test-Path -LiteralPath $path)) { continue }
        $checked++
        $text = Get-Content -Raw -Encoding UTF8 -LiteralPath $path
        if ($text.Length -gt $largest) { $largest = [int64]$text.Length }
        if ($text -match "\[\.{3} telemetry preview truncated \.\{3}\]" -or $text -match "\[\.\.\. telemetry preview truncated \.\.\.\]") {
            $largeReplay++
        }
    }
    [pscustomobject]@{
        schema_version = "taskspace-replay-summary-v1"
        availability = if ($checked -gt 0) { "heuristic" } else { "source_missing" }
        checked_artifact_count = [int]$checked
        largest_tool_output_bytes = $largest
        large_output_replay_count = [int]$largeReplay
        raw_output_in_prompt_violation = $false
    }
}

function Write-TaskspaceCostInstrumentationArtifacts {
    param(
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [AllowEmptyString()][string]$JsonlPath = "",
        [AllowEmptyString()][string]$ObservabilityJsonPath = ""
    )
    if (-not (Test-Path -LiteralPath $ArtifactDir)) {
        New-Item -ItemType Directory -Path $ArtifactDir -Force | Out-Null
    }
    $token = New-TaskspaceTokenSummary $JsonlPath
    $request = New-TaskspaceRequestSummary $JsonlPath $token
    $control = New-TaskspaceControlUsageSummary $JsonlPath $ObservabilityJsonPath
    $replay = New-TaskspaceReplaySummary $ArtifactDir
    $tokenPath = Join-Path $ArtifactDir "token-summary.json"
    $requestPath = Join-Path $ArtifactDir "request-summary.json"
    $controlPath = Join-Path $ArtifactDir "taskspace-control-usage.json"
    $token | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $tokenPath -Encoding UTF8
    $request | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $requestPath -Encoding UTF8
    $control | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $controlPath -Encoding UTF8
    [pscustomobject]@{
        token_summary_path = $tokenPath
        request_summary_path = $requestPath
        taskspace_control_usage_path = $controlPath
        token_summary = $token
        request_summary = $request
        taskspace_control_usage = $control
        replay_summary = $replay
    }
}

function Get-TaskspaceCostMetricNumber {
    param($Metric, [string]$Name)
    if ($Metric -and $Metric.PSObject.Properties.Name -contains $Name -and $null -ne $Metric.$Name) {
        try { return [double]$Metric.$Name } catch { return $null }
    }
    $null
}

function Add-TaskspaceCostMetricTotal {
    param([System.Collections.IDictionary]$Totals, $Metric, [string]$Field)
    $value = Get-TaskspaceCostMetricNumber $Metric $Field
    if ($null -eq $value) {
        $Totals["missing_$Field"]++
        return
    }
    $Totals[$Field] = [double]$Totals[$Field] + [double]$value
}

function New-TaskspaceCostSideTotals {
    param([string]$Mode)
    [ordered]@{
        logical_mode = $Mode
        side_count = 0
        complete_side_count = 0
        model_request_count = [double]0
        input_tokens = [double]0
        output_tokens = [double]0
        cached_input_tokens = [double]0
        uncached_input_tokens = [double]0
        wall_time_ms = [double]0
        taskspace_control_count = [double]0
        state_commit_count = [double]0
        runtime_state_commit_count = [double]0
        taskspace_runtime_event_count = [double]0
        large_output_replay_count = [double]0
        missing_model_request_count = 0
        missing_input_tokens = 0
        missing_output_tokens = 0
        missing_cached_input_tokens = 0
        missing_uncached_input_tokens = 0
        missing_wall_time_ms = 0
        missing_taskspace_control_count = 0
        missing_state_commit_count = 0
        missing_runtime_state_commit_count = 0
        missing_taskspace_runtime_event_count = 0
        missing_large_output_replay_count = 0
    }
}

function Get-TaskspaceCostRatio {
    param($Numerator, $Denominator)
    if ($null -eq $Numerator -or $null -eq $Denominator -or [double]$Denominator -le 0) { return $null }
    [Math]::Round([double]$Numerator / [double]$Denominator, 4)
}

function New-TaskspaceCostGate {
    param($Standard, $Taskspace)
    $missing = New-Object System.Collections.Generic.List[string]
    foreach ($field in @("model_request_count", "input_tokens", "output_tokens", "wall_time_ms")) {
        if ($Standard["missing_$field"] -gt 0 -or $Taskspace["missing_$field"] -gt 0 -or [double]$Standard[$field] -le 0) {
            $missing.Add($field)
        }
    }
    $standardDirect = [double]$Standard.input_tokens + [double]$Standard.output_tokens
    $taskspaceDirect = [double]$Taskspace.input_tokens + [double]$Taskspace.output_tokens
    $directRatio = Get-TaskspaceCostRatio $taskspaceDirect $standardDirect
    $wallRatio = Get-TaskspaceCostRatio $Taskspace.wall_time_ms $Standard.wall_time_ms
    $requestRatio = Get-TaskspaceCostRatio $Taskspace.model_request_count $Standard.model_request_count
    $status = "FAIL"
    $reason = "cost_gate_failed"
    if ($missing.Count -gt 0 -or $null -eq $directRatio -or $null -eq $wallRatio -or $null -eq $requestRatio) {
        $status = "FAIL"
        $reason = "missing_cost_data"
    } elseif ($directRatio -le 2.0 -and $wallRatio -le 2.0) {
        $status = "PASS"
        $reason = "primary_cost_gate_passed"
    } elseif ($directRatio -le 3.0 -and $wallRatio -le 3.0 -and $requestRatio -le 2.5) {
        $status = "PARTIAL"
        $reason = "engineering_partial_cost_gate_passed"
    }
    [pscustomobject]@{
        schema_version = "taskspace-suite-cost-gate-v1"
        status = $status
        reason = $reason
        missing_fields = @($missing.ToArray())
        ratios = [pscustomobject]@{
            direct_input_output_ratio = $directRatio
            walltime_ratio = $wallRatio
            model_request_count_ratio = $requestRatio
        }
        thresholds = [pscustomobject]@{
            pass_direct_input_output_ratio = 2.0
            pass_walltime_ratio = 2.0
            partial_direct_input_output_ratio = 3.0
            partial_walltime_ratio = 3.0
            partial_model_request_count_ratio = 2.5
        }
    }
}

function Write-TaskspaceCostAggregateArtifacts {
    param(
        [Parameter(Mandatory = $true)][string]$RootDir,
        [Parameter(Mandatory = $true)][ValidateSet("pair", "sample", "suite")][string]$Scope
    )
    if (-not (Test-Path -LiteralPath $RootDir)) { throw "Cost aggregate root does not exist: $RootDir" }
    $metricFiles = @(Get-ChildItem -LiteralPath $RootDir -Filter "metrics.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object FullName)
    $parseErrors = New-Object System.Collections.Generic.List[string]
    $byMode = @{
        standard = New-TaskspaceCostSideTotals "standard"
        taskspace = New-TaskspaceCostSideTotals "taskspace"
        other = New-TaskspaceCostSideTotals "other"
    }
    foreach ($file in $metricFiles) {
        try { $metric = Get-Content -Raw -Encoding UTF8 -LiteralPath $file.FullName | ConvertFrom-Json } catch { $parseErrors.Add($file.FullName); continue }
        $mode = if ($metric.PSObject.Properties.Name -contains "logical_mode") { [string]$metric.logical_mode } else { "other" }
        if (-not $byMode.ContainsKey($mode)) { $mode = "other" }
        $totals = $byMode[$mode]
        $totals.side_count++
        if ([string]$metric.token_summary_availability -eq "measured") { $totals.complete_side_count++ }
        foreach ($field in @("model_request_count", "input_tokens", "output_tokens", "cached_input_tokens", "uncached_input_tokens", "wall_time_ms", "taskspace_control_count", "state_commit_count", "runtime_state_commit_count", "taskspace_runtime_event_count", "large_output_replay_count")) {
            Add-TaskspaceCostMetricTotal $totals $metric $field
        }
    }
    $tokenPath = Join-Path $RootDir "token-summary.json"
    $requestPath = Join-Path $RootDir "request-summary.json"
    $controlPath = Join-Path $RootDir "taskspace-control-usage.json"
    $gatePath = Join-Path $RootDir "suite-cost-gate.json"
    $summary = [pscustomobject]@{
        schema_version = "taskspace-cost-aggregate-v1"
        scope = $Scope
        root_dir = $RootDir
        metric_file_count = @($metricFiles).Count
        parse_error_count = $parseErrors.Count
        parse_error_paths = @($parseErrors.ToArray())
        modes = [pscustomobject]@{
            standard = [pscustomobject]$byMode.standard
            taskspace = [pscustomobject]$byMode.taskspace
            other = [pscustomobject]$byMode.other
        }
        generated_at = (Get-Date).ToString("o")
    }
    $request = [pscustomobject]@{
        schema_version = "taskspace-request-aggregate-v1"
        scope = $Scope
        standard = [pscustomobject]@{
            model_request_count = $byMode.standard.model_request_count
            avg_input_tokens_per_request = Get-TaskspaceCostRatio $byMode.standard.input_tokens $byMode.standard.model_request_count
            avg_output_tokens_per_request = Get-TaskspaceCostRatio $byMode.standard.output_tokens $byMode.standard.model_request_count
        }
        taskspace = [pscustomobject]@{
            model_request_count = $byMode.taskspace.model_request_count
            avg_input_tokens_per_request = Get-TaskspaceCostRatio $byMode.taskspace.input_tokens $byMode.taskspace.model_request_count
            avg_output_tokens_per_request = Get-TaskspaceCostRatio $byMode.taskspace.output_tokens $byMode.taskspace.model_request_count
        }
    }
    $control = [pscustomobject]@{
        schema_version = "taskspace-control-usage-aggregate-v1"
        scope = $Scope
        taskspace_control_count = $byMode.taskspace.taskspace_control_count
        state_commit_count = $byMode.taskspace.state_commit_count
        runtime_state_commit_count = $byMode.taskspace.runtime_state_commit_count
        taskspace_runtime_event_count = $byMode.taskspace.taskspace_runtime_event_count
        standard_taskspace_control_count = $byMode.standard.taskspace_control_count
        standard_runtime_state_commit_count = $byMode.standard.runtime_state_commit_count
        standard_taskspace_runtime_event_count = $byMode.standard.taskspace_runtime_event_count
    }
    $gate = New-TaskspaceCostGate $byMode.standard $byMode.taskspace
    $summary | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $tokenPath -Encoding UTF8
    $request | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $requestPath -Encoding UTF8
    $control | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $controlPath -Encoding UTF8
    $gate | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $gatePath -Encoding UTF8
    [pscustomobject]@{
        token_summary_path = $tokenPath
        request_summary_path = $requestPath
        taskspace_control_usage_path = $controlPath
        suite_cost_gate_path = $gatePath
        gate = $gate
    }
}
