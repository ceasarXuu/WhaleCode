function New-Dir([string]$PathValue) { (New-Item -ItemType Directory -Force -Path $PathValue).FullName }
function Write-Text([string]$PathValue, [string]$Text) { [System.IO.File]::WriteAllText($PathValue, $Text, [System.Text.UTF8Encoding]::new($false)) }
function Count-Matches([string]$Text, [string]$Pattern) { ([regex]::Matches($Text, $Pattern)).Count }
function Get-InternalOrchestrationLeakPattern {
    "(?i)taskspace|action map|task map|\bmap-\d+\b|\bnode-\d+\b|\btask-\d+\b|subagents?|spawn_agent|taskspace_control|final_synthesis|\b(?:parallel(?:ize|ized|izing)?|concurrent(?:ly)?|simultaneous(?:ly)?)\s+(?:agents?|subagents?|explorers?|evidence tracks?)\b|\bdelegat(?:e|ed|ing|ion)\b|\bmultiple agents?\b|\bmulti[- ]agent\b|\bsplit\s+.*\bagents?\b|\bfan[- ]?out\b|\bexplorers?\b|\bevidence tracks?\b"
}
function Get-RegexFirstMatchExcerpt([string]$Text, [string]$Pattern) {
    if ([string]::IsNullOrWhiteSpace($Text)) { return "" }
    $match = [regex]::Match($Text, $Pattern)
    if (-not $match.Success) { return "" }
    $start = [Math]::Max(0, $match.Index - 60)
    $length = [Math]::Min($Text.Length - $start, $match.Length + 120)
    (($Text.Substring($start, $length)) -replace "\s+", " ").Trim()
}
function Get-ObjectPropertyNames($Value) {
    if ($null -eq $Value) { return @() }
    return @($Value.PSObject.Properties.Name)
}

function Invoke-RealProcess {
    param(
        [string]$FilePath,
        [string[]]$ArgumentList,
        [string]$WorkingDirectory,
        [string]$StdoutPath,
        [string]$StderrPath,
        [int]$TimeoutSeconds,
        [string]$StdinPath = "",
        [string]$TimingPath = "",
        [hashtable]$Environment = @{}
    )
    $encoding = [System.Text.UTF8Encoding]::new($false)
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $StdoutPath) | Out-Null
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $StderrPath) | Out-Null
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FilePath
    $startInfo.WorkingDirectory = $WorkingDirectory
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.StandardOutputEncoding = $encoding
    $startInfo.StandardErrorEncoding = $encoding
    foreach ($key in @($Environment.Keys)) {
        if ([string]::IsNullOrWhiteSpace([string]$key)) { continue }
        $startInfo.Environment[[string]$key] = [string]$Environment[$key]
    }
    if ($StdinPath) { $startInfo.RedirectStandardInput = $true }
    $startInfo.Arguments = (($ArgumentList | ForEach-Object {
        $arg = [string]$_
        if ($arg -match '[\s"]') { '"' + ($arg -replace '"', '\"') + '"' } else { $arg }
    }) -join " ")
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    $stdoutStream = [System.IO.FileStream]::new($StdoutPath, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write, [System.IO.FileShare]::Read)
    $stderrStream = [System.IO.FileStream]::new($StderrPath, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write, [System.IO.FileShare]::Read)
    $launchStartedAt = Get-Date
    $processStartedAt = $null
    $timedOut = $false
    $exitCode = $null
    try {
        [void]$process.Start()
        $processStartedAt = Get-Date
        $stdoutCopyTask = $process.StandardOutput.BaseStream.CopyToAsync($stdoutStream)
        $stderrCopyTask = $process.StandardError.BaseStream.CopyToAsync($stderrStream)
        if (-not [string]::IsNullOrWhiteSpace($TimingPath)) {
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $TimingPath) | Out-Null
            [pscustomobject]@{
                schema_version = 1
                process_launch_started_at = $launchStartedAt.ToString("o")
                process_started_at = $processStartedAt.ToString("o")
                process_launch_wait_ms = [int64](($processStartedAt - $launchStartedAt).TotalMilliseconds)
                timed_out = $false
                completed = $false
            } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $TimingPath -Encoding UTF8
        }
        if ($StdinPath) {
            $stdinBytes = [System.IO.File]::ReadAllBytes($StdinPath)
            $process.StandardInput.BaseStream.Write($stdinBytes, 0, $stdinBytes.Length)
            $process.StandardInput.Close()
        }
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            $timedOut = $true
            try { $process.Kill($true) } catch { try { $process.Kill() } catch {} }
            try { $process.WaitForExit(5000) | Out-Null } catch {}
        } else {
            $process.WaitForExit()
            $exitCode = $process.ExitCode
        }
    } finally {
        try {
            if ($null -ne $stdoutCopyTask) { $stdoutCopyTask.Wait(5000) | Out-Null }
        } catch {}
        try {
            if ($null -ne $stderrCopyTask) { $stderrCopyTask.Wait(5000) | Out-Null }
        } catch {}
        if ($timedOut) {
            $timeoutBytes = $encoding.GetBytes("Process timed out after $TimeoutSeconds seconds: $FilePath $($ArgumentList -join ' ')`n")
            try { $stderrStream.Write($timeoutBytes, 0, $timeoutBytes.Length) } catch {}
        }
        $completedAt = Get-Date
        if (-not [string]::IsNullOrWhiteSpace($TimingPath) -and $null -ne $processStartedAt) {
            [pscustomobject]@{
                schema_version = 1
                process_launch_started_at = $launchStartedAt.ToString("o")
                process_started_at = $processStartedAt.ToString("o")
                process_completed_at = $completedAt.ToString("o")
                process_launch_wait_ms = [int64](($processStartedAt - $launchStartedAt).TotalMilliseconds)
                wall_time_ms = [int64](($completedAt - $processStartedAt).TotalMilliseconds)
                timed_out = $timedOut
                completed = (-not $timedOut)
                exit_code = $exitCode
            } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $TimingPath -Encoding UTF8
        }
        try { $stdoutStream.Dispose() } catch {}
        try { $stderrStream.Dispose() } catch {}
        try { $process.Dispose() } catch {}
    }
    if ($timedOut) {
        throw "Process timed out after $TimeoutSeconds seconds: $FilePath $($ArgumentList -join ' ')"
    }
    $exitCode
}

