$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$contractPath = Join-Path $repoRoot "benchmarks/taskspace/r6/rooted-dag-contract.json"
$fixturesPath = Join-Path $repoRoot "benchmarks/taskspace/r6/rooted-dag-contract-fixtures.json"
$contract = Get-Content -Raw -Encoding UTF8 -LiteralPath $contractPath | ConvertFrom-Json -Depth 30
$fixtures = Get-Content -Raw -Encoding UTF8 -LiteralPath $fixturesPath | ConvertFrom-Json -Depth 30

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

Write-Host "R6 rooted DAG contract tests passed: $(@($fixtures.cases).Count) fixtures"
