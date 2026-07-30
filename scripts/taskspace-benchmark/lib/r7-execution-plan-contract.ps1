function Assert-R7ExecutionPlan {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) {
        throw $Message
    }
}
. (Join-Path $PSScriptRoot "r7-execution-plan-evidence.ps1")
. (Join-Path $PSScriptRoot "r7-execution-plan-projection.ps1")
function Assert-R7ExecutionPlanUniqueJsonProperties {
    param(
        [System.Text.Json.JsonElement]$Element,
        [string]$Path = "$"
    )
    if ($Element.ValueKind -eq [System.Text.Json.JsonValueKind]::Object) {
        $names = [Collections.Generic.HashSet[string]]::new(
            [StringComparer]::OrdinalIgnoreCase
        )
        foreach ($property in $Element.EnumerateObject()) {
            if (-not $names.Add([string]$property.Name)) {
                throw "Duplicate execution plan JSON property: $Path.$($property.Name)"
            }
            Assert-R7ExecutionPlanUniqueJsonProperties $property.Value "$Path.$($property.Name)"
        }
    } elseif ($Element.ValueKind -eq [System.Text.Json.JsonValueKind]::Array) {
        $index = 0
        foreach ($item in $Element.EnumerateArray()) {
            Assert-R7ExecutionPlanUniqueJsonProperties $item "$Path[$index]"
            $index++
        }
    }
}
function Get-R7ExecutionPlanManifest {
    param(
        [string]$DocumentPath,
        [string]$SchemaPath
    )
    $text = Get-Content -Raw -Encoding UTF8 -LiteralPath $DocumentPath
    $begin = "<!-- R71_EXECUTION_PLAN_MANIFEST_BEGIN -->"
    $end = "<!-- R71_EXECUTION_PLAN_MANIFEST_END -->"
    $beginIndex = $text.IndexOf($begin, [StringComparison]::Ordinal)
    $endIndex = $text.IndexOf($end, [StringComparison]::Ordinal)
    Assert-R7ExecutionPlan ($beginIndex -ge 0) "Execution plan manifest begin marker is missing"
    Assert-R7ExecutionPlan ($endIndex -gt $beginIndex) "Execution plan manifest end marker is missing"
    $block = $text.Substring($beginIndex + $begin.Length, $endIndex - $beginIndex - $begin.Length)
    $match = [regex]::Match($block, '(?s)^\s*```json\s*(\{.*\})\s*```\s*$')
    Assert-R7ExecutionPlan $match.Success "Execution plan manifest block must contain exactly one JSON object"
    $raw = $match.Groups[1].Value
    $document = [System.Text.Json.JsonDocument]::Parse($raw)
    try {
        Assert-R7ExecutionPlanUniqueJsonProperties $document.RootElement
        Assert-R7ExecutionPlan (
            $document.RootElement.ValueKind -eq [System.Text.Json.JsonValueKind]::Object
        ) "Execution plan manifest root must be an object"
    } finally {
        $document.Dispose()
    }
    Assert-R7ExecutionPlan (
        $raw | Test-Json -SchemaFile $SchemaPath -ErrorAction Stop
    ) "Execution plan manifest does not match its JSON Schema"
    [pscustomobject]@{
        text = $text
        raw = $raw
        manifest = $raw | ConvertFrom-Json -Depth 100 -NoEnumerate
    }
}
function Get-R7ExecutionPlanAncestors {
    param([string]$PhaseId, [hashtable]$ById)
    $ancestors = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $stack = [Collections.Generic.Stack[string]]::new()
    foreach ($dependency in @($ById[$PhaseId].depends_on)) {
        $stack.Push([string]$dependency)
    }
    while ($stack.Count -gt 0) {
        $candidate = $stack.Pop()
        if ($ancestors.Add($candidate)) {
            foreach ($dependency in @($ById[$candidate].depends_on)) {
                $stack.Push([string]$dependency)
            }
        }
    }
    @($ancestors)
}
function Test-R7ExecutionPlanReady {
    param($Phase, [hashtable]$ById)
    if ([string]$Phase.status -in @("closed", "gated")) {
        return $false
    }
    foreach ($dependency in @($Phase.depends_on)) {
        if ([string]$ById[[string]$dependency].status -ne "closed") {
            return $false
        }
    }
    $true
}
function Assert-R7ExecutionPlanSemantics {
    param($Manifest, [string]$RepoRoot)
    $phases = @($Manifest.phases)
    Assert-R7ExecutionPlan ($phases.Count -eq [int]$Manifest.phase_count) `
        "Execution plan phase_count does not match phases"
    $byId = @{}
    for ($index = 0; $index -lt $phases.Count; $index++) {
        $phase = $phases[$index]
        $expectedId = "R71-{0:D2}" -f ($index + 1)
        Assert-R7ExecutionPlan ([string]$phase.id -eq $expectedId) `
            "Execution plan phase IDs must be contiguous: expected $expectedId"
        Assert-R7ExecutionPlan (-not $byId.ContainsKey([string]$phase.id)) `
            "Duplicate execution plan phase ID: $($phase.id)"
        $byId[[string]$phase.id] = $phase
    }
    $specialIds = @(
        [string]$Manifest.dynamic_cost_phase_id,
        [string]$Manifest.candidate_freeze_phase_id,
        [string]$Manifest.formal_evaluation_phase_id,
        [string]$Manifest.promotion_decision_phase_id
    )
    $allSpecialIds = @([string]$Manifest.current_phase_id) + $specialIds
    foreach ($specialId in $allSpecialIds) {
        Assert-R7ExecutionPlan $byId.ContainsKey($specialId) `
            "Execution plan special phase is missing: $specialId"
    }
    Assert-R7ExecutionPlan (@($specialIds | Select-Object -Unique).Count -eq 4) `
        "Cost, freeze, formal evaluation, and promotion phases must be distinct"
    $costId, $freezeId, $formalId, $promotionId = $specialIds
    $costNumber = [int]$costId.Substring(4)
    Assert-R7ExecutionPlan ($costNumber -eq $phases.Count - 3) `
        "Dynamic cost must start the final four-phase gate chain"
    Assert-R7ExecutionPlan ([string]$byId[$costId].kind -eq "evaluation") `
        "Dynamic cost role must bind to an evaluation phase"
    Assert-R7ExecutionPlan ([string]$byId[$freezeId].kind -eq "release") `
        "Candidate freeze role must bind to a release phase"
    Assert-R7ExecutionPlan ([string]$byId[$formalId].kind -eq "evaluation") `
        "Formal evaluation role must bind to an evaluation phase"
    Assert-R7ExecutionPlan ([string]$byId[$promotionId].kind -eq "promotion") `
        "Promotion role must bind to a promotion phase"
    Assert-R7ExecutionPlan ((@($byId[$freezeId].depends_on) -join ",") -eq $costId) `
        "Candidate freeze must depend directly and only on dynamic cost"
    Assert-R7ExecutionPlan ((@($byId[$formalId].depends_on) -join ",") -eq $freezeId) `
        "Formal evaluation must depend directly and only on candidate freeze"
    Assert-R7ExecutionPlan ((@($byId[$promotionId].depends_on) -join ",") -eq $formalId) `
        "Promotion must depend directly and only on formal evaluation"
    Assert-R7ExecutionPlan ($promotionId -eq [string]$phases[-1].id) `
        "Promotion decision must be the final phase"
    $routeRoleContracts = @{
        nested_dispatch_boundary = @{
            kind = "implementation"; domain = "nested_capability.boundary"
        }
        multi_patch_runtime_safety = @{
            kind = "validation"; domain = "benchmark.multi_patch_runtime_boundary"
        }
        multi_patch_agent_behavior = @{
            kind = "diagnosis"; domain = "benchmark.multi_patch_agent_behavior"
        }
    }
    $routeRoleIds = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($role in $routeRoleContracts.Keys) {
        $phaseId = [string]$Manifest.route_role_phase_ids.$role
        Assert-R7ExecutionPlan $byId.ContainsKey($phaseId) `
            "Route role $role references unknown phase $phaseId"
        Assert-R7ExecutionPlan $routeRoleIds.Add($phaseId) `
            "Route roles must bind to distinct phases"
        $phase = $byId[$phaseId]
        Assert-R7ExecutionPlan (
            [string]$phase.kind -eq [string]$routeRoleContracts[$role].kind -and
            [string]$phase.change_domain_key -eq [string]$routeRoleContracts[$role].domain
        ) "Route role $role is bound to the wrong phase"
    }
    $nestedBoundaryId = [string]$Manifest.route_role_phase_ids.nested_dispatch_boundary
    $runtimeSafetyId = [string]$Manifest.route_role_phase_ids.multi_patch_runtime_safety
    $agentBehaviorId = [string]$Manifest.route_role_phase_ids.multi_patch_agent_behavior
    Assert-R7ExecutionPlan (
        $nestedBoundaryId -in @($byId[$runtimeSafetyId].depends_on)
    ) "Runtime safety route role must depend on the nested boundary role"
    Assert-R7ExecutionPlan (
        (@($byId[$agentBehaviorId].depends_on) -join ",") -eq $runtimeSafetyId
    ) "Agent behavior route role must depend directly and only on runtime safety"
    $expectedRoots = @(1..10 | ForEach-Object { "R71-GI-{0:D3}" -f $_ })
    $declaredRoots = @($Manifest.defect_roots | ForEach-Object { [string]$_ } | Sort-Object)
    Assert-R7ExecutionPlan (($declaredRoots -join ",") -eq ($expectedRoots -join ",")) `
        "Execution plan defect root set drifted"
    $coveredRoots = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $inProgressDomains = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $eventNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $evidenceTypes = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    Assert-R7ExecutionPlan $eventNames.Add([string]$Manifest.state_transition_audit.event_name) `
        "Duplicate state transition audit event name"
    foreach ($field in @(
            "phase_id", "previous_status", "new_status", "closure_outcome",
            "dependency_snapshot", "spawned_repairs", "evidence_sha256", "actor"
        )) {
        Assert-R7ExecutionPlan ($field -in @($Manifest.state_transition_audit.required_fields)) `
            "State transition audit is missing required field: $field"
    }
    $freezeNumber = [int]([string]$Manifest.candidate_freeze_phase_id).Substring(4)
    foreach ($phase in $phases) {
        $phaseId = [string]$phase.id
        $phaseNumber = [int]$phaseId.Substring(4)
        foreach ($dependency in @($phase.depends_on)) {
            $dependencyId = [string]$dependency
            Assert-R7ExecutionPlan $byId.ContainsKey($dependencyId) `
                "$phaseId depends on unknown phase $dependencyId"
            $dependencyNumber = [int]$dependencyId.Substring(4)
            Assert-R7ExecutionPlan ($dependencyNumber -lt $phaseNumber) `
                "$phaseId has a future or cyclic dependency on $dependencyId"
        }
        foreach ($rootId in @($phase.root_ids)) {
            Assert-R7ExecutionPlan ($rootId -in $declaredRoots) `
                "$phaseId references undeclared defect root $rootId"
            [void]$coveredRoots.Add([string]$rootId)
        }
        if ($phaseNumber -lt $freezeNumber) {
            Assert-R7ExecutionPlan (@($phase.root_ids).Count -gt 0) `
                "$phaseId has no defect root ownership"
        }
        Assert-R7ExecutionPlan (
            [string]$phase.closure_outcome -eq "pending" -or
            [string]$phase.closure_outcome -in @($phase.allowed_closure_outcomes)
        ) "$phaseId closure outcome is not allowed"
        Assert-R7ExecutionPlan $eventNames.Add([string]$phase.observability.event_name) `
            "Duplicate execution plan event name: $($phase.observability.event_name)"
        Assert-R7ExecutionPlan $evidenceTypes.Add([string]$phase.acceptance_evidence_type) `
            "Duplicate acceptance_evidence_type: $($phase.acceptance_evidence_type)"

        if ([string]$phase.status -eq "closed") {
            foreach ($dependency in @($phase.depends_on)) {
                Assert-R7ExecutionPlan ([string]$byId[[string]$dependency].status -eq "closed") `
                    "$phaseId is closed before dependency $dependency"
            }
            Assert-R7ExecutionPlanPhaseEvidence $phase $RepoRoot
        }
        if ([string]$phase.status -in @("decision_pending", "in_progress")) {
            foreach ($dependency in @($phase.depends_on)) {
                Assert-R7ExecutionPlan ([string]$byId[[string]$dependency].status -eq "closed") `
                    "$phaseId is active before dependency $dependency is closed"
            }
        }
        if ([string]$phase.status -eq "in_progress") {
            Assert-R7ExecutionPlan $inProgressDomains.Add([string]$phase.change_domain_key) `
                "Concurrent phases share change_domain_key $($phase.change_domain_key)"
        }
        if ([string]$phase.closure_outcome -eq "root_causes_identified") {
            Assert-R7ExecutionPlan ([string]$phase.kind -eq "diagnosis") `
                "$phaseId identifies roots but is not a diagnosis phase"
            Assert-R7ExecutionPlan (@($phase.spawned_repairs).Count -gt 0) `
                "$phaseId identified roots without spawned phases"
        } else {
            Assert-R7ExecutionPlan (@($phase.spawned_repairs).Count -eq 0) `
                "$phaseId declares spawned repairs without a root-causes closure"
        }
        if ("root_causes_identified" -in @($phase.allowed_closure_outcomes)) {
            Assert-R7ExecutionPlan ($null -ne $phase.failure_route) `
                "$phaseId can identify roots but has no machine failure route"
        }
    }
    $covered = @($coveredRoots | Sort-Object)
    Assert-R7ExecutionPlan (($covered -join ",") -eq ($declaredRoots -join ",")) `
        "Execution plan does not cover every declared defect root"

    $costAncestors = @(Get-R7ExecutionPlanAncestors $costId $byId)
    foreach ($phase in $phases | Where-Object {
            [int]([string]$_.id).Substring(4) -lt $costNumber
        }) {
        Assert-R7ExecutionPlan ([string]$phase.id -in $costAncestors) `
            "$($phase.id) precedes dynamic cost but is not its ancestor"
    }

    $promotionAncestors = @(Get-R7ExecutionPlanAncestors $promotionId $byId)
    foreach ($phase in $phases | Where-Object { [string]$_.id -ne $promotionId }) {
        Assert-R7ExecutionPlan ([string]$phase.id -in $promotionAncestors) `
            "$($phase.id) is not an ancestor of promotion"
    }

    $spawnedTargets = @{}
    foreach ($source in $phases) {
        $sourceId = [string]$source.id
        $sourceAncestors = @(Get-R7ExecutionPlanAncestors $sourceId $byId)
        $mappedRoots = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
        foreach ($repair in @($source.spawned_repairs)) {
            $rootId = [string]$repair.root_cause_id
            $targetId = [string]$repair.phase_id
            Assert-R7ExecutionPlan $mappedRoots.Add($rootId) `
                "$sourceId maps root cause $rootId more than once"
            Assert-R7ExecutionPlan $byId.ContainsKey($targetId) `
                "$sourceId references missing spawned phase $targetId"
            Assert-R7ExecutionPlan (-not $spawnedTargets.ContainsKey($targetId)) `
                "$targetId is spawned by multiple diagnosis phases"
            $target = $byId[$targetId]
            Assert-R7ExecutionPlan (
                $targetId -ne $sourceId -and $targetId -notin $sourceAncestors
            ) "$sourceId spawns itself or an ancestor: $targetId"
            Assert-R7ExecutionPlan ([string]$target.status -ne "closed") `
                "$sourceId spawns an already closed phase: $targetId"
            Assert-R7ExecutionPlan (
                [string]$target.kind -in @("decision", "repair", "implementation")
            ) "$sourceId spawns a non-remediation phase: $targetId"
            $targetNumber = [int]$targetId.Substring(4)
            Assert-R7ExecutionPlan (
                $targetNumber -gt [int]$sourceId.Substring(4) -and
                $targetNumber -lt $costNumber
            ) "$sourceId spawned phase must be later and precede dynamic cost: $targetId"
            Assert-R7ExecutionPlan ([string]$target.parent_diagnosis_id -eq $sourceId) `
                "$targetId does not map back to diagnosis $sourceId"
            Assert-R7ExecutionPlan ($rootId -in @($target.root_ids)) `
                "$targetId does not own mapped root cause $rootId"
            $spawnedTargets[$targetId] = $sourceId
        }
    }
    foreach ($phase in $phases | Where-Object { $null -ne $_.parent_diagnosis_id }) {
        Assert-R7ExecutionPlan $spawnedTargets.ContainsKey([string]$phase.id) `
            "$($phase.id) declares a parent diagnosis without a spawned repair mapping"
    }
    foreach ($phaseId in @($runtimeSafetyId, $agentBehaviorId)) {
        Assert-R7ExecutionPlan ($null -ne $byId[$phaseId].failure_route) `
            "$phaseId lacks a machine failure route"
    }

    $current = $byId[[string]$Manifest.current_phase_id]
    Assert-R7ExecutionPlan (Test-R7ExecutionPlanReady $current $byId) `
        "current_phase_id must identify an open and ready phase"

    $engineering = $Manifest.held_out_sets.engineering
    $promotion = $Manifest.held_out_sets.promotion
    Assert-R7ExecutionPlan (
        [string]$engineering.owner_phase_id -eq [string]$Manifest.dynamic_cost_phase_id
    ) "Engineering held-out owner must be the dynamic cost phase"
    Assert-R7ExecutionPlan (
        [string]$promotion.owner_phase_id -eq [string]$Manifest.formal_evaluation_phase_id
    ) "Promotion held-out owner must be the formal evaluation phase"
    Assert-R7ExecutionPlan (
        [string]$engineering.identity -ne [string]$promotion.identity
    ) "Engineering and promotion held-out identities must differ"
    Assert-R7ExecutionPlan (
        [string]$promotion.must_differ_from -eq [string]$engineering.identity
    ) "Promotion held-out separation reference drifted"
    $heldOutSamples = @{}
    foreach ($entry in @($engineering, $promotion)) {
        $owner = $byId[[string]$entry.owner_phase_id]
        if ([string]$owner.status -eq "closed") {
            Assert-R7ExecutionPlan ($null -ne $entry.sample_manifest) `
                "$($entry.identity) lacks a sealed sample manifest"
        }
        if ($null -ne $entry.sample_manifest) {
            $heldOutSamples[[string]$entry.identity] = @(
                Get-R7ExecutionPlanHeldOutSamples `
                    $entry.sample_manifest $RepoRoot "$($entry.identity) sample set"
            )
        }
    }
    if ($heldOutSamples.Count -eq 2) {
        $overlap = @(
            $heldOutSamples[[string]$engineering.identity] |
                Where-Object { $_ -in $heldOutSamples[[string]$promotion.identity] }
        )
        Assert-R7ExecutionPlan ($overlap.Count -eq 0) `
            "Engineering and promotion held-out sample sets overlap"
    }
}