function Get-ThreadId([string]$JsonlText) {
    foreach ($line in ($JsonlText -split "`r?`n")) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try {
            $event = $line | ConvertFrom-Json
            if ($event.type -eq "thread.started" -and $event.thread_id) { return [string]$event.thread_id }
        } catch {}
    }
    ""
}

function Find-LatestRollout([datetime]$StartedAt, [string]$ThreadId) {
    $homes = @()
    if ($env:WHALE_HOME) { $homes += $env:WHALE_HOME }
    $homes += (Join-Path $env:USERPROFILE ".whale")
    foreach ($candidateHome in $homes | Select-Object -Unique) {
        if (-not (Test-Path $candidateHome)) { continue }
        $recent = Get-ChildItem -Path $candidateHome -Recurse -Filter "rollout-*.jsonl" -ErrorAction SilentlyContinue |
            Where-Object { $_.LastWriteTime -ge $StartedAt.AddMinutes(-2) } |
            Sort-Object LastWriteTime -Descending
        foreach ($candidate in $recent) {
            if (-not $ThreadId) { return $candidate }
            $raw = Get-Content -Raw -Encoding UTF8 $candidate.FullName -ErrorAction SilentlyContinue
            if ($raw -match [regex]::Escape($ThreadId)) { return $candidate }
        }
    }
    $null
}

function Add-ReportLine([System.Collections.Generic.List[string]]$Report, [string]$Key, $Value) {
    $Report.Add("- ${Key}: $Value")
}

function Get-CommandStats([string]$JsonlText) {
    $ids = [System.Collections.Generic.HashSet[string]]::new()
    $failed = 0
    $pytestPassed = $false
    $pytestCount = 0
    foreach ($line in ($JsonlText -split "`r?`n")) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $evt = $line | ConvertFrom-Json } catch { continue }
        if ($evt.type -ne "item.completed" -or $evt.item.type -ne "command_execution") { continue }
        [void]$ids.Add([string]$evt.item.id)
        if ($evt.item.status -eq "failed" -or $evt.item.exit_code -ne 0) { $failed++ }
        $command = [string]$evt.item.command
        $output = [string]$evt.item.aggregated_output
        if ($command -match "pytest") {
            $pytestCount++
            if ($evt.item.exit_code -eq 0 -and $output -match "passed") { $pytestPassed = $true }
        }
    }
    [pscustomobject]@{
        Completed = $ids.Count
        Failed = $failed
        PytestCount = $pytestCount
        AgentRanPassingPytest = $pytestPassed
    }
}

