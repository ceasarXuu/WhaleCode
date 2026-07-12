Set-StrictMode -Version Latest

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
            if ([string](Get-PerformanceProperty $row "type") -ne "response_item") { continue }
            $payload = Get-PerformanceProperty $row "payload"
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
}
