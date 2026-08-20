function Set-R7ExpectedReservations {
    param([hashtable]$CallsById, $ControlCall, $Payload)
    $assignments = [Collections.Generic.List[object]]::new()
    $reservations = @(Get-R7JsonProperty $Payload "reserved_actions" @())
    $declaredActions = @($ControlCall.declared_actions)
    if ($reservations.Count -ne $declaredActions.Count) {
        return "reservation count does not match the control manifest"
    }
    for ($index = 0; $index -lt $reservations.Count; $index++) {
        $reservation = $reservations[$index]
        $callId = [string](Get-R7JsonProperty $reservation "call_id" "")
        $reservationId = [string](Get-R7JsonProperty $reservation "reservation_id" "")
        if ([string]::IsNullOrWhiteSpace($callId) -or
            [string]::IsNullOrWhiteSpace($reservationId) -or
            -not $CallsById.ContainsKey($callId)) {
            return "reservation identity has no observed sibling Tool call"
        }
        $targetCall = $CallsById[$callId]
        if ([int]$ControlCall.request_index -lt 1 -or
            [int]$targetCall.request_index -ne [int]$ControlCall.request_index) {
            return "reservation crosses the provider request identity"
        }
        $declared = $declaredActions[$index]
        if ([string](Get-R7JsonProperty $reservation "tool" "") -cne
                [string]$targetCall.tool -or
            [string](Get-R7JsonProperty $reservation "tool" "") -cne
                [string]$declared.tool -or
            [string](Get-R7JsonProperty $reservation "node_id" "") -cne
                [string]$declared.node_id) {
            return "reservation facts do not match the control manifest"
        }
        $assignments.Add([pscustomobject]@{
                call = $targetCall
                reservation_id = $reservationId
            })
    }
    foreach ($assignment in $assignments) {
        $assignment.call.expected_reservation_id = $assignment.reservation_id
    }
    ""
}

function Get-R7FinalControlResultRequestBindingError {
    param($ControlCall, $Payload)
    if ([string]$ControlCall.tool -cne "taskspace_control") {
        return "final control result did not originate from taskspace_control"
    }
    if ([string]$ControlCall.argument_parse_status -cne "valid_json") {
        return "taskspace_control request arguments were not valid strict JSON"
    }
    $submittedAction = [string]$ControlCall.control_action
    $carrierAction = [string](Get-R7JsonProperty $Payload "action" "")
    if ($submittedAction -cne $carrierAction) {
        return "final control result action does not match the submitted action"
    }
    $carrierRevision = ConvertTo-R7NonnegativeInt64Fact (
        Get-R7JsonProperty $Payload "canonical_revision"
    )
    if ($null -eq $carrierRevision) {
        return "final control result canonical_revision is not an exact nonnegative Int64"
    }
    $hasExpectedRevision =
        $ControlCall.submitted_expected_revision_present -is [bool] -and
        [bool]$ControlCall.submitted_expected_revision_present
    if ($submittedAction -ceq "initialize_and_execute") {
        if ($hasExpectedRevision -or $carrierRevision -lt 1) {
            return "initialize final control result has an invalid revision binding"
        }
        return ""
    }
    if ($submittedAction -cnotin @("execute", "reopen_map")) {
        return "final control result action is not response-executable"
    }
    if (-not $hasExpectedRevision) {
        return "taskspace_control request is missing expected_revision"
    }
    $submittedRevision = ConvertTo-R7NonnegativeInt64Fact (
        $ControlCall.submitted_expected_revision
    )
    if ($null -eq $submittedRevision -or $carrierRevision -le $submittedRevision) {
        return "final control result canonical_revision did not advance beyond expected_revision"
    }
    ""
}
