Set-StrictMode -Version Latest
if (-not (Get-Command Get-TaskspaceCanonicalResponseItem -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "canonical-rollout.ps1")
}

function Get-PerformanceSha256 {
    param([string]$Text)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    $hash = [System.Security.Cryptography.SHA256]::HashData($bytes)
    [Convert]::ToHexString($hash).ToLowerInvariant()
}

function Get-PerformanceDuplicateStats {
    param([object[]]$Entries, [string]$HashField, [string]$BytesField)
    $duplicateCount = 0
    $duplicateBytes = 0
    foreach ($group in @($Entries | Group-Object -Property $HashField)) {
        if ([string]::IsNullOrWhiteSpace([string]$group.Name) -or $group.Count -lt 2) { continue }
        $duplicateCount += $group.Count - 1
        $duplicateBytes += @($group.Group | Select-Object -Skip 1 | ForEach-Object { [int64]$_.$BytesField } | Measure-Object -Sum).Sum
    }
    [pscustomobject]@{ count = $duplicateCount; bytes = $duplicateBytes }
}

function ConvertTo-PerformanceStableObject {
    param($Value)
    if ($Value -is [string]) {
        try { return ConvertTo-PerformanceStableObject ($Value | ConvertFrom-Json) } catch { return [string]$Value }
    }
    if ($null -eq $Value -or $Value -is [ValueType]) { return $Value }
    if ($Value -is [System.Collections.IEnumerable] -and $Value -isnot [string] -and $Value -isnot [pscustomobject]) {
        return @($Value | ForEach-Object { ConvertTo-PerformanceStableObject $_ })
    }
    if (@($Value.PSObject.Properties).Count -gt 0) {
        $ordered = [ordered]@{}
        foreach ($property in @($Value.PSObject.Properties | Sort-Object Name)) {
            $ordered[$property.Name] = ConvertTo-PerformanceStableObject $property.Value
        }
        return [pscustomobject]$ordered
    }
    $Value
}

function ConvertTo-PerformanceCanonicalJson {
    param($Value)
    (ConvertTo-PerformanceStableObject $Value) | ConvertTo-Json -Compress -Depth 40
}

function Test-PerformanceNestedActionMatch {
    param($OuterAction, [string]$NestedToolName, $NestedArguments)
    if ([string](Get-PerformanceProperty $OuterAction "tool_name") -ne $NestedToolName) { return $false }
    $outerArguments = Get-PerformanceProperty $OuterAction "arguments"
    (ConvertTo-PerformanceCanonicalJson $outerArguments) -eq (ConvertTo-PerformanceCanonicalJson $NestedArguments)
}

function Get-PerformanceDeclaredNestedActions {
    param($Arguments)
    $continuation = Get-PerformanceProperty $Arguments "continuation"
    if ($null -eq $continuation) { return @() }
    if ($continuation -is [string]) { return @() }
    $kind = [string](Get-PerformanceProperty $continuation "kind")
    if ($kind -eq "actions") {
        return @((Get-PerformanceProperty $continuation "actions" @()))
    }
    if ($kind -eq "patch_then_actions") {
        $declared = New-Object System.Collections.Generic.List[object]
        $patch = Get-PerformanceProperty $continuation "patch"
        if ($null -ne $patch) { $declared.Add($patch) }
        foreach ($action in @((Get-PerformanceProperty $continuation "actions" @()))) {
            $declared.Add($action)
        }
        return @($declared.ToArray())
    }
    @()
}

function Test-PerformanceObjectContainsStringValue {
    param($Value, [string]$Needle)
    if ([string]::IsNullOrWhiteSpace($Needle) -or $null -eq $Value) { return $false }
    if ($Value -is [string] -or $Value -is [ValueType]) { return ([string]$Value) -eq $Needle }
    if ($Value -is [System.Collections.IEnumerable] -and $Value -isnot [string] -and $Value -isnot [pscustomobject]) {
        foreach ($item in @($Value)) {
            if (Test-PerformanceObjectContainsStringValue $item $Needle) { return $true }
        }
        return $false
    }
    foreach ($property in @($Value.PSObject.Properties)) {
        if (Test-PerformanceObjectContainsStringValue $property.Value $Needle) { return $true }
    }
    $false
}

