$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$contractPath = Join-Path $repoRoot "benchmarks/taskspace/r6/rooted-dag-contract.json"
$fixturesPath = Join-Path $repoRoot "benchmarks/taskspace/r6/rooted-dag-contract-fixtures.json"
$inventoryPath = Join-Path $repoRoot "benchmarks/taskspace/r6/phase-a-ownership-inventory.json"
$baselinePath = Join-Path $repoRoot "benchmarks/taskspace/r6/phase-a-baseline-contract.json"
$baselineResultPath = Join-Path $repoRoot "benchmarks/taskspace/r6/phase-a-baseline-result.json"
$contract = Get-Content -Raw -Encoding UTF8 -LiteralPath $contractPath | ConvertFrom-Json -Depth 30
$fixtures = Get-Content -Raw -Encoding UTF8 -LiteralPath $fixturesPath | ConvertFrom-Json -Depth 30
$inventory = Get-Content -Raw -Encoding UTF8 -LiteralPath $inventoryPath | ConvertFrom-Json -Depth 30
$baseline = Get-Content -Raw -Encoding UTF8 -LiteralPath $baselinePath | ConvertFrom-Json -Depth 30
$baselineResult = Get-Content -Raw -Encoding UTF8 -LiteralPath $baselineResultPath | ConvertFrom-Json -Depth 30

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -ne $Expected) {
        throw "$Message. expected=$Expected actual=$Actual"
    }
}

function Add-Code {
    param([System.Collections.Generic.HashSet[string]]$Codes, [string]$Code)
    [void]$Codes.Add($Code)
}

function Get-Reachable {
    param([string]$Start, [hashtable]$Adjacency)
    $seen = [System.Collections.Generic.HashSet[string]]::new()
    $queue = [System.Collections.Generic.Queue[string]]::new()
    if (-not [string]::IsNullOrWhiteSpace($Start)) { $queue.Enqueue($Start) }
    while ($queue.Count -gt 0) {
        $node = $queue.Dequeue()
        if (-not $seen.Add($node)) { continue }
        foreach ($next in @($Adjacency[$node])) {
            if (-not $seen.Contains([string]$next)) { $queue.Enqueue([string]$next) }
        }
    }
    $seen
}

function Test-Cycle {
    param([string[]]$NodeIds, [hashtable]$Forward, [hashtable]$Indegree)
    $remaining = @{}
    foreach ($id in $NodeIds) { $remaining[$id] = [int]$Indegree[$id] }
    $queue = [System.Collections.Generic.Queue[string]]::new()
    foreach ($id in $NodeIds) {
        if ($remaining[$id] -eq 0) { $queue.Enqueue($id) }
    }
    $visited = 0
    while ($queue.Count -gt 0) {
        $id = $queue.Dequeue()
        $visited++
        foreach ($next in @($Forward[$id])) {
            $remaining[[string]$next]--
            if ($remaining[[string]$next] -eq 0) { $queue.Enqueue([string]$next) }
        }
    }
    $visited -ne $NodeIds.Count
}

