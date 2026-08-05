param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/r7-five-layer-evidence-freshness.ps1")
if ([string]::IsNullOrWhiteSpace($RunRoot)) {
    $RunRoot = Join-Path $repoRoot "target/r7-five-layer-evidence-freshness-selftest"
}
$fixtureRoot = Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff")
$runDir = Join-Path $fixtureRoot "sample/run-001"
$pairDir = Join-Path $runDir "pair-001"
New-Item -ItemType Directory -Path (Join-Path $pairDir "left/artifacts") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $pairDir "right/artifacts") -Force | Out-Null

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function Write-FixtureJson([string]$Path, $Value) {
    $Value | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $Path -Encoding UTF8
}

$baseContract = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $repoRoot "benchmarks/taskspace/r7/base-instructions-contract.json") | ConvertFrom-Json -Depth 100
$manifestPath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json"
$manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 100
$manifestSha = Get-R7EvidenceSha256 $manifestPath
$sourceCommit = ((& git -C $repoRoot log -1 --format=%H -- third_party/codex-cli) | Select-Object -First 1).Trim()
$binaryPath = Join-Path $fixtureRoot "whale"
"fixture whale" | Set-Content -LiteralPath $binaryPath -Encoding ASCII
$binarySha = Get-R7EvidenceSha256 $binaryPath
$attestationPath = "$binaryPath.build-attestation.json"
Write-FixtureJson $attestationPath ([ordered]@{
        schema_version = 1
        status = "pass"
        repo_root = $repoRoot
        current_git_head = $sourceCommit
        codex_source_latest_commit = $sourceCommit
        whale_binary_sha256 = $binarySha
    })
Write-FixtureJson (Join-Path $runDir "whale-binary-preflight-health.json") ([ordered]@{
        status = "pass"
        run_validity = "valid"
        build_attestation_status = "pass"
        whale_binary_sha256 = $binarySha
        codex_source_latest_commit = @{ hash = $sourceCommit }
    })
Write-FixtureJson (Join-Path $pairDir "logical-mode-map.json") ([ordered]@{ repeat = 1; left = "standard"; right = "taskspace" })

$standardTrace = [ordered]@{
    schema_version = "provider-chat-wire-trace-v10"
    status = "payload_captured"
    request_id = "standard-request-1"; logical_request_id = "standard-logical-1"; attempt_seq = 1
    request_index = 1; provider_payload_sha256 = ("1" * 64) -join ""
    base_instructions_identity = [ordered]@{
        count = 1; profile = "standard"; version = $baseContract.profiles.standard.version
        sha256 = $baseContract.profiles.standard.sha256; matches_current_contract = $true
    }
    taskspace_core_protocol_identity = @{ count = 0 }
    taskspace_contract_manifest_identity = @{ count = 0 }
    taskspace_wire_contract_identity = @{ map_handle_count = 0 }
}
$taskspaceTrace = [ordered]@{
    schema_version = "provider-chat-wire-trace-v10"
    status = "payload_captured"
    request_id = "taskspace-request-1"; logical_request_id = "taskspace-logical-1"; attempt_seq = 1
    request_index = 1; provider_payload_sha256 = ("2" * 64) -join ""
    base_instructions_identity = [ordered]@{
        count = 1; profile = "taskspace"; version = $baseContract.profiles.taskspace.version
        sha256 = $baseContract.profiles.taskspace.sha256; matches_current_contract = $true
    }
    taskspace_core_protocol_identity = [ordered]@{
        count = 1; message_index = 1; version = $baseContract.taskspace_core_protocol.version
        sha256 = $baseContract.taskspace_core_protocol.sha256; matches_current_contract = $true
    }
    taskspace_contract_manifest_identity = [ordered]@{
        count = 1; version = $manifest.manifest_version; sha256 = $manifestSha; matches_current_contract = $true
    }
    taskspace_wire_contract_identity = [ordered]@{
        system_message_count = 2; expected_system_message_count = 2; map_handle_count = 1
        map_handle_wire_role = "user"; map_handle_is_request_tail = $true; matches_current_contract = $true
    }
}
function New-Terminal([string]$RequestId, [string]$LogicalId) {
    [ordered]@{
        schema_version = "provider-chat-wire-trace-v10"; status = "response_completed"
        request_id = $RequestId; logical_request_id = $LogicalId; attempt_seq = 1
        input_tokens = 100; cached_input_tokens = 20; output_tokens = 10
        reasoning_output_tokens = 2; total_tokens = 110
    } | ConvertTo-Json -Compress
}
$standardTerminal = New-Terminal "standard-request-1" "standard-logical-1"
$taskspaceTerminal = New-Terminal "taskspace-request-1" "taskspace-logical-1"
@(($standardTrace | ConvertTo-Json -Compress -Depth 100), $standardTerminal) | Set-Content -LiteralPath (Join-Path $pairDir "left/artifacts/provider-wire-trace.jsonl") -Encoding UTF8
@(($taskspaceTrace | ConvertTo-Json -Compress -Depth 100), $taskspaceTerminal) | Set-Content -LiteralPath (Join-Path $pairDir "right/artifacts/provider-wire-trace.jsonl") -Encoding UTF8
$boundaryStart = [ordered]@{
    schema_version = 1; event = "provider_boundary_started"; limit = 3; allowed_method = "POST"
    allowed_path = "/responses"; allowed_model = "deepseek-v4-flash"
}
function Write-BoundaryFixture([string]$Path, [string]$Digest) {
    @(
        $boundaryStart,
        [ordered]@{ schema_version = 1; event = "provider_request_claimed"; count = 1; method = "POST"; path = "/responses"; model = "deepseek-v4-flash"; body_sha256 = $Digest },
        [ordered]@{ schema_version = 1; event = "provider_boundary_stopped"; request_count = 1 }
    ) | ForEach-Object { $_ | ConvertTo-Json -Compress } | Set-Content -LiteralPath $Path -Encoding UTF8
}
$standardBoundaryPath = Join-Path $pairDir "left/artifacts/provider-boundary-events.jsonl"
$taskspaceBoundaryPath = Join-Path $pairDir "right/artifacts/provider-boundary-events.jsonl"
Write-BoundaryFixture $standardBoundaryPath ([string]$standardTrace.provider_payload_sha256)
Write-BoundaryFixture $taskspaceBoundaryPath ([string]$taskspaceTrace.provider_payload_sha256)

