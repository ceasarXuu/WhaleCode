param(
    [ValidateSet("FLA-0", "FLA-1", "FLA-2", "FLA-4", "FLA-5", "All")]
    [string]$Phase = "All"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$authorityPath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json"
$manifestPath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json"
$taskspaceBasePath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/protocol/src/prompts/base_instructions/whalecode_taskspace.md"
$standardBasePath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/protocol/src/prompts/base_instructions/whalecode_standard.md"
$l1Path = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-l1-taskspace-base-section-v2.md"
$l2Path = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-l2-core-protocol-v2.md"
$productionL2Path = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_core_protocol_v2.md"
$l4Path = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-taskspace-control-v2.schema.json"
$l5ResultPath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-taskspace-result-v2.schema.json"

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -cne $Expected) {
        throw "$Message. expected=$Expected actual=$Actual"
    }
}

function Get-Sha256 {
    param([string]$Path)
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Get-TextSha256 {
    param([string]$Text)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    [System.BitConverter]::ToString(
        [System.Security.Cryptography.SHA256]::Create().ComputeHash($bytes)
    ).Replace("-", "").ToLowerInvariant()
}

function Get-GitBlobText {
    param([string]$Commit, [string]$Path)
    $text = & git -C $repoRoot show "${Commit}:$Path" 2>$null
    if ($LASTEXITCODE -ne 0) { throw "Unable to read frozen blob ${Commit}:$Path" }
    ([string]::Join("`n", @($text))) + "`n"
}

function Test-PhaseEnabled {
    param([string]$Name)
    $Phase -eq "All" -or $Phase -eq $Name
}

$authority = Get-Content -Raw -Encoding UTF8 -LiteralPath $authorityPath | ConvertFrom-Json -Depth 50
Assert-Equal $authority.contract_id "r7-five-layer-contract-authority-v1" "Unexpected authority contract"
Assert-Equal $authority.compatibility_policy "none" "Five-layer migration must not keep compatibility paths"

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

if (Test-PhaseEnabled "FLA-0") {
    $baseline = $authority.baseline
    foreach ($entry in @($baseline.taskspace_base, $baseline.tool_schema_source, $baseline.argument_parser, $baseline.result_formatter, $baseline.projection_contract)) {
        $frozenText = Get-GitBlobText ([string]$baseline.commit) ([string]$entry.path)
        Assert-Equal (Get-TextSha256 $frozenText) ([string]$entry.sha256) "Frozen baseline hash drifted: $($entry.path)"
    }
    & git -C $repoRoot cat-file -e "$($baseline.commit)^{commit}" 2>$null
    $baselineCommitExit = $LASTEXITCODE
    Assert-True ($baselineCommitExit -eq 0) "Frozen baseline commit is unavailable"
    Write-Output "FLA-0 frozen source contracts passed."
}

if (Test-PhaseEnabled "FLA-1") {
    Assert-True (Test-Path -LiteralPath $manifestPath -PathType Leaf) "Production contract manifest is missing"
    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50
    Assert-Equal $manifest.contract_id "r7-taskspace-five-layer-production-v1" "Unexpected production manifest"
    Assert-Equal $manifest.source_authority.contract_id $authority.contract_id "Manifest authority id drifted"
    Assert-Equal $manifest.source_authority.sha256 (Get-Sha256 $authorityPath) "Manifest authority hash drifted"
    Assert-Equal @($manifest.layers).Count 5 "Production manifest must own exactly five layers"
    Assert-Equal ((@($manifest.layers | ForEach-Object { [string]$_.id } | Sort-Object)) -join ",") "L1,L2,L3,L4,L5" "Layer ids drifted"
    Assert-Equal $manifest.wire_order.deepseek_chat[0] "L1" "L1 must be first on DeepSeek wire"
    Assert-Equal $manifest.wire_order.deepseek_chat[1] "L2" "L2 must be the second logical section"
    foreach ($layer in @($manifest.layers)) {
        Assert-True (-not [string]::IsNullOrWhiteSpace([string]$layer.owner)) "Layer owner missing: $($layer.id)"
        Assert-True (-not [string]::IsNullOrWhiteSpace([string]$layer.carrier)) "Layer carrier missing: $($layer.id)"
    }
    $contextModule = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/context/taskspace_contract.rs")
    $traceSource = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/provider_wire_trace.rs")
    Assert-True $contextModule.Contains("taskspace_contract_manifest_v1.json") "Context module does not own the production manifest"
    Assert-True $traceSource.Contains("taskspace_contract_manifest_identity") "Provider wire trace lacks manifest identity"
    Write-Output "FLA-1 ownership and observability contracts passed."
}

if (Test-PhaseEnabled "FLA-2") {
    Assert-True (Test-Path -LiteralPath $productionL2Path -PathType Leaf) "Production L2 artifact is missing"
    Assert-Equal (Get-Sha256 $productionL2Path) (Get-Sha256 $l2Path) "Production L2 bytes differ from authority artifact"

    $taskspaceBase = [System.IO.File]::ReadAllText($taskspaceBasePath)
    $standardBase = [System.IO.File]::ReadAllText($standardBasePath)
    $l1 = [System.IO.File]::ReadAllText($l1Path)
    $l2 = [System.IO.File]::ReadAllText($l2Path)
    $l1Start = $taskspaceBase.IndexOf("## TaskSpace work map", [System.StringComparison]::Ordinal)
    $l1End = $taskspaceBase.IndexOf("## Task execution", $l1Start, [System.StringComparison]::Ordinal)
    Assert-True ($l1Start -ge 0 -and $l1End -gt $l1Start) "TaskSpace L1 section boundaries are missing"
    $actualL1 = $taskspaceBase.Substring($l1Start, $l1End - $l1Start).TrimEnd("`r", "`n") + "`n"
    Assert-Equal $actualL1 $l1 "Production L1 section differs from authority artifact"
    Assert-Equal ([regex]::Matches($taskspaceBase, [regex]::Escape($l1)).Count) 1 "TaskSpace base must contain L1 exactly once"
    Assert-Equal ([regex]::Matches($taskspaceBase, [regex]::Escape($l2)).Count) 0 "L2 must not be embedded in TaskSpace base"
    Assert-Equal ([regex]::Matches($standardBase, "TaskSpace work map|taskspace_core_protocol").Count) 0 "Standard base contains TaskSpace content"
    foreach ($fragment in @('*** Begin Patch', '*** Update File:', '{"command"', '{"input"', '"arguments"')) {
        Assert-True (-not $standardBase.Contains($fragment)) "Standard Base embeds L4 Tool wire syntax: $fragment"
        Assert-True (-not $taskspaceBase.Contains($fragment)) "TaskSpace Base embeds L4 Tool wire syntax: $fragment"
    }

    $sessionSource = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/session/mod.rs")
    $traceSource = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/provider_wire_trace.rs")
    Assert-True $sessionSource.Contains("taskspace_core_protocol(map_runtime_mode)") "Session does not select L2 from runtime mode"
    Assert-True $sessionSource.Contains("developer_sections.push(core_protocol.to_string())") "L2 is not prepended to the stable developer bundle"
    Assert-True $traceSource.Contains("taskspace_core_protocol_identity") "Provider wire trace lacks L2 identity"

    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50
    Assert-Equal $manifest.activation_through "FLA-2" "Production manifest activation status drifted"
    Assert-Equal ([string](@($manifest.layers | Where-Object id -eq "L1")[0].status)) "active" "L1 is not active"
    Assert-Equal ([string](@($manifest.layers | Where-Object id -eq "L2")[0].status)) "active" "L2 is not active"
    Write-Output "FLA-2 L1/L2 production contracts passed."
}

if (Test-PhaseEnabled "FLA-4") {
    $l4Target = @($authority.selected_targets | Where-Object layer -eq "L4")[0]
    Assert-Equal ([string]$l4Target.status) "active_repair_verified" "L4 repair activation status drifted"
    $selectedSchema = Get-Content -Raw -Encoding UTF8 -LiteralPath $l4Path | ConvertFrom-Json -Depth 50
    $selectedActions = @($selectedSchema.provider_tool.function.parameters.anyOf | ForEach-Object { [string]$_.properties.action.enum[0] })
    foreach ($action in @("bind_node", "block_node", "unblock_node", "rework_node")) {
        Assert-True ($selectedActions -contains $action) "Selected L4 schema omits direct action: $action"
    }
    Assert-True ($selectedActions -notcontains "transition_node") "Selected L4 schema retains transition_node"

    $toolSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs"))
    $wireSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_args_wire.rs"))
    Assert-True (-not $toolSource.Contains('"transition_node"')) "Provider Tool still exposes transition_node"
    Assert-True (-not $wireSource.Contains('TransitionNode')) "Argument wire still accepts transition_node"
    foreach ($action in @("bind_node", "block_node", "unblock_node", "rework_node")) {
        Assert-True ($toolSource.Contains('"' + $action + '"')) "Provider Tool source omits direct action: $action"
    }
    foreach ($variant in @("BindNode", "BlockNode", "UnblockNode", "ReworkNode")) {
        Assert-True $wireSource.Contains("Action::$variant") "Argument wire omits direct action variant: $variant"
    }
    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50
    Assert-Equal ([string](@($manifest.layers | Where-Object id -eq "L4")[0].status)) "repair_active" "Production manifest does not expose the L4 repair"
    Write-Output "FLA-4 selected input contract repair passed."
}

if (Test-PhaseEnabled "FLA-5") {
    $l5Target = @($authority.selected_targets | Where-Object layer -eq "L5-result")[0]
    Assert-Equal ([string]$l5Target.status) "active_repair_verified" "L5 result repair activation status drifted"
    $resultSchema = Get-Content -Raw -Encoding UTF8 -LiteralPath $l5ResultPath | ConvertFrom-Json -Depth 50
    Assert-Equal ([string]$resultSchema.properties.schema_version.const) "TaskSpaceControlResultV2" "Selected result schema version drifted"
    Assert-Equal ([bool]$resultSchema.properties.partial_commit.const) $false "partial_commit must remain false"

    $argsSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_args.rs"))
    $outputSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_output.rs"))
    $preflightSource = [System.IO.File]::ReadAllText((Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/sequence_preflight.rs"))
    Assert-True $argsSource.Contains('TaskSpaceControlResultV2') "Production result version is not V2"
    Assert-True $outputSource.Contains('"partial_commit": false') "Production result formatter does not emit boolean partial_commit=false"
    Assert-True $preflightSource.Contains('TASKSPACE_REQUIRED_SIBLING_MISSING') "Control preflight does not emit the selected factual error"
    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 50
    Assert-Equal ([string](@($manifest.layers | Where-Object id -eq "L5")[0].status)) "result_repair_active_projection_baseline" "Production manifest does not expose the L5 result repair"
    Write-Output "FLA-5 selected result contract repair passed."
}

Write-Output "R7 five-layer contract validation passed for $Phase."
