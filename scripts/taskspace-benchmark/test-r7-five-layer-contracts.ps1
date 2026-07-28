param(
    [ValidateSet("A2-B5", "All")]
    [string]$Phase = "All"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path

. (Join-Path $PSScriptRoot "r7-contract-test-primitives.ps1")

function Read-StrictJson {
    param([string]$Path, [string]$Label)
    $raw = Get-Content -Raw -Encoding UTF8 -LiteralPath $Path
    Assert-StrictJson $raw $Label
    return $raw | ConvertFrom-Json -Depth 50
}

function Assert-SourceContains {
    param([string]$Path, [string]$Needle, [string]$Message)
    $source = [System.IO.File]::ReadAllText($Path)
    Assert-True $source.Contains($Needle) $Message
}

function Assert-SourceExcludes {
    param([string]$Path, [string[]]$Needles, [string]$Label)
    $source = [System.IO.File]::ReadAllText($Path)
    foreach ($needle in $Needles) {
        Assert-True (-not $source.Contains($needle)) "$Label retains obsolete '$needle'"
    }
}

$authorityPath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json"
$authoritySchemaPath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-contract-authority-v1.schema.json"
$integratedPath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-integrated-change-constraints-v1.json"
$manifestPath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json"
$manifestSchemaPath = Join-Path $repoRoot "benchmarks/taskspace/r7/taskspace-contract-manifest-v1.schema.json"
$controlContractPath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-taskspace-control-v4.contract.json"
$resultContractPath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-taskspace-result-v3.contract.json"
$lifecyclePath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-lifecycle-oracles-v2.json"
$projectionPath = Join-Path $repoRoot "benchmarks/taskspace/r7/projection-policy-contract.json"
$l2Path = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_core_protocol_v3.md"
$toolPath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs"
$wirePath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_args_wire.rs"
$preflightPath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs"
$canonicalPath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/protocol/src/taskspace.rs"
$projectionSourcePath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/action_map/projection.rs"
$contextLogPath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/context.rs"
$controlOutputPath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_output.rs"
$sessionTestsPath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/session/tests.rs"
$multiAgentTestsPath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_tests.rs"
$cliPath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/cli/src/main.rs"
$storeExportPath = Join-Path $repoRoot "scripts/action-map-store-export-lib.ps1"
$observabilityPath = Join-Path $repoRoot "scripts/action-map-observability-lib.ps1"
$observerPaths = @(
    "scripts/taskspace-benchmark/lib/cost-instrumentation.ps1",
    "scripts/taskspace-benchmark/lib/metrics-extractor.ps1",
    "scripts/taskspace-benchmark/lib/native-cadence.ps1",
    "scripts/taskspace-benchmark/lib/performance-duplication.ps1",
    "scripts/taskspace-benchmark/lib/performance-observation.ps1",
    "scripts/taskspace-benchmark/lib/r7-five-layer-trace-analysis.ps1",
    "scripts/taskspace-benchmark/report-r7-five-layer-matrix.ps1"
) | ForEach-Object { Join-Path $repoRoot $_ }
$lifecycleProductionPaths = @(
    "third_party/codex-cli/codex-rs/exec/src/cli.rs",
    "third_party/codex-cli/codex-rs/exec/src/lib.rs",
    "third_party/codex-cli/codex-rs/protocol/src/protocol.rs",
    "third_party/codex-cli/codex-rs/core/src/action_map/runtime/state.rs",
    "third_party/codex-cli/codex-rs/core/src/session/handlers.rs",
    "third_party/codex-cli/codex-rs/core/src/session/mod.rs",
    "third_party/codex-cli/codex-rs/app-server-protocol/src/protocol/common.rs",
    "third_party/codex-cli/codex-rs/app-server-protocol/src/protocol/v2.rs",
    "third_party/codex-cli/codex-rs/app-server/src/codex_message_processor.rs",
    "third_party/codex-cli/codex-rs/tui/src/app_server_session.rs",
    "third_party/codex-cli/codex-rs/tui/src/app/thread_routing.rs",
    "third_party/codex-cli/codex-rs/tui/src/app_command.rs",
    "third_party/codex-cli/codex-rs/tui/src/chatwidget/slash_dispatch.rs",
    "third_party/codex-cli/codex-rs/tui/src/slash_command.rs"
) | ForEach-Object { Join-Path $repoRoot $_ }

$authorityRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $authorityPath
Assert-StrictJson $authorityRaw "R7 authority"
Assert-True ($authorityRaw | Test-Json -SchemaFile $authoritySchemaPath -ErrorAction Stop) "Authority JSON does not match its schema"
$authority = $authorityRaw | ConvertFrom-Json -Depth 50
Assert-Equal ([string]$authority.compatibility_policy) "none" "R7.1 must not retain compatibility paths"
Assert-Equal ([string]$authority.current_milestone.id) "R7.1" "Unexpected milestone"

& (Join-Path $PSScriptRoot "test-r7-integrated-change-constraints.ps1")
$integrated = Read-StrictJson $integratedPath "integrated constraints"
$openRegressions = @(
    $integrated.regression_invariants |
        Where-Object status -eq "open" |
        ForEach-Object { [string]$_.id }
)
Assert-Equal ($openRegressions -join ",") "R-10,R-19,R-22" "A2-B5 open regression set drifted"
Assert-Equal (@($authority.current_milestone.open_regressions) -join ",") ($openRegressions -join ",") "Authority and integrated gate disagree"

foreach ($document in @($authority.governing_documents)) {
    $path = Join-Path $repoRoot ([string]$document.path)
    Assert-True (Test-Path -LiteralPath $path -PathType Leaf) "Governing document missing: $($document.path)"
    Assert-Equal (Get-Sha256 $path) ([string]$document.sha256) "Governing document hash drifted: $($document.path)"
}
foreach ($target in @($authority.selected_targets)) {
    $path = Join-Path $repoRoot ([string]$target.artifact)
    Assert-True (Test-Path -LiteralPath $path -PathType Leaf) "Selected artifact missing: $($target.artifact)"
    Assert-Equal (Get-Sha256 $path) ([string]$target.sha256) "Selected artifact hash drifted: $($target.artifact)"
}

$manifestRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath
Assert-StrictJson $manifestRaw "production manifest"
Assert-True ($manifestRaw | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction Stop) "Production manifest does not match its schema"
$manifest = $manifestRaw | ConvertFrom-Json -Depth 50
Assert-Equal ([string]$manifest.activation_through) "A2-B5" "Production manifest is not activated through A2-B5"
Assert-Equal ([string]$manifest.source_authority.sha256) (Get-Sha256 $authorityPath) "Production manifest authority hash drifted"
Assert-Equal @($manifest.layers).Count 5 "Production manifest must retain exactly five content layers"
foreach ($layer in @($manifest.layers)) {
    Assert-Equal ([string]$layer.runtime_status) "active" "Layer $($layer.id) is not active"
    foreach ($target in @($layer.selected_targets)) {
        $targetPath = Join-Path $repoRoot ([string]$target.artifact)
        Assert-True (Test-Path -LiteralPath $targetPath -PathType Leaf) "Production target missing: $($target.artifact)"
        Assert-Equal (Get-Sha256 $targetPath) ([string]$target.sha256) "Production target hash drifted: $($target.artifact)"
    }
}

$control = Read-StrictJson $controlContractPath "current control contract"
$actions = @($control.tool.top_level_actions | ForEach-Object { [string]$_.action })
Assert-Equal ($actions -join ",") "initialize_and_execute,execute,reopen_map,read_map,read_output_ref,finish_map" "Top-level action set drifted"
Assert-Equal ([string]$control.response_manifest.ordinary_tool_arguments) "native_unchanged" "Ordinary Tool arguments are not native"
Assert-Equal ([string]$control.response_manifest.ownership) "agent_declared_actions_index_matches_ordinary_sibling_index" "Action ownership is not Agent-declared"

$result = Read-StrictJson $resultContractPath "current result contract"
Assert-Equal ([string]$result.model_visible_schema_version) "TaskSpaceControlResultV2" "Model-visible result version drifted"
Assert-Equal ([bool]$result.partial_commit) $false "TaskSpace result permits partial commit"
Assert-Equal (@($result.accepted_actions) -join ",") ($actions -join ",") "Control and result action sets disagree"

$lifecycle = Read-StrictJson $lifecyclePath "current lifecycle contract"
Assert-Equal ([string]$lifecycle.canonical_schema) "taskspace-canonical-map-v2" "Canonical lifecycle schema drifted"
Assert-Equal @($lifecycle.oracles).Count 4 "Lifecycle contract must cover close, reopen, invalid reopen, and restart"

$projection = Read-StrictJson $projectionPath "projection policy contract"
foreach ($field in @("current_terminal", "terminal_history", "complete", "active_frontier", "map_nodes", "map_edges", "node_details")) {
    Assert-True (@($projection.rendered_projection_contract.required_fields) -contains $field) "Projection contract omits $field"
}
foreach ($removed in @("current_binding", "current_node", "singleton_main_lease")) {
    Assert-True (-not (@($projection.rendered_projection_contract.required_fields) -contains $removed)) "Projection contract retains $removed"
}

foreach ($entry in @(
    @{ Path = $toolPath; Label = "taskspace_control schema" },
    @{ Path = $wirePath; Label = "taskspace_control wire parser" },
    @{ Path = $preflightPath; Label = "response preflight" },
    @{ Path = $canonicalPath; Label = "canonical Map model" },
    @{ Path = $projectionSourcePath; Label = "projection renderer" },
    @{ Path = $controlOutputPath; Label = "control result projection" }
)) {
    Assert-SourceExcludes $entry.Path @(
        "taskspace_binding",
        "initialize_map",
        "bind_node",
        "complete_then_continue",
        "rework_node",
        "current_node",
        "current_binding"
    ) $entry.Label
}
Assert-SourceExcludes $sessionTestsPath @(
    "taskspace_binding",
    "initialize_map_for_main",
    "mutate_graph_for_main",
    "complete_then_continue",
    "rework_node",
    "#[cfg(any())]"
) "session tests"
Assert-SourceExcludes $multiAgentTestsPath @(
    "taskspace_binding",
    "bind_node",
    "complete_then_continue",
    "current_node",
    "current_binding",
    "start_action_map_task_node",
    "active_action_map_lease_count",
    "#[cfg(any())]"
) "multi-agent TaskSpace tests"
foreach ($observerPath in $observerPaths) {
    Assert-SourceExcludes $observerPath @(
        "taskspace_binding",
        "initialize_map",
        "bind_node",
        "complete_then_continue",
        "rework_node",
        "current_node",
        "current_binding",
        "mutate_graph"
    ) "active benchmark observer $observerPath"
}
foreach ($productionPath in $lifecycleProductionPaths) {
    Assert-SourceExcludes $productionPath @(
        "RestartActionMap",
        "request_action_map_reborn",
        "clear_active_map",
        "ThreadActionMapRestart",
        "reborn_requested",
        "TaskReborn",
        "task_reborn",
        "map_restart"
    ) "TaskSpace lifecycle production source $productionPath"
}
Assert-SourceExcludes $cliPath @(
    "TaskSpaceMapExportR7V1",
    "record.snapshot",
    "record.snapshot_sha256",
    "record.graph_revision",
    "binding.node_id",
    "binding.lease_id"
) "TaskSpace Map Store CLI export"
Assert-SourceExcludes $storeExportPath @(
    "TaskSpaceMapExportR7V1",
    "snapshot_sha256",
    "graph_revision",
    "currentNodeId",
    "lease_id"
) "TaskSpace Map Store observer source"
Assert-SourceExcludes $observabilityPath @(
    "currentNodeId"
) "TaskSpace observability model"
foreach ($removedPath in @(
    "third_party/codex-cli/codex-rs/tools/src/taskspace_binding.rs",
    "third_party/codex-cli/codex-rs/core/src/tools/taskspace_binding.rs",
    "third_party/codex-cli/codex-rs/core/src/tools/taskspace_initialization.rs",
    "third_party/codex-cli/codex-rs/core/src/action_map/runtime_tests.rs",
    "third_party/codex-cli/codex-rs/core/src/action_map/runtime_phase_d_tests.rs",
    "third_party/codex-cli/codex-rs/core/src/action_map/runtime_fla7_tests.rs",
    "third_party/codex-cli/codex-rs/tools/examples/r6_f5_control_schema.rs",
    "scripts/taskspace-benchmark/probe-r6-f5-bootstrap-contract.ps1",
    "scripts/taskspace-benchmark/probe-r6-f5-finish-identity.ps1",
    "scripts/taskspace-benchmark/test-r6-f5-bootstrap-contract.ps1",
    "scripts/taskspace-benchmark/test-r6-f5-finish-identity.ps1",
    "scripts/action-map-snapshot-normalizer.ps1",
    "scripts/test-action-map-store-fixture-lib.ps1",
    "scripts/test-action-map-observability-summary-export.ps1",
    "scripts/test-r6-action-map-observability.ps1"
)) {
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $repoRoot $removedPath))) "Obsolete source remains: $removedPath"
}