function Get-SuccessfulTaskspaceOrdering([string]$RolloutText) {
    $firstBinding = $null
    $firstOrdinary = $null
    $pendingOrdinary = @{}
    foreach ($line in ($RolloutText -split "`r?`n")) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $evt = $line | ConvertFrom-Json } catch { continue }
        if (-not $firstBinding -and $evt.type -eq "event_msg") {
            $eventKind = [string]$evt.payload.type
            if ($eventKind -eq "lease_created") {
                $firstBinding = [pscustomobject]@{ Timestamp = [string]$evt.timestamp; Evidence = "lease_created" }
            }
        }
        if ($evt.type -eq "response_item" -and $evt.payload.type -eq "function_call") {
            $name = [string]$evt.payload.name
            if ($name -match "^(shell_command|apply_patch|spawn_agent)$" -and $evt.payload.call_id) {
                $pendingOrdinary[[string]$evt.payload.call_id] = [pscustomobject]@{
                    Timestamp = [string]$evt.timestamp
                    Tool = $name
                }
            }
        }
        if (-not $firstOrdinary -and $evt.type -eq "response_item" -and $evt.payload.type -eq "function_call_output") {
            $callId = [string]$evt.payload.call_id
            if ($pendingOrdinary.ContainsKey($callId)) {
                $output = [string]$evt.payload.output
                $blockedByTaskspace = $output -match "TaskSpace (mode is active|blocked this tool call)|Call taskspace_control"
                if (-not $blockedByTaskspace) {
                    $firstOrdinary = $pendingOrdinary[$callId]
                }
            }
        }
    }
    $ordinaryBeforeBinding = $false
    if ($firstOrdinary -and -not $firstBinding) { $ordinaryBeforeBinding = $true }
    elseif ($firstOrdinary -and $firstBinding) {
        $ordinaryBeforeBinding = ([datetime]$firstOrdinary.Timestamp) -lt ([datetime]$firstBinding.Timestamp)
    }
    [pscustomobject]@{
        FirstBindingTimestamp = if ($firstBinding) { $firstBinding.Timestamp } else { "" }
        FirstBindingEvidence = if ($firstBinding) { $firstBinding.Evidence } else { "" }
        FirstOrdinaryToolTimestamp = if ($firstOrdinary) { $firstOrdinary.Timestamp } else { "" }
        FirstOrdinaryTool = if ($firstOrdinary) { $firstOrdinary.Tool } else { "" }
        OrdinaryToolBeforeBinding = $ordinaryBeforeBinding
    }
}

function Get-PytestOwnership($Obs, $ToolCallArgs) {
    $empty = [pscustomobject]@{ Owned = $false; NodeId = ""; NodeKind = ""; NodeTitle = ""; ResultId = ""; CallId = ""; Command = ""; At = "" }
    if (-not $Obs) { return $empty }
    $testKinds = @("smoke_test", "regression_test")
    foreach ($node in @($Obs.nodes | Where-Object { $testKinds -contains [string]$_.kind })) {
        foreach ($result in @($node.results | Where-Object { $_.kind -eq "main_tool_call" -and $_.actionClass -eq "test" })) {
            if ((Get-ObjectPropertyNames $result) -contains "success" -and $result.success -ne $true) {
                continue
            }
            $body = [string]$result.body
            $callId = ""
            if ($body -match 'call_id:\s*(call_[A-Za-z0-9_]+)') {
                $callId = $Matches[1]
            }
            $command = ""
            if ($callId -and $ToolCallArgs.ContainsKey($callId)) {
                $command = [string]$ToolCallArgs[$callId]
            }
            $combined = "$body`n$command`n$([string]$result.preview)"
            if ($combined -match "pytest" -and $combined -match "Exit code:\s*0" -and $combined -match "(?i)\bpassed\b") {
                return [pscustomobject]@{
                    Owned = $true
                    NodeId = [string]$node.id
                    NodeKind = [string]$node.kind
                    NodeTitle = [string]$node.title
                    ResultId = [string]$result.resultId
                    CallId = $callId
                    Command = $command
                    At = [string]$result.at
                }
            }
        }
    }
    $empty
}

function Get-ChangedPathsFromDiff([string]$DiffText) {
    $paths = [System.Collections.Generic.List[string]]::new()
    foreach ($match in [regex]::Matches($DiffText, 'diff --git a/\S+ b/([^\s]+)')) {
        $path = $match.Groups[1].Value
        if (-not $paths.Contains($path)) { $paths.Add($path) }
    }
    @($paths)
}

