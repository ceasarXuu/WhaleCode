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

Assert-Plan $current.StartsWith("# R7.1 统一执行清单") "Current R7.1 authority title drifted"
Assert-Plan ($current.Contains("- Open problems: 10")) "Current open problem count drifted"
Assert-Plan ($current.Contains("- Promotion gate: 1")) "Promotion gate count drifted"

$stageSection = Get-Section $current "## 2. 唯一阶段总览" "## 3. 当前问题全集"
foreach ($stage in 1..7) {
    $id = "{0:D2}" -f $stage
    Assert-Plan ($stageSection.Contains("| $id |")) "Missing current stage R71-$id"
}
Assert-Plan (-not $stageSection.Contains("| 08 |")) "Unexpected stage after R71-07"

$issueSection = Get-Section $current "## 3. 当前问题全集" "## 4. 任务定义与关闭标准"
$actualIds = @(
    [regex]::Matches($issueSection, '(?m)^\| (R71-\d{2}\.\d+) \|') |
        ForEach-Object { $_.Groups[1].Value }
)
$expectedIds = @(
    "R71-01.1", "R71-01.2",
    "R71-02.1", "R71-02.2",
    "R71-03.1", "R71-03.2", "R71-03.3",
    "R71-04.1", "R71-05.1", "R71-06.1"
)
Assert-Plan (($actualIds -join ",") -eq ($expectedIds -join ",")) `
    "Current issue IDs or order drifted: $($actualIds -join ',')"
Assert-Plan (($actualIds | Select-Object -Unique).Count -eq 10) "Current issue IDs are not unique"

$activeExecution = Get-Section $current "## 2. 唯一阶段总览" "## 8. 旧编号迁移"
foreach ($legacyPattern in @("R71-GI-", "W0B", "W1 Feedback", "W2 动作", "W3 残余", "W4 成本", "A2-D")) {
    Assert-Plan (-not $activeExecution.Contains($legacyPattern)) `
        "Active execution plan retains legacy identifier: $legacyPattern"
}
Assert-Plan ($activeExecution.Contains("### R71-07.1：repeat-10 与产品晋升")) `
    "Promotion gate is not part of the unified sequence"

Assert-Plan ($legacy.StartsWith("# R7.1 历史全局问题清单")) "Legacy register is not frozen as history"
Assert-Plan ($legacy.Contains("不得继续在本文件更新")) "Legacy register lacks update prohibition"
Assert-Plan ($legacy.Contains("- Status: Frozen legacy / superseded 2026-07-31")) `
    "Legacy register still presents itself as active"
Assert-Plan ($milestone.Contains('当前阶段：`R71-01 测量可信度`')) `
    "Milestone does not identify the current unified stage"
Assert-Plan (-not $milestone.Contains("8 个稳定 ID")) "Milestone retains stale issue count"
Assert-Plan ($w0.Contains("- Current mapping: R71-01.1、R71-03.1")) `
    "Historical W0 result lacks current task mapping"
Assert-Plan ($w0.Contains("两者不得再次混在同一阶段实施")) `
    "Historical W0 result does not preserve the measurement/feedback boundary"

Write-Output "R7.1 unified execution plan validation passed."