function Get-GraphViolationCodes {
    param($Case, $Contract)
    $codes = [System.Collections.Generic.HashSet[string]]::new()
    $nodes = @($Case.nodes)
    $nodeIds = @($nodes | ForEach-Object { [string]$_.id })
    $nodeSet = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($id in $nodeIds) { [void]$nodeSet.Add($id) }

    $roots = @($nodes | Where-Object { [string]$_.role -eq "task_root" })
    $finishes = @($nodes | Where-Object { [string]$_.role -eq "finish" })
    if ($roots.Count -eq 0) { Add-Code $codes "root_missing" }
    if ($roots.Count -gt 1) { Add-Code $codes "multiple_roots" }
    if ($finishes.Count -eq 0) { Add-Code $codes "finish_missing" }
    if ($finishes.Count -gt 1) { Add-Code $codes "multiple_finishes" }
    if ($roots.Count -eq 1 -and [string]$roots[0].id -ne [string]$Case.root_node_id) {
        Add-Code $codes "root_id_mismatch"
    }
    if ($finishes.Count -eq 1 -and [string]$finishes[0].id -ne [string]$Case.finish_node_id) {
        Add-Code $codes "finish_id_mismatch"
    }

    foreach ($node in $nodes) {
        $roleSpec = $Contract.map.node_roles.PSObject.Properties[[string]$node.role].Value
        if ($null -eq $roleSpec -or @($roleSpec.statuses) -notcontains [string]$node.status) {
            Add-Code $codes "role_status_invalid"
        }
    }

    $forward = @{}
    $reverse = @{}
    $indegree = @{}
    $outdegree = @{}
    foreach ($id in $nodeIds) {
        $forward[$id] = [System.Collections.Generic.List[string]]::new()
        $reverse[$id] = [System.Collections.Generic.List[string]]::new()
        $indegree[$id] = 0
        $outdegree[$id] = 0
    }
    $edgeSet = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($edge in @($Case.edges)) {
        $from = [string]$edge.from
        $to = [string]$edge.to
        $key = "$from`0$to"
        if (-not $edgeSet.Add($key)) { Add-Code $codes "duplicate_edge" }
        if ($from -eq $to) { Add-Code $codes "self_loop" }
        if (-not $nodeSet.Contains($from) -or -not $nodeSet.Contains($to)) {
            Add-Code $codes "edge_endpoint_missing"
            continue
        }
        $forward[$from].Add($to)
        $reverse[$to].Add($from)
        $indegree[$to]++
        $outdegree[$from]++
    }

    if ($nodeSet.Contains([string]$Case.root_node_id)) {
        foreach ($id in $nodeIds) {
            if ($id -ne [string]$Case.root_node_id -and $indegree[$id] -eq 0) {
                Add-Code $codes "non_root_zero_indegree"
            }
        }
    }
    if ($nodeSet.Contains([string]$Case.finish_node_id)) {
        foreach ($id in $nodeIds) {
            if ($id -ne [string]$Case.finish_node_id -and $outdegree[$id] -eq 0) {
                Add-Code $codes "non_finish_zero_outdegree"
            }
        }
    }

    if (Test-Cycle $nodeIds $forward $indegree) { Add-Code $codes "cycle_detected" }
    if ($nodeSet.Contains([string]$Case.root_node_id)) {
        $reachable = Get-Reachable ([string]$Case.root_node_id) $forward
        if ($reachable.Count -ne $nodeIds.Count) { Add-Code $codes "node_unreachable_from_root" }
    }
    if ($nodeSet.Contains([string]$Case.finish_node_id)) {
        $canReachFinish = Get-Reachable ([string]$Case.finish_node_id) $reverse
        if ($canReachFinish.Count -ne $nodeIds.Count) { Add-Code $codes "finish_unreachable_from_node" }
    }
    @($codes | Sort-Object)
}

Assert-Equal $contract.schema_version 1 "Unexpected contract schema"
Assert-Equal $contract.contract_id $fixtures.contract_id "Fixture contract identity mismatch"
Assert-Equal $contract.status "phase_a_frozen" "Contract must be frozen before Phase B"
Assert-Equal $contract.compatibility "none" "Compatibility policy drifted"
Assert-Equal $contract.authority.canonical_state "taskspace_map" "Map is not canonical"
Assert-Equal $contract.authority.completion_source "root_and_finish_node_states" "Completion authority drifted"
Assert-Equal $contract.map.edge_semantics "to_depends_on_from" "Edge semantics drifted"
Assert-Equal $contract.map.separate_parent_relation_allowed $false "A second hierarchy was enabled"
Assert-Equal $contract.readiness.join_policy "all_predecessors_satisfied" "Join policy drifted"
Assert-Equal $contract.transactions.finish_end.automatic_trigger_allowed $false "Automatic finish was enabled"
Assert-Equal $contract.transactions.mutate_graph.partial_commit_allowed $false "Partial graph commit was enabled"
Assert-Equal $contract.tool_contract_draft.semantic_prompt_allowed $false "Semantic prompt was enabled"
Assert-Equal $contract.projection.topology_pagination_allowed $false "Topology pagination was enabled"