function Get-ToolResultCallId($Result) {
    if ((Get-ObjectPropertyNames $Result) -contains "callId" -and -not [string]::IsNullOrWhiteSpace([string]$Result.callId)) {
        return [string]$Result.callId
    }
    $body = [string]$Result.body
    if ($body -match 'call_id:\s*(call_[A-Za-z0-9_]+)') { return $Matches[1] }
    ""
}

function Test-TextMentionsChangedPath([string]$Text, [string]$Path) {
    $windowsPath = $Path.Replace("/", "\")
    $escapedWindowsPath = $windowsPath.Replace("\", "\\")
    $forwardPath = $Path.Replace("\", "/")
    $escapedForwardPath = $forwardPath.Replace("/", "\/")
    return (
        $Text.Contains($Path) -or
        $Text.Contains($windowsPath) -or
        $Text.Contains($escapedWindowsPath) -or
        $Text.Contains($forwardPath) -or
        $Text.Contains($escapedForwardPath)
    )
}

function Get-ImplementationOwnershipGap($Obs, [string]$DiffText, $ToolCallArgs = $null) {
    $changed = @(Get-ChangedPathsFromDiff $DiffText)
    $missing = [System.Collections.Generic.List[string]]::new()
    if (-not $Obs) {
        foreach ($path in $changed) { $missing.Add($path) }
        return [pscustomobject]@{
            ChangedCount = $changed.Count
            MissingCount = $missing.Count
            MissingPaths = @($missing)
        }
    }
    foreach ($path in $changed) {
        $owned = $false
        foreach ($node in @($Obs.nodes | Where-Object { $_.kind -eq "implement_solution" })) {
            foreach ($result in @($node.results | Where-Object { $_.kind -eq "main_tool_call" -and $_.actionClass -eq "edit" })) {
                if ((Get-ObjectPropertyNames $result) -contains "success" -and $result.success -ne $true) { continue }
                $text = "$([string]$result.body)`n$([string]$result.preview)"
                $callId = Get-ToolResultCallId $result
                if ($ToolCallArgs -and $callId -and $ToolCallArgs.ContainsKey($callId)) {
                    $text = "$text`n$([string]$ToolCallArgs[$callId])"
                }
                if (Test-TextMentionsChangedPath $text $path) {
                    $owned = $true
                    break
                }
            }
            if ($owned) { break }
        }
        if (-not $owned) { $missing.Add($path) }
    }
    [pscustomobject]@{
        ChangedCount = $changed.Count
        MissingCount = $missing.Count
        MissingPaths = @($missing)
    }
}

function Test-ToolResultHasShellError($Result) {
    $text = "$([string]$Result.body)`n$([string]$Result.preview)"
    $text -match "(?m)(Cannot find path|Get-Content :|At line:\d+|CategoryInfo\s*:|FullyQualifiedErrorId\s*:|Traceback \(most recent call last\))"
}

function Count-ProblematicSuccessfulToolResults($Obs) {
    if (-not $Obs) { return 0 }
    @($Obs.nodes | ForEach-Object {
            @($_.results | Where-Object {
                    $_.kind -eq "main_tool_call" -and
                    (Get-ObjectPropertyNames $_) -contains "success" -and
                    $_.success -eq $true -and
                    (Test-ToolResultHasShellError $_)
                })
        }).Count
}

function Test-ExpectedFailedTaskspaceToolResult($Node, $Result) {
    if ($Result.kind -ne "main_tool_call") { return $false }
    if ((Get-ObjectPropertyNames $Result) -notcontains "success" -or $Result.success -ne $false) { return $false }
    $isDiagnosticOrValidationNode =
        [string]$Node.kind -eq "inspect_code_context" -or
        [string]$Node.kind -eq "smoke_test" -or
        [string]$Node.kind -eq "regression_test"
    $isTestAction = [string]$Result.actionClass -eq "test"
    if ($isDiagnosticOrValidationNode -and $isTestAction) { return $true }
    $isRecoveredReadRetry =
        [string]$Result.actionClass -eq "read" -and
        "$([string]$Result.body)`n$([string]$Result.preview)" -match "Tool call failed before producing a result" -and
        @($Node.results | Where-Object {
                $_.kind -eq "main_tool_call" -and
                [string]$_.actionClass -eq "read" -and
                (Get-ObjectPropertyNames $_) -contains "success" -and
                $_.success -eq $true
            }).Count -gt 0
    if ($isRecoveredReadRetry) { return $true }
    $isRecoveredEditRetry =
        [string]$Node.kind -eq "implement_solution" -and
        [string]$Result.actionClass -eq "edit" -and
        "$([string]$Result.body)`n$([string]$Result.preview)" -match "Tool call failed before producing a result" -and
        @($Node.results | Where-Object {
                $_.kind -eq "main_tool_call" -and
                [string]$_.actionClass -eq "edit" -and
                (Get-ObjectPropertyNames $_) -contains "success" -and
                $_.success -eq $true
            }).Count -gt 0
    $isRecoveredEditRetry
}

function Count-UnexpectedBlockedTaskspaceToolActions($Obs) {
    if (-not $Obs) { return 0 }
    @($Obs.nodes | ForEach-Object {
            $node = $_
            @($node.blockedActions | Where-Object {
                    -not (
                        ([string]$node.kind -eq "implement_solution" -and [string]$_.actionClass -eq "test") -or
                        ([string]$node.kind -eq "inspect_code_context" -and [string]$_.actionClass -eq "edit") -or
                        ([string]$node.kind -match "smoke_test|regression_test" -and [string]$_.actionClass -eq "edit") -or
                        ([string]$node.kind -eq "inspect_code_context" -and [string]$_.actionClass -eq "unknown")
                    )
                })
        }).Count
}

function Count-FailedTaskspaceToolResults($Obs) {
    if (-not $Obs) { return 0 }
    @($Obs.nodes | ForEach-Object {
            @($_.results | Where-Object {
                    $_.kind -eq "main_tool_call" -and
                    (Get-ObjectPropertyNames $_) -contains "success" -and
                    $_.success -eq $false
                })
        }).Count
}

function Count-UnexpectedFailedTaskspaceToolResults($Obs) {
    if (-not $Obs) { return 0 }
    @($Obs.nodes | ForEach-Object {
            $node = $_
            @($node.results | Where-Object {
                    $_.kind -eq "main_tool_call" -and
                    (Get-ObjectPropertyNames $_) -contains "success" -and
                    $_.success -eq $false -and
                    -not (Test-ExpectedFailedTaskspaceToolResult $node $_)
                })
        }).Count
}

function Count-FailedCollabToolCalls($Obs) {
    if (-not $Obs) { return 0 }
    @($Obs.toolCalls | Where-Object { $_.status -eq "failed" }).Count
}

function Test-ExpectedFailedCollabToolCall($ToolCall) {
    if (-not $ToolCall -or [string]$ToolCall.status -ne "failed") { return $false }
    $text = "$([string]$ToolCall.promptPreview)`n$([string]$ToolCall.outputPreview)"
    $isRecoveredStaleSpawn =
        [string]$ToolCall.tool -eq "spawn_agent" -and
        $text -match "is completed; create or choose an open ready node"
    $isRecoveredLifecycleGate =
        [string]$ToolCall.tool -eq "spawn_agent" -and
        $text -match "still unreviewed.*mark_result_validity"
    $isRecoveredActiveLeaseGate =
        [string]$ToolCall.tool -eq "spawn_agent" -and
        $text -match "already held by an active lease"
    $isRecoveredNarrowInspectGate =
        [string]$ToolCall.tool -eq "spawn_agent" -and
        $text -match "completed narrow inspect node already exists"
    $isRecoveredStaleSpawn -or $isRecoveredLifecycleGate -or $isRecoveredActiveLeaseGate -or $isRecoveredNarrowInspectGate
}

function Count-UnexpectedFailedCollabToolCalls($Obs) {
    if (-not $Obs) { return 0 }
    @($Obs.toolCalls | Where-Object {
            $_.status -eq "failed" -and -not (Test-ExpectedFailedCollabToolCall $_)
        }).Count
}

function Count-EditResultsAfter([object[]]$Nodes, [string]$Timestamp) {
    if ([string]::IsNullOrWhiteSpace($Timestamp)) { return 0 }
    $cutoff = [datetime]$Timestamp
    @($Nodes | ForEach-Object {
            @($_.results | Where-Object {
                    $_.kind -eq "main_tool_call" -and
                    $_.actionClass -eq "edit" -and
                    -not [string]::IsNullOrWhiteSpace([string]$_.at) -and
                    ([datetime]$_.at) -gt $cutoff
                })
        }).Count
}

