$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$contractPath = Join-Path $repoRoot "benchmarks/taskspace/r7/projection-policy-contract.json"
$inventoryPath = Join-Path $repoRoot "benchmarks/taskspace/r7/phase-a-ownership-inventory.json"
$contract = Get-Content -Raw -Encoding UTF8 -LiteralPath $contractPath | ConvertFrom-Json -Depth 50
$inventory = Get-Content -Raw -Encoding UTF8 -LiteralPath $inventoryPath | ConvertFrom-Json -Depth 50

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -ne $Expected) {
        throw "$Message. expected=$Expected actual=$Actual"
    }
}

function Assert-SetEqual {
    param([object[]]$Actual, [object[]]$Expected, [string]$Message)
    $actualSorted = @($Actual | ForEach-Object { [string]$_ } | Sort-Object)
    $expectedSorted = @($Expected | ForEach-Object { [string]$_ } | Sort-Object)
    Assert-Equal $actualSorted.Count $expectedSorted.Count "$Message count mismatch"
    for ($i = 0; $i -lt $expectedSorted.Count; $i++) {
        Assert-Equal $actualSorted[$i] $expectedSorted[$i] "$Message item[$i] mismatch"
    }
}

function Get-PropValue {
    param($Object, [string]$Name)
    $prop = $Object.PSObject.Properties[$Name]
    if ($null -eq $prop) { return $null }
    $prop.Value
}

function Get-RepoRelativePath {
    param([string]$Path)
    $full = [System.IO.Path]::GetFullPath($Path)
    $root = [System.IO.Path]::GetFullPath($repoRoot)
    if (-not $root.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
        $root = "$root$([System.IO.Path]::DirectorySeparatorChar)"
    }
    $relative = [System.IO.Path]::GetRelativePath($root, $full)
    $relative.Replace([System.IO.Path]::DirectorySeparatorChar, "/")
}

Assert-Equal $contract.schema_version 1 "Unexpected contract schema"
Assert-Equal $contract.contract_id "r7-projection-policy-contract" "Unexpected contract id"
Assert-Equal $contract.status "phase_a_frozen" "Contract must be frozen"
Assert-Equal $contract.compatibility_policy "none" "R6 compatibility must be disabled"
Assert-Equal $contract.default_policy $null "Default policy must remain null until Phase G"
Assert-Equal $contract.policy_field "taskspace_projection_policy" "Unexpected policy field"
Assert-SetEqual $contract.policy_values @("map-always", "map-append", "map-request") "Policy enum"
Assert-True (@($contract.policy_values).Count -eq 3) "There must be exactly three policy values"
foreach ($forbidden in @("standard", "r6-epoch-baseline", "epoch-baseline", "delta-journal")) {
    Assert-True (@($contract.policy_values) -notcontains $forbidden) "$forbidden must not be a policy value"
    Assert-True (@($contract.not_policy_values) -contains $forbidden) "$forbidden must be explicitly excluded"
}

$shared = $contract.shared_architecture
Assert-Equal $shared.canonical_map "single_shared_rooted_dag" "Canonical Map must be shared"
Assert-Equal $shared.renderer "single_shared_projection_renderer" "Renderer must be shared"
Assert-Equal $shared.composer "single_shared_provider_context_composer" "Composer must be shared"
Assert-Equal $shared.tool_contract "single_shared_taskspace_control_schema" "Tool contract must be shared"
Assert-Equal $shared.event_store "single_shared_event_store" "Event Store must be shared"
Assert-Equal $shared.read_map_action.shared $true "read_map must be shared"
Assert-Equal $shared.read_map_action.tool "taskspace_control" "read_map must remain a taskspace_control action"
Assert-Equal $shared.read_map_action.name "read_map" "Unexpected shared Map read action"
Assert-Equal $shared.read_map_action.exclusive_to_map_request $false "read_map must be visible to all policies"
Assert-SetEqual $shared.read_map_action.visible_to_policies $contract.policy_values "read_map visibility"

foreach ($name in @("AlwaysRuntime", "AppendRuntime", "RequestRuntime", "r6_migration", "r6_adapter", "dual_reader", "silent_fallback")) {
    Assert-True (@($contract.forbidden_architecture) -contains $name) "Missing forbidden architecture: $name"
}

