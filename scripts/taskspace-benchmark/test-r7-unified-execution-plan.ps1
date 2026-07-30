$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path

function Assert-Plan {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) {
        throw $Message
    }
}

function Get-Section {
    param([string]$Text, [string]$Start, [string]$End)
    $startIndex = $Text.IndexOf($Start, [StringComparison]::Ordinal)
    Assert-Plan ($startIndex -ge 0) "Missing section: $Start"
    $endIndex = $Text.IndexOf($End, $startIndex + $Start.Length, [StringComparison]::Ordinal)
    Assert-Plan ($endIndex -gt $startIndex) "Missing section boundary: $End"
    $Text.Substring($startIndex, $endIndex - $startIndex)
}

$currentPath = Join-Path $repoRoot "docs/v0.0.5/build-R7/47-r7.1-global-issue-register.md"
$legacyPath = Join-Path $repoRoot "docs/v0.0.5/build-R7/47-r7.1-global-issue-register-legacy.md"
$milestonePath = Join-Path $repoRoot "docs/v0.0.5/build-R7/40-r7.1-milestone-baseline.md"
$w0Path = Join-Path $repoRoot "docs/v0.0.5/build-R7/48-r7.1-w0-factual-foundation-result.md"

foreach ($path in @($currentPath, $legacyPath, $milestonePath, $w0Path)) {
    Assert-Plan (Test-Path -LiteralPath $path -PathType Leaf) "Missing R7.1 plan document: $path"
}

$current = Get-Content -Raw -Encoding UTF8 -LiteralPath $currentPath
$legacy = Get-Content -Raw -Encoding UTF8 -LiteralPath $legacyPath
$milestone = Get-Content -Raw -Encoding UTF8 -LiteralPath $milestonePath
$w0 = Get-Content -Raw -Encoding UTF8 -LiteralPath $w0Path

Assert-Plan $current.StartsWith("# R7.1 原子执行清单") "Current R7.1 authority title drifted"
Assert-Plan ($current.Contains("- Confirmed defect roots: 10")) "Defect root count drifted"
Assert-Plan ($current.Contains("- Pending atomic units: 21")) "Atomic unit count drifted"
Assert-Plan ($current.Contains("一个 Phase 只能有一个根因域、一个主要生产改动域")) `
    "Atomic phase boundary rule is missing"

$unitSection = Get-Section $current "## 2. 原子 Phase 全集" "## 3. 原子 Phase 定义"
$actualIds = @(
    [regex]::Matches($unitSection, '(?m)^\| (R71-\d{2}) \|') |
        ForEach-Object { $_.Groups[1].Value }
)
$expectedIds = @(1..21 | ForEach-Object { "R71-{0:D2}" -f $_ })
Assert-Plan (($actualIds -join ",") -eq ($expectedIds -join ",")) `
    "Current atomic unit IDs or order drifted: $($actualIds -join ',')"
Assert-Plan (($actualIds | Select-Object -Unique).Count -eq 21) `
    "Current atomic unit IDs are not unique"

$requiredFields = @(
    "- 类型：",
    "- 入口：",
    "- 唯一改动域：",
    "- 不包含：",
    "- 产物：",
    "- 预期收益：",
    "- 独立验收：",
    "- 退出：",
    "- 回退："
)
foreach ($index in 1..21) {
    $id = "R71-{0:D2}" -f $index
    $nextBoundary = if ($index -lt 21) {
        "### R71-{0:D2}：" -f ($index + 1)
    } else {
        "## 4. 依赖与并行批次"
    }
    $phaseStart = "### ${id}："
    $phase = Get-Section $current $phaseStart $nextBoundary
    foreach ($field in $requiredFields) {
        Assert-Plan ($phase.Contains($field)) "$id is missing atomic contract field: $field"
    }
}

$activeExecution = Get-Section $current "## 2. 原子 Phase 全集" "## 6. 旧编号迁移"
Assert-Plan (-not [regex]::IsMatch($activeExecution, 'R71-\d{2}\.\d+')) `
    "Active execution plan retains a decimal subtask ID"
foreach ($legacyPattern in @("R71-GI-", "W0B", "W1 Feedback", "W2 动作", "W3 残余", "W4 成本", "A2-D")) {
    Assert-Plan (-not $activeExecution.Contains($legacyPattern)) `
        "Active execution plan retains legacy identifier: $legacyPattern"
}
Assert-Plan ($activeExecution.Contains("R71-07 -> R71-08")) `
    "Nested route decision and implementation are not explicitly separated"
Assert-Plan ($activeExecution.Contains("R71-12 -> [仍复现] R71-13 -> R71-14")) `
    "Action diagnosis, decision, and implementation are not explicitly separated"
Assert-Plan ($activeExecution.Contains("R71-18 -> R71-19 -> R71-20 -> R71-21")) `
    "Cost, freeze, formal run, and promotion order drifted"

Assert-Plan ($legacy.StartsWith("# R7.1 历史全局问题清单")) `
    "Legacy register is not frozen as history"
Assert-Plan ($legacy.Contains("不得继续在本文件更新")) `
    "Legacy register lacks update prohibition"
Assert-Plan ($legacy.Contains("- Status: Frozen legacy / superseded 2026-07-31")) `
    "Legacy register still presents itself as active"
Assert-Plan ($milestone.Contains('当前 Phase：`R71-01 direct failure carrier 证据合同`')) `
    "Milestone does not identify the current atomic phase"
Assert-Plan ($milestone.Contains('`R71-01` 至 `R71-21`')) `
    "Milestone does not reference the full atomic sequence"
Assert-Plan ($w0.Contains("- Current mapping: R71-01～R71-04、R71-09")) `
    "Historical W0 result lacks the current atomic mapping"
Assert-Plan ($w0.Contains("两类工作不得再次混在同一 Phase 实施")) `
    "Historical W0 result does not preserve the measurement/feedback boundary"

Write-Output "R7.1 atomic execution plan validation passed."