function Get-PerformanceAssistantText {
    param($Payload)
    $content = Get-PerformanceProperty $Payload "content"
    if ($content -is [string]) { return $content }
    $parts = New-Object System.Collections.Generic.List[string]
    foreach ($part in @($content)) {
        $text = Get-PerformanceProperty $part "text"
        if ($null -ne $text) { $parts.Add([string]$text) }
    }
    if ($parts.Count -gt 0) { return ($parts.ToArray() -join "") }
    $null
}

function Get-PerformanceCrossCarrierLineage {
    param([string]$Path, [System.Collections.Generic.List[object]]$Events)
    if (-not (Test-Path -LiteralPath $Path)) {
        return [pscustomobject]@{
            availability = "unavailable"; unknown_count = $null
            final_candidate_count = $null; final_candidate_assistant_exact_equal_count = $null; final_candidate_assistant_exact_equal_bytes = $null
            control_continuation_action_count = $null; expanded_nested_call_count = $null; expanded_nested_call_exact_json_match_count = $null
            control_output_step_count = $null; control_output_init_node_id_echo_count = $null; control_output_init_current_node_id_echo_count = $null
            control_output_finished_node_id_echo_count = $null; control_output_next_node_echo_count = $null; control_output_current_node_echo_count = $null
            control_success_count = $null; control_identity_step_count = $null; control_identity_missing_count = $null
            control_delta_present_count = $null; control_delta_missing_count = $null; control_graph_event_ref_count = $null; control_node_detail_event_ref_count = $null
            terminal_failure_nonzero_commit_count = $null
            stale_blank_developer_marker_count = $null; stale_mode_developer_marker_count = $null
        }
    }
    $unknown = 0
    $finalCandidates = New-Object System.Collections.Generic.List[object]
    $assistantFinals = New-Object System.Collections.Generic.List[object]
    $outerActionsByCall = @{}
    $expandedNested = 0; $expandedMatches = 0
    $controlCalls = @{}
    $stepCount = 0; $initNodeEcho = 0; $initCurrentEcho = 0
    $finishedNodeEcho = 0; $nextNodeEcho = 0; $currentNodeEcho = 0
    $controlSuccess = 0; $identitySteps = 0; $identityMissing = 0
    $deltaPresent = 0; $deltaMissing = 0; $graphEventRefCount = 0; $nodeDetailEventRefCount = 0; $terminalFailureNonzeroCommit = 0
    $blankDeveloper = 0; $modeDeveloper = 0
    $index = 0
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        $index++
        try {
            $row = $line | ConvertFrom-Json
            $envelope = Get-PerformanceProperty $row "payload"
            $payload = Get-TaskspaceCanonicalResponseItem $row
            if ($null -eq $payload) { $payload = Get-PerformanceProperty $row "payload" $row }
            $type = [string](Get-PerformanceProperty $payload "type")
            $role = [string](Get-PerformanceProperty $payload "role")
            if ($role -eq "developer") {
                $text = Get-PerformanceAssistantText $payload
                if ([string]$text -match 'active_task_path_without_nodes|TaskSpace blank|TaskSpace v0\.0\.5 thin bootstrap') { $blankDeveloper++ }
                if ([string]$text -match 'TaskSpace mode is now active\.') { $modeDeveloper++ }
            }
            if ($type -eq "message" -and $role -eq "assistant") {
                $phase = [string](Get-PerformanceProperty $payload "phase")
                if ($phase -eq "final_answer") {
                    $text = Get-PerformanceAssistantText $payload
                    if ($null -ne $text) { $assistantFinals.Add([pscustomobject]@{ index = $index; text = [string]$text }) }
                }
                continue
            }
            if ($type -notin @("function_call", "custom_tool_call", "function_call_output", "custom_tool_call_output")) { continue }
            $callId = [string](Get-PerformanceProperty $payload "call_id")
            if ($type -in @("function_call", "custom_tool_call")) {
                $name = [string](Get-PerformanceProperty $payload "name")
                $argsText = [string](Get-PerformanceProperty $payload "arguments")
                $parentCallId = [string](Get-PerformanceProperty $payload "parentCallId")
                if ([string]::IsNullOrWhiteSpace($parentCallId)) { $parentCallId = [string](Get-PerformanceProperty $payload "parent_call_id") }
                if ([string]::IsNullOrWhiteSpace($parentCallId) -and $null -ne $envelope) { $parentCallId = [string](Get-PerformanceProperty $envelope "parentCallId") }
                if ([string]::IsNullOrWhiteSpace($parentCallId) -and $null -ne $envelope) { $parentCallId = [string](Get-PerformanceProperty $envelope "parent_call_id") }
                $args = $null
                if (-not [string]::IsNullOrWhiteSpace($argsText)) {
                    try { $args = $argsText | ConvertFrom-Json } catch { $unknown++ }
                }
                if ($name -eq "taskspace_control" -and $null -ne $args) {
                    $controlCalls[$callId] = $args
                    $action = [string](Get-PerformanceProperty $args "action")
                    if ($action -eq "finish_end") {
                        $candidate = Get-PerformanceProperty $args "final_summary"
                        if ($null -ne $candidate) { $finalCandidates.Add([pscustomobject]@{ index = $index; text = [string]$candidate }) }
                    }
                    if ($null -ne (Get-PerformanceProperty $args "continuation") -and -not [string]::IsNullOrWhiteSpace($callId)) {
                        $outerActionsByCall[$callId] = @(Get-PerformanceDeclaredNestedActions $args)
                    }
                } elseif (-not [string]::IsNullOrWhiteSpace($parentCallId) -and $outerActionsByCall.ContainsKey($parentCallId)) {
                    $expandedNested++
                    $nestedArgs = if ($null -ne $args) { $args } else { $argsText }
                    if (@($outerActionsByCall[$parentCallId] | Where-Object { Test-PerformanceNestedActionMatch $_ $name $nestedArgs }).Count -gt 0) { $expandedMatches++ }
                }
                continue
            }
            if (-not $controlCalls.ContainsKey($callId)) { continue }
            $output = Get-PerformanceProperty $payload "output"
            try {
                $outputObject = if ($output -is [string]) { $output | ConvertFrom-Json } else { $output }
                $isControlResult = [string](Get-PerformanceProperty $outputObject "schema_version") -eq "TaskSpaceControlResultR6V1"
                if ($isControlResult -and [bool](Get-PerformanceProperty $outputObject "success" $false)) { $controlSuccess++ }
                if ($isControlResult) {
                    $delta = Get-PerformanceProperty $outputObject "delta"
                    if ($null -eq $delta) {
                        $deltaMissing++
                    } else {
                        $deltaPresent++
                        $graphEventRefCount += @((Get-PerformanceProperty $delta "graph_event_refs" @())).Count
                        $nodeDetailEventRefCount += @((Get-PerformanceProperty $delta "node_detail_event_refs" @())).Count
                    }
                    $action = [string](Get-PerformanceProperty $controlCalls[$callId] "action")
                    $success = [bool](Get-PerformanceProperty $outputObject "success" $false)
                    $stateCommit = [bool](Get-PerformanceProperty $outputObject "state_commit" $false)
                    if ($action -eq "finish_end" -and -not $success -and $stateCommit) {
                        $terminalFailureNonzeroCommit++
                    }
                }
                foreach ($step in @((Get-PerformanceProperty $outputObject "steps" @()))) {
                    $stepCount++
                    $kind = [string](Get-PerformanceProperty $step "kind")
                    $finishedNodeId = [string](Get-PerformanceProperty $step "finished_node_id")
                    $next = Get-PerformanceProperty $step "next"
                    $nextNodeId = [string](Get-PerformanceProperty $next "node_id")
                    $currentNodeId = [string](Get-PerformanceProperty $step "current_node_id")
                    if (Test-PerformanceObjectContainsStringValue $controlCalls[$callId] $finishedNodeId) { $finishedNodeEcho++ }
                    if (Test-PerformanceObjectContainsStringValue $controlCalls[$callId] $nextNodeId) { $nextNodeEcho++ }
                    if (Test-PerformanceObjectContainsStringValue $controlCalls[$callId] $currentNodeId) { $currentNodeEcho++ }
                    if ($kind -eq "map_initialized") {
                        foreach ($nodeId in @((Get-PerformanceProperty $step "created_node_ids" @()))) {
                            if (Test-PerformanceObjectContainsStringValue $controlCalls[$callId] ([string]$nodeId)) { $initNodeEcho++ }
                        }
                        if (Test-PerformanceObjectContainsStringValue $controlCalls[$callId] $currentNodeId) { $initCurrentEcho++ }
                    }
                    if ($isControlResult -and $kind -in @("map_initialized", "graph_mutation", "node_transition", "terminal_transition", "node_detail_expanded") -and
                        -not ([bool](Get-PerformanceProperty $step "success" $true) -eq $false)) {
                        $identitySteps++
                        $complete = if ($kind -eq "map_initialized") {
                            -not [string]::IsNullOrWhiteSpace([string](Get-PerformanceProperty $step "map_id")) -and $null -ne (Get-PerformanceProperty $step "revision")
                        } elseif ($kind -eq "graph_mutation") {
                            -not [string]::IsNullOrWhiteSpace([string](Get-PerformanceProperty $step "map_id")) -and $null -ne (Get-PerformanceProperty $step "revision")
                        } elseif ($kind -eq "node_transition") {
                            -not [string]::IsNullOrWhiteSpace([string](Get-PerformanceProperty $step "map_id")) -and -not [string]::IsNullOrWhiteSpace([string](Get-PerformanceProperty $step "node_id")) -and $null -ne (Get-PerformanceProperty $step "revision") -and -not [string]::IsNullOrWhiteSpace([string](Get-PerformanceProperty $step "status"))
                        } elseif ($kind -eq "node_detail_expanded") {
                            -not [string]::IsNullOrWhiteSpace([string](Get-PerformanceProperty $step "node_id")) -and -not [string]::IsNullOrWhiteSpace([string](Get-PerformanceProperty $step "expansion_event_id"))
                        } else {
                            -not [string]::IsNullOrWhiteSpace([string](Get-PerformanceProperty $step "map_id")) -and $null -ne (Get-PerformanceProperty $step "revision") -and [bool](Get-PerformanceProperty $step "finish_closed" $false) -and [bool](Get-PerformanceProperty $step "root_closed" $false)
                        }
                        if (-not $complete) { $identityMissing++ }
                    }
                }
            } catch {
                if ($output -is [string] -and -not ([string]$output).TrimStart().StartsWith("{") -and -not ([string]$output).TrimStart().StartsWith("[")) { continue }
                $unknown++
            }
        } catch {
            $unknown++
            if ($Events) { $Events.Add([pscustomobject]@{ event = "cross_carrier_lineage_line_parse_failed"; path = $Path; error = [string]$_.Exception.Message }) }
        }
    }
    $finalMatches = 0; $finalBytes = 0
    foreach ($candidate in @($finalCandidates.ToArray())) {
        foreach ($final in @($assistantFinals.ToArray() | Where-Object { $_.index -gt $candidate.index })) {
            if ([string]$final.text -eq [string]$candidate.text) {
                $finalMatches++
                $finalBytes += [System.Text.Encoding]::UTF8.GetByteCount([string]$candidate.text)
                break
            }
        }
    }
    [pscustomobject]@{
        availability = if ($unknown -gt 0) { "partial_with_unknown" } else { "rollout" }
        unknown_count = $unknown
        final_candidate_count = $finalCandidates.Count
        final_candidate_assistant_exact_equal_count = $finalMatches
        final_candidate_assistant_exact_equal_bytes = $finalBytes
        control_continuation_action_count = [int](@($outerActionsByCall.Values | ForEach-Object { @($_).Count } | Measure-Object -Sum).Sum)
        expanded_nested_call_count = $expandedNested
        expanded_nested_call_exact_json_match_count = $expandedMatches
        control_output_step_count = $stepCount
        control_output_init_node_id_echo_count = $initNodeEcho
        control_output_init_current_node_id_echo_count = $initCurrentEcho
        control_output_finished_node_id_echo_count = $finishedNodeEcho
        control_output_next_node_echo_count = $nextNodeEcho
        control_output_current_node_echo_count = $currentNodeEcho
        control_success_count = $controlSuccess
        control_identity_step_count = $identitySteps
        control_identity_missing_count = $identityMissing
        control_delta_present_count = $deltaPresent
        control_delta_missing_count = $deltaMissing
        control_graph_event_ref_count = $graphEventRefCount
        control_node_detail_event_ref_count = $nodeDetailEventRefCount
        terminal_failure_nonzero_commit_count = $terminalFailureNonzeroCommit
        stale_blank_developer_marker_count = $blankDeveloper
        stale_mode_developer_marker_count = $modeDeveloper
    }
}

