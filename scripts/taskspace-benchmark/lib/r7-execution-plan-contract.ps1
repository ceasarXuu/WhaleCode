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

function Assert-R7ExecutionPlanSemantics {
    param($Manifest)
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

    foreach ($specialId in @(
            [string]$Manifest.current_phase_id,
            [string]$Manifest.dynamic_cost_phase_id,
            [string]$Manifest.candidate_freeze_phase_id,
            [string]$Manifest.formal_evaluation_phase_id,
            [string]$Manifest.promotion_decision_phase_id
        )) {
        Assert-R7ExecutionPlan $byId.ContainsKey($specialId) `
            "Execution plan special phase is missing: $specialId"
    }

    $expectedRoots = @(1..10 | ForEach-Object { "R71-GI-{0:D3}" -f $_ })
    $declaredRoots = @($Manifest.defect_roots | ForEach-Object { [string]$_ } | Sort-Object)
    Assert-R7ExecutionPlan (($declaredRoots -join ",") -eq ($expectedRoots -join ",")) `
        "Execution plan defect root set drifted"

    $coveredRoots = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $inProgressDomains = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
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

        if ([string]$phase.status -eq "closed") {
            foreach ($dependency in @($phase.depends_on)) {
                Assert-R7ExecutionPlan ([string]$byId[[string]$dependency].status -eq "closed") `
                    "$phaseId is closed before dependency $dependency"
            }
        }
        if ([string]$phase.status -eq "decision_pending") {
            foreach ($dependency in @($phase.depends_on)) {
                Assert-R7ExecutionPlan ([string]$byId[[string]$dependency].status -eq "closed") `
                    "$phaseId requests a decision before dependency $dependency is closed"
            }
        }
        if ([string]$phase.status -eq "in_progress") {
            Assert-R7ExecutionPlan $inProgressDomains.Add([string]$phase.change_domain_key) `
                "Concurrent phases share change_domain_key $($phase.change_domain_key)"
        }
        foreach ($spawnedId in @($phase.spawned_phase_ids)) {
            $spawned = [string]$spawnedId
            Assert-R7ExecutionPlan $byId.ContainsKey($spawned) `
                "$phaseId references missing spawned phase $spawned"
            Assert-R7ExecutionPlan ([bool]$byId[$spawned].blocks_dynamic_cost) `
                "$phaseId spawned phase $spawned does not block dynamic cost"
        }
        if ([string]$phase.closure_outcome -eq "root_causes_identified") {
            Assert-R7ExecutionPlan (@($phase.spawned_phase_ids).Count -gt 0) `
                "$phaseId identified roots without spawned phases"
        }
    }
    $covered = @($coveredRoots | Sort-Object)
    Assert-R7ExecutionPlan (($covered -join ",") -eq ($declaredRoots -join ",")) `
        "Execution plan does not cover every declared defect root"

    $costId = [string]$Manifest.dynamic_cost_phase_id
    $costAncestors = @(Get-R7ExecutionPlanAncestors $costId $byId)
    foreach ($phase in $phases | Where-Object { [bool]$_.blocks_dynamic_cost }) {
        Assert-R7ExecutionPlan ([string]$phase.id -in $costAncestors) `
            "$($phase.id) blocks dynamic cost but is not an ancestor of $costId"
    }

    $promotionId = [string]$Manifest.promotion_decision_phase_id
    $promotionAncestors = @(Get-R7ExecutionPlanAncestors $promotionId $byId)
    foreach ($phase in $phases | Where-Object { [bool]$_.blocks_promotion }) {
        Assert-R7ExecutionPlan ([string]$phase.id -in $promotionAncestors) `
            "$($phase.id) blocks promotion but is not an ancestor of $promotionId"
    }

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
    $lines.Add("| ID | 单一主题 | 类型 | 严重度 | 状态 | 直接依赖 | 根因 | 独立退出证据 |")
    $lines.Add("|---|---|---|---|---|---|---|---|")
    foreach ($phase in @($Manifest.phases)) {
        $severity = if ([string]$phase.severity -eq "blocking") { "阻断级" } else { "高" }
        $roots = if (@($phase.root_ids).Count) {
            @($phase.root_ids | ForEach-Object { ([string]$_).Replace("R71-", "") }) -join "、"
        } else {
            "无"
        }
        $evidence = ([string]$phase.acceptance_evidence_type).Replace("_", " ")
        $lines.Add(
            "| $($phase.id) | $($phase.title) | $($kindMap[[string]$phase.kind]) | " +
            "$severity | $($statusMap[[string]$phase.status]) | " +
            "$(Compress-R7ExecutionPlanIds @($phase.depends_on)) | $roots | $evidence |"
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
