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

function Assert-Plan {
    param($Plan)
    $raw = $Plan | ConvertTo-Json -Depth 100
    Assert-R7ExecutionPlan (
        $raw | Test-Json -SchemaFile $schemaPath -ErrorAction Stop
    ) "Mutated execution plan does not match its JSON Schema"
    Assert-R7ExecutionPlanSemantics $Plan $repoRoot
}

. (Join-Path $PSScriptRoot "lib/r7-execution-plan-test-fixtures.ps1")

$currentPath = Join-Path $repoRoot "docs/v0.0.5/build-R7/47-r7.1-global-issue-register.md"
$legacyPath = Join-Path $repoRoot "docs/v0.0.5/build-R7/47-r7.1-global-issue-register-legacy.md"
$milestonePath = Join-Path $repoRoot "docs/v0.0.5/build-R7/40-r7.1-milestone-baseline.md"
$w0Path = Join-Path $repoRoot "docs/v0.0.5/build-R7/48-r7.1-w0-factual-foundation-result.md"
$schemaPath = Join-Path $repoRoot "benchmarks/taskspace/r7/r7-1-execution-plan-v1.schema.json"
$phaseEvidenceSchemaPath = Join-Path $repoRoot `
    "benchmarks/taskspace/r7/r7-phase-evidence-v1.schema.json"
$heldOutSchemaPath = Join-Path $repoRoot `
    "benchmarks/taskspace/r7/r7-held-out-set-v1.schema.json"