$requiredInvariants = @(
    "single_root",
    "single_finish",
    "root_is_only_source",
    "finish_is_only_sink",
    "acyclic",
    "root_reaches_all",
    "all_reach_finish",
    "valid_references",
    "role_status_coherent",
    "terminal_is_manual"
)
$actualInvariants = @($contract.invariants | ForEach-Object { [string]$_.code })
foreach ($code in $requiredInvariants) {
    if ($actualInvariants -notcontains $code) { throw "Missing invariant: $code" }
}

$declaredErrors = @($contract.stable_error_codes | ForEach-Object { [string]$_ })
foreach ($case in @($fixtures.cases)) {
    $actual = @(Get-GraphViolationCodes $case $contract)
    foreach ($code in @($case.expected_codes)) {
        if ($declaredErrors -notcontains [string]$code) {
            throw "Fixture $($case.id) expects undeclared error code: $code"
        }
        if ($actual -notcontains [string]$code) {
            throw "Fixture $($case.id) did not produce expected code $code. actual=$($actual -join ',')"
        }
    }
    if ([bool]$case.valid -and $actual.Count -ne 0) {
        throw "Valid fixture $($case.id) produced violations: $($actual -join ',')"
    }
    if (-not [bool]$case.valid -and $actual.Count -eq 0) {
        throw "Invalid fixture $($case.id) produced no violations"
    }
}

Assert-Equal $inventory.status "complete" "Ownership inventory is incomplete"
$coveredDomains = @($inventory.items | ForEach-Object { [string]$_.domain } | Sort-Object -Unique)
foreach ($domain in @($inventory.coverage_domains)) {
    if ($coveredDomains -notcontains [string]$domain) {
        throw "Ownership inventory does not cover domain: $domain"
    }
}
$allowedClassifications = @($inventory.allowed_classifications | ForEach-Object { [string]$_ })
foreach ($item in @($inventory.items)) {
    foreach ($field in @("id", "domain", "current_owner", "current_path", "classification", "target_owner", "target_phase", "reason")) {
        if ([string]::IsNullOrWhiteSpace([string]$item.$field)) {
            throw "Ownership inventory item is missing ${field}: $($item.id)"
        }
    }
    if ($allowedClassifications -notcontains [string]$item.classification) {
        throw "Ownership inventory item has unknown classification: $($item.id)=$($item.classification)"
    }
    $fullPath = Join-Path $repoRoot ([string]$item.current_path)
    if (-not (Test-Path -LiteralPath $fullPath)) {
        throw "Ownership inventory path does not exist: $($item.id)=$fullPath"
    }
}

Assert-Equal $baseline.status "frozen_pre_run" "Phase A baseline must be frozen before execution"
Assert-Equal $baseline.execution_substrate "docker_hard_boundary" "Baseline is not Docker-only"
Assert-Equal $baseline.repeats_per_arm 1 "Phase A quick baseline repeat count drifted"
Assert-Equal $baseline.aggregate_utility_allowed $false "Single-run utility aggregation was enabled"
$binaryPath = Join-Path $repoRoot ([string]$baseline.r5.binary_path)
$actualBinarySha = (Get-FileHash -Algorithm SHA256 -LiteralPath $binaryPath).Hash.ToLowerInvariant()
Assert-Equal $actualBinarySha ([string]$baseline.r5.binary_sha256) "Frozen R5 binary hash mismatch"
$attestationPath = Join-Path $repoRoot ([string]$baseline.r5.attestation_path)
$attestation = Get-Content -Raw -Encoding UTF8 -LiteralPath $attestationPath | ConvertFrom-Json
Assert-Equal $attestation.codex_source_latest_commit $baseline.r5.runtime_commit "Frozen R5 binary source mismatch"
foreach ($scenario in @($baseline.scenarios)) {
    $scenarioPath = Join-Path $repoRoot ([string]$scenario.scenario_path)
    if (-not (Test-Path -LiteralPath $scenarioPath -PathType Leaf)) {
        throw "Baseline scenario is missing: $scenarioPath"
    }
}
$r6Arm = @($baseline.arms | Where-Object { [string]$_.id -eq "r6_a0" })
Assert-Equal $r6Arm.Count 1 "R6 A0 identity arm is missing"
Assert-Equal $r6Arm[0].provider_execution $false "Code-identical R6 A0 should not duplicate provider execution"
foreach ($productionPath in @($baseline.r6_a0_identity.production_paths)) {
    $fullPath = Join-Path $repoRoot ([string]$productionPath)
    if (-not (Test-Path -LiteralPath $fullPath)) {
        throw "R6 A0 identity path is missing: $fullPath"
    }
}

