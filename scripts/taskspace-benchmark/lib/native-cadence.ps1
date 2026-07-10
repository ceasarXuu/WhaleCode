Set-StrictMode -Version Latest

function Get-TaskspaceNativeCadenceFacts {
    param(
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [System.Collections.Generic.List[object]]$Events
    )
    $rolloutPath = Join-Path $ArtifactDir "rollout.jsonl"
    if (-not (Test-Path -LiteralPath $rolloutPath -PathType Leaf)) {
        return [pscustomobject]@{
            availability = "missing"; tool_bearing_response_count = $null
            control_only_response_count = $null; mixed_barrier_batch_count = $null
            terminal_candidate_count = $null; terminal_extra_request_count = $null
        }
    }

    $batches = New-Object System.Collections.Generic.List[object]
    $current = New-Object System.Collections.Generic.List[object]
    $rowIndex = 0
    $lastFinishIndex = -1
    $lastFinalIndex = -1
    $terminalCandidateCount = 0
    foreach ($line in [System.IO.File]::ReadLines($rolloutPath)) {
        $rowIndex++
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $row = $line | ConvertFrom-Json } catch {
            if ($Events) {
                $Events.Add([pscustomobject]@{
                        event = "cadence_rollout_line_parse_failed"; path = $rolloutPath
                        row = $rowIndex; error = [string]$_.Exception.Message
                    })
            }
            continue
        }
        if ([string]$row.type -ne "response_item" -or $null -eq $row.payload) { continue }
        $payload = $row.payload
        $payloadType = [string]$payload.type
        $isCall = $payloadType -in @("function_call", "custom_tool_call", "local_shell_call")
        if ($isCall) {
            $name = if ($payloadType -eq "local_shell_call") { "local_shell" } else { [string]$payload.name }
            $action = ""
            $hasTerminalCandidate = $false
            if ($name -eq "taskspace_control") {
                try {
                    $arguments = ([string]$payload.arguments) | ConvertFrom-Json
                    $action = [string]$arguments.action
                    $candidateProperty = $arguments.PSObject.Properties["final_candidate"]
                    $hasTerminalCandidate = $null -ne $candidateProperty -and -not [string]::IsNullOrWhiteSpace([string]$candidateProperty.Value)
                } catch { }
                if ($action -eq "finish_node") { $lastFinishIndex = $rowIndex }
                if ($hasTerminalCandidate) { $terminalCandidateCount++ }
            }
            $current.Add([pscustomobject]@{
                    name = $name; action = $action
                    terminal_candidate = [bool]$hasTerminalCandidate
                })
            continue
        }

        if ($current.Count -gt 0) {
            $batches.Add(@($current.ToArray()))
            $current.Clear()
        }
        if ($payloadType -eq "message" -and [string]$payload.role -eq "assistant") {
            $content = @($payload.content | ForEach-Object { [string]$_.text }) -join ""
            if (-not [string]::IsNullOrWhiteSpace($content)) { $lastFinalIndex = $rowIndex }
        }
    }
    if ($current.Count -gt 0) { $batches.Add(@($current.ToArray())) }

    $controlOnly = 0
    $mixedBarrier = 0
    foreach ($batch in $batches) {
        $calls = @($batch)
        $controlCount = @($calls | Where-Object { $_.name -eq "taskspace_control" }).Count
        if ($controlCount -eq $calls.Count) { $controlOnly++ }
        elseif ($controlCount -gt 0) { $mixedBarrier++ }
    }
    $terminalExtra = if ($terminalCandidateCount -gt 0) {
        0
    } elseif ($lastFinishIndex -ge 0 -and $lastFinalIndex -gt $lastFinishIndex) {
        1
    } else {
        $null
    }
    [pscustomobject]@{
        availability = "measured"
        tool_bearing_response_count = [int]$batches.Count
        control_only_response_count = [int]$controlOnly
        mixed_barrier_batch_count = [int]$mixedBarrier
        terminal_candidate_count = [int]$terminalCandidateCount
        terminal_extra_request_count = $terminalExtra
    }
}
