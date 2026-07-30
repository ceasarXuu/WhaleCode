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
    $lines.Add("| ID | 单一主题 | 类型 | 严重度 | 状态 | Ready | 直接依赖 | 根因 | 允许关闭结果 | 当前关闭结果 | 派生修复 | 独立退出证据 |")
    $lines.Add("|---|---|---|---|---|---|---|---|---|---|---|---|")
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
        $allowed = @($phase.allowed_closure_outcomes) -join "、"
        $lines.Add(
            "| $($phase.id) | $($phase.title) | $($kindMap[[string]$phase.kind]) | " +
            "$severity | $($statusMap[[string]$phase.status]) | $ready | " +
            "$(Compress-R7ExecutionPlanIds @($phase.depends_on)) | $roots | " +
            "$allowed | $outcome | $spawned | $evidence |"
        )
    }
    $lines -join "`n"
}

function Get-R7ExecutionPlanFailureRouteTable {
    param($Manifest)
    $lines = [Collections.Generic.List[string]]::new()
    $lines.Add("| Phase | 失败信号 | 机械动作 | existing_phase_reuse |")
    $lines.Add("|---|---|---|---|")
    foreach ($phase in @($Manifest.phases | Where-Object { $null -ne $_.failure_route })) {
        $lines.Add(
            "| $($phase.id) $($phase.title) | $($phase.failure_route.failure_signal) | " +
            "$($phase.failure_route.action) | $($phase.failure_route.existing_phase_reuse) |"
        )
    }
    $lines -join "`n"
}

function Assert-R7ExecutionPlanMarkerProjection {
    param(
        [string]$DocumentText,
        [string]$Begin,
        [string]$End,
        [string]$Expected,
        [string]$Label
    )
    $beginIndex = $DocumentText.IndexOf($Begin, [StringComparison]::Ordinal)
    $endIndex = $DocumentText.IndexOf($End, [StringComparison]::Ordinal)
    Assert-R7ExecutionPlan ($beginIndex -ge 0 -and $endIndex -gt $beginIndex) `
        "$Label projection markers are missing"
    $actual = $DocumentText.Substring(
        $beginIndex + $Begin.Length,
        $endIndex - $beginIndex - $Begin.Length
    ).Trim()
    Assert-R7ExecutionPlan ($actual -eq $Expected.Trim()) `
        "$Label projection drifted from the embedded manifest"
}

function Assert-R7ExecutionPlanProjection {
    param([string]$DocumentText, $Manifest)
    Assert-R7ExecutionPlanMarkerProjection $DocumentText `
        "<!-- R71_EXECUTION_PLAN_TABLE_BEGIN -->" `
        "<!-- R71_EXECUTION_PLAN_TABLE_END -->" `
        (Get-R7ExecutionPlanProjectionTable $Manifest) "Execution plan reader"
}

function ConvertTo-R7RouteLabelText {
    param([string]$Text)
    $normalized = $Text.Normalize([Text.NormalizationForm]::FormKC)
    $normalized = [regex]::Replace($normalized, '<[^>]+>', '')
    $normalized = [regex]::Replace($normalized, '\]\([^)]+\)', ']')
    $normalized = [regex]::Replace($normalized, '[\\*_~`\[\]\(\)]', '')
    $normalized = [regex]::Replace($normalized, '\s+', '')
    return [regex]::Replace($normalized, '[∕⁄]', '/')
}

function Assert-R7ExecutionPlanFailureRoutes {
    param([string]$DocumentText, $Manifest)
    Assert-R7ExecutionPlanMarkerProjection $DocumentText `
        "<!-- R71_FAILURE_ROUTE_TABLE_BEGIN -->" `
        "<!-- R71_FAILURE_ROUTE_TABLE_END -->" `
        (Get-R7ExecutionPlanFailureRouteTable $Manifest) "Failure route"
    $canonical = "- 退出/分流：关闭结果使用机器合同；失败路由只使用第 3.1 节机器投影，不复用既有 Phase。"
    $canonicalRollback = "- 回退：本节不维护失败目标；只使用第 3.1 节机器投影。"
    foreach ($phase in @($Manifest.phases | Where-Object { $null -ne $_.failure_route })) {
        $start = "### $($phase.id)："
        $startIndex = $DocumentText.IndexOf($start, [StringComparison]::Ordinal)
        Assert-R7ExecutionPlan ($startIndex -ge 0) "Missing definition for $($phase.id)"
        $nextIndex = $DocumentText.IndexOf("### R71-", $startIndex + $start.Length)
        if ($nextIndex -lt 0) {
            $nextIndex = $DocumentText.IndexOf("## 5.", $startIndex)
        }
        $section = $DocumentText.Substring($startIndex, $nextIndex - $startIndex)
        $routeLabelText = ConvertTo-R7RouteLabelText $section
        $exitLabels = @([regex]::Matches($routeLabelText, [regex]::Escape("退出/分流")))
        $rollbackLabels = @([regex]::Matches($routeLabelText, [regex]::Escape("回退")))
        $canonicalExitLines = @(
            [regex]::Matches($section, "(?m)^$([regex]::Escape($canonical))$")
        )
        $canonicalRollbackLines = @(
            [regex]::Matches($section, "(?m)^$([regex]::Escape($canonicalRollback))$")
        )
        Assert-R7ExecutionPlan (
            $exitLabels.Count -eq 1 -and $canonicalExitLines.Count -eq 1
        ) "$($phase.id) must contain exactly one canonical failure-route exit"
        Assert-R7ExecutionPlan (
            $rollbackLabels.Count -eq 1 -and $canonicalRollbackLines.Count -eq 1
        ) "$($phase.id) must contain exactly one canonical failure-route rollback"
    }
}
