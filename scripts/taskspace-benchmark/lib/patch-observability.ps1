Set-StrictMode -Version Latest
if (-not (Get-Command Get-TaskspaceCanonicalResponseItem -ErrorAction SilentlyContinue)) {
    . (Join-Path $PSScriptRoot "canonical-rollout.ps1")
}

function Get-PatchObservationProperty {
    param($Object, [Parameter(Mandatory = $true)][string]$Name, $Default = $null)
    if ($null -ne $Object) {
        $property = $Object.PSObject.Properties[$Name]
        if ($null -ne $property) { return $property.Value }
    }
    $Default
}

function Convert-PatchObservationArguments {
    param($Value)
    if ($null -eq $Value) { return $null }
    if ($Value -isnot [string]) { return $Value }
    if ([string]::IsNullOrWhiteSpace($Value)) { return $null }
    try { $Value | ConvertFrom-Json } catch { $null }
}

function Get-PatchObservationInput {
    param($Action)
    $input = Get-PatchObservationProperty $Action "input"
    if ($null -ne $input) { return [string]$input }
    $arguments = Convert-PatchObservationArguments (Get-PatchObservationProperty $Action "arguments")
    [string](Get-PatchObservationProperty $arguments "input" "")
}

function Get-PatchObservationFileCount {
    param([AllowEmptyString()][string]$Patch)
    if ([string]::IsNullOrWhiteSpace($Patch)) { return 0 }
    $paths = @([regex]::Matches($Patch, '(?m)^\*\*\* (?:Add|Update|Delete) File:\s*(.+?)\s*$') |
        ForEach-Object { [string]$_.Groups[1].Value } | Sort-Object -Unique)
    [int]$paths.Count
}

function Get-PatchObservationNestedActions {
    param($ControlPayload)
    $arguments = Convert-PatchObservationArguments (Get-PatchObservationProperty $ControlPayload "arguments")
    if ($null -eq $arguments) { return @() }
    $continuation = Get-PatchObservationProperty $arguments "continuation"
    if ($null -ne $continuation) {
        $kind = [string](Get-PatchObservationProperty $continuation "kind")
        if ($kind -eq "patch_then_actions") {
            $declared = @()
            $patch = Get-PatchObservationProperty $continuation "patch"
            if ($null -ne $patch) { $declared += $patch }
            $declared += @((Get-PatchObservationProperty $continuation "actions" @()))
            return @($declared)
        }
        return @((Get-PatchObservationProperty $continuation "actions" @()))
    }
    @((Get-PatchObservationProperty $arguments "actions" @()))
}

function Get-PatchObservationOutputText {
    param($Payload)
    $output = Get-PatchObservationProperty $Payload "output"
    if ($null -eq $output) { return "" }
    if ($output -is [string]) { return [string]$output }
    try { $output | ConvertTo-Json -Compress -Depth 20 } catch { [string]$output }
}

function Get-PatchObservationReadKey {
    param($Action)
    $name = [string](Get-PatchObservationProperty $Action "tool_name" (Get-PatchObservationProperty $Action "name"))
    if ($name -notin @("read_file", "read_output_ref")) { return "" }
    $arguments = Convert-PatchObservationArguments (Get-PatchObservationProperty $Action "arguments")
    foreach ($field in @("path", "file_path", "output_ref")) {
        $value = [string](Get-PatchObservationProperty $arguments $field "")
        if (-not [string]::IsNullOrWhiteSpace($value)) { return "$name|$value" }
    }
    ""
}