foreach ($path in @(
        $currentPath, $legacyPath, $milestonePath, $w0Path,
        $schemaPath, $phaseEvidenceSchemaPath, $heldOutSchemaPath
    )) {
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
Assert-R7ExecutionPlan ($current.Contains(
        "- Pending atomic units: $($manifest.phase_count)"
    )) `
    "Atomic unit count drifted"

Assert-Plan $manifest
Assert-R7ExecutionPlanProjection $current $manifest
Assert-R7ExecutionPlanFailureRoutes $current $manifest

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
$currentPhase = $manifest.phases | Where-Object {
    [string]$_.id -eq [string]$manifest.current_phase_id
}
Assert-R7ExecutionPlan ($milestone.Contains(
        "当前 Phase：``$($currentPhase.id) $($currentPhase.title)``"
    )) "Milestone does not identify the current atomic phase"
Assert-R7ExecutionPlan ($milestone.Contains(
        "``R71-01`` 至 ``$($manifest.promotion_decision_phase_id)``"
    )) `
    "Milestone does not reference the full atomic sequence"
Assert-R7ExecutionPlan ($w0.Contains("- Current mapping: R71-01～R71-04、R71-09")) `
    "Historical W0 result lacks the current atomic mapping"

# Dependency graph, state, closure, evidence, routing, and promotion mutants.
$directFailurePhase = $manifest.phases[0]
$nestedBoundaryPhase = $manifest.phases[7]
$responseDiagnosisPhase = $manifest.phases[11]
$directFailureEvidence = New-R7TestEvidenceReference "strict_failure_carrier" `
    "direct-failure" @($directFailurePhase.observability.required_fields)
$nestedBoundaryEvidence = New-R7TestEvidenceReference "nested_boundary_pair" `
    "nested-boundary" @($nestedBoundaryPhase.observability.required_fields)
$responseDiagnosisEvidence = New-R7TestEvidenceReference `
    "sealed_response_action_trace" "response-diagnosis" `
    @($responseDiagnosisPhase.observability.required_fields)
$engineeringSetPath = "target/r7-execution-plan-selftest/engineering-samples.json"
$promotionSetPath = "target/r7-execution-plan-selftest/promotion-samples.json"
'{"schema_version":"r71-held-out-set-v1","artifact_type":"held_out_sample_manifest","sample_ids":["engineering-a"]}' |
    Set-Content -NoNewline -Encoding UTF8 -LiteralPath (Join-Path $repoRoot $engineeringSetPath)
'{"schema_version":"r71-held-out-set-v1","artifact_type":"held_out_sample_manifest","sample_ids":["promotion-a"]}' |
    Set-Content -NoNewline -Encoding UTF8 -LiteralPath (Join-Path $repoRoot $promotionSetPath)
$engineeringSetRef = [pscustomobject]@{
    path = $engineeringSetPath
    sha256 = (Get-FileHash -Algorithm SHA256 `
        -LiteralPath (Join-Path $repoRoot $engineeringSetPath)).Hash.ToLowerInvariant()
    artifact_type = "held_out_sample_manifest"
    schema_path = "benchmarks/taskspace/r7/r7-held-out-set-v1.schema.json"
    schema_version = "r71-held-out-set-v1"
}
$promotionSetRef = [pscustomobject]@{
    path = $promotionSetPath
    sha256 = (Get-FileHash -Algorithm SHA256 `
        -LiteralPath (Join-Path $repoRoot $promotionSetPath)).Hash.ToLowerInvariant()
    artifact_type = "held_out_sample_manifest"
    schema_path = "benchmarks/taskspace/r7/r7-held-out-set-v1.schema.json"
    schema_version = "r71-held-out-set-v1"
}

$futureDependency = Copy-Plan $manifest
$futureDependency.phases[0].depends_on = @("R71-02")
Assert-Rejected "future_dependency" {
    Assert-Plan $futureDependency
}

$unknownDependency = Copy-Plan $manifest
$unknownDependency.phases[1].depends_on = @("R71-99")
Assert-Rejected "unknown_dependency" {
    Assert-Plan $unknownDependency
}

$uncoveredRoot = Copy-Plan $manifest
$uncoveredRoot.phases[9].root_ids = @("R71-GI-007")
Assert-Rejected "defect_root_uncovered" {
    Assert-Plan $uncoveredRoot
}

$orphanPreCost = Copy-Plan $manifest
$fixedCost = $orphanPreCost.phases | Where-Object {
    [string]$_.acceptance_evidence_type -eq "fixed_component_ledger"
}
$fixedCost.depends_on = @($fixedCost.depends_on | Where-Object { $_ -ne "R71-15" })
Assert-Rejected "pre_cost_phase_orphaned_from_gate_chain" {
    Assert-Plan $orphanPreCost
}

$legacyBlockerFlag = Copy-Plan $manifest
$legacyBlockerFlag.phases[0] | Add-Member -NotePropertyName blocks_dynamic_cost `
    -NotePropertyValue $false
Assert-Rejected "mutable_blocker_flag_reintroduced" {
    Assert-Plan $legacyBlockerFlag
}

$duplicateParallelDomain = Copy-Plan $manifest
$duplicateParallelDomain.phases[0].status = "in_progress"
$duplicateParallelDomain.phases[1].status = "in_progress"
$duplicateParallelDomain.phases[1].change_domain_key = `
    [string]$duplicateParallelDomain.phases[0].change_domain_key
Assert-Rejected "parallel_change_domain_collision" {
    Assert-Plan $duplicateParallelDomain
}

$blockedInProgress = Copy-Plan $manifest
$blockedInProgress.phases[7].status = "in_progress"
Assert-Rejected "in_progress_before_dependency" {
    Assert-Plan $blockedInProgress
}

$notReadyCurrent = Copy-Plan $manifest
$notReadyCurrent.current_phase_id = "R71-08"
Assert-Rejected "current_phase_not_ready" {
    Assert-Plan $notReadyCurrent
}

$prematureClose = Copy-Plan $manifest
$prematureClose.phases[7].status = "closed"
$prematureClose.phases[7].closure_outcome = "implemented"
$prematureClose.phases[7].evidence_artifact = $nestedBoundaryEvidence
Assert-Rejected "closed_before_dependency" {
    Assert-Plan $prematureClose
}

$rootWithoutPhases = Copy-Plan $manifest
$rootWithoutPhases.phases[11].status = "closed"
$rootWithoutPhases.phases[11].closure_outcome = "root_causes_identified"
$rootWithoutPhases.phases[11].evidence_artifact = $responseDiagnosisEvidence
Assert-Rejected "root_outcome_without_spawned_phases" {
    Assert-Plan $rootWithoutPhases
}

$spawnedAncestor = Copy-Plan $manifest
foreach ($phaseId in @(
        "R71-01", "R71-02", "R71-03", "R71-04",
        "R71-05", "R71-09", "R71-10", "R71-11"
    )) {
    $phase = $spawnedAncestor.phases | Where-Object { [string]$_.id -eq $phaseId }
    $phase.status = "closed"
    $phase.closure_outcome = [string]$phase.allowed_closure_outcomes[0]
    $phase.evidence_artifact = New-R7TestEvidenceReference `
        ([string]$phase.acceptance_evidence_type) "closed-$phaseId" `
        @($phase.observability.required_fields)
}
$spawnedAncestor.phases[11].status = "closed"
$spawnedAncestor.phases[11].closure_outcome = "root_causes_identified"
$spawnedAncestor.phases[11].evidence_artifact = $responseDiagnosisEvidence
$spawnedAncestor.phases[11].spawned_repairs = @(
    [pscustomobject]@{root_cause_id = "R71-GI-007"; phase_id = "R71-01"}
)
$spawnedAncestor.phases[0].parent_diagnosis_id = "R71-12"
$spawnedAncestor.current_phase_id = "R71-06"
Assert-Rejected "diagnosis_spawns_existing_ancestor" {
    Assert-Plan $spawnedAncestor
}

$earlyDecision = Copy-Plan $manifest
$earlyDecision.phases[19].status = "decision_pending"
Assert-Rejected "decision_before_dependency" {
    Assert-Plan $earlyDecision
}

$heldOutCollision = Copy-Plan $manifest
$heldOutCollision.held_out_sets.promotion.identity = `
    [string]$heldOutCollision.held_out_sets.engineering.identity
Assert-Rejected "held_out_identity_collision" {
    Assert-Plan $heldOutCollision
}

$heldOutSeparated = Copy-Plan $manifest
$heldOutSeparated.held_out_sets.engineering.sample_manifest = $engineeringSetRef
$heldOutSeparated.held_out_sets.promotion.sample_manifest = $promotionSetRef
Assert-Plan $heldOutSeparated
$heldOutOverlap = Copy-Plan $heldOutSeparated
$heldOutOverlap.held_out_sets.promotion.sample_manifest = `
    Copy-Plan $engineeringSetRef
Assert-Rejected "held_out_sample_overlap" {
    Assert-Plan $heldOutOverlap
}

$specialRoleRebind = Copy-Plan $manifest
$specialRoleRebind.promotion_decision_phase_id = `
    [string]$specialRoleRebind.formal_evaluation_phase_id
Assert-Rejected "special_role_rebound" {
    Assert-Plan $specialRoleRebind
}

$routeRoleRebind = Copy-Plan $manifest
$routeRoleRebind.route_role_phase_ids.nested_dispatch_boundary = "R71-13"
Assert-Rejected "failure_route_role_rebound" {
    Assert-Plan $routeRoleRebind
}

$duplicateEvidenceType = Copy-Plan $manifest
$duplicateEvidenceType.phases[12].acceptance_evidence_type = `
    [string]$duplicateEvidenceType.phases[7].acceptance_evidence_type
Assert-Rejected "duplicate_acceptance_evidence_type" {
    Assert-Plan $duplicateEvidenceType
}

$existingPhaseReuse = Copy-Plan $manifest
$existingPhaseReuse.phases[13].failure_route.existing_phase_reuse = "permitted"
Assert-Rejected "existing_phase_reuse_permitted" {
    Assert-Plan $existingPhaseReuse
}

$duplicateEvent = Copy-Plan $manifest
$duplicateEvent.phases[1].observability.event_name = `
    [string]$duplicateEvent.phases[0].observability.event_name
Assert-Rejected "duplicate_observability_event" {
    Assert-Plan $duplicateEvent
}

$missingEvidence = Copy-Plan $manifest
$missingEvidence.phases[0].status = "closed"
$missingEvidence.phases[0].closure_outcome = "implemented"
$missingEvidence.phases[0].evidence_artifact = [pscustomobject]@{
    path = "missing://phase-evidence.json"
    sha256 = "0" * 64
    artifact_type = "strict_failure_carrier"
    schema_path = "benchmarks/taskspace/r7/r7-phase-evidence-v1.schema.json"
    schema_version = "r71-phase-evidence-v1"
}
$missingEvidence.current_phase_id = "R71-02"
Assert-Rejected "closed_with_missing_evidence" {
    Assert-Plan $missingEvidence
}

$wrongEvidenceHash = Copy-Plan $manifest
$wrongEvidenceHash.phases[0].status = "closed"
$wrongEvidenceHash.phases[0].closure_outcome = "implemented"
$wrongEvidenceHash.phases[0].evidence_artifact = Copy-Plan $directFailureEvidence
$wrongEvidenceHash.phases[0].evidence_artifact.sha256 = "0" * 64
$wrongEvidenceHash.current_phase_id = "R71-02"
Assert-Rejected "closed_with_wrong_evidence_hash" {
    Assert-Plan $wrongEvidenceHash
}

$wrongEvidenceType = Copy-Plan $manifest
$wrongEvidenceType.phases[0].status = "closed"
$wrongEvidenceType.phases[0].closure_outcome = "implemented"
$wrongEvidenceType.phases[0].evidence_artifact = $nestedBoundaryEvidence
$wrongEvidenceType.current_phase_id = "R71-02"
Assert-Rejected "closed_with_wrong_evidence_type" {
    Assert-Plan $wrongEvidenceType
}

$wrongEvidenceSchema = Copy-Plan $manifest
$wrongEvidenceSchema.phases[0].status = "closed"
$wrongEvidenceSchema.phases[0].closure_outcome = "implemented"
$wrongEvidenceSchema.phases[0].evidence_artifact = Copy-Plan $directFailureEvidence
$wrongEvidenceSchema.phases[0].evidence_artifact.schema_version = "unknown"
$wrongEvidenceSchema.current_phase_id = "R71-02"
Assert-Rejected "closed_with_unapproved_evidence_schema" {
    Assert-Plan $wrongEvidenceSchema
}

$evidenceLinkPath = "target/r7-execution-plan-selftest/direct-failure-link.json"
$evidenceLinkFullPath = Join-Path $repoRoot $evidenceLinkPath
if (-not (Test-Path -LiteralPath $evidenceLinkFullPath)) {
    [void](New-Item -ItemType SymbolicLink -Path $evidenceLinkFullPath `
        -Target (Join-Path $repoRoot $directFailureEvidence.path))
}
$symlinkEvidence = Copy-Plan $directFailureEvidence
$symlinkEvidence.path = $evidenceLinkPath
$symlinkClosure = Copy-Plan $manifest
$symlinkClosure.phases[0].status = "closed"
$symlinkClosure.phases[0].closure_outcome = "implemented"
$symlinkClosure.phases[0].evidence_artifact = $symlinkEvidence
$symlinkClosure.current_phase_id = "R71-02"
Assert-Rejected "evidence_symlink_escape_surface" {
    Assert-Plan $symlinkClosure
}

$malformedEvidencePath = "target/r7-execution-plan-selftest/malformed-same-type.json"
$malformedEvidenceFullPath = Join-Path $repoRoot $malformedEvidencePath
'{"schema_version":"r71-phase-evidence-v1","artifact_type":"strict_failure_carrier","records":[{"unrelated":"fixture"}]}' |
    Set-Content -NoNewline -Encoding UTF8 -LiteralPath $malformedEvidenceFullPath
$malformedEvidence = [pscustomobject]@{
    path = $malformedEvidencePath
    sha256 = (Get-FileHash -Algorithm SHA256 `
        -LiteralPath $malformedEvidenceFullPath).Hash.ToLowerInvariant()
    artifact_type = "strict_failure_carrier"
    schema_path = "benchmarks/taskspace/r7/r7-phase-evidence-v1.schema.json"
    schema_version = "r71-phase-evidence-v1"
}
$sameTypeInvalidEvidence = Copy-Plan $manifest
$sameTypeInvalidEvidence.phases[0].status = "closed"
$sameTypeInvalidEvidence.phases[0].closure_outcome = "implemented"
$sameTypeInvalidEvidence.phases[0].evidence_artifact = $malformedEvidence
$sameTypeInvalidEvidence.current_phase_id = "R71-02"
Assert-Rejected "same_type_evidence_missing_required_fields" {
    Assert-Plan $sameTypeInvalidEvidence
}

$projectionDrift = $current.Replace(
    "direct failure carrier 证据合同 | 返修",
    "direct failure carrier 漂移 | 返修"
)
Assert-Rejected "reader_projection_drift" {
    Assert-R7ExecutionPlanProjection $projectionDrift $manifest
}

$failureRouteProjectionDrift = $current.Replace(
    "actual_dispatch_or_commit_bypass | spawn_atomic_phase | forbidden",
    "actual_dispatch_or_commit_bypass | spawn_atomic_phase | permitted"
)
Assert-Rejected "failure_route_projection_drift" {
    Assert-R7ExecutionPlanFailureRoutes $failureRouteProjectionDrift $manifest
}

$failureRouteAlias = $current.Replace(
    "- 退出/分流：关闭结果使用机器合同；失败路由只使用第 3.1 节机器投影，不复用既有 Phase。",
    "- 退出/分流：把 nested dispatcher 硬边界实现直接返修。"
)
Assert-Rejected "failure_route_title_alias" {
    Assert-R7ExecutionPlanFailureRoutes $failureRouteAlias $manifest
}

$failureRollbackAlias = $current.Replace(
    "- 回退：本节不维护失败目标；只使用第 3.1 节机器投影。",
    "- 回退：重新打开 nested dispatcher 硬边界实现。"
)
Assert-Rejected "failure_route_rollback_alias" {
    Assert-R7ExecutionPlanFailureRoutes $failureRollbackAlias $manifest
}

$canonicalExit = "- 退出/分流：关闭结果使用机器合同；失败路由只使用第 3.1 节机器投影，不复用既有 Phase。"
$canonicalRollback = "- 回退：本节不维护失败目标；只使用第 3.1 节机器投影。"
$routeDirectiveMutants = @(
    @{ Name = "additive_exit_dash"; Anchor = $canonicalExit; Addition = "- 退出/分流：重新打开既有 Phase。" }
    @{ Name = "additive_exit_star"; Anchor = $canonicalExit; Addition = "* 退出/分流：重新打开既有 Phase。" }
    @{ Name = "additive_exit_indent"; Anchor = $canonicalExit; Addition = "  - 退出/分流：重新打开既有 Phase。" }
    @{ Name = "additive_exit_bold"; Anchor = $canonicalExit; Addition = "- **退出/分流：**重新打开既有 Phase。" }
    @{ Name = "additive_exit_partial_bold"; Anchor = $canonicalExit; Addition = "- 退**出/分流**：重新打开既有 Phase。" }
    @{ Name = "additive_exit_fullwidth_slash"; Anchor = $canonicalExit; Addition = "- 退出／分流：重新打开既有 Phase。" }
    @{ Name = "additive_exit_escaped_slash"; Anchor = $canonicalExit; Addition = "- 退出\/分流：重新打开既有 Phase。" }
    @{ Name = "additive_exit_reference_link"; Anchor = $canonicalExit; Addition = "- [退出][route]/分流：重新打开既有 Phase。`n[route]: https://example.invalid" }
    @{ Name = "duplicate_canonical_exit"; Anchor = $canonicalExit; Addition = $canonicalExit }
    @{ Name = "additive_rollback_dash"; Anchor = $canonicalRollback; Addition = "- 回退：重新打开既有 Phase。" }
    @{ Name = "additive_rollback_star"; Anchor = $canonicalRollback; Addition = "* 回退：重新打开既有 Phase。" }
    @{ Name = "additive_rollback_indent"; Anchor = $canonicalRollback; Addition = "  - 回退：重新打开既有 Phase。" }
    @{ Name = "additive_rollback_bold"; Anchor = $canonicalRollback; Addition = "- **回退：**重新打开既有 Phase。" }
    @{ Name = "additive_rollback_partial_bold"; Anchor = $canonicalRollback; Addition = "- 回**退**：重新打开既有 Phase。" }
    @{ Name = "additive_rollback_partial_italic"; Anchor = $canonicalRollback; Addition = "- 回_退_：重新打开既有 Phase。" }
    @{ Name = "additive_rollback_link_title"; Anchor = $canonicalRollback; Addition = '- [回](https://example.invalid "route (rollback)")退：重新打开既有 Phase。' }
    @{ Name = "additive_rollback_html"; Anchor = $canonicalRollback; Addition = '- 回<strong title="route > rollback">退</strong>：重新打开既有 Phase。' }
    @{ Name = "duplicate_canonical_rollback"; Anchor = $canonicalRollback; Addition = $canonicalRollback }
)
foreach ($mutant in $routeDirectiveMutants) {
    $mutatedDocument = $current.Replace($mutant.Anchor, "$($mutant.Anchor)`n$($mutant.Addition)")
    Assert-Rejected "failure_route_$($mutant.Name)" {
        Assert-R7ExecutionPlanFailureRoutes $mutatedDocument $manifest
    }
}
foreach ($emptyValue in @("", " ", "`t")) {
    $emptyFieldDocument = $current.Replace("- 入口：测量与反馈前置关闭。", "- 入口：$emptyValue")
    Assert-Rejected "failure_route_empty_entry" {
        Assert-R7ExecutionPlanFailureRoutes $emptyFieldDocument $manifest
    }
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
Assert-Rejected "schema_invalid_status" {
    Assert-Plan $invalidSchema
}

$nearCostInsertNumber = [int]([string]$manifest.dynamic_cost_phase_id).Substring(4) - 1
$insertedPhasePlan = Add-R7TestPlanPhaseAt $manifest $nearCostInsertNumber
Assert-Plan $insertedPhasePlan
Assert-R7ExecutionPlan (
    [int]$insertedPhasePlan.phase_count -eq [int]$manifest.phase_count + 1
) "Positive insertion fixture did not increase phase_count"
Assert-R7ExecutionPlan (
    [string]$insertedPhasePlan.promotion_decision_phase_id -eq "R71-21"
) "Positive insertion fixture did not reindex the promotion phase"
$arbitraryInsertionPlan = Add-R7TestPlanPhaseAt $manifest 10
Assert-Plan $arbitraryInsertionPlan
Assert-R7ExecutionPlan (
    [string]$arbitraryInsertionPlan.route_role_phase_ids.multi_patch_runtime_safety -eq "R71-15"
) "Arbitrary insertion fixture did not reindex route roles"

Write-Output "R7.1 execution plan contract, mutants, and insertion fixture passed."