$controlArguments = @{ action = "initialize_map" } | ConvertTo-Json -Compress
$controlOutput = [ordered]@{
    schema_version = "TaskSpaceControlResultV2"; action = "initialize_map"; status = "committed"
    success = $true; state_commit = $true; partial_commit = $false; canonical_revision = 2
    steps = @(@{ kind = "map_initialized" }, @{ kind = "node_bound" }); error = $null
} | ConvertTo-Json -Compress -Depth 20
$rolloutEvents = @(
    [ordered]@{
        type = "event_msg"
        payload = [ordered]@{
            map_event_type = "task_context_event_recorded"; eventType = "function_call"
            rawPayload = [ordered]@{ type = "function_call"; name = "taskspace_control"; call_id = "control-1"; arguments = $controlArguments }
        }
    },
    [ordered]@{
        type = "event_msg"
        payload = [ordered]@{
            map_event_type = "task_context_event_recorded"; eventType = "function_call_output"
            rawPayload = [ordered]@{ type = "function_call_output"; call_id = "control-1"; output = $controlOutput }
        }
    }
)
@($rolloutEvents | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 100 }) | Set-Content -LiteralPath (Join-Path $pairDir "right/artifacts/rollout.jsonl") -Encoding UTF8
Invoke-TaskspaceRequestFactsGenerator `
    -WireTracePath (Join-Path $pairDir "left/artifacts/provider-wire-trace.jsonl") `
    -BoundaryEventsPath $standardBoundaryPath `
    -OutputPath (Join-Path $pairDir "left/artifacts/request-facts.json") | Out-Null
Invoke-TaskspaceRequestFactsGenerator `
    -RolloutJsonlPath (Join-Path $pairDir "right/artifacts/rollout.jsonl") `
    -WireTracePath (Join-Path $pairDir "right/artifacts/provider-wire-trace.jsonl") `
    -BoundaryEventsPath $taskspaceBoundaryPath `
    -OutputPath (Join-Path $pairDir "right/artifacts/request-facts.json") | Out-Null

$resultPath = Join-Path $fixtureRoot "result.json"
$result = [ordered]@{
    binary = [ordered]@{ path = $binaryPath; sha256 = $binarySha; attested_codex_source_commit = $sourceCommit; attestation_path = $attestationPath }
    contracts = [ordered]@{
        standard_base = [ordered]@{ version = $baseContract.profiles.standard.version; sha256 = $baseContract.profiles.standard.sha256 }
        taskspace_base = [ordered]@{ version = $baseContract.profiles.taskspace.version; sha256 = $baseContract.profiles.taskspace.sha256 }
        taskspace_core_protocol = [ordered]@{ version = $baseContract.taskspace_core_protocol.version; sha256 = $baseContract.taskspace_core_protocol.sha256 }
        production_manifest = [ordered]@{ version = $manifest.manifest_version; sha256 = $manifestSha }
    }
    repair_acceptance = [ordered]@{
        b1_static_system_handle = [ordered]@{ taskspace_requests = 1 }
        b2_result_capability_mismatch = [ordered]@{
            control_results = 1; v2_control_results = 1; non_v2_control_results = 0
            initialize_commits_with_node_bound = 1; rejected_results_with_state_commit_false = 0
        }
        h4_observability = [ordered]@{
            control_calls = 1; control_failures = 0; preflight_failures = 0; ordinary_gate_failures = 0
            committed_controls = 1; graph_revision_commits = 1; state_commit_count = 1
        }
        h6_direct_actions = [ordered]@{ nested_transition_calls = 0; direct_complete_then_continue_calls = 0; direct_finish_map_calls = 0 }
        h7_binding_feedback = [ordered]@{ read_map_calls = 0; redundant_bind_calls = 0 }
    }
    runs = @([ordered]@{
            run_root = $runDir
            standard = [ordered]@{ provider_requests = 1 }
            taskspace = [ordered]@{
                provider_requests = 1; control_calls = 1; control_failures = 0; preflight_failures = 0
                ordinary_gate_failures = 0; committed_controls = 1; state_commit_count = 1
            }
        })
}
Write-FixtureJson $resultPath $result

$pass = Test-R7FiveLayerEvidenceFreshness -RepoRoot $repoRoot -WhaleBin $binaryPath -ResultPath $resultPath -RunRoots @($runDir)
Assert-True ([string]$pass.status -eq "pass") "Fresh evidence fixture did not pass: $(@($pass.findings.stable_code) -join ',')"

$taskspaceFactsPath = Join-Path $pairDir "right/artifacts/request-facts.json"
$originalTaskspaceBoundary = Get-Content -Raw -Encoding UTF8 -LiteralPath $taskspaceBoundaryPath
$originalTaskspaceFacts = Get-Content -Raw -Encoding UTF8 -LiteralPath $taskspaceFactsPath
$boundaryStart | ConvertTo-Json -Compress | Set-Content -LiteralPath $taskspaceBoundaryPath -Encoding UTF8
Invoke-TaskspaceRequestFactsGenerator -RolloutJsonlPath (Join-Path $pairDir "right/artifacts/rollout.jsonl") -WireTracePath (Join-Path $pairDir "right/artifacts/provider-wire-trace.jsonl") -BoundaryEventsPath $taskspaceBoundaryPath -OutputPath $taskspaceFactsPath | Out-Null
$unavailableBoundary = Test-R7FiveLayerEvidenceFreshness -RepoRoot $repoRoot -WhaleBin $binaryPath -ResultPath $resultPath -RunRoots @($runDir)
Assert-True ($null -eq $unavailableBoundary.runs[0].taskspace_provider_requests) "Unavailable boundary lifecycle was serialized as zero"
Assert-True (@($unavailableBoundary.findings | Where-Object stable_code -eq "request_facts_count_unavailable").Count -eq 1) "Unavailable boundary lifecycle did not emit a stable finding"

@($boundaryStart, [ordered]@{ schema_version = 1; event = "provider_boundary_stopped"; request_count = 0 }) | ForEach-Object { $_ | ConvertTo-Json -Compress } | Set-Content -LiteralPath $taskspaceBoundaryPath -Encoding UTF8
Invoke-TaskspaceRequestFactsGenerator -RolloutJsonlPath (Join-Path $pairDir "right/artifacts/rollout.jsonl") -WireTracePath (Join-Path $pairDir "right/artifacts/provider-wire-trace.jsonl") -BoundaryEventsPath $taskspaceBoundaryPath -OutputPath $taskspaceFactsPath | Out-Null
$healthyZeroBoundary = Test-R7FiveLayerEvidenceFreshness -RepoRoot $repoRoot -WhaleBin $binaryPath -ResultPath $resultPath -RunRoots @($runDir)
Assert-True ([int64]$healthyZeroBoundary.runs[0].taskspace_provider_requests -eq 0) "Healthy zero boundary was not preserved"
Assert-True (@($healthyZeroBoundary.findings | Where-Object { $_.stable_code -in @("request_facts_count_unavailable", "paired_trace_coverage_missing") }).Count -eq 0) "Healthy zero boundary was treated as missing coverage"
[IO.File]::WriteAllText($taskspaceBoundaryPath, $originalTaskspaceBoundary, [Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText($taskspaceFactsPath, $originalTaskspaceFacts, [Text.UTF8Encoding]::new($false))
$modeMapPath = Join-Path $pairDir "logical-mode-map.json"
$originalModeMap = Get-Content -Raw -Encoding UTF8 -LiteralPath $modeMapPath
'{' | Set-Content -LiteralPath $modeMapPath -Encoding UTF8
$invalidModeMap = Test-R7FiveLayerEvidenceFreshness -RepoRoot $repoRoot -WhaleBin $binaryPath -ResultPath $resultPath -RunRoots @($runDir)
Assert-True ($null -eq $invalidModeMap.runs[0].standard_provider_requests -and $null -eq $invalidModeMap.runs[0].taskspace_provider_requests) "Invalid mode map produced exact provider totals"
Assert-True (@($invalidModeMap.findings | Where-Object stable_code -eq "logical_mode_map_invalid").Count -eq 1) "Invalid mode map did not emit a stable finding"
[IO.File]::WriteAllText($modeMapPath, $originalModeMap, [Text.UTF8Encoding]::new($false))
Write-FixtureJson $modeMapPath ([ordered]@{ repeat = 1; left = "taskspace"; right = "taskspace" })
$duplicateModeMap = Test-R7FiveLayerEvidenceFreshness -RepoRoot $repoRoot -WhaleBin $binaryPath -ResultPath $resultPath -RunRoots @($runDir)
Assert-True ($null -eq $duplicateModeMap.runs[0].standard_provider_requests -and $null -eq $duplicateModeMap.runs[0].taskspace_provider_requests) "Duplicate logical modes produced exact provider totals"
Assert-True (@($duplicateModeMap.findings | Where-Object stable_code -eq "logical_mode_map_invalid").Count -eq 1) "Duplicate logical modes did not emit a stable finding"
[IO.File]::WriteAllText($modeMapPath, $originalModeMap, [Text.UTF8Encoding]::new($false))

$staleResult = Get-Content -Raw -Encoding UTF8 -LiteralPath $resultPath | ConvertFrom-Json -Depth 100
$staleResult.contracts.taskspace_base.version = "stale"
Write-FixtureJson $resultPath $staleResult
$staleContract = Test-R7FiveLayerEvidenceFreshness -RepoRoot $repoRoot -WhaleBin $binaryPath -ResultPath $resultPath -RunRoots @($runDir)
Assert-True ([string]$staleContract.status -eq "fail") "Stale result contract unexpectedly passed"
Assert-True (@($staleContract.findings | Where-Object stable_code -eq "result_taskspace_base_identity_mismatch").Count -eq 1) "Stale result contract did not emit the stable finding"

function Assert-ResultMutationFails([scriptblock]$Mutation, [string]$ExpectedCode) {
    Write-FixtureJson $resultPath $result
    $mutated = Get-Content -Raw -Encoding UTF8 -LiteralPath $resultPath | ConvertFrom-Json -Depth 100
    & $Mutation $mutated
    Write-FixtureJson $resultPath $mutated
    $checked = Test-R7FiveLayerEvidenceFreshness -RepoRoot $repoRoot -WhaleBin $binaryPath -ResultPath $resultPath -RunRoots @($runDir)
    Assert-True ([string]$checked.status -eq "fail") "Mutated result unexpectedly passed for $ExpectedCode"
    Assert-True (@($checked.findings | Where-Object stable_code -eq $ExpectedCode).Count -eq 1) "Mutation did not emit $ExpectedCode"
}

Assert-ResultMutationFails { param($value) $value.runs[0].standard.provider_requests = 2 } "result_standard_request_count_mismatch"
Assert-ResultMutationFails { param($value) $value.runs[0].taskspace.provider_requests = 2 } "result_taskspace_request_count_mismatch"
Assert-ResultMutationFails { param($value) $value.runs[0].taskspace.control_calls = 2 } "result_taskspace_control_calls_mismatch"
Assert-ResultMutationFails { param($value) $value.runs[0].taskspace.control_failures = 1 } "result_taskspace_control_failures_mismatch"
Assert-ResultMutationFails { param($value) $value.runs[0].taskspace.PSObject.Properties.Remove("ordinary_gate_failures") } "result_taskspace_ordinary_gate_failures_mismatch"

Write-FixtureJson $resultPath $result
$staleTrace = (Get-Content -Encoding UTF8 -LiteralPath (Join-Path $pairDir "right/artifacts/provider-wire-trace.jsonl") | Select-Object -First 1) | ConvertFrom-Json -Depth 100
$staleTrace.base_instructions_identity.version = "stale"
@(($staleTrace | ConvertTo-Json -Compress -Depth 100), $taskspaceTerminal) | Set-Content -LiteralPath (Join-Path $pairDir "right/artifacts/provider-wire-trace.jsonl") -Encoding UTF8
$staleWire = Test-R7FiveLayerEvidenceFreshness -RepoRoot $repoRoot -WhaleBin $binaryPath -ResultPath $resultPath -RunRoots @($runDir)
Assert-True ([string]$staleWire.status -eq "fail") "Stale provider trace unexpectedly passed"
Assert-True (@($staleWire.findings | Where-Object stable_code -eq "taskspace_base_identity_mismatch").Count -eq 1) "Stale provider trace did not emit the stable finding"
Assert-True (@($staleWire.findings | Where-Object stable_code -eq "request_facts_stale").Count -eq 1) "Changed request source did not invalidate sealed facts"

Write-Output "R7 five-layer evidence freshness self-test passed."
