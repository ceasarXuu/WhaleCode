$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "lib/r7-five-layer-trace-analysis.ps1")

function Assert-R71Closure([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function New-R71PrepareJson {
    param(
        [string]$Action = "execute",
        [int64]$RevisionBefore = 1,
        [object[]]$Reservations = @(
            [ordered]@{
                call_index = 0
                call_id = "tool-1"
                node_id = "work"
                tool = "exec_command"
                reservation_id = "reservation:tool-1"
            }
        )
    )
    [ordered]@{
        schema_version = "TaskSpaceResponseCommitV1"
        status = "accepted"
        success = $true
        state_commit = $true
        map_id = "map-1"
        action = $Action
        revision_before = $RevisionBefore
        revision_after = $RevisionBefore + 1
        reserved_actions = $Reservations
    } | ConvertTo-Json -Compress -Depth 20
}

function Invoke-R71PrepareApply {
    param(
        [string]$ControlArguments,
        [string]$Output,
        $ToolSuccess = $true,
        [object[]]$Siblings = @(
            [pscustomobject]@{
                call_id = "tool-1"
                tool = "exec_command"
                arguments = '{"cmd":"true"}'
            }
        )
    )
    $control = ConvertTo-R7CallDescriptor `
        "control" "taskspace_control" $ControlArguments
    $control.request_index = 1
    $calls = @{ control = $control }
    foreach ($sibling in $Siblings) {
        $calls[$sibling.call_id] = ConvertTo-R7CallDescriptor `
            $sibling.call_id $sibling.tool $sibling.arguments
        $calls[$sibling.call_id].request_index = 1
    }
    $observed = Get-R7ResponseItemOutcome `
        ([pscustomobject]@{
            type = "function_call_output"
            call_id = "control"
            output = $Output
        }) `
        ([pscustomobject]@{ toolSuccess = $ToolSuccess })
    Apply-R7ObservedOutcome $calls $observed
    [pscustomobject]@{ control = $control; calls = $calls }
}

$executeArgs =
    '{"action":"execute","expected_revision":1,' +
    '"actions":[{"node_id":"work","tool":"exec_command"}]}'
$actionMismatch = Invoke-R71PrepareApply `
    $executeArgs `
    (New-R71PrepareJson "reopen_map" 1)
Assert-R71Closure (
    -not $actionMismatch.control.evidence_valid -and
    $actionMismatch.control.reason_code -eq
        "response_commit_request_mismatch" -and
    [string]::IsNullOrWhiteSpace(
        [string]$actionMismatch.calls["tool-1"].expected_reservation_id
    )
) "Response-prepare action mismatch mutated sibling attribution"

$matching = Invoke-R71PrepareApply `
    $executeArgs `
    (New-R71PrepareJson "execute" 1)
Assert-R71Closure (
    $matching.control.evidence_valid -and
    $matching.control.reservation_mutated -and
    $matching.control.submitted_expected_revision -eq 1 -and
    $matching.control.carrier_revision_before -eq 1 -and
    $matching.calls["tool-1"].expected_reservation_id -eq
        "reservation:tool-1"
) "Matching response-prepare carrier did not bind its reservation"

$revisionMismatchArgs =
    '{"action":"execute","expected_revision":7,' +
    '"actions":[{"node_id":"work","tool":"exec_command"}]}'
$revisionMismatch = Invoke-R71PrepareApply `
    $revisionMismatchArgs `
    (New-R71PrepareJson "execute" 1)
Assert-R71Closure (
    -not $revisionMismatch.control.evidence_valid -and
    $revisionMismatch.control.reason_code -eq
        "response_commit_request_mismatch" -and
    [string]::IsNullOrWhiteSpace(
        [string]$revisionMismatch.calls["tool-1"].expected_reservation_id
    )
) "Response-prepare revision mismatch mutated sibling attribution"

foreach ($invalidExpectedRevision in @(
        '{"action":"execute","actions":' +
            '[{"node_id":"work","tool":"exec_command"}]}',
        '{"action":"execute","expected_revision":"1","actions":' +
            '[{"node_id":"work","tool":"exec_command"}]}',
        '{"action":"execute","expected_revision":true,"actions":' +
            '[{"node_id":"work","tool":"exec_command"}]}'
    )) {
    $invalidBinding = Invoke-R71PrepareApply `
        $invalidExpectedRevision `
        (New-R71PrepareJson "execute" 1)
    Assert-R71Closure (
        -not $invalidBinding.control.evidence_valid -and
        $invalidBinding.control.reason_code -eq
            "response_commit_request_mismatch" -and
        -not $invalidBinding.control.reservation_mutated -and
        [string]::IsNullOrWhiteSpace(
            [string]$invalidBinding.calls["tool-1"].expected_reservation_id
        )
    ) "Invalid expected_revision was coerced into a response binding"
}

$initializeArgs =
    '{"action":"initialize_and_execute",' +
    '"actions":[{"node_id":"work","tool":"exec_command"}]}'
$initializeMatch = Invoke-R71PrepareApply `
    $initializeArgs `
    (New-R71PrepareJson "initialize_and_execute" 0)
Assert-R71Closure (
    $initializeMatch.control.evidence_valid -and
    $initializeMatch.control.reservation_mutated
) "Valid initialization response commit was not bound"

$missingTransport = Invoke-R71PrepareApply `
    $executeArgs `
    (New-R71PrepareJson) `
    $null
Assert-R71Closure (
    -not $missingTransport.control.evidence_valid -and
    $missingTransport.control.reason_code -eq
        "tool_transport_status_missing" -and
    [string]::IsNullOrWhiteSpace(
        [string]$missingTransport.calls["tool-1"].expected_reservation_id
    )
) "Missing TaskSpace transport status failed open"

$nonBooleanTransport = Invoke-R71PrepareApply `
    $executeArgs `
    (New-R71PrepareJson) `
    "true"
Assert-R71Closure (
    -not $nonBooleanTransport.control.evidence_valid -and
    $nonBooleanTransport.control.reason_code -eq
        "tool_transport_status_missing"
) "Non-boolean TaskSpace transport status failed open"

$twoReservations = @(
    [ordered]@{
        call_index = 0
        call_id = "tool-1"
        node_id = "work-1"
        tool = "exec_command"
        reservation_id = "reservation:tool-1"
    }
    [ordered]@{
        call_index = 1
        call_id = "tool-2"
        node_id = "work-2"
        tool = "mcp__domain__lookup"
        reservation_id = "reservation:tool-2"
    }
)
$twoArgs =
    '{"action":"execute","expected_revision":1,"actions":[' +
    '{"node_id":"work-1","tool":"exec_command"},' +
    '{"node_id":"work-2","tool":"mcp__domain__lookup"}]}'
$siblings = @(
    [pscustomobject]@{
        call_id = "tool-1"
        tool = "exec_command"
        arguments = '{"cmd":"true"}'
    }
    [pscustomobject]@{
        call_id = "tool-2"
        tool = "exec_command"
        arguments = '{"cmd":"true"}'
    }
)
$lateMismatch = Invoke-R71PrepareApply `
    $twoArgs `
    (New-R71PrepareJson "execute" 1 $twoReservations) `
    $true `
    $siblings
Assert-R71Closure (
    -not $lateMismatch.control.evidence_valid -and
    [string]::IsNullOrWhiteSpace(
        [string]$lateMismatch.calls["tool-1"].expected_reservation_id
    ) -and
    [string]::IsNullOrWhiteSpace(
        [string]$lateMismatch.calls["tool-2"].expected_reservation_id
    )
) "Late reservation mismatch partially mutated attribution"

$caseDrift = (
    New-R71PrepareJson
).Replace("TaskSpaceResponseCommitV1", "taskspaceresponsecommitv1")
$caseMismatch = Invoke-R71PrepareApply $executeArgs $caseDrift
Assert-R71Closure (
    -not $caseMismatch.control.evidence_valid
) "Case-drifted TaskSpace carrier identity was accepted"

function Assert-R71DirectControlFailure {
    param(
        [string]$Output,
        [string]$ExpectedClass,
        [string]$ExpectedCode
    )
    $call = ConvertTo-R7CallDescriptor `
        "direct-control" `
        "taskspace_control" `
        '{"action":"execute","expected_revision":1,"actions":[]}'
    $call.request_index = 1
    $calls = @{ "direct-control" = $call }
    $observed = Get-R7ResponseItemOutcome `
        ([pscustomobject]@{
            type = "function_call_output"
            call_id = "direct-control"
            output = $Output
        }) `
        ([pscustomobject]@{ toolSuccess = $false })
    Apply-R7ObservedOutcome $calls $observed
    Assert-R71Closure (
        $call.evidence_valid -and
        $call.failure_class -ceq $ExpectedClass -and
        $call.failure_code -ceq $ExpectedCode -and
        $call.reason_code -ceq $ExpectedCode
    ) "Direct taskspace_control failure carrier was misclassified"
}

$sequenceFailure = [ordered]@{
    schema_version = "ToolSequencePreflightResultV3"
    status = "protocol_failed"
    success = $false
    state_commit = $false
    failure_provenance = [ordered]@{
        scope = "provider_response"
        copy_group_id = "provider_response:direct-control"
        zero_dispatch = $true
        affected_call_ids = @("direct-control")
    }
    error = [ordered]@{
        class = "protocol"
        code = "taskspace_action_count_mismatch"
    }
} | ConvertTo-Json -Compress -Depth 20
Assert-R71DirectControlFailure `
    $sequenceFailure `
    "tool_sequence_protocol" `
    "taskspace_action_count_mismatch"

$responseStateFailure = [ordered]@{
    schema_version = "TaskSpaceResponseCommitFailureV3"
    status = "state_rejected"
    success = $false
    state_commit = $false
    canonical_revision = 4
    current_revision = 4
    rejected_candidate_committed = $false
    executed_tool_call_count = 0
    failure_provenance = [ordered]@{
        scope = "provider_response"
        copy_group_id = "provider_response:direct-control"
        zero_dispatch = $true
        affected_call_ids = @("direct-control")
    }
    error = [ordered]@{
        class = "state_machine"
        code = "taskspace_response_state_commit_failed"
        violations = @(
            [ordered]@{
                code = "stale_revision"
                subjects = @("map")
            }
        )
    }
} | ConvertTo-Json -Compress -Depth 20
Assert-R71DirectControlFailure `
    $responseStateFailure `
    "taskspace_state_machine" `
    "taskspace_response_state_commit_failed"

Write-Output "R71-01 closure counterexamples passed."