Assert-Equal $baselineResult.status "complete" "Phase A result is incomplete"
Assert-Equal $baselineResult.execution.substrate "docker_hard_boundary" "Phase A result is not Docker-only"
Assert-Equal $baselineResult.execution.aggregate_utility_allowed $false "Single-run result enabled utility aggregation"
$baselineContractSha = (Get-FileHash -Algorithm SHA256 -LiteralPath $baselinePath).Hash.ToLowerInvariant()
Assert-Equal $baselineResult.baseline_contract.sha256 $baselineContractSha "Phase A baseline contract hash mismatch"
Assert-Equal $baselineResult.r6_a0_identity.production_diff_count 0 "R6 A0 is not code-identical to R5"
Assert-Equal $baselineResult.r6_a0_identity.passed $true "R6 A0 identity gate failed"
Assert-Equal @($baselineResult.scenarios).Count @($baseline.scenarios).Count "Baseline result scenario count mismatch"
foreach ($scenarioResult in @($baselineResult.scenarios)) {
    foreach ($arm in @($scenarioResult.standard, $scenarioResult.r5)) {
        Assert-Equal $arm.business_success $true "Baseline arm did not solve $($scenarioResult.scenario_id)"
        Assert-Equal $arm.agent_completion_status "complete" "Baseline arm did not complete $($scenarioResult.scenario_id)"
        Assert-Equal $arm.external_validation_status "passed" "Baseline arm failed validation $($scenarioResult.scenario_id)"
    }
    Assert-Equal $scenarioResult.r5.control_state_failures 0 "R5 state failure in $($scenarioResult.scenario_id)"
    Assert-Equal $scenarioResult.r5.control_protocol_failures 0 "R5 protocol failure in $($scenarioResult.scenario_id)"
    Assert-Equal $scenarioResult.r5.nested_action_failures 0 "R5 nested action failure in $($scenarioResult.scenario_id)"
    Assert-Equal $scenarioResult.r5.edge_count 0 "Phase A no-edge observation drifted for $($scenarioResult.scenario_id)"

    $runRoot = Join-Path $repoRoot ([string]$scenarioResult.run_root)
    $evidenceFiles = @{
        performance_observation_sha256 = Join-Path $runRoot "performance-observation.json"
        pair_report_sha256 = Join-Path $runRoot "pair-001/pair-report.md"
        resolved_manifest_sha256 = Join-Path $runRoot "pair-001/manifest.resolved.json"
    }
    foreach ($entry in $evidenceFiles.GetEnumerator()) {
        $actualSha = (Get-FileHash -Algorithm SHA256 -LiteralPath $entry.Value).Hash.ToLowerInvariant()
        Assert-Equal $scenarioResult.evidence.($entry.Key) $actualSha "Evidence hash mismatch for $($scenarioResult.scenario_id):$($entry.Key)"
    }
}
Assert-Equal $baselineResult.phase_a_gates.contract_fixture_count @($fixtures.cases).Count "Fixture count result drifted"
Assert-Equal $baselineResult.phase_a_gates.ownership_item_count @($inventory.items).Count "Ownership count result drifted"
Assert-Equal $baselineResult.phase_a_gates.unknown_ownership_count 0 "Unknown ownership remains"
Assert-Equal $baselineResult.phase_a_gates.production_behavior_changed $false "Phase A changed production behavior"
Assert-Equal $baselineResult.phase_a_gates.phase_b_ready $true "Phase B readiness gate failed"

Write-Host "R6 rooted DAG contract tests passed: $(@($fixtures.cases).Count) fixtures, $(@($inventory.items).Count) ownership items, $(@($baseline.scenarios).Count) baseline results"