Assert-SourceContains $toolPath '"reopen_map"' "Tool schema omits reopen_map"
Assert-SourceContains $toolPath '"complete_work_node_ids"' "Tool schema omits explicit final Work"
Assert-SourceContains $wirePath "Action::ReopenMap" "Wire parser omits reopen_map"
Assert-SourceContains $preflightPath "TaskSpaceControlArgs::ReopenMap" "Response preflight omits reopen_map"
Assert-SourceContains $canonicalPath "terminal_history" "Canonical Map omits terminal history"
Assert-SourceContains $projectionSourcePath '"current_terminal"' "Projection omits current terminal"
Assert-SourceContains $projectionSourcePath '"terminal_history"' "Projection omits terminal history"
Assert-SourceContains $contextLogPath 'object.get_mut("exact_summary")' "Tool logs do not redact exact_summary"
Assert-SourceContains $controlOutputPath 'serde_json::from_str::<JsonValue>(error)' "Control rejection projection does not preserve the runtime error"

$l2Hash = Get-Sha256 $l2Path
$manifestL2 = @($manifest.layers | Where-Object id -eq "L2")[0]
Assert-Equal ([string]$manifestL2.selected_targets[0].sha256) $l2Hash "L2 manifest hash drifted"
$l4 = @($manifest.layers | Where-Object id -eq "L4")[0]
Assert-Equal ([string]$l4.selected_targets[0].sha256) (Get-Sha256 $toolPath) "L4 manifest hash drifted"

Write-Output "R7.1 A2-B5 five-layer contract validation passed for $Phase."
