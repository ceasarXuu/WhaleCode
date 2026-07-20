param(
    [string]$AuthorityPath = "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json",
    [string]$OutputPath = "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json",
    [ValidateSet("FLA-1", "FLA-2")]
    [string]$ActivationThrough = "FLA-1"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$authorityFile = Join-Path $repoRoot $AuthorityPath
$outputFile = Join-Path $repoRoot $OutputPath
$authority = Get-Content -Raw -Encoding UTF8 -LiteralPath $authorityFile | ConvertFrom-Json -Depth 50
$authoritySha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $authorityFile).Hash.ToLowerInvariant()

function Get-Target {
    param([string]$Layer)
    @($authority.selected_targets | Where-Object { [string]$_.layer -eq $Layer })[0]
}

function New-Layer {
    param(
        [string]$Id,
        [string]$Owner,
        [string]$Carrier,
        [string]$Status,
        [object[]]$Targets
    )
    [ordered]@{
        id = $Id
        owner = $Owner
        carrier = $Carrier
        status = $Status
        selected_targets = @($Targets | ForEach-Object {
                [ordered]@{
                    artifact = [string]$_.artifact
                    sha256 = [string]$_.sha256
                    activation_phase = [string]$_.activation_phase
                }
            })
    }
}

$l1Status = if ($ActivationThrough -eq "FLA-2") { "active" } else { "selected_not_active" }
$l2Status = if ($ActivationThrough -eq "FLA-2") { "active" } else { "selected_not_active" }
$layers = @(
    New-Layer "L1" "base_instructions_profile" "first_system_message" $l1Status @((Get-Target "L1"))
    New-Layer "L2" "taskspace_contract" "stable_developer_bundle_first_section" $l2Status @((Get-Target "L2"))
    New-Layer "L3" "bundled_skill_registry" "skill_catalog_and_explicit_load" "selected_not_active" @((Get-Target "L3"))
    New-Layer "L4" "taskspace_tool" "provider_tools" "baseline_active_target_pending" @((Get-Target "L4"))
    New-Layer "L5" "taskspace_runtime" "tool_result_and_projection" "baseline_active_target_pending" @(
        (Get-Target "L5-result"),
        (Get-Target "L5-projection"),
        (Get-Target "L5-lifecycle")
    )
)

$manifest = [ordered]@{
    schema_version = 1
    contract_id = "r7-taskspace-five-layer-production-v1"
    manifest_version = "1.0.0"
    activation_through = $ActivationThrough
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
Write-Output "Wrote $OutputPath from $AuthorityPath ($ActivationThrough)."