$requiredProjectionFields = @(
    "schema_version", "projection_kind", "map_id", "revision", "canonical_sha256",
    "root_node_id", "finish_node_id", "complete", "current_terminal",
    "terminal_history", "root_source_event_ids", "active_frontier", "map_nodes",
    "map_edges", "node_details"
)
Assert-Equal $contract.rendered_projection_contract.policy_dependent_content_allowed $false "Projection content must not vary by policy"
Assert-Equal $contract.rendered_projection_contract.next_action_suggestions_allowed $false "Projection must not suggest next actions"
Assert-SetEqual $contract.rendered_projection_contract.required_fields $requiredProjectionFields "RenderedProjection fields"

$triggers = @("provider_request", "explicit_read")
Assert-SetEqual $contract.triggers $triggers "Trigger list"

$expectedMatrix = @{
    "map-always" = @{
        provider_request = "replace_latest"
        explicit_read = "return_as_shared_tool_result"
    }
    "map-append" = @{
        provider_request = "append_latest_if_not_current_tail"
        explicit_read = "return_as_shared_tool_result"
    }
    "map-request" = @{
        provider_request = "current_non_persistent_map_handle_only"
        explicit_read = "return_as_shared_tool_result"
    }
}

foreach ($policy in $contract.policy_values) {
    $policySpec = Get-PropValue $contract.policies ([string]$policy)
    Assert-True ($null -ne $policySpec) "Missing policy spec: $policy"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$policySpec.emission)) "$policy emission missing"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$policySpec.persistence)) "$policy persistence missing"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$policySpec.freshness)) "$policy freshness missing"
    Assert-True (@($policySpec.known_characteristics).Count -gt 0) "$policy known characteristics missing"
    Assert-True (@($policySpec.bugs).Count -gt 0) "$policy bug list missing"
    foreach ($trigger in $triggers) {
        $actual = Get-PropValue $policySpec.trigger_matrix $trigger
        Assert-Equal $actual $expectedMatrix[[string]$policy][$trigger] "$policy/$trigger trigger decision drifted"
    }
}

$request = $contract.policies."map-request"
foreach ($gate in @(
    "agent_must_initialize_legal_map_with_ordinary_sibling_actions",
    "ordinary_tool_must_have_agent_declared_node_ownership_in_outer_manifest",
    "call_and_result_must_be_mechanically_attributed_to_reserved_node",
    "map_mutation_finish_and_reopen_only_via_taskspace_control",
    "update_plan_hidden_in_taskspace",
    "root_and_finish_close_only_through_explicit_current_terminal",
    "cannot_end_taskspace_before_legal_map_termination",
    "subagent_store_map_identity_constraints_unchanged"
)) {
    Assert-True (@($request.hard_gates) -contains $gate) "map-request hard gate missing: $gate"
}
foreach ($forbiddenHandleField in @("nodes", "edges", "frontier", "next_action", "semantic_summary")) {
    Assert-True (@($request.map_handle_must_not_include) -contains $forbiddenHandleField) "map-request handle forbidden field missing: $forbiddenHandleField"
}

Assert-Equal $contract.session_lifecycle.policy_selected_at_session_creation $true "Policy must be selected at session creation"
Assert-Equal $contract.session_lifecycle.agent_can_switch_policy $false "Agent must not switch policy"
Assert-Equal $contract.session_lifecycle.resume_restores_original_policy $true "Resume must restore policy"
Assert-Equal $contract.session_lifecycle.fork_restores_original_policy $true "Fork must restore policy"
Assert-Equal $contract.session_lifecycle.mid_session_migration_allowed $false "Mid-session migration must be disabled"
Assert-Equal $contract.phase_g.default_policy_until_phase_g $null "Phase G default must be null"
Assert-Equal $contract.phase_g.standard_is_policy $false "Standard must not be a policy"
Assert-Equal $contract.phase_g.r6_epoch_baseline_is_policy $false "R6 epoch baseline must not be a policy"

Assert-Equal $inventory.schema_version 1 "Unexpected inventory schema"
Assert-Equal $inventory.inventory_id "r7-phase-a-projection-policy-ownership" "Unexpected inventory id"
Assert-Equal $inventory.status "complete" "Inventory must be complete"
$allowedClassifications = @("retain_shared", "replace_b", "delete_b", "adapt_b", "adapt_c", "adapt_d", "adapt_f")
$requiredDomains = @("renderer", "runtime", "session", "state", "protocol", "tools", "observer", "benchmark", "compaction", "resume_replay", "viewer", "tests")
Assert-SetEqual $inventory.allowed_classifications $allowedClassifications "Allowed inventory classifications"
Assert-SetEqual $inventory.coverage_domains $requiredDomains "Inventory domains"