function Get-PerformanceRolloutStorage {
    param([string]$Path, [System.Collections.Generic.List[object]]$Events)
    if (-not (Test-Path -LiteralPath $Path)) {
        return [pscustomobject]@{ availability = "unavailable"; unknown_count = $null; rollout_total_bytes = $null; snapshot_updated_line_count = $null; snapshot_updated_payload_bytes = $null; snapshot_updated_payload_ratio = $null; snapshot_delta_line_count = $null; snapshot_delta_payload_bytes = $null; internal_replay_payload_bytes = $null; internal_replay_payload_ratio = $null }
    }
    $unknown = 0; $totalBytes = 0; $snapshotLines = 0; $snapshotPayloadBytes = 0; $deltaLines = 0; $deltaPayloadBytes = 0
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        $totalBytes += [System.Text.Encoding]::UTF8.GetByteCount($line)
        if ($line -notmatch 'snapshot_(updated|delta)') { continue }
        try {
            $row = $line | ConvertFrom-Json
            $payload = Get-PerformanceProperty $row "payload" $row
            $payloadType = Get-TaskspaceRolloutPayloadType $payload
            $payloadBytes = [System.Text.Encoding]::UTF8.GetByteCount((ConvertTo-PerformanceCanonicalJson $payload))
            if ($payloadType -eq "snapshot_updated") {
                $snapshotLines++
                $snapshotPayloadBytes += $payloadBytes
            } elseif ($payloadType -eq "snapshot_delta") {
                $deltaLines++
                $deltaPayloadBytes += $payloadBytes
            } else {
                $unknown++
            }
        } catch {
            $unknown++
            if ($Events) { $Events.Add([pscustomobject]@{ event = "rollout_storage_snapshot_parse_failed"; path = $Path; error = [string]$_.Exception.Message }) }
        }
    }
    [pscustomobject]@{
        availability = if ($unknown -gt 0) { "partial_with_unknown" } else { "rollout" }
        unknown_count = $unknown
        rollout_total_bytes = $totalBytes
        snapshot_updated_line_count = $snapshotLines
        snapshot_updated_payload_bytes = $snapshotPayloadBytes
        snapshot_updated_payload_ratio = if ($totalBytes -gt 0) { [Math]::Round($snapshotPayloadBytes / [double]$totalBytes, 4) } else { $null }
        snapshot_delta_line_count = $deltaLines
        snapshot_delta_payload_bytes = $deltaPayloadBytes
        internal_replay_payload_bytes = $snapshotPayloadBytes + $deltaPayloadBytes
        internal_replay_payload_ratio = if ($totalBytes -gt 0) { [Math]::Round(($snapshotPayloadBytes + $deltaPayloadBytes) / [double]$totalBytes, 4) } else { $null }
    }
}

