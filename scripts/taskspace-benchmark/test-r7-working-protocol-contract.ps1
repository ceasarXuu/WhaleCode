param(
    [string]$ContractPath = "benchmarks/taskspace/r7/working-protocol-contract.json",
    [string]$ProtocolSourcePath = "third_party/codex-cli/codex-rs/core/src/context/taskspace_working_protocol.rs",
    [string]$TurnSourcePath = "third_party/codex-cli/codex-rs/core/src/session/turn.rs",
    [string]$WireTraceSourcePath = "third_party/codex-cli/codex-rs/core/src/provider_wire_trace.rs"
)

$ErrorActionPreference = "Stop"

function Assert-ProtocolContract {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

$contract = Get-Content -Raw -Encoding UTF8 -LiteralPath $ContractPath | ConvertFrom-Json
$protocolSource = Get-Content -Raw -Encoding UTF8 -LiteralPath $ProtocolSourcePath
$turnSource = Get-Content -Raw -Encoding UTF8 -LiteralPath $TurnSourcePath
$wireTraceSource = Get-Content -Raw -Encoding UTF8 -LiteralPath $WireTraceSourcePath

$schema = [string]$contract.protocol.schema_version
$version = [string]$contract.protocol.protocol_version
$rulesHash = [string]$contract.protocol.rules_sha256

Assert-ProtocolContract ($contract.schema_version -eq 1) "working protocol contract schema changed"
Assert-ProtocolContract ($contract.scope.taskspace_projection_policies.Count -eq 3) "working protocol must cover exactly three R7 projection policies"
Assert-ProtocolContract (-not [bool]$contract.scope.policy_specific_variants_allowed) "policy-specific working protocol variants are forbidden"
Assert-ProtocolContract ($contract.delivery.role -eq "developer" -and $contract.delivery.provider_input_position -eq 0) "working protocol delivery prefix changed"
Assert-ProtocolContract ($contract.delivery.taskspace_count_per_request -eq 1 -and $contract.scope.standard_injection_count -eq 0) "working protocol cardinality contract changed"
Assert-ProtocolContract (-not [bool]$contract.delivery.dynamic_fields_allowed -and -not [bool]$contract.delivery.map_state_allowed) "dynamic state leaked into the working protocol"
Assert-ProtocolContract ($rulesHash -match '^[0-9a-f]{64}$') "working protocol rules hash is invalid"

Assert-ProtocolContract ($protocolSource.Contains(('"{0}"' -f $schema))) "protocol schema constant does not match contract"
Assert-ProtocolContract ($protocolSource.Contains(('"{0}"' -f $version))) "protocol version constant does not match contract"
Assert-ProtocolContract ($protocolSource.Contains(('"{0}"' -f $rulesHash))) "protocol rules hash constant does not match contract"
Assert-ProtocolContract ($turnSource.Contains("TaskspaceProviderToolVisibility::TaskspaceNative")) "TaskSpace-only injection gate is missing"
Assert-ProtocolContract ($turnSource.Contains("prepend_taskspace_working_protocol")) "provider-prefix injection is missing"
Assert-ProtocolContract ($wireTraceSource.Contains('schema_version: "provider-chat-wire-trace-v4"')) "wire trace v4 identity carrier is missing"
Assert-ProtocolContract ($wireTraceSource.Contains("taskspace_working_protocol_identity")) "wire trace protocol observer is missing"

Write-Output "R7 working protocol contract tests passed."
