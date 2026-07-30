function Assert-R7ExecutionPlan {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) {
        throw $Message
    }
}
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
function Get-R7ExecutionPlanEvidence {
    param($Reference, [string]$RepoRoot, [string]$Label)
    $relativePath = [string]$Reference.path
    Assert-R7ExecutionPlan (-not [IO.Path]::IsPathRooted($relativePath)) `
        "$Label evidence path must be repository-relative"
    Assert-R7ExecutionPlan ($relativePath -notmatch '(^|[\\/])\.\.([\\/]|$)') `
        "$Label evidence path escapes the repository"
    Assert-R7ExecutionPlan ($relativePath -notmatch '^[a-zA-Z][a-zA-Z0-9+.-]*:') `
        "$Label evidence path must not be a URI"
    $root = [IO.Path]::GetFullPath($RepoRoot).TrimEnd(
        [IO.Path]::DirectorySeparatorChar,
        [IO.Path]::AltDirectorySeparatorChar
    )
    $fullPath = [IO.Path]::GetFullPath((Join-Path $root $relativePath))
    $prefix = $root + [IO.Path]::DirectorySeparatorChar
    Assert-R7ExecutionPlan (
        $fullPath.StartsWith($prefix, [StringComparison]::Ordinal)
    ) "$Label evidence path resolves outside the repository"
    Assert-R7ExecutionPlan (Test-Path -LiteralPath $fullPath -PathType Leaf) `
        "$Label evidence artifact is missing: $relativePath"
    $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $fullPath).Hash.ToLowerInvariant()
    Assert-R7ExecutionPlan ($actualHash -eq [string]$Reference.sha256) `
        "$Label evidence SHA-256 does not match: $relativePath"
    $artifact = Get-Content -Raw -Encoding UTF8 -LiteralPath $fullPath |
        ConvertFrom-Json -Depth 100 -NoEnumerate
    Assert-R7ExecutionPlan (
        [string]$artifact.schema_version -eq [string]$Reference.schema_version
    ) "$Label evidence schema_version does not match: $relativePath"
    Assert-R7ExecutionPlan (
        [string]$artifact.artifact_type -eq [string]$Reference.artifact_type
    ) "$Label evidence artifact_type does not match: $relativePath"
    $artifact
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
    $expectedRoots = @(1..10 | ForEach-Object { "R71-GI-{0:D3}" -f $_ })
    $declaredRoots = @($Manifest.defect_roots | ForEach-Object { [string]$_ } | Sort-Object)
    Assert-R7ExecutionPlan (($declaredRoots -join ",") -eq ($expectedRoots -join ",")) `
        "Execution plan defect root set drifted"
    $coveredRoots = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $inProgressDomains = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $eventNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
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
        if ($null -ne $phase.failure_route) {
            foreach ($forbiddenId in @($phase.failure_route.forbidden_target_ids)) {
                Assert-R7ExecutionPlan $byId.ContainsKey([string]$forbiddenId) `
                    "$phaseId failure route names unknown phase $forbiddenId"
            }
        }

        if ([string]$phase.status -eq "closed") {
            foreach ($dependency in @($phase.depends_on)) {
                Assert-R7ExecutionPlan ([string]$byId[[string]$dependency].status -eq "closed") `
                    "$phaseId is closed before dependency $dependency"
            }
            Assert-R7ExecutionPlan (
                [string]$phase.evidence_artifact.artifact_type -eq
                [string]$phase.acceptance_evidence_type
            ) "$phaseId evidence artifact_type does not match its acceptance contract"
            [void](Get-R7ExecutionPlanEvidence $phase.evidence_artifact $RepoRoot $phaseId)
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
    $phaseByEvidence = @{}
    foreach ($phase in $phases) {
        $phaseByEvidence[[string]$phase.acceptance_evidence_type] = [string]$phase.id
    }
    $routeContracts = @{
        patch_dispatch_safety_matrix = @(
            $phaseByEvidence.nested_boundary_pair,
            $phaseByEvidence.patch_attempt_behavior_matrix
        )
        patch_attempt_behavior_matrix = @(
            $phaseByEvidence.nested_boundary_pair,
            $phaseByEvidence.patch_dispatch_safety_matrix
        )
    }
    foreach ($evidenceType in $routeContracts.Keys) {
        $phase = $phases | Where-Object {
            [string]$_.acceptance_evidence_type -eq $evidenceType
        }
        Assert-R7ExecutionPlan ($null -ne $phase.failure_route) `
            "$evidenceType lacks a machine failure route"
        $actual = @($phase.failure_route.forbidden_target_ids | Sort-Object)
        $expected = @($routeContracts[$evidenceType] | Sort-Object)
        Assert-R7ExecutionPlan (($actual -join ",") -eq ($expected -join ",")) `
            "$evidenceType forbidden failure targets drifted"
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
            Assert-R7ExecutionPlan (
                [string]$entry.sample_manifest.artifact_type -eq "held_out_sample_manifest"
            ) "$($entry.identity) uses the wrong artifact_type"
            $artifact = Get-R7ExecutionPlanEvidence `
                $entry.sample_manifest $RepoRoot "$($entry.identity) sample set"
            Assert-R7ExecutionPlan (@($artifact.sample_ids).Count -gt 0) `
                "$($entry.identity) sample manifest has no sample_ids"
            $heldOutSamples[[string]$entry.identity] = @($artifact.sample_ids)
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

function Assert-R7ExecutionPlanDefinitionRoutes {
    param([string]$DocumentText, $Manifest)
    foreach ($phase in @($Manifest.phases | Where-Object { $null -ne $_.failure_route })) {
        $start = "### $($phase.id)："
        $startIndex = $DocumentText.IndexOf($start, [StringComparison]::Ordinal)
        Assert-R7ExecutionPlan ($startIndex -ge 0) "Missing definition for $($phase.id)"
        $exitIndex = $DocumentText.IndexOf("- 退出/分流：", $startIndex, [StringComparison]::Ordinal)
        $rollbackIndex = $DocumentText.IndexOf("- 回退：", $exitIndex, [StringComparison]::Ordinal)
        Assert-R7ExecutionPlan ($exitIndex -ge 0 -and $rollbackIndex -gt $exitIndex) `
            "Missing failure routing prose for $($phase.id)"
        $routingText = $DocumentText.Substring($exitIndex, $rollbackIndex - $exitIndex)
        foreach ($forbiddenId in @($phase.failure_route.forbidden_target_ids)) {
            Assert-R7ExecutionPlan (-not $routingText.Contains([string]$forbiddenId)) `
                "$($phase.id) prose routes a failure to forbidden target $forbiddenId"
        }
    }
}

function Compress-R7ExecutionPlanIds {
    param([object[]]$Ids)
    if (-not $Ids.Count) {
        return "无"
    }
    $numbers = @($Ids | ForEach-Object { [int]([string]$_).Substring(4) } | Sort-Object)
    $segments = [Collections.Generic.List[string]]::new()
    $start = $numbers[0]
    $previous = $start
    for ($index = 1; $index -le $numbers.Count; $index++) {
        $current = if ($index -lt $numbers.Count) { $numbers[$index] } else { $null }
        if ($null -ne $current -and $current -eq $previous + 1) {
            $previous = $current
            continue
        }
        if ($previous -gt $start) {
            $segments.Add(("R71-{0:D2}～{1:D2}" -f $start, $previous))
        } else {
            $segments.Add(("R71-{0:D2}" -f $start))
        }
        if ($null -ne $current) {
            $start = $current
            $previous = $current
        }
    }
    $segments -join "、"
}

function Get-R7ExecutionPlanProjectionTable {
    param($Manifest)
    $kindMap = @{
        repair = "返修"; implementation = "实现"; decision = "决策"
        diagnosis = "诊断"; validation = "复验"; audit = "审计"
        evaluation = "验收"; release = "发布准备"; promotion = "用户决策"
    }
    $statusMap = @{
        repair = "返修"; planned = "待实施"; decision_pending = "待决策"
        verification_pending = "待复验"; gated = "待准入"
        in_progress = "实施中"; closed = "已关闭"
    }
    $lines = [Collections.Generic.List[string]]::new()
    $lines.Add("| ID | 单一主题 | 类型 | 严重度 | 状态 | Ready | 直接依赖 | 根因 | 关闭结果 | 派生修复 | 独立退出证据 |")
    $lines.Add("|---|---|---|---|---|---|---|---|---|---|---|")
    $byId = @{}
    foreach ($phase in @($Manifest.phases)) {
        $byId[[string]$phase.id] = $phase
    }
    foreach ($phase in @($Manifest.phases)) {
        $severity = if ([string]$phase.severity -eq "blocking") { "阻断级" } else { "高" }
        $roots = if (@($phase.root_ids).Count) {
            @($phase.root_ids | ForEach-Object { ([string]$_).Replace("R71-", "") }) -join "、"
        } else {
            "无"
        }
        $evidence = ([string]$phase.acceptance_evidence_type).Replace("_", " ")
        $ready = if (Test-R7ExecutionPlanReady $phase $byId) { "是" } else { "否" }
        $outcome = if ([string]$phase.closure_outcome -eq "pending") {
            "待关闭"
        } else {
            [string]$phase.closure_outcome
        }
        $spawned = if (@($phase.spawned_repairs).Count) {
            @($phase.spawned_repairs | ForEach-Object {
                "$($_.root_cause_id)→$($_.phase_id)"
            }) -join "、"
        } else {
            "无"
        }
        $lines.Add(
            "| $($phase.id) | $($phase.title) | $($kindMap[[string]$phase.kind]) | " +
            "$severity | $($statusMap[[string]$phase.status]) | $ready | " +
            "$(Compress-R7ExecutionPlanIds @($phase.depends_on)) | $roots | " +
            "$outcome | $spawned | $evidence |"
        )
    }
    $lines -join "`n"
}

function Assert-R7ExecutionPlanProjection {
    param([string]$DocumentText, $Manifest)
    $begin = "<!-- R71_EXECUTION_PLAN_TABLE_BEGIN -->"
    $end = "<!-- R71_EXECUTION_PLAN_TABLE_END -->"
    $beginIndex = $DocumentText.IndexOf($begin, [StringComparison]::Ordinal)
    $endIndex = $DocumentText.IndexOf($end, [StringComparison]::Ordinal)
    Assert-R7ExecutionPlan ($beginIndex -ge 0 -and $endIndex -gt $beginIndex) `
        "Execution plan projection markers are missing"
    $actual = $DocumentText.Substring(
        $beginIndex + $begin.Length,
        $endIndex - $beginIndex - $begin.Length
    ).Trim()
    $expected = (Get-R7ExecutionPlanProjectionTable $Manifest).Trim()
    Assert-R7ExecutionPlan ($actual -eq $expected) `
        "Execution plan reader projection drifted from the embedded manifest"
}
