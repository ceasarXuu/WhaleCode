param(
    [string]$AuthorityPath = "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json",
    [string]$OutputPath = "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$authorityFile = Join-Path $repoRoot $AuthorityPath
$outputFile = Join-Path $repoRoot $OutputPath
$authority = Get-Content -Raw -Encoding UTF8 -LiteralPath $authorityFile | ConvertFrom-Json -Depth 50
$authoritySha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $authorityFile).Hash.ToLowerInvariant()

function New-Target {
    param(
        [string]$Artifact,
        [string]$ActivationPhase,
        [string]$SequencePreflight = ""
    )
    $artifactPath = Join-Path $repoRoot $Artifact
    if (-not (Test-Path -LiteralPath $artifactPath -PathType Leaf)) {
        throw "Production target does not exist: $Artifact"
    }
    $target = [ordered]@{
        artifact = $Artifact.Replace("\", "/")
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $artifactPath).Hash.ToLowerInvariant()
        activation_phase = $ActivationPhase
    }
    if (-not [string]::IsNullOrWhiteSpace($SequencePreflight)) {
        $target.sequence_preflight = $SequencePreflight.Replace("\", "/")
    }
    $target
}

function New-Layer {
    param(
        [string]$Id,
        [string]$Owner,
        [string]$Carrier,
        [object[]]$Targets
    )
    [ordered]@{
        id = $Id
        owner = $Owner
        carrier = $Carrier
        runtime_status = "active"
        selected_targets = @($Targets)
    }
}

$layers = @(
    New-Layer "L1" "base_instructions_profile" "first_system_message" @(
        (New-Target `
            "benchmarks/taskspace/r7/five-layer-l1-taskspace-base-section-v2.md" `
            "FLA-2")
    )
    New-Layer "L2" "taskspace_contract" "stable_developer_bundle_first_section" @(
        (New-Target `
            "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_core_protocol_v3.md" `
            "A2-C Agent-visible execution contract repair")
    )
    New-Layer "L3" "bundled_skill_registry" "skill_catalog_and_explicit_load" @(
        (New-Target `
            "benchmarks/taskspace/r7/five-layer-l3-taskspace-advanced-v1.SKILL.md" `
            "FLA-3")
    )
    New-Layer "L4" "taskspace_tool" "provider_tools" @(
        (New-Target `
            "third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs" `
            "A2-C optional mutations, provider identity, ownership, and single-Patch contract" `
            "third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs")
    )
    New-Layer "L5" "taskspace_runtime" "tool_result_and_projection" @(
        (New-Target `
            "third_party/codex-cli/codex-rs/core/src/tools/sequence.rs" `
            "W0 trusted provider-response failure carrier"),
        (New-Target `
            "benchmarks/taskspace/r7/projection-policy-contract.json" `
            "A2-B2.5 current terminal and terminal history projection"),
        (New-Target `
            "third_party/codex-cli/codex-rs/core/src/tools/parallel.rs" `
            "W0 exact failure provenance and trusted supplemental carrier"),
        (New-Target `
            "benchmarks/taskspace/r7/five-layer-taskspace-result-v3.contract.json" `
            "A2-C prepare and response-final factual result contract"),
        (New-Target `
            "benchmarks/taskspace/r7/five-layer-lifecycle-oracles-v2.json" `
            "A2-B2.5 close, reopen, terminal history, restart, and replay")
    )
)

$manifest = [ordered]@{
    schema_version = 1
    schema_path = "benchmarks/taskspace/r7/taskspace-contract-manifest-v1.schema.json"
    contract_id = "r7-taskspace-five-layer-production-v1"
    manifest_version = "1.0.48"
    contract_status = "production_active"
    runtime_status_enum = @(
        "selected_not_active",
        "active",
        "repair_active",
        "result_repair_active_projection_baseline",
        "carrier_active_projection_baseline",
        "carrier_repair_active",
        "carrier_result_repair_active_projection_baseline"
    )
    activation_through = "W0 evidence authority repair"
    source_authority = [ordered]@{
        contract_id = [string]$authority.contract_id
        path = $AuthorityPath.Replace("\", "/")
        sha256 = $authoritySha256
    }
    compatibility_policy = [string]$authority.compatibility_policy
    wire_order = [ordered]@{
        deepseek_chat = @("L1", "L2", "other_developer_sections_and_L3_catalog", "natural_history_and_loaded_L3", "L4", "L5")
    }
    layers = $layers
}

$parent = Split-Path -Parent $outputFile
if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
}
$json = $manifest | ConvertTo-Json -Depth 20
[System.IO.File]::WriteAllText($outputFile, "$json`n", [System.Text.UTF8Encoding]::new($false))
Write-Output "Wrote $OutputPath from $AuthorityPath (R7.1 atomic execution plan authority sync)."
