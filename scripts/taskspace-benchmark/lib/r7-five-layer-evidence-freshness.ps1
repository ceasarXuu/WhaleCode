function Resolve-R7EvidencePath {
    param([string]$RepoRoot, [string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }
    [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $Path))
}

. (Join-Path $PSScriptRoot "request-facts.ps1")
. (Join-Path $PSScriptRoot "r7-request-facts-provenance.ps1")

function Get-R7EvidenceSha256 {
    param([string]$Path)
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Add-R7EvidenceFinding {
    param(
        [System.Collections.Generic.List[object]]$Findings,
        [string]$Code,
        [string]$Message,
        [string]$Path = ""
    )
    if (@($Findings | Where-Object { [string]$_.stable_code -eq $Code -and [string]$_.path -eq $Path }).Count -gt 0) {
        return
    }
    $Findings.Add([pscustomobject]@{
            severity = "fail"
            stable_code = $Code
            message = $Message
            path = $Path
        }) | Out-Null
}

function Read-R7EvidenceJson {
    param(
        [string]$Path,
        [System.Collections.Generic.List[object]]$Findings,
        [string]$MissingCode,
        [string]$InvalidCode
    )
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Add-R7EvidenceFinding $Findings $MissingCode "Required evidence file is missing." $Path
        return $null
    }
    try {
        Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json -Depth 100
    } catch {
        Add-R7EvidenceFinding $Findings $InvalidCode ([string]$_.Exception.Message) $Path
        $null
    }
}

function Read-R7EvidenceJsonLines {
    param(
        [string]$Path,
        [System.Collections.Generic.List[object]]$Findings
    )
    $items = [System.Collections.Generic.List[object]]::new()
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Add-R7EvidenceFinding $Findings "provider_trace_missing" "Provider trace is missing." $Path
        return @()
    }
    $lineNumber = 0
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $Path) {
        $lineNumber++
        if ([string]::IsNullOrWhiteSpace([string]$line)) { continue }
        try {
            $items.Add(($line | ConvertFrom-Json -Depth 100)) | Out-Null
        } catch {
            Add-R7EvidenceFinding $Findings "provider_trace_invalid_json" "Invalid JSON at line $lineNumber." $Path
        }
    }
    @($items.ToArray())
}

function Get-R7RolloutControlSummary {
    param([string]$Path, [System.Collections.Generic.List[object]]$Findings)
    $summary = [ordered]@{
        control_calls = 0; v2_results = 0; control_failures = 0; preflight_failures = 0
        ordinary_gate_failures = 0; committed_controls = 0; state_commit_count = 0
        initialize_commits_with_node_bound = 0; rejected_without_commit = 0
        complete_then_continue_calls = 0; finish_map_calls = 0
        transition_node_calls = 0; read_map_calls = 0; bind_node_calls = 0
    }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Add-R7EvidenceFinding $Findings "rollout_missing" "TaskSpace rollout is missing." $Path
        return [pscustomobject]$summary
    }
    $lineNumber = 0
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $Path) {
        $lineNumber++
        if ([string]::IsNullOrWhiteSpace([string]$line)) { continue }
        try { $event = $line | ConvertFrom-Json -Depth 100 } catch {
            Add-R7EvidenceFinding $Findings "rollout_invalid_json" "Invalid rollout JSON at line $lineNumber." $Path
            continue
        }
        if ([string]$event.payload.map_event_type -ne "task_context_event_recorded") { continue }
        $raw = $event.payload.rawPayload
        if ([string]$event.payload.eventType -eq "function_call" -and [string]$raw.name -eq "taskspace_control") {
            $summary.control_calls++
            try {
                $arguments = ([string]$raw.arguments) | ConvertFrom-Json -Depth 100
                switch ([string]$arguments.action) {
                    "complete_then_continue" { $summary.complete_then_continue_calls++ }
                    "finish_map" { $summary.finish_map_calls++ }
                    "transition_node" { $summary.transition_node_calls++ }
                    "read_map" { $summary.read_map_calls++ }
                    "bind_node" { $summary.bind_node_calls++ }
                }
            } catch {
                Add-R7EvidenceFinding $Findings "control_arguments_invalid_json" "TaskSpace control arguments are invalid JSON at line $lineNumber." $Path
            }
            continue
        }
        if ([string]$event.payload.eventType -ne "function_call_output") { continue }
        try { $output = ([string]$raw.output) | ConvertFrom-Json -Depth 100 } catch { continue }
        if ([string]$output.schema_version -eq "TaskSpaceControlResultV2") {
            $summary.v2_results++
            if (-not [bool]$output.success) { $summary.control_failures++ }
            if ([bool]$output.state_commit) {
                $summary.committed_controls++
                $summary.state_commit_count++
            }
            if (-not [bool]$output.success -and -not [bool]$output.state_commit) {
                $summary.rejected_without_commit++
            }
            if ([string]$output.error.code -eq "TASKSPACE_REQUIRED_SIBLING_MISSING") {
                $summary.preflight_failures++
            }
            if ([string]$output.action -eq "initialize_map" -and [bool]$output.success -and
                @($output.steps | Where-Object { [string]$_.kind -eq "node_bound" }).Count -gt 0) {
                $summary.initialize_commits_with_node_bound++
            }
        } elseif ([string]$output.schema_version -eq "TaskSpaceGateResultV1" -and -not [bool]$output.success) {
            $summary.ordinary_gate_failures++
        }
    }
    [pscustomobject]$summary
}

