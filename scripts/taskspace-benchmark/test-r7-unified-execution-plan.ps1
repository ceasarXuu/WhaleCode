$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/r7-execution-plan-contract.ps1")

function Get-Section {
    param([string]$Text, [string]$Start, [string]$End)
    $startIndex = $Text.IndexOf($Start, [StringComparison]::Ordinal)
    Assert-R7ExecutionPlan ($startIndex -ge 0) "Missing section: $Start"
    $endIndex = $Text.IndexOf($End, $startIndex + $Start.Length, [StringComparison]::Ordinal)
    Assert-R7ExecutionPlan ($endIndex -gt $startIndex) "Missing section boundary: $End"
    $Text.Substring($startIndex, $endIndex - $startIndex)
}

function Copy-Plan {
    param($Plan)
    $Plan | ConvertTo-Json -Depth 100 | ConvertFrom-Json -Depth 100 -NoEnumerate
}

function Assert-Rejected {
    param([string]$Name, [scriptblock]$Action)
    $rejected = $false
    try {
        & $Action
    } catch {
        $rejected = $true
    }
    Assert-R7ExecutionPlan $rejected "Negative execution plan fixture passed: $Name"
}

$currentPath = Join-Path $repoRoot "docs/v0.0.5/build-R7/47-r7.1-global-issue-register.md"
$legacyPath = Join-Path $repoRoot "docs/v0.0.5/build-R7/47-r7.1-global-issue-register-legacy.md"
$milestonePath = Join-Path $repoRoot "docs/v0.0.5/build-R7/40-r7.1-milestone-baseline.md"
$w0Path = Join-Path $repoRoot "docs/v0.0.5/build-R7/48-r7.1-w0-factual-foundation-result.md"
$schemaPath = Join-Path $repoRoot "benchmarks/taskspace/r7/r7-1-execution-plan-v1.schema.json"

foreach ($path in @($currentPath, $legacyPath, $milestonePath, $w0Path, $schemaPath)) {
    Assert-R7ExecutionPlan (Test-Path -LiteralPath $path -PathType Leaf) `
        "Missing R7.1 plan artifact: $path"
}

$bundle = Get-R7ExecutionPlanManifest $currentPath $schemaPath
$current = $bundle.text
$manifest = $bundle.manifest
$legacy = Get-Content -Raw -Encoding UTF8 -LiteralPath $legacyPath
$milestone = Get-Content -Raw -Encoding UTF8 -LiteralPath $milestonePath
$w0 = Get-Content -Raw -Encoding UTF8 -LiteralPath $w0Path

Assert-R7ExecutionPlan $current.StartsWith("# R7.1 原子执行清单") `
    "Current R7.1 authority title drifted"
Assert-R7ExecutionPlan ($current.Contains("- Confirmed defect roots: 10")) `
    "Defect root count drifted"
Assert-R7ExecutionPlan ($current.Contains("- Pending atomic units: 20")) `
    "Atomic unit count drifted"

Assert-R7ExecutionPlanSemantics $manifest
Assert-R7ExecutionPlanProjection $current $manifest

$definitionSection = Get-Section $current "## 4. 原子 Phase 工程说明" "## 5. 依赖、并行与停止规则"
$requiredFields = @(
    "- 入口：",
    "- 唯一改动域：",
    "- 不包含：",
    "- 产物：",
    "- 预期收益：",
    "- 独立验收：",
    "- 退出/分流：",
    "- 回退："
)
foreach ($phase in @($manifest.phases)) {
    $phaseId = [string]$phase.id
    $heading = "### ${phaseId}：$($phase.title)"
    Assert-R7ExecutionPlan ($definitionSection.Contains($heading)) `
        "$phaseId definition title drifted"
    $index = [array]::IndexOf(@($manifest.phases), $phase)
    $nextBoundary = if ($index -lt @($manifest.phases).Count - 1) {
        "### $($manifest.phases[$index + 1].id)："
    } else {
        "## 5. 依赖、并行与停止规则"
    }
    $phaseSection = Get-Section $current "### ${phaseId}：" $nextBoundary
    foreach ($field in $requiredFields) {
        Assert-R7ExecutionPlan ($phaseSection.Contains($field)) `
            "$phaseId is missing engineering field: $field"
    }
}

$activeExecution = Get-Section $current "## 2. 机器可读执行合同" "## 6. 历史根因迁移投影"
Assert-R7ExecutionPlan (-not [regex]::IsMatch($activeExecution, 'R71-\d{2}\.\d+')) `
    "Active execution plan retains a decimal subtask ID"
Assert-R7ExecutionPlan ($activeExecution.Contains(
        "R71-08 + R71-12 -> R71-14 -> R71-15"
    )) "Runtime multi-Patch safety and Agent behavior are not separated"
Assert-R7ExecutionPlan (-not $activeExecution.Contains("不触发关闭")) `
    "Active execution plan retains ambiguous not-triggered closure"

