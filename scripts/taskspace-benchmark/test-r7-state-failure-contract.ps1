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

$supplemental = [pscustomobject]@{
    schema_version = "TaskSpaceResponseCommitFailureV3"
    status = "state_rejected"
    success = $false
    state_commit = $false
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
$supplemental.error.PSObject.Properties.Remove("violations")
Assert-True (
    (Get-R7SupplementalFailureShapeError $supplemental) -like
        "*violations must be a non-empty array"
) "supplemental state failure without violations remained valid"

Write-Output "R7 state failure contract passed."
