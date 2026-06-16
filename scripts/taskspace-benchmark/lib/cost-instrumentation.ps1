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
    param([string]$JsonlPath)
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
    [pscustomobject]@{
        schema_version = "taskspace-control-usage-v1"
        source_path = $JsonlPath
        source_status = [string]$parsed.source_status
        parse_errors = [int]$parsed.parse_errors
        availability = if ($parsed.source_status -eq "read") { "measured" } else { "source_missing" }
        taskspace_control_count = [int]$total
        state_commit_count = [int]$stateCommit
        action_counts = Convert-TaskspaceCostTable $actions
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
        [AllowEmptyString()][string]$JsonlPath = ""
    )
    if (-not (Test-Path -LiteralPath $ArtifactDir)) {
        New-Item -ItemType Directory -Path $ArtifactDir -Force | Out-Null
    }
    $token = New-TaskspaceTokenSummary $JsonlPath
    $request = New-TaskspaceRequestSummary $JsonlPath $token
    $control = New-TaskspaceControlUsageSummary $JsonlPath
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