Assert-R7ExecutionPlan ($legacy.StartsWith("# R7.1 历史全局问题清单")) `
    "Legacy register is not frozen as history"
Assert-R7ExecutionPlan ($legacy.Contains("不得继续在本文件更新")) `
    "Legacy register lacks update prohibition"
Assert-R7ExecutionPlan ($milestone.Contains(
        '当前 Phase：`R71-01 direct failure carrier 证据合同`'
    )) "Milestone does not identify the current atomic phase"
Assert-R7ExecutionPlan ($milestone.Contains('`R71-01` 至 `R71-20`')) `
    "Milestone does not reference the full atomic sequence"
Assert-R7ExecutionPlan ($w0.Contains("- Current mapping: R71-01～R71-04、R71-09")) `
    "Historical W0 result lacks the current atomic mapping"

# Dependency graph, state, closure, root ownership, and promotion reachability mutants.
$futureDependency = Copy-Plan $manifest
$futureDependency.phases[0].depends_on = @("R71-02")
Assert-Rejected "future_dependency" {
    Assert-R7ExecutionPlanSemantics $futureDependency
}

$unknownDependency = Copy-Plan $manifest
$unknownDependency.phases[1].depends_on = @("R71-99")
Assert-Rejected "unknown_dependency" {
    Assert-R7ExecutionPlanSemantics $unknownDependency
}

$uncoveredRoot = Copy-Plan $manifest
$uncoveredRoot.phases[9].root_ids = @("R71-GI-007")
Assert-Rejected "defect_root_uncovered" {
    Assert-R7ExecutionPlanSemantics $uncoveredRoot
}

$orphanCostBlocker = Copy-Plan $manifest
$orphanCostBlocker.phases[18].blocks_dynamic_cost = $true
Assert-Rejected "dynamic_cost_blocker_not_ancestor" {
    Assert-R7ExecutionPlanSemantics $orphanCostBlocker
}

$orphanPromotionBlocker = Copy-Plan $manifest
$orphanPromotionBlocker.phases[19].blocks_promotion = $true
Assert-Rejected "promotion_blocker_not_ancestor" {
    Assert-R7ExecutionPlanSemantics $orphanPromotionBlocker
}

$duplicateParallelDomain = Copy-Plan $manifest
$duplicateParallelDomain.phases[0].status = "in_progress"
$duplicateParallelDomain.phases[1].status = "in_progress"
$duplicateParallelDomain.phases[1].change_domain_key = `
    [string]$duplicateParallelDomain.phases[0].change_domain_key
Assert-Rejected "parallel_change_domain_collision" {
    Assert-R7ExecutionPlanSemantics $duplicateParallelDomain
}

$prematureClose = Copy-Plan $manifest
$prematureClose.phases[7].status = "closed"
$prematureClose.phases[7].closure_outcome = "implemented"
$prematureClose.phases[7].evidence_artifact = "fixture.json"
Assert-Rejected "closed_before_dependency" {
    Assert-R7ExecutionPlanSemantics $prematureClose
}

$rootWithoutPhases = Copy-Plan $manifest
$rootWithoutPhases.phases[11].status = "closed"
$rootWithoutPhases.phases[11].closure_outcome = "root_causes_identified"
$rootWithoutPhases.phases[11].evidence_artifact = "fixture.json"
Assert-Rejected "root_outcome_without_spawned_phases" {
    Assert-R7ExecutionPlanSemantics $rootWithoutPhases
}

$earlyDecision = Copy-Plan $manifest
$earlyDecision.phases[19].status = "decision_pending"
Assert-Rejected "decision_before_dependency" {
    Assert-R7ExecutionPlanSemantics $earlyDecision
}

$heldOutCollision = Copy-Plan $manifest
$heldOutCollision.held_out_sets.promotion.identity = `
    [string]$heldOutCollision.held_out_sets.engineering.identity
Assert-Rejected "held_out_identity_collision" {
    Assert-R7ExecutionPlanSemantics $heldOutCollision
}

$projectionDrift = $current.Replace(
    "direct failure carrier 证据合同 | 返修",
    "direct failure carrier 漂移 | 返修"
)
Assert-Rejected "reader_projection_drift" {
    Assert-R7ExecutionPlanProjection $projectionDrift $manifest
}

$duplicateJson = '{"plan_id":"R7.1","plan_id":"drift"}'
$duplicateDocument = [System.Text.Json.JsonDocument]::Parse($duplicateJson)
try {
    Assert-Rejected "duplicate_manifest_property" {
        Assert-R7ExecutionPlanUniqueJsonProperties $duplicateDocument.RootElement
    }
} finally {
    $duplicateDocument.Dispose()
}

$invalidSchema = Copy-Plan $manifest
$invalidSchema.phases[0].status = "unknown_status"
$invalidRaw = $invalidSchema | ConvertTo-Json -Depth 100
Assert-Rejected "schema_invalid_status" {
    Assert-R7ExecutionPlan (
        $invalidRaw | Test-Json -SchemaFile $schemaPath -ErrorAction Stop
    ) "Schema accepted invalid status"
}

Write-Output "R7.1 atomic execution plan contract and negative fixtures passed."