$seenDomains = [System.Collections.Generic.HashSet[string]]::new()
$inventoryCoverage = [System.Collections.Generic.HashSet[string]]::new()
foreach ($item in @($inventory.items)) {
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$item.id)) "Inventory item id missing"
    Assert-True ($requiredDomains -contains [string]$item.domain) "Invalid inventory domain for $($item.id): $($item.domain)"
    Assert-True ($allowedClassifications -contains [string]$item.classification) "Invalid classification for $($item.id): $($item.classification)"
    Assert-True ([string]$item.classification -ne "unknown") "Unknown classification is forbidden for $($item.id)"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$item.path)) "Inventory path missing for $($item.id)"
    $itemPathExists = Test-Path -LiteralPath (Join-Path $repoRoot ([string]$item.path)) -PathType Leaf
    if ([string]$item.classification -eq "delete_b") {
        Assert-True (-not $itemPathExists) "Phase B deleted path still exists: $($item.path)"
    }
    else {
        Assert-True $itemPathExists "Inventory path does not exist: $($item.path)"
    }
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$item.r7_owner)) "R7 owner missing for $($item.id)"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$item.reason)) "Reason missing for $($item.id)"
    [void]$seenDomains.Add([string]$item.domain)
    [void]$inventoryCoverage.Add([string]$item.path)
    foreach ($covered in @($item.covered_paths)) {
        Assert-True (Test-Path -LiteralPath (Join-Path $repoRoot ([string]$covered)) -PathType Leaf) "Covered path does not exist: $covered"
        [void]$inventoryCoverage.Add([string]$covered)
    }
}
Assert-SetEqual @($seenDomains) $requiredDomains "Inventory domain coverage"

$mustHaveIds = @(
    "renderer_projection_schema_and_rendering",
    "runtime_canonical_map_snapshot_source",
    "r6_projection_epoch_state",
    "session_provider_context_composer",
    "taskspace_control_arguments",
    "benchmark_cost_instrumentation_projection_scan",
    "resume_and_fork_policy_restore",
    "app_server_taskspace_messages",
    "r7_phase_a_contract_test"
)
$actualIds = @($inventory.items | ForEach-Object { [string]$_.id })
foreach ($id in $mustHaveIds) {
    Assert-True ($actualIds -contains $id) "Inventory missing required item: $id"
}

$productionAndTestRoots = @(
    "third_party/codex-cli/codex-rs",
    "scripts/taskspace-benchmark",
    "scripts"
)
$markerPattern = "TaskSpaceMapEpochSnapshotR6V1|epoch_baseline|projection_epoch|taskspace_projection_epoch|projection_epoch_identity|TaskSpaceProjectionEpoch|ContextProjectionV1|rooted_map_epoch|active_projection"
$markerFiles = [System.Collections.Generic.HashSet[string]]::new()
foreach ($root in $productionAndTestRoots) {
    $absoluteRoot = Join-Path $repoRoot $root
    if (-not (Test-Path -LiteralPath $absoluteRoot)) { continue }
    $matches = & rg -l $markerPattern $absoluteRoot --glob '!target/**'
    if ($LASTEXITCODE -gt 1) { throw "rg failed while scanning $root" }
    foreach ($match in @($matches)) {
        if ([string]::IsNullOrWhiteSpace($match)) { continue }
        $relative = Get-RepoRelativePath $match
        if ($relative -eq "scripts/taskspace-benchmark/test-r7-projection-policy-contract.ps1") { continue }
        [void]$markerFiles.Add($relative)
    }
}

foreach ($markerFile in @($markerFiles | Sort-Object)) {
    Assert-True ($inventoryCoverage.Contains($markerFile)) "R6 epoch marker file is not represented by inventory path or covered_path: $markerFile"
}

Write-Host "R7 projection policy contract passed."
Write-Host "Policies: $(@($contract.policy_values) -join ', ')"
Write-Host "Inventory items: $(@($inventory.items).Count)"
Write-Host "R6 epoch marker files covered: $($markerFiles.Count)"