function Test-R7StandardTraceIdentity {
    param($Trace, $ExpectedBase, [string]$Path, [System.Collections.Generic.List[object]]$Findings)
    $base = $Trace.base_instructions_identity
    if ($null -eq $base -or [int]$base.count -ne 1 -or [string]$base.profile -ne "standard" -or
        [string]$base.version -ne [string]$ExpectedBase.version -or [string]$base.sha256 -ne [string]$ExpectedBase.sha256 -or
        -not [bool]$base.matches_current_contract) {
        Add-R7EvidenceFinding $Findings "standard_base_identity_mismatch" "Standard provider trace does not match the current Standard Base identity." $Path
    }
    if ([int]$Trace.taskspace_core_protocol_identity.count -ne 0 -or
        [int]$Trace.taskspace_contract_manifest_identity.count -ne 0 -or
        [int]$Trace.taskspace_wire_contract_identity.map_handle_count -ne 0) {
        Add-R7EvidenceFinding $Findings "standard_taskspace_injection" "Standard provider trace contains TaskSpace identity or handle content." $Path
    }
}

function Test-R7TaskspaceTraceIdentity {
    param(
        $Trace,
        $ExpectedBase,
        $ExpectedCore,
        [string]$ManifestVersion,
        [string]$ManifestSha256,
        [string]$Path,
        [System.Collections.Generic.List[object]]$Findings
    )
    $base = $Trace.base_instructions_identity
    if ($null -eq $base -or [int]$base.count -ne 1 -or [string]$base.profile -ne "taskspace" -or
        [string]$base.version -ne [string]$ExpectedBase.version -or [string]$base.sha256 -ne [string]$ExpectedBase.sha256 -or
        -not [bool]$base.matches_current_contract) {
        Add-R7EvidenceFinding $Findings "taskspace_base_identity_mismatch" "TaskSpace provider trace does not match the current TaskSpace Base identity." $Path
    }
    $core = $Trace.taskspace_core_protocol_identity
    if ($null -eq $core -or [int]$core.count -ne 1 -or [int]$core.message_index -ne 1 -or
        [string]$core.version -ne [string]$ExpectedCore.version -or [string]$core.sha256 -ne [string]$ExpectedCore.sha256 -or
        -not [bool]$core.matches_current_contract) {
        Add-R7EvidenceFinding $Findings "taskspace_core_identity_mismatch" "TaskSpace provider trace does not match the current L2 identity." $Path
    }
    $manifest = $Trace.taskspace_contract_manifest_identity
    if ($null -eq $manifest -or [int]$manifest.count -ne 1 -or [string]$manifest.version -ne $ManifestVersion -or
        [string]$manifest.sha256 -ne $ManifestSha256 -or -not [bool]$manifest.matches_current_contract) {
        Add-R7EvidenceFinding $Findings "taskspace_manifest_identity_mismatch" "TaskSpace provider trace does not match the current production manifest identity." $Path
    }
    $wire = $Trace.taskspace_wire_contract_identity
    if ($null -eq $wire -or [int]$wire.system_message_count -ne 2 -or [int]$wire.expected_system_message_count -ne 2 -or
        [int]$wire.map_handle_count -ne 1 -or [string]$wire.map_handle_wire_role -ne "user" -or
        -not [bool]$wire.map_handle_is_request_tail -or -not [bool]$wire.matches_current_contract) {
        Add-R7EvidenceFinding $Findings "taskspace_wire_identity_mismatch" "TaskSpace provider carrier shape does not match the current contract." $Path
    }
}