function Get-PerformanceRolloutDuplication {
    param([string]$Path, [System.Collections.Generic.List[object]]$Events)
    if (-not (Test-Path -LiteralPath $Path)) {
        return [pscustomobject]@{ availability = "unavailable"; exact_payload_duplicates = $null; duplicate_output_bodies = $null; duplicate_output_body_bytes = $null; duplicate_call_records = $null; duplicate_output_records = $null; orphan_calls = $null; orphan_outputs = $null }
    }
    $payloads = New-Object System.Collections.Generic.List[object]
    $outputs = New-Object System.Collections.Generic.List[object]
    $calls = @{}
    $callOutputs = @{}
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try {
            $row = $line | ConvertFrom-Json
            $payload = Get-TaskspaceCanonicalResponseItem $row
            if ($null -eq $payload) { continue }
            $payloadJson = $payload | ConvertTo-Json -Compress -Depth 40
            $payloads.Add([pscustomobject]@{ hash = Get-PerformanceSha256 $payloadJson; bytes = [System.Text.Encoding]::UTF8.GetByteCount($payloadJson) })
            $type = [string](Get-PerformanceProperty $payload "type")
            $callId = [string](Get-PerformanceProperty $payload "call_id")
            if ($type -in @("function_call", "custom_tool_call", "local_shell_call", "mcp_tool_call")) {
                if (-not [string]::IsNullOrWhiteSpace($callId)) {
                    $prior = if ($calls.ContainsKey($callId)) { [int]$calls[$callId] } else { 0 }
                    $calls[$callId] = $prior + 1
                }
                continue
            }
            if ($type -notin @("function_call_output", "custom_tool_call_output", "local_shell_call_output", "mcp_tool_call_output")) { continue }
            if (-not [string]::IsNullOrWhiteSpace($callId)) {
                $prior = if ($callOutputs.ContainsKey($callId)) { [int]$callOutputs[$callId] } else { 0 }
                $callOutputs[$callId] = $prior + 1
            }
            $output = Get-PerformanceProperty $payload "output"
            $outputJson = if ($output -is [string]) { [string]$output } else { $output | ConvertTo-Json -Compress -Depth 40 }
            if (-not [string]::IsNullOrEmpty($outputJson)) {
                $outputs.Add([pscustomobject]@{ hash = Get-PerformanceSha256 $outputJson; bytes = [System.Text.Encoding]::UTF8.GetByteCount($outputJson) })
            }
        } catch {
            if ($Events) { $Events.Add([pscustomobject]@{ event = "duplication_rollout_line_parse_failed"; path = $Path; error = [string]$_.Exception.Message }) }
        }
    }
    $payloadDup = Get-PerformanceDuplicateStats $payloads.ToArray() "hash" "bytes"
    $outputDup = Get-PerformanceDuplicateStats $outputs.ToArray() "hash" "bytes"
    [pscustomobject]@{
        availability = "rollout"
        exact_payload_duplicates = $payloadDup.count
        duplicate_output_bodies = $outputDup.count
        duplicate_output_body_bytes = $outputDup.bytes
        duplicate_call_records = @($calls.GetEnumerator() | Where-Object Value -gt 1 | ForEach-Object { $_.Value - 1 } | Measure-Object -Sum).Sum
        duplicate_output_records = @($callOutputs.GetEnumerator() | Where-Object Value -gt 1 | ForEach-Object { $_.Value - 1 } | Measure-Object -Sum).Sum
        orphan_calls = @($calls.Keys | Where-Object { -not $callOutputs.ContainsKey($_) }).Count
        orphan_outputs = @($callOutputs.Keys | Where-Object { -not $calls.ContainsKey($_) }).Count
    }
}

