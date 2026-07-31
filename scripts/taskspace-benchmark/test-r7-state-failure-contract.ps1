$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "lib/r7-five-layer-trace-analysis.ps1")

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function New-NodeStateViolation {
    [pscustomobject]@{
        code = "node_state_invalid"
        subjects = @("reservation-1")
        node_id = "verify"
        canonical_before_transaction = [pscustomobject]@{
            node_present = $true
            state = "waiting"
            unsatisfied_predecessor_ids = @("fix")
        }
        rejected_candidate_at_violation = [pscustomobject]@{
            committed = $false
            state = "completed"
            allowed_states = @("ready", "in_flight")
            unsatisfied_predecessor_ids = @()
        }
    }
}

function New-ControlFailure {
    param([bool]$IncludeViolations = $true)
    $actual = [pscustomobject]@{}
    if ($IncludeViolations) {
        $actual | Add-Member -NotePropertyName violations -NotePropertyValue @(
            New-NodeStateViolation
        )
    }
    [pscustomobject]@{
        schema_version = "TaskSpaceControlResultV2"
        action = "execute"
        status = "state_machine_failed"
        success = $false
        state_commit = $false
        partial_commit = $false
        canonical_revision = 4
        submitted_expected_revision = 4
        committed_revision = $null
        delta = $null
        steps = @()
        read = $null
        error = [pscustomobject]@{
            class = "state_machine"
            code = "node_state_invalid"
            message = "canonical TaskSpace state rejected the submitted action"
            actual = $actual
            expected = [pscustomobject]@{ action = "execute" }
        }
    }
}

$valid = Get-R7StructuredFailureOutcome (New-ControlFailure)
Assert-True $valid.evidence_valid "complete direct state failure was rejected"
Assert-True (
    @($valid.violation_contexts).Count -eq 1
) "direct state failure violations were not preserved"

$missing = Get-R7StructuredFailureOutcome (New-ControlFailure $false)
Assert-True (
    -not $missing.evidence_valid -and
    $missing.failure_class -eq "evidence_unclassified"
) "direct state failure without violations remained valid"

$candidateMissing = New-ControlFailure
$candidateMissing.error.actual.violations[0].
    rejected_candidate_at_violation.PSObject.Properties.Remove("allowed_states")
$incomplete = Get-R7StructuredFailureOutcome $candidateMissing
Assert-True (
    -not $incomplete.evidence_valid -and
    $incomplete.parse_status -eq "incomplete_failure_payload"
) "node state failure without candidate facts remained valid"

$protocol = New-ControlFailure
$protocol.action = $null
$protocol.status = "protocol_failed"
$protocol.canonical_revision = $null
$protocol.submitted_expected_revision = $null
$protocol.error = [pscustomobject]@{
    class = "protocol"
    code = "TASKSPACE_PROTOCOL_FAILURE"
    message = "invalid carrier"
    actual = [pscustomobject]@{ condition = "invalid_carrier" }
    expected = $null
}
Assert-True (
    (Get-R7StructuredFailureOutcome $protocol).evidence_valid
) "complete non-state control failure was rejected"

$missingEnvelope = New-ControlFailure
$missingEnvelope.PSObject.Properties.Remove("steps")
Assert-True (
    -not (Get-R7StructuredFailureOutcome $missingEnvelope).evidence_valid
) "control failure with an incomplete envelope remained valid"

$mismatchedStatus = New-ControlFailure
$mismatchedStatus.status = "protocol_failed"
Assert-True (
    -not (Get-R7StructuredFailureOutcome $mismatchedStatus).evidence_valid
) "control failure with a mismatched status remained valid"

$fractionalRevision = New-ControlFailure
$fractionalRevision.canonical_revision = 1.5
Assert-True (
    -not (Get-R7StructuredFailureOutcome $fractionalRevision).evidence_valid
) "control failure with a fractional revision remained valid"

$revisionMismatch = New-ControlFailure
$revisionMismatch.error.actual | Add-Member `
    -NotePropertyName canonical_revision `
    -NotePropertyValue 99
Assert-True (
    -not (Get-R7StructuredFailureOutcome $revisionMismatch).evidence_valid
) "control failure accepted conflicting canonical revisions"

$absentNodeState = New-ControlFailure
$absentNodeState.error.actual.violations[0].
    canonical_before_transaction.node_present = $false
Assert-True (
    -not (Get-R7StructuredFailureOutcome $absentNodeState).evidence_valid
) "absent canonical node retained a concrete state"

$supplemental = [pscustomobject]@{
    schema_version = "TaskSpaceResponseCommitFailureV3"
    status = "state_rejected"
    success = $false
    state_commit = $false
    canonical_revision = 4
    current_revision = 4
    rejected_candidate_committed = $false
    executed_tool_call_count = 0
    failure_provenance = [pscustomobject]@{
        scope = "provider_response"
        copy_group_id = "provider_response:control"
        zero_dispatch = $true
        affected_call_ids = @("control")
    }
    error = [pscustomobject]@{
        class = "state_machine"
        code = "taskspace_response_state_commit_failed"
        violations = @(New-NodeStateViolation)
    }
}
Assert-True (
    [string]::IsNullOrWhiteSpace(
        (Get-R7SupplementalFailureShapeError $supplemental)
    )
) "complete supplemental state failure was rejected"
$supplementalClassMismatch =
    $supplemental | ConvertTo-Json -Depth 30 | ConvertFrom-Json -Depth 30
$supplementalClassMismatch.error.class = "resource"
Assert-True (
    -not [string]::IsNullOrWhiteSpace(
        (Get-R7SupplementalFailureShapeError $supplementalClassMismatch)
    )
) "supplemental state status accepted resource error.class"
$supplementalProtocol =
    $supplemental | ConvertTo-Json -Depth 30 | ConvertFrom-Json -Depth 30
$supplementalProtocol.status = "protocol_rejected"
$supplementalProtocol.canonical_revision = $null
$supplementalProtocol.PSObject.Properties.Remove("current_revision")
$supplementalProtocol.error.class = "protocol"
$supplementalProtocol.error.code = "taskspace_control_operation_invalid"
$supplementalProtocol.error.PSObject.Properties.Remove("violations")
$supplementalProtocol.error | Add-Member `
    -NotePropertyName detail `
    -NotePropertyValue "invalid operation"
Assert-True (
    [string]::IsNullOrWhiteSpace(
        (Get-R7SupplementalFailureShapeError $supplementalProtocol)
    )
) "complete supplemental protocol failure was rejected"
$supplementalResource =
    $supplementalProtocol | ConvertTo-Json -Depth 30 | ConvertFrom-Json -Depth 30
$supplementalResource.status = "resource_failed"
$supplementalResource.error.class = "resource"
$supplementalResource.error.code = "taskspace_canonical_store_unavailable"
Assert-True (
    [string]::IsNullOrWhiteSpace(
        (Get-R7SupplementalFailureShapeError $supplementalResource)
    )
) "complete supplemental resource failure was rejected"
$supplemental.error.PSObject.Properties.Remove("violations")
Assert-True (
    (Get-R7SupplementalFailureShapeError $supplemental) -like
        "*violations must be a non-empty array"
) "supplemental state failure without violations remained valid"

Write-Output "R7 state failure contract passed."
