$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$path = Join-Path $repoRoot 'benchmarks/taskspace/native-control-contract.json'
$contract = Get-Content -Raw -Encoding UTF8 -LiteralPath $path | ConvertFrom-Json

if ([int]$contract.schema_version -ne 1) { throw 'Native control contract schema mismatch' }
if (@($contract.fixed_topology.nodes).Count -ne 3 -or @($contract.fixed_topology.edges).Count -ne 2) {
    throw 'Fixed topology must remain three nodes and two edges'
}
if ([string]$contract.hard_state_selection.state -ne 'active_task_path_without_nodes') {
    throw 'Hard-state selection source changed'
}
if ([string]$contract.hard_state_selection.chat_tool_choice.function.name -ne 'taskspace_control') {
    throw 'Named tool choice must select taskspace_control'
}
if ([string]$contract.hard_state_selection.thinking.type -ne 'disabled') {
    throw 'Named tool choice must disable thinking for DeepSeek'
}
if (-not [bool]$contract.ordered_barrier.stop_after_first_failure) {
    throw 'Barrier failure must stop the dependent tail'
}
$requiredSkipped = @('schema', 'status', 'reason', 'failed_call_id', 'sequence_index')
foreach ($field in $requiredSkipped) {
    if (@($contract.ordered_barrier.skipped_output.required_fields) -notcontains $field) {
        throw "Skipped output field missing: $field"
    }
}
if ([string]$contract.terminal_transaction.candidate_field -ne 'final_candidate' -or
    [string]$contract.terminal_transaction.candidate_transform -ne 'none' -or
    [bool]$contract.terminal_transaction.runtime_generated_candidate_allowed) {
    throw 'Terminal candidate provenance contract changed'
}
foreach ($forbidden in @('runtime_semantic_order_inference', 'runtime_generated_final', 'map_coarsening')) {
    if (@($contract.forbidden) -notcontains $forbidden) { throw "Forbidden behavior missing: $forbidden" }
}

Write-Host 'native control contract tests passed'
