function Get-R7NodeStateRejectionSummary {
    param([Parameter(Mandatory = $true)][object[]]$TraceRuns)
    $facts = [Collections.Generic.List[object]]::new()
    foreach ($run in $TraceRuns) {
        $requestPath = @($run.request_path)
        for ($index = 0; $index -lt $requestPath.Count; $index++) {
            $request = $requestPath[$index]
            $seen = @{}
            foreach ($violation in @(
                    $request.calls |
                        ForEach-Object { @($_.violation_contexts) } |
                        Where-Object { [string]$_.code -eq "node_state_invalid" }
                )) {
                $canonical = Get-R7JsonProperty $violation "canonical_before_transaction"
                $candidate = Get-R7JsonProperty $violation "rejected_candidate_at_violation"
                $nodeId = [string](Get-R7JsonProperty $violation "node_id" "")
                $canonicalNodePresent = [bool](
                    Get-R7JsonProperty $canonical "node_present" $true
                )
                $canonicalState = [string](Get-R7JsonProperty $canonical "state" "")
                if (-not $canonicalNodePresent -and
                    [string]::IsNullOrWhiteSpace($canonicalState)) {
                    $canonicalState = "absent"
                }
                $candidateState = [string](Get-R7JsonProperty $candidate "state" "")
                $key = "$nodeId|$canonicalState|$candidateState"
                if ($seen.ContainsKey($key)) { continue }
                $seen[$key] = $true
                $nextRequest = if ($index + 1 -lt $requestPath.Count) {
                    $requestPath[$index + 1]
                } else {
                    $null
                }
                $nextControlAction = @(
                    Get-R7JsonProperty $nextRequest "calls" @() |
                        Where-Object tool -eq "taskspace_control" |
                        ForEach-Object { [string]$_.control_action } |
                        Where-Object { $_ } |
                        Select-Object -First 1
                )
                $facts.Add([pscustomobject]@{
                        sample = [string]$run.sample
                        repeat = [int]$run.repeat
                        arm = [string]$run.arm
                        request_index = [int]$request.request_index
                        node_id = $nodeId
                        canonical_node_present = $canonicalNodePresent
                        canonical_state = $canonicalState
                        candidate_state = $candidateState
                        next_control_action = if ($nextControlAction.Count) {
                            [string]$nextControlAction[0]
                        } else {
                            ""
                        }
                    })
            }
        }
    }
    $requestGroups = @(
        $facts |
            Group-Object sample, repeat, arm, request_index
    )
    $byArm = @(
        $facts |
            Group-Object arm |
            ForEach-Object {
                $rows = @($_.Group)
                $armRequestGroups = @(
                    $rows |
                        Group-Object sample, repeat, request_index
                )
                [pscustomobject]@{
                    arm = [string]$_.Name
                    request_count = $armRequestGroups.Count
                    violation_count = $rows.Count
                    next_read_map_request_count = @(
                        $armRequestGroups |
                            Where-Object {
                                @(
                                    $_.Group |
                                        Where-Object next_control_action -eq "read_map"
                                ).Count
                            }
                    ).Count
                    state_pairs = @(
                        $rows |
                            Group-Object canonical_state, candidate_state |
                            ForEach-Object {
                                [pscustomobject]@{
                                    canonical_state = [string]$_.Group[0].canonical_state
                                    candidate_state = [string]$_.Group[0].candidate_state
                                    violation_count = $_.Count
                                }
                            } |
                            Sort-Object canonical_state, candidate_state
                    )
                }
            } |
            Sort-Object arm
    )
    [pscustomobject]@{
        request_count = $requestGroups.Count
        violation_count = $facts.Count
        next_read_map_request_count = @(
            $requestGroups |
                Where-Object {
                    @(
                        $_.Group |
                            Where-Object next_control_action -eq "read_map"
                    ).Count
                }
        ).Count
        by_arm = $byArm
        facts = @($facts)
    }
}
