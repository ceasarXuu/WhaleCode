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
    [bigint]$duplicateCount = 0
    [bigint]$duplicateBytes = 0
    foreach ($group in @($Entries | Group-Object -Property $HashField)) {
        if ([string]::IsNullOrWhiteSpace([string]$group.Name) -or $group.Count -lt 2) { continue }
        $duplicateCount += $group.Count - 1
        foreach ($entry in @($group.Group | Select-Object -Skip 1)) {
            $duplicateBytes += [bigint][int64]$entry.$BytesField
        }
    }
    if ($duplicateCount -gt [int64]::MaxValue -or
        $duplicateBytes -gt [int64]::MaxValue) {
        throw "Performance duplicate statistics exceed Int64"
    }
    [pscustomobject]@{
        count = [int64]$duplicateCount
        bytes = [int64]$duplicateBytes
    }
}

function Get-PerformanceExactExcessCount {
    param($Counts)
    [bigint]$total = 0
    foreach ($count in $Counts) {
        if ([bigint]$count -gt 1) { $total += [bigint]$count - 1 }
    }
    if ($total -gt [int64]::MaxValue) {
        throw "Performance duplicate record count exceeds Int64"
    }
    [int64]$total
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
            declared_action_count = $null; ordinary_sibling_count = $null; declared_action_name_match_count = $null
            control_output_step_count = $null; control_output_completed_work_id_count = $null; control_output_finish_id_count = $null
            control_success_count = $null
            control_delta_present_count = $null; control_delta_missing_count = $null; control_graph_event_ref_count = $null; control_node_detail_event_ref_count = $null
            terminal_failure_nonzero_commit_count = $null
        }
    }
    $unknown = 0
    $finalCandidates = New-Object System.Collections.Generic.List[object]
    $assistantFinals = New-Object System.Collections.Generic.List[object]
    $controlCalls = @{}
    $pendingManifest = $null
    $pendingManifestIndex = 0
    $declaredActionCount = 0
    $ordinarySiblingCount = 0
    $declaredActionNameMatchCount = 0
    $stepCount = 0
    $completedWorkIdCount = 0
    $finishIdCount = 0
    $controlSuccess = 0
    $deltaPresent = 0; $deltaMissing = 0; $graphEventRefCount = 0; $nodeDetailEventRefCount = 0; $terminalFailureNonzeroCommit = 0
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
                $args = $null
                $parentCallId = [string](Get-PerformanceProperty $payload "parentCallId")
                if ([string]::IsNullOrWhiteSpace($parentCallId)) {
                    $parentCallId = [string](Get-PerformanceProperty $envelope "parentCallId")
                }
                if (-not [string]::IsNullOrWhiteSpace($argsText)) {
                    try { $args = $argsText | ConvertFrom-Json } catch { $unknown++ }
                }
                if ($name -eq "taskspace_control" -and $null -ne $args) {
                    $controlCalls[$callId] = $args
                    $action = [string](Get-PerformanceProperty $args "action")
                    if ($action -eq "finish_map") {
                        $candidate = Get-PerformanceProperty $args "exact_summary"
                        if ($null -ne $candidate) { $finalCandidates.Add([pscustomobject]@{ index = $index; text = [string]$candidate }) }
                    }
                    if ($action -in @("initialize_and_execute", "execute", "reopen_map")) {
                        $pendingManifest = @((Get-PerformanceProperty $args "actions" @()))
                        $pendingManifestIndex = 0
                        $declaredActionCount += $pendingManifest.Count
                    }
                } elseif ($null -ne $pendingManifest -and [string]::IsNullOrWhiteSpace($parentCallId)) {
                    $ordinarySiblingCount++
                    if ($pendingManifestIndex -lt $pendingManifest.Count -and
                        [string](Get-PerformanceProperty $pendingManifest[$pendingManifestIndex] "tool") -eq $name) {
                        $declaredActionNameMatchCount++
                    }
                    $pendingManifestIndex++
                }
                continue
            }
            if (-not $controlCalls.ContainsKey($callId)) { continue }
            $output = Get-PerformanceProperty $payload "output"
            try {
                $outputObject = if ($output -is [string]) { $output | ConvertFrom-Json } else { $output }
                $schemaVersion = [string](Get-PerformanceProperty $outputObject "schema_version")
                $isControlResult = $schemaVersion -eq "TaskSpaceControlResultV2"
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
                    if ($action -eq "finish_map" -and -not $success -and $stateCommit) {
                        $terminalFailureNonzeroCommit++
                    }
                }
                foreach ($step in @((Get-PerformanceProperty $outputObject "steps" @()))) {
                    $stepCount++
                    $completedWorkIdCount += @((Get-PerformanceProperty $step "completed_work_node_ids" @())).Count
                    if (-not [string]::IsNullOrWhiteSpace([string](Get-PerformanceProperty $step "finish_node_id"))) {
                        $finishIdCount++
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
        declared_action_count = $declaredActionCount
        ordinary_sibling_count = $ordinarySiblingCount
        declared_action_name_match_count = $declaredActionNameMatchCount
        control_output_step_count = $stepCount
        control_output_completed_work_id_count = $completedWorkIdCount
        control_output_finish_id_count = $finishIdCount
        control_success_count = $controlSuccess
        control_delta_present_count = $deltaPresent
        control_delta_missing_count = $deltaMissing
        control_graph_event_ref_count = $graphEventRefCount
        control_node_detail_event_ref_count = $nodeDetailEventRefCount
        terminal_failure_nonzero_commit_count = $terminalFailureNonzeroCommit
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
                    $prior = if ($calls.ContainsKey($callId)) { [int64]$calls[$callId] } else { [int64]0 }
                    $calls[$callId] = [int64]$prior + 1
                }
                continue
            }
            if ($type -notin @("function_call_output", "custom_tool_call_output", "local_shell_call_output", "mcp_tool_call_output")) { continue }
            if (-not [string]::IsNullOrWhiteSpace($callId)) {
                $prior = if ($callOutputs.ContainsKey($callId)) { [int64]$callOutputs[$callId] } else { [int64]0 }
                $callOutputs[$callId] = [int64]$prior + 1
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
        duplicate_call_records = Get-PerformanceExactExcessCount @($calls.Values)
        duplicate_output_records = Get-PerformanceExactExcessCount @($callOutputs.Values)
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
        max_content_duplicates = if ($requests.Count) {
            [int64]@($requests.ToArray() | Sort-Object count -Descending)[0].count
        } else { $null }
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
    $Lines.Add("| Repeat | Mode | Availability | Unknown | Final candidates | Final exact | Declared actions | Siblings | Name matches | Control success | Steps | Completed Work IDs | Finish IDs | Delta | Delta missing | Graph event refs | Detail event refs | Terminal bad commit |")
    $Lines.Add("|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    foreach ($row in $Rows) {
        $lineage = $row.duplication.cross_carrier_lineage
        $Lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $lineage.availability) | $(Format-PerformanceValue $lineage.unknown_count) | $(Format-PerformanceValue $lineage.final_candidate_count) | $(Format-PerformanceValue $lineage.final_candidate_assistant_exact_equal_count) | $(Format-PerformanceValue $lineage.declared_action_count) | $(Format-PerformanceValue $lineage.ordinary_sibling_count) | $(Format-PerformanceValue $lineage.declared_action_name_match_count) | $(Format-PerformanceValue $lineage.control_success_count) | $(Format-PerformanceValue $lineage.control_output_step_count) | $(Format-PerformanceValue $lineage.control_output_completed_work_id_count) | $(Format-PerformanceValue $lineage.control_output_finish_id_count) | $(Format-PerformanceValue $lineage.control_delta_present_count) | $(Format-PerformanceValue $lineage.control_delta_missing_count) | $(Format-PerformanceValue $lineage.control_graph_event_ref_count) | $(Format-PerformanceValue $lineage.control_node_detail_event_ref_count) | $(Format-PerformanceValue $lineage.terminal_failure_nonzero_commit_count) |")
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