function Get-TaskspacePatchObservability {
    param(
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [System.Collections.Generic.List[object]]$Events
    )
    $rolloutPath = Join-Path $ArtifactDir "rollout.jsonl"
    if (-not (Test-Path -LiteralPath $rolloutPath -PathType Leaf)) {
        return [pscustomobject]@{ schema_version = "taskspace-patch-observability-v1"; availability = "missing" }
    }

    $records = New-Object System.Collections.Generic.List[object]
    $rowIndex = 0
    foreach ($line in [System.IO.File]::ReadLines($rolloutPath)) {
        $rowIndex++
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $row = $line | ConvertFrom-Json } catch {
            if ($Events) {
                $Events.Add([pscustomobject]@{ event = "patch_observation_rollout_parse_failed"; path = $rolloutPath; row = $rowIndex; error = [string]$_.Exception.Message })
            }
            continue
        }
        $payload = Get-TaskspaceCanonicalResponseItem $row
        if ($null -eq $payload) { continue }
        $parentCallId = ""
        $toolSuccess = $null
        if ([string]$row.type -eq "event_msg" -and [string]$row.payload.type -eq "task_context_event_recorded") {
            $parentCallId = [string](Get-PatchObservationProperty $row.payload "parentCallId" "")
            $toolSuccess = Get-PatchObservationProperty $row.payload "toolSuccess"
        }
        $records.Add([pscustomobject]@{
                row = $rowIndex; payload = $payload; parent_call_id = $parentCallId
                tool_success = $toolSuccess
            })
    }

    $batches = New-Object System.Collections.Generic.List[object]
    $current = New-Object System.Collections.Generic.List[object]
    foreach ($record in $records) {
        $type = [string](Get-PatchObservationProperty $record.payload "type")
        $isProviderCall = $type -in @("function_call", "custom_tool_call", "local_shell_call") -and
            [string]::IsNullOrWhiteSpace([string]$record.parent_call_id)
        if ($isProviderCall) {
            $current.Add($record)
        } elseif ($current.Count -gt 0) {
            $batches.Add(@($current.ToArray()))
            $current.Clear()
        }
    }
    if ($current.Count -gt 0) { $batches.Add(@($current.ToArray())) }

    $calls = New-Object System.Collections.Generic.List[object]
    $callById = @{}
    $requestRows = New-Object System.Collections.Generic.List[object]
    $singlePatchCarrierCount = 0
    $multiPatchCarrierAttemptCount = 0
    $totalRequestPatchCount = 0
    $maxRequestPatchCount = 0
    for ($batchIndex = 0; $batchIndex -lt $batches.Count; $batchIndex++) {
        $declared = New-Object System.Collections.Generic.List[object]
        $outerCalls = @($batches[$batchIndex])
        foreach ($record in $outerCalls) {
            $payload = $record.payload
            $name = if ([string]$payload.type -eq "local_shell_call") { "local_shell" } else { [string]$payload.name }
            $callId = [string](Get-PatchObservationProperty $payload "call_id" "")
            if ($name -eq "taskspace_control") {
                $nested = @(Get-PatchObservationNestedActions $payload)
                $nestedPatchCount = @($nested | Where-Object {
                        [string](Get-PatchObservationProperty $_ "tool_name") -eq "apply_patch" -and
                        [string]::IsNullOrWhiteSpace([string](Get-PatchObservationProperty $_ "namespace" ""))
                    }).Count
                if ($nestedPatchCount -eq 1) { $singlePatchCarrierCount++ }
                if ($nestedPatchCount -gt 1) { $multiPatchCarrierAttemptCount++ }
                for ($nestedIndex = 0; $nestedIndex -lt $nested.Count; $nestedIndex++) {
                    $action = $nested[$nestedIndex]
                    $nestedName = [string](Get-PatchObservationProperty $action "tool_name")
                    $declared.Add([pscustomobject]@{
                            call_id = "${callId}:nested:$nestedIndex"; name = $nestedName
                            action = $action; row = [int]$record.row; source = "taskspace_continuation"
                            batch_index = $batchIndex; post_patch = $false
                        })
                }
                continue
            }
            $declared.Add([pscustomobject]@{
                    call_id = $callId; name = $name; action = $payload; row = [int]$record.row
                    source = "provider_top_level"; batch_index = $batchIndex; post_patch = $false
                })
        }
        $patchSeen = $false
        $requestPatchCount = 0
        foreach ($call in $declared) {
            $isPatch = $call.name -eq "apply_patch"
            if ($isPatch) {
                $patchSeen = $true
                $requestPatchCount++
            } elseif ($patchSeen) {
                $call.post_patch = $true
            }
            $patchText = if ($isPatch) { Get-PatchObservationInput $call.action } else { "" }
            $call | Add-Member -NotePropertyName is_patch -NotePropertyValue $isPatch
            $call | Add-Member -NotePropertyName patch_file_count -NotePropertyValue (Get-PatchObservationFileCount $patchText)
            $calls.Add($call)
            if (-not [string]::IsNullOrWhiteSpace($call.call_id)) { $callById[$call.call_id] = $call }
        }
        $totalRequestPatchCount += $requestPatchCount
        $maxRequestPatchCount = [Math]::Max($maxRequestPatchCount, $requestPatchCount)
        $requestRows.Add([pscustomobject]@{ request_index = $batchIndex + 1; patch_count = $requestPatchCount; rejected = $false })
    }

    $outputById = @{}
    foreach ($record in $records) {
        $type = [string](Get-PatchObservationProperty $record.payload "type")
        if ($type -notin @("function_call_output", "custom_tool_call_output", "mcp_tool_call_output")) { continue }
        $callId = [string](Get-PatchObservationProperty $record.payload "call_id" "")
        if (-not [string]::IsNullOrWhiteSpace($callId)) {
            $outputById[$callId] = [pscustomobject]@{
                row = [int]$record.row; text = Get-PatchObservationOutputText $record.payload
                success = $record.tool_success
            }
        }
    }

    $prepareFailures = 0; $commitFailures = 0; $partialCommits = 0
    $multiFilePatches = 0; $postActions = 0; $postSkipped = 0
    foreach ($call in $calls) {
        if ($call.patch_file_count -gt 1) { $multiFilePatches++ }
        if ($call.post_patch) {
            $postActions++
            if ($outputById.ContainsKey($call.call_id) -and $outputById[$call.call_id].text -match 'skipped_due_to_prior_failure') { $postSkipped++ }
        }
        if (-not $call.is_patch -or -not $outputById.ContainsKey($call.call_id)) { continue }
        $text = [string]$outputById[$call.call_id].text
        if ($text -match 'patch commit failed') {
            $commitFailures++
            if ($text -match 'rollback_status=best_effort_partial') { $partialCommits++ }
        } elseif ($text -notmatch 'Success\. Updated the following files' -and
            $text -notmatch 'request_multiple_apply_patch_calls_not_allowed' -and
            $text -notmatch 'skipped_due_to_') {
            $prepareFailures++
        }
    }

    $rejectedRequests = 0
    foreach ($request in $requestRows) {
        $batchCallIds = @($batches[$request.request_index - 1] | ForEach-Object { [string](Get-PatchObservationProperty $_.payload "call_id" "") })
        $rejected = @($batchCallIds | Where-Object {
                $outputById.ContainsKey($_) -and $outputById[$_].text -match 'request_multiple_apply_patch_calls_not_allowed'
            }).Count -gt 0
        $request.rejected = $rejected
        if ($rejected) { $rejectedRequests++ }
    }

    $readCalls = @($calls | Where-Object { -not [string]::IsNullOrWhiteSpace((Get-PatchObservationReadKey $_.action)) } | Sort-Object row)
    $readTargets = @{}
    $lastVisibleOutput = @{}
    $repeatReads = 0; $readFeedbackVisible = 0
    foreach ($call in $readCalls) {
        $key = Get-PatchObservationReadKey $call.action
        $readTargets[$key] = $true
        if ($lastVisibleOutput.ContainsKey($key) -and [int]$lastVisibleOutput[$key] -lt [int]$call.row) { $repeatReads++ }
        if ($outputById.ContainsKey($call.call_id)) {
            $readFeedbackVisible++
            $lastVisibleOutput[$key] = [int]$outputById[$call.call_id].row
        }
    }
    $readCoverage = if ($readCalls.Count -gt 0) { [Math]::Round($readFeedbackVisible / [double]$readCalls.Count, 4) } else { $null }

    [pscustomobject]@{
        schema_version = "taskspace-patch-observability-v1"
        availability = "measured"
        single_patch_carrier_count = [int]$singlePatchCarrierCount
        multi_patch_carrier_attempt_count = [int]$multiPatchCarrierAttemptCount
        request_patch_count = [int]$totalRequestPatchCount
        max_request_patch_count = [int]$maxRequestPatchCount
        request_multi_patch_attempt_count = [int]@($requestRows | Where-Object { $_.patch_count -gt 1 }).Count
        request_multi_patch_preflight_reject_count = [int]$rejectedRequests
        multi_file_patch_count = [int]$multiFilePatches
        patch_prepare_failure_count = [int]$prepareFailures
        patch_commit_failure_count = [int]$commitFailures
        patch_partial_commit_count = [int]$partialCommits
        post_patch_action_count = [int]$postActions
        post_patch_skipped_count = [int]$postSkipped
        unique_read_target_count = [int]$readTargets.Count
        exact_repeat_read_after_visible_feedback_count = [int]$repeatReads
        read_feedback_visibility_coverage = $readCoverage
        requests = @($requestRows.ToArray())
    }
}