function Test-R7FiveLayerEvidenceFreshness {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$WhaleBin,
        [Parameter(Mandatory = $true)][string]$ResultPath,
        [Parameter(Mandatory = $true)][string[]]$RunRoots
    )
    $repo = [System.IO.Path]::GetFullPath($RepoRoot)
    $findings = [System.Collections.Generic.List[object]]::new()
    $baseContractPath = Join-Path $repo "benchmarks/taskspace/r7/base-instructions-contract.json"
    $manifestPath = Join-Path $repo "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json"
    $baseContract = Read-R7EvidenceJson $baseContractPath $findings "base_contract_missing" "base_contract_invalid"
    $manifest = Read-R7EvidenceJson $manifestPath $findings "production_manifest_missing" "production_manifest_invalid"
    $resultFullPath = Resolve-R7EvidencePath $repo $ResultPath
    $result = Read-R7EvidenceJson $resultFullPath $findings "result_missing" "result_invalid"
    $binaryPath = Resolve-R7EvidencePath $repo $WhaleBin
    $binarySha = ""
    if (Test-Path -LiteralPath $binaryPath -PathType Leaf) {
        $binarySha = Get-R7EvidenceSha256 $binaryPath
    } else {
        Add-R7EvidenceFinding $findings "binary_missing" "Candidate Whale binary is missing." $binaryPath
    }
    $sourceCommit = ((& git -C $repo log -1 --format=%H -- third_party/codex-cli 2>$null) | Select-Object -First 1).Trim()
    if ([string]::IsNullOrWhiteSpace($sourceCommit)) {
        Add-R7EvidenceFinding $findings "source_commit_missing" "Cannot resolve the current Codex source commit." $repo
    }
    $attestationPath = "$binaryPath.build-attestation.json"
    $attestation = Read-R7EvidenceJson $attestationPath $findings "binary_attestation_missing" "binary_attestation_invalid"
    if ($attestation) {
        if ([string]$attestation.status -ne "pass" -or [string]$attestation.whale_binary_sha256 -ne $binarySha) {
            Add-R7EvidenceFinding $findings "binary_attestation_sha_mismatch" "Binary attestation does not match the candidate binary." $attestationPath
        }
        if ([string]$attestation.codex_source_latest_commit -ne $sourceCommit) {
            Add-R7EvidenceFinding $findings "binary_attestation_source_mismatch" "Binary attestation does not match the current Codex source commit." $attestationPath
        }
    }
    $manifestSha = if (Test-Path -LiteralPath $manifestPath -PathType Leaf) { Get-R7EvidenceSha256 $manifestPath } else { "" }
    if ($result -and $baseContract -and $manifest) {
        if ([string]$result.binary.sha256 -ne $binarySha -or [string]$result.binary.attested_codex_source_commit -ne $sourceCommit) {
            Add-R7EvidenceFinding $findings "result_binary_identity_mismatch" "Result binary identity is not fresh for the current candidate." $resultFullPath
        }
        foreach ($profile in @("standard", "taskspace")) {
            $actual = $result.contracts."${profile}_base"
            $expected = $baseContract.profiles.$profile
            if ($null -eq $actual -or [string]$actual.version -ne [string]$expected.version -or [string]$actual.sha256 -ne [string]$expected.sha256) {
                Add-R7EvidenceFinding $findings "result_${profile}_base_identity_mismatch" "Result does not record the current $profile Base identity." $resultFullPath
            }
        }
        $resultCore = $result.contracts.taskspace_core_protocol
        if ([string]$resultCore.version -ne [string]$baseContract.taskspace_core_protocol.version -or
            [string]$resultCore.sha256 -ne [string]$baseContract.taskspace_core_protocol.sha256) {
            Add-R7EvidenceFinding $findings "result_core_identity_mismatch" "Result does not record the current L2 identity." $resultFullPath
        }
        $resultManifest = $result.contracts.production_manifest
        if ([string]$resultManifest.version -ne [string]$manifest.manifest_version -or [string]$resultManifest.sha256 -ne $manifestSha) {
            Add-R7EvidenceFinding $findings "result_manifest_identity_mismatch" "Result does not record the current manifest identity." $resultFullPath
        }
    }
    $runSummaries = [System.Collections.Generic.List[object]]::new()
    $totals = [ordered]@{
        standard_provider_requests = 0; taskspace_provider_requests = 0
        control_calls = 0; v2_results = 0; control_failures = 0; preflight_failures = 0
        ordinary_gate_failures = 0; committed_controls = 0; state_commit_count = 0
        initialize_commits_with_node_bound = 0; rejected_without_commit = 0
        complete_then_continue_calls = 0; finish_map_calls = 0
        transition_node_calls = 0; read_map_calls = 0; bind_node_calls = 0
    }
    foreach ($runRootInput in $RunRoots) {
        $runRoot = Resolve-R7EvidencePath $repo $runRootInput
        $healthPath = Join-Path $runRoot "whale-binary-preflight-health.json"
        $health = Read-R7EvidenceJson $healthPath $findings "run_binary_health_missing" "run_binary_health_invalid"
        if ($health -and ([string]$health.status -ne "pass" -or [string]$health.build_attestation_status -ne "pass" -or
                [string]$health.whale_binary_sha256 -ne $binarySha -or [string]$health.codex_source_latest_commit.hash -ne $sourceCommit)) {
            Add-R7EvidenceFinding $findings "run_binary_identity_mismatch" "Run binary health is not attested for the current candidate." $healthPath
        }
        $standardRequests = 0
        $taskspaceRequests = 0
        $control = $null
        foreach ($pairDir in @(Get-ChildItem -LiteralPath $runRoot -Directory -Filter "pair-*" -ErrorAction SilentlyContinue)) {
            $modePath = Join-Path $pairDir.FullName "logical-mode-map.json"
            $modeMap = Read-R7EvidenceJson $modePath $findings "logical_mode_map_missing" "logical_mode_map_invalid"
            if (-not $modeMap) { continue }
            foreach ($side in @("left", "right")) {
                $mode = [string]$modeMap.$side
                $artifactDir = Join-Path $pairDir.FullName "$side/artifacts"
                $tracePath = Join-Path $artifactDir "provider-wire-trace.jsonl"
                $traces = @(Read-R7EvidenceJsonLines $tracePath $findings)
                $payloadTraces = @($traces | Where-Object { [string]$_.status -eq "payload_captured" })
                if ($payloadTraces.Count -eq 0) {
                    Add-R7EvidenceFinding $findings "provider_trace_empty" "Provider trace contains no captured requests." $tracePath
                }
                foreach ($trace in $payloadTraces) {
                    if ($mode -eq "standard") {
                        Test-R7StandardTraceIdentity $trace $baseContract.profiles.standard $tracePath $findings
                    } elseif ($mode -eq "taskspace") {
                        Test-R7TaskspaceTraceIdentity $trace $baseContract.profiles.taskspace $baseContract.taskspace_core_protocol ([string]$manifest.manifest_version) $manifestSha $tracePath $findings
                    } else {
                        Add-R7EvidenceFinding $findings "logical_mode_unknown" "Unsupported logical mode: $mode" $modePath
                    }
                }
                $requestFacts = Test-R7RequestFactsFreshness $artifactDir $findings
                if ($requestFacts) {
                    $requestCount = if ([string]$requestFacts.availability.boundary -eq "measured") {
                        [int64]$requestFacts.summary.boundary_request_count
                    } else {
                        Add-R7EvidenceFinding $findings "request_facts_count_unavailable" "Canonical Provider request count is unavailable." (Join-Path $artifactDir "request-facts.json")
                        $null
                    }
                    if ($null -ne $requestCount -and $mode -eq "standard") { $standardRequests += $requestCount }
                    elseif ($null -ne $requestCount -and $mode -eq "taskspace") { $taskspaceRequests += $requestCount }
                }
            }
            $taskspaceSide = if ([string]$modeMap.left -eq "taskspace") { "left" } elseif ([string]$modeMap.right -eq "taskspace") { "right" } else { "" }
            if (-not [string]::IsNullOrWhiteSpace($taskspaceSide)) {
                $pairControl = Get-R7RolloutControlSummary (Join-Path $pairDir.FullName "$taskspaceSide/artifacts/rollout.jsonl") $findings
                if ($null -eq $control) {
                    $control = $pairControl
                } else {
                    foreach ($field in @($totals.Keys | Where-Object { $_ -notmatch "provider_requests" })) {
                        $control.$field = [int]$control.$field + [int]$pairControl.$field
                    }
                }
            }
        }
        if ($standardRequests -eq 0 -or $taskspaceRequests -eq 0) {
            Add-R7EvidenceFinding $findings "paired_trace_coverage_missing" "Run does not contain both Standard and TaskSpace provider traces." $runRoot
        }
        if ($null -eq $control) { $control = [pscustomobject]@{} }
        $matchingResultRuns = if ($result) { @($result.runs | Where-Object { (Resolve-R7EvidencePath $repo ([string]$_.run_root)) -eq $runRoot }) } else { @() }
        if ($matchingResultRuns.Count -ne 1) {
            Add-R7EvidenceFinding $findings "result_run_root_mismatch" "Result does not reference this run root exactly once." $resultFullPath
        } else {
            $resultRun = $matchingResultRuns[0]
            if ([int]$resultRun.standard.provider_requests -ne $standardRequests) {
                Add-R7EvidenceFinding $findings "result_standard_request_count_mismatch" "Result Standard request count differs from canonical request facts." $resultFullPath
            }
            if ([int]$resultRun.taskspace.provider_requests -ne $taskspaceRequests) {
                Add-R7EvidenceFinding $findings "result_taskspace_request_count_mismatch" "Result TaskSpace request count differs from canonical request facts." $resultFullPath
            }
            foreach ($field in @("control_calls", "control_failures", "preflight_failures", "ordinary_gate_failures", "committed_controls", "state_commit_count")) {
                if ($null -eq $resultRun.taskspace.PSObject.Properties[$field] -or [int]$resultRun.taskspace.$field -ne [int]$control.$field) {
                    Add-R7EvidenceFinding $findings "result_taskspace_${field}_mismatch" "Result TaskSpace $field differs from raw rollout." $resultFullPath
                }
            }
        }
        $totals.standard_provider_requests += $standardRequests
        $totals.taskspace_provider_requests += $taskspaceRequests
        foreach ($field in @($totals.Keys | Where-Object { $_ -notmatch "provider_requests" })) {
            $totals.$field += [int]$control.$field
        }
        $runSummaries.Add([pscustomobject]@{
                run_root = $runRootInput
                standard_provider_requests = $standardRequests
                taskspace_provider_requests = $taskspaceRequests
                control = $control
            }) | Out-Null
    }
    if ($result) {
        $acceptance = $result.repair_acceptance
        $aggregateChecks = [ordered]@{
            "b1_static_system_handle.taskspace_requests" = $totals.taskspace_provider_requests
            "b2_result_capability_mismatch.control_results" = $totals.control_calls
            "b2_result_capability_mismatch.v2_control_results" = $totals.v2_results
            "b2_result_capability_mismatch.non_v2_control_results" = ($totals.control_calls - $totals.v2_results)
            "b2_result_capability_mismatch.initialize_commits_with_node_bound" = $totals.initialize_commits_with_node_bound
            "b2_result_capability_mismatch.rejected_results_with_state_commit_false" = $totals.rejected_without_commit
            "h4_observability.control_calls" = $totals.control_calls
            "h4_observability.control_failures" = $totals.control_failures
            "h4_observability.preflight_failures" = $totals.preflight_failures
            "h4_observability.ordinary_gate_failures" = $totals.ordinary_gate_failures
            "h4_observability.committed_controls" = $totals.committed_controls
            "h4_observability.graph_revision_commits" = $totals.committed_controls
            "h4_observability.state_commit_count" = $totals.state_commit_count
            "h6_direct_actions.nested_transition_calls" = $totals.transition_node_calls
            "h6_direct_actions.direct_complete_then_continue_calls" = $totals.complete_then_continue_calls
            "h6_direct_actions.direct_finish_map_calls" = $totals.finish_map_calls
            "h7_binding_feedback.read_map_calls" = $totals.read_map_calls
            "h7_binding_feedback.redundant_bind_calls" = $totals.bind_node_calls
        }
        foreach ($path in $aggregateChecks.Keys) {
            $segments = $path -split "\."
            $actual = $acceptance.($segments[0]).($segments[1])
            if ($null -eq $actual -or [int]$actual -ne [int]$aggregateChecks[$path]) {
                Add-R7EvidenceFinding $findings "result_aggregate_count_mismatch" "Result aggregate $path differs from raw evidence." $resultFullPath
            }
        }
    }
    [pscustomobject]@{
        schema_version = "r7-five-layer-evidence-freshness-v1"
        status = if ($findings.Count -eq 0) { "pass" } else { "fail" }
        current_codex_source_commit = $sourceCommit
        binary = [pscustomobject]@{ path = $WhaleBin; sha256 = $binarySha; attestation_path = $attestationPath }
        contracts = [pscustomobject]@{
            standard_base = $baseContract.profiles.standard
            taskspace_base = $baseContract.profiles.taskspace
            taskspace_core_protocol = $baseContract.taskspace_core_protocol
            production_manifest = [pscustomobject]@{ version = [string]$manifest.manifest_version; sha256 = $manifestSha }
        }
        runs = @($runSummaries.ToArray())
        findings = @($findings.ToArray())
        generated_at = (Get-Date).ToString("o")
    }
}