function Get-PerformanceWireDuplication {
    param([string]$Path, [System.Collections.Generic.List[object]]$Events)
    if (-not (Test-Path -LiteralPath $Path)) {
        return [pscustomobject]@{ availability = "unavailable"; request_count = 0; final_content_duplicates = $null; final_duplicate_content_bytes = $null; max_content_duplicates = $null }
    }
    $requests = New-Object System.Collections.Generic.List[object]
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try {
            $row = $line | ConvertFrom-Json
            if ([string](Get-PerformanceProperty $row "event_name") -notmatch '^provider\.chat_wire_(shape_recorded|prefix_preserved|prefix_broken)$') { continue }
            $stats = Get-PerformanceDuplicateStats @((Get-PerformanceProperty $row "message_shapes" @())) "content_sha256" "bytes"
            $requests.Add([pscustomobject]@{ request_index = [int](Get-PerformanceProperty $row "request_index" 0); count = $stats.count; bytes = $stats.bytes })
        } catch {
            if ($Events) { $Events.Add([pscustomobject]@{ event = "duplication_wire_line_parse_failed"; path = $Path; error = [string]$_.Exception.Message }) }
        }
    }
    $final = @($requests.ToArray() | Sort-Object request_index | Select-Object -Last 1)
    [pscustomobject]@{
        availability = if ($requests.Count) { "provider_wire_trace" } else { "unavailable" }
        request_count = $requests.Count
        final_content_duplicates = if ($final.Count) { $final[0].count } else { $null }
        final_duplicate_content_bytes = if ($final.Count) { $final[0].bytes } else { $null }
        max_content_duplicates = if ($requests.Count) { @($requests.ToArray() | Measure-Object count -Maximum).Maximum } else { $null }
    }
}

