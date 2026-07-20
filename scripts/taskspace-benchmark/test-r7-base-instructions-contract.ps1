param(
    [string]$ContractPath = "benchmarks/taskspace/r7/base-instructions-contract.json",
    [string]$CodexBasePath = "third_party/codex-cli/codex-rs/protocol/src/prompts/base_instructions/default.md",
    [string]$ModelCatalogPath = "third_party/codex-cli/codex-rs/models-manager/models.json",
    [string]$ProfileSourcePath = "third_party/codex-cli/codex-rs/core/src/context/base_instructions_profile.rs",
    [string]$TurnSourcePath = "third_party/codex-cli/codex-rs/core/src/session/turn.rs",
    [string]$WireTraceSourcePath = "third_party/codex-cli/codex-rs/core/src/provider_wire_trace.rs"
)

$ErrorActionPreference = "Stop"

function Assert-BaseInstructionsContract {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

$contract = Get-Content -Raw -Encoding UTF8 -LiteralPath $ContractPath | ConvertFrom-Json
$profileSource = Get-Content -Raw -Encoding UTF8 -LiteralPath $ProfileSourcePath
$turnSource = Get-Content -Raw -Encoding UTF8 -LiteralPath $TurnSourcePath
$wireTraceSource = Get-Content -Raw -Encoding UTF8 -LiteralPath $WireTraceSourcePath

$standard = $contract.profiles.standard
$taskspace = $contract.profiles.taskspace
$codexBase = Get-Content -Encoding UTF8 -LiteralPath $CodexBasePath
$standardBase = Get-Content -Encoding UTF8 -LiteralPath ([string]$standard.source)
$taskspaceBase = Get-Content -Raw -Encoding UTF8 -LiteralPath ([string]$taskspace.source)
$modelCatalog = Get-Content -Raw -Encoding UTF8 -LiteralPath $ModelCatalogPath
$standardDiffCount = 0
Assert-BaseInstructionsContract ($codexBase.Count -eq $standardBase.Count) "standard base must retain the Codex prompt structure"
for ($index = 0; $index -lt $codexBase.Count; $index++) {
    if ([string]$codexBase[$index] -cne [string]$standardBase[$index]) { $standardDiffCount++ }
}

Assert-BaseInstructionsContract ($contract.schema_version -eq 1) "base instructions contract schema changed"
Assert-BaseInstructionsContract ([bool]$contract.selection.exactly_one_complete_base_per_request) "requests must select one complete base"
Assert-BaseInstructionsContract (-not [bool]$contract.selection.separate_taskspace_developer_protocol_allowed) "separate TaskSpace developer protocol is forbidden"
Assert-BaseInstructionsContract (-not [bool]$contract.selection.projection_policy_variants_allowed) "projection policies must share the TaskSpace base"
Assert-BaseInstructionsContract ($contract.shared_runtime.projection_policies.Count -eq 3) "dual base contract must cover all R7 projection policies"
Assert-BaseInstructionsContract ([string]$standard.sha256 -match '^[0-9a-f]{64}$') "standard base hash is invalid"
Assert-BaseInstructionsContract ([string]$taskspace.sha256 -match '^[0-9a-f]{64}$') "TaskSpace base hash is invalid"
Assert-BaseInstructionsContract ($standardDiffCount -eq 2) "standard base must differ from Codex in exactly two branding lines"
Assert-BaseInstructionsContract ((Get-FileHash -Algorithm SHA256 -LiteralPath ([string]$standard.source)).Hash.ToLowerInvariant() -eq [string]$standard.sha256) "standard file hash does not match contract"
Assert-BaseInstructionsContract ((Get-FileHash -Algorithm SHA256 -LiteralPath ([string]$taskspace.source)).Hash.ToLowerInvariant() -eq [string]$taskspace.sha256) "TaskSpace file hash does not match contract"
Assert-BaseInstructionsContract (-not (($standardBase -join "`n").Contains("Codex agent foundation") -or ($standardBase -join "`n").Contains("optimized for DeepSeek"))) "standard base contains non-operational product background"
Assert-BaseInstructionsContract (-not ($taskspaceBase.Contains("Codex agent foundation") -or $taskspaceBase.Contains("optimized for DeepSeek"))) "TaskSpace base contains non-operational product background"
Assert-BaseInstructionsContract ($taskspaceBase.Contains("## TaskSpace work map")) "TaskSpace base lacks the integrated work-map section"
Assert-BaseInstructionsContract (-not $taskspaceBase.Contains('`update_plan`')) "TaskSpace base still teaches the linear plan tool"
Assert-BaseInstructionsContract (-not $modelCatalog.Contains("You are Whale, a terminal coding agent optimized for DeepSeek")) "obsolete short DeepSeek base remains in the model catalog"

Assert-BaseInstructionsContract ($profileSource.Contains(('"{0}"' -f [string]$standard.version))) "standard version constant does not match contract"
Assert-BaseInstructionsContract ($profileSource.Contains(('"{0}"' -f [string]$standard.sha256))) "standard hash constant does not match contract"
Assert-BaseInstructionsContract ($profileSource.Contains(('"{0}"' -f [string]$taskspace.version))) "TaskSpace version constant does not match contract"
Assert-BaseInstructionsContract ($profileSource.Contains(('"{0}"' -f [string]$taskspace.sha256))) "TaskSpace hash constant does not match contract"
Assert-BaseInstructionsContract ($turnSource.Contains("resolved_base_instructions.profile.is_taskspace()")) "base profile and tool visibility are not selected from one snapshot"
Assert-BaseInstructionsContract (-not $turnSource.Contains("prepend_taskspace_working_protocol")) "obsolete TaskSpace developer protocol injection remains"
Assert-BaseInstructionsContract ($wireTraceSource.Contains('schema_version: "provider-chat-wire-trace-v6"')) "wire trace v6 identity carrier is missing"
Assert-BaseInstructionsContract ($wireTraceSource.Contains("base_instructions_identity")) "wire trace base identity observer is missing"

Write-Output "R7 dual base instructions contract tests passed."