function Add-TaskspacePatchAggregateFields {
    param([System.Collections.IDictionary]$Sum, [object[]]$Selected)
    foreach ($field in @("single_patch_carrier_count", "multi_patch_carrier_attempt_count", "request_patch_count", "request_multi_patch_attempt_count", "request_multi_patch_preflight_reject_count", "multi_file_patch_count", "patch_prepare_failure_count", "patch_commit_failure_count", "patch_partial_commit_count", "post_patch_action_count", "post_patch_skipped_count")) {
        $values = @($Selected | ForEach-Object { Get-PerformanceNumber (Get-PatchObservationProperty $_.patch $field) } | Where-Object { $null -ne $_ })
        $Sum[$field] = if ($values.Count) { [double](($values | Measure-Object -Sum).Sum) } else { $null }
    }
}

function Add-TaskspacePatchObservationMarkdown {
    param([System.Collections.Generic.List[string]]$Lines, [object[]]$Rows)
    $Lines.Add("## Patch lifecycle")
    $Lines.Add("")
    $Lines.Add("| Repeat | Mode | Patch declarations | Max/request | Single carrier | Multi carrier | Multi request | Preflight rejects | Multi-file | Prepare fail | Commit fail | Partial commit | Post actions | Post skipped | Unique reads | Exact repeat reads | Read feedback |")
    $Lines.Add("|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    foreach ($row in $Rows) {
        if ($row.observation_status -eq "skipped" -or $row.patch.availability -ne "measured") {
            $Lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A | N/A |")
        } else {
            $Lines.Add("| $(Format-PerformanceValue $row.repeat) | $($row.logical_mode) | $(Format-PerformanceValue $row.patch.request_patch_count) | $(Format-PerformanceValue $row.patch.max_request_patch_count) | $(Format-PerformanceValue $row.patch.single_patch_carrier_count) | $(Format-PerformanceValue $row.patch.multi_patch_carrier_attempt_count) | $(Format-PerformanceValue $row.patch.request_multi_patch_attempt_count) | $(Format-PerformanceValue $row.patch.request_multi_patch_preflight_reject_count) | $(Format-PerformanceValue $row.patch.multi_file_patch_count) | $(Format-PerformanceValue $row.patch.patch_prepare_failure_count) | $(Format-PerformanceValue $row.patch.patch_commit_failure_count) | $(Format-PerformanceValue $row.patch.patch_partial_commit_count) | $(Format-PerformanceValue $row.patch.post_patch_action_count) | $(Format-PerformanceValue $row.patch.post_patch_skipped_count) | $(Format-PerformanceValue $row.patch.unique_read_target_count) | $(Format-PerformanceValue $row.patch.exact_repeat_read_after_visible_feedback_count) | $(Format-PerformanceValue $row.patch.read_feedback_visibility_coverage percent) |")
        }
    }
    $Lines.Add("")
}