function Get-PerformanceDuplicationFacts {
    param([string]$ArtifactDir, [System.Collections.Generic.List[object]]$Events)
    [pscustomobject]@{
        contract = "exact_hash_only_no_semantic_similarity"
        rollout = Get-PerformanceRolloutDuplication (Join-Path $ArtifactDir "rollout.jsonl") $Events
        provider_wire = Get-PerformanceWireDuplication (Join-Path $ArtifactDir "provider-wire-trace.jsonl") $Events
        cross_carrier_lineage = Get-PerformanceCrossCarrierLineage (Join-Path $ArtifactDir "rollout.jsonl") $Events
        rollout_storage = Get-PerformanceRolloutStorage (Join-Path $ArtifactDir "rollout.jsonl") $Events
    }
}

function Add-PerformanceDuplicationMarkdown {
    param([System.Collections.Generic.List[string]]$Lines, [object[]]$Rows)
    $Lines.Add("")
    $Lines.Add("## 精确重复载体")
    $Lines.Add("")
    $Lines.Add("仅按完整 payload、原始 output body、call_id 或 provider message content SHA-256 计数，不执行语义相似度判断。")
    $Lines.Add("")
    $Lines.Add("| Repeat | Mode | Payload dup | Output body dup | Output dup bytes | Call dup | Output record dup | Orphan calls | Orphan outputs | Final wire content dup | Final wire dup bytes | Max wire dup |")
    $Lines.Add("|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    foreach ($row in $Rows) {
        $rollout = $row.duplication.rollout
        $wire = $row.duplication.provider_wire
        $Lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $rollout.exact_payload_duplicates) | $(Format-PerformanceValue $rollout.duplicate_output_bodies) | $(Format-PerformanceValue $rollout.duplicate_output_body_bytes) | $(Format-PerformanceValue $rollout.duplicate_call_records) | $(Format-PerformanceValue $rollout.duplicate_output_records) | $(Format-PerformanceValue $rollout.orphan_calls) | $(Format-PerformanceValue $rollout.orphan_outputs) | $(Format-PerformanceValue $wire.final_content_duplicates) | $(Format-PerformanceValue $wire.final_duplicate_content_bytes) | $(Format-PerformanceValue $wire.max_content_duplicates) |")
    }
    $Lines.Add("")
    $Lines.Add("## Cross carrier lineage")
    $Lines.Add("")
    $Lines.Add("| Repeat | Mode | Availability | Unknown | Final candidates | Final exact | Continuation actions | Nested exact | Control success | Identity steps | Identity missing | Delta | Delta missing | Graph event refs | Detail event refs | Terminal bad commit | Init IDs | Finished IDs | Next IDs | Current IDs |")
    $Lines.Add("|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    foreach ($row in $Rows) {
        $lineage = $row.duplication.cross_carrier_lineage
        $Lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $lineage.availability) | $(Format-PerformanceValue $lineage.unknown_count) | $(Format-PerformanceValue $lineage.final_candidate_count) | $(Format-PerformanceValue $lineage.final_candidate_assistant_exact_equal_count) | $(Format-PerformanceValue $lineage.control_continuation_action_count) | $(Format-PerformanceValue $lineage.expanded_nested_call_exact_json_match_count) | $(Format-PerformanceValue $lineage.control_success_count) | $(Format-PerformanceValue $lineage.control_identity_step_count) | $(Format-PerformanceValue $lineage.control_identity_missing_count) | $(Format-PerformanceValue $lineage.control_delta_present_count) | $(Format-PerformanceValue $lineage.control_delta_missing_count) | $(Format-PerformanceValue $lineage.control_graph_event_ref_count) | $(Format-PerformanceValue $lineage.control_node_detail_event_ref_count) | $(Format-PerformanceValue $lineage.terminal_failure_nonzero_commit_count) | $(Format-PerformanceValue $lineage.control_output_init_node_id_echo_count) | $(Format-PerformanceValue $lineage.control_output_finished_node_id_echo_count) | $(Format-PerformanceValue $lineage.control_output_next_node_echo_count) | $(Format-PerformanceValue $lineage.control_output_current_node_echo_count) |")
    }
    $Lines.Add("")
    $Lines.Add("## Rollout storage")
    $Lines.Add("")
    $Lines.Add("| Repeat | Mode | Availability | Unknown | Rollout bytes | Checkpoints | Checkpoint bytes | Checkpoint ratio | Deltas | Delta bytes | Replay bytes | Replay ratio |")
    $Lines.Add("|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    foreach ($row in $Rows) {
        $storage = $row.duplication.rollout_storage
        $Lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $storage.availability) | $(Format-PerformanceValue $storage.unknown_count) | $(Format-PerformanceValue $storage.rollout_total_bytes) | $(Format-PerformanceValue $storage.snapshot_updated_line_count) | $(Format-PerformanceValue $storage.snapshot_updated_payload_bytes) | $(Format-PerformanceValue $storage.snapshot_updated_payload_ratio percent) | $(Format-PerformanceValue $storage.snapshot_delta_line_count) | $(Format-PerformanceValue $storage.snapshot_delta_payload_bytes) | $(Format-PerformanceValue $storage.internal_replay_payload_bytes) | $(Format-PerformanceValue $storage.internal_replay_payload_ratio percent) |")
    }
}
