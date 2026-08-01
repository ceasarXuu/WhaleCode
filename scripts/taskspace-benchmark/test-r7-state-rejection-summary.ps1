$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/r7-five-layer-trace-analysis.ps1")

$sharedViolations = @(
    [pscustomobject]@{
        code = "node_state_invalid"
        subjects = @("reservation-a")
        node_id = "work"
        canonical_before_transaction = [pscustomobject]@{
            state = "ready"
            unsatisfied_predecessor_ids = @()
        }
        rejected_candidate_at_violation = [pscustomobject]@{
            state = "completed"
            allowed_states = @("ready")
            unsatisfied_predecessor_ids = @("left")
        }
    },
    [pscustomobject]@{
        code = "node_state_invalid"
        subjects = @("reservation-b")
        node_id = "work"
        canonical_before_transaction = [pscustomobject]@{
            state = "ready"
            unsatisfied_predecessor_ids = @("right")
        }
        rejected_candidate_at_violation = [pscustomobject]@{
            state = "completed"
            allowed_states = @("in_flight")
            unsatisfied_predecessor_ids = @()
        }
    },
    [pscustomobject]@{
        code = "node_state_invalid"
        subjects = @("reservation-c")
        node_id = "verify"
        canonical_before_transaction = [pscustomobject]@{
            node_present = $false
            state = ""
        }
        rejected_candidate_at_violation = [pscustomobject]@{
            state = "waiting"
            allowed_states = @("ready", "in_flight")
        }
    }
)
$stateSummary = Get-R7NodeStateRejectionSummary @(
    [pscustomobject]@{
        sample = "state-fixture"
        repeat = 1
        arm = "map-request"
        request_path = @(
            [pscustomobject]@{
                request_index = 1
                calls = @(
                    [pscustomobject]@{
                        call_id = "state-control"
                        zero_dispatch = $true
                        failure_copy_group_id = "provider_response:state"
                        failure_affected_call_ids = @(
                            "state-control",
                            "state-search"
                        )
                        violation_contexts = $sharedViolations
                    },
                    [pscustomobject]@{
                        call_id = "state-search"
                        zero_dispatch = $true
                        failure_copy_group_id = "provider_response:state"
                        failure_affected_call_ids = @(
                            "state-control",
                            "state-search"
                        )
                        violation_contexts = $sharedViolations
                    }
                )
            },
            [pscustomobject]@{
                request_index = 2
                calls = @(
                    [pscustomobject]@{
                        tool = "taskspace_control"
                        control_action = "read_map"
                    }
                )
            }
        )
    }
)
if ([int]$stateSummary.request_count -ne 1 -or
    [int]$stateSummary.violation_count -ne 3 -or
    [int]$stateSummary.next_read_map_request_count -ne 1 -or
    [int]$stateSummary.by_arm[0].request_count -ne 1 -or
    [int]$stateSummary.by_arm[0].violation_count -ne 3 -or
    [int]$stateSummary.by_arm[0].state_pairs.Count -ne 2 -or
    @(
        $stateSummary.by_arm[0].state_pairs |
            Where-Object {
                [string]$_.canonical_state -eq "ready" -and
                [string]$_.candidate_state -eq "completed" -and
                [int]$_.violation_count -eq 2
            }
    ).Count -ne 1 -or
    @(
        $stateSummary.facts |
            Where-Object {
                [string]$_.node_id -eq "work" -and
                @($_.subjects).Count -eq 1
            }
    ).Count -ne 2) {
    throw "Node-state rejection summary did not preserve independent facts and sibling identity"
}
Write-Output "R7 state rejection summary passed."
