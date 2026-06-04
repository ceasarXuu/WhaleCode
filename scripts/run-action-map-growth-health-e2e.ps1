param(
    [string]$ScenarioId = "action-map-growth-health-order-pipeline",
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 1200,
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "action-map-real-user-e2e-lib.ps1")
. (Join-Path $PSScriptRoot "action-map-graph-health-lib.ps1")

if (-not $RunRoot) {
    $RunRoot = Join-Path $PSScriptRoot "..\target\real-user-e2e"
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
$runDir = New-Dir (Join-Path $RunRoot "$ScenarioId\$stamp")
$repoDir = New-Dir (Join-Path $runDir "repo")
$artifactDir = New-Dir (Join-Path $runDir "artifacts")
$srcDir = New-Dir (Join-Path $repoDir "src\order_pipeline")
$testDir = New-Dir (Join-Path $repoDir "tests")

$promptPath = Join-Path $artifactDir "user-prompt.txt"
$jsonlPath = Join-Path $artifactDir "whale-exec.jsonl"
$stderrPath = Join-Path $artifactDir "whale-exec.stderr.log"
$lastMessagePath = Join-Path $artifactDir "last-message.md"
$validationStdoutPath = Join-Path $artifactDir "validation.stdout.log"
$validationStderrPath = Join-Path $artifactDir "validation.stderr.log"
$gitDiffPath = Join-Path $artifactDir "git-diff.patch"
$reportPath = Join-Path $artifactDir "report.md"
$scriptSha256 = (Get-FileHash -Algorithm SHA256 $PSCommandPath).Hash
$whaleSha256 = if (Test-Path $WhaleBin) { (Get-FileHash -Algorithm SHA256 $WhaleBin).Hash } else { "" }

@'
from .invoice import invoice_total
from .parser import parse_order_line
from .pricing import add_shipping, apply_discount
'@ | ForEach-Object { Write-Text (Join-Path $srcDir "__init__.py") $_ }

@'
def parse_order_line(line):
    sku, quantity, unit_price = line.split(",")
    return {
        "sku": sku,
        "quantity": int(quantity),
        "unit_price": float(unit_price),
    }
'@ | ForEach-Object { Write-Text (Join-Path $srcDir "parser.py") $_ }

@'
def apply_discount(subtotal, customer_tier):
    if customer_tier == "premium":
        return subtotal - 10
    if customer_tier == "vip":
        return subtotal * 0.85
    return subtotal

def add_shipping(total_after_discount):
    if total_after_discount >= 50:
        return total_after_discount
    return total_after_discount + 5
'@ | ForEach-Object { Write-Text (Join-Path $srcDir "pricing.py") $_ }

@'
from .parser import parse_order_line
from .pricing import add_shipping, apply_discount

def invoice_total(lines, customer_tier="standard"):
    items = [parse_order_line(line) for line in lines]
    subtotal = sum(item["quantity"] * item["unit_price"] for item in items)
    discounted = apply_discount(subtotal, customer_tier)
    return round(add_shipping(discounted), 2)
'@ | ForEach-Object { Write-Text (Join-Path $srcDir "invoice.py") $_ }

@'
from order_pipeline.parser import parse_order_line

def test_parse_order_line_normalizes_sku_and_numbers():
    item = parse_order_line(" SKU-1 , 2 , 19.50 ")
    assert item == {"sku": "sku-1", "quantity": 2, "unit_price": 19.50}

def test_parse_order_line_rejects_non_positive_quantity():
    try:
        parse_order_line("sku-1,0,19.50")
    except ValueError as exc:
        assert "quantity" in str(exc).lower()
    else:
        raise AssertionError("expected ValueError")
'@ | ForEach-Object { Write-Text (Join-Path $testDir "test_parser.py") $_ }

@'
from order_pipeline.pricing import add_shipping, apply_discount

def test_premium_discount_is_percent_and_case_insensitive():
    assert apply_discount(100, "Premium") == 90

def test_vip_discount_is_percent_and_case_insensitive():
    assert apply_discount(200, "VIP") == 170

def test_shipping_uses_discounted_total():
    assert add_shipping(49.99) == 54.99
    assert add_shipping(50) == 50
'@ | ForEach-Object { Write-Text (Join-Path $testDir "test_pricing.py") $_ }

@'
from order_pipeline.invoice import invoice_total

def test_invoice_total_combines_parser_discount_and_shipping():
    lines = [" SKU-1 , 2 , 20.00 ", "sku-2,1,10.00"]
    assert invoice_total(lines, "Premium") == 45.0

def test_invoice_total_vip_large_order_gets_free_shipping():
    lines = ["sku-1,3,25.00"]
    assert invoice_total(lines, "vip") == 63.75
'@ | ForEach-Object { Write-Text (Join-Path $testDir "test_invoice.py") $_ }

@'
# Order Pipeline E2E Sandbox

Requirements:
- SKU values must be trimmed and lowercased.
- Quantity must be a positive integer.
- Premium customers receive 10 percent off, case-insensitive.
- VIP customers receive 15 percent off, case-insensitive.
- Shipping is added only when the discounted total is below 50.
'@ | ForEach-Object { Write-Text (Join-Path $repoDir "README.md") $_ }

@'
[tool.pytest.ini_options]
pythonpath = ["src"]
'@ | ForEach-Object { Write-Text (Join-Path $repoDir "pyproject.toml") $_ }

Push-Location $repoDir
try {
    git init | Out-Null
    git config user.email "map-growth-e2e@example.local" | Out-Null
    git config user.name "Map Growth E2E" | Out-Null
    git add . | Out-Null
    git commit -m "baseline order pipeline regressions" | Out-Null
} finally {
    Pop-Location
}

$prompt = @"
I inherited this small order-pipeline project and need you to treat it like a real handoff. Parser behavior, pricing discounts, shipping, and invoice totals all look connected, and some tests may be wrong relative to the README.

Please inspect the README, tests, and implementation before editing. Separate product truth from broken expectations, organize the work in whatever way best fits the project, then integrate the findings yourself and make the final changes.

Run the current tests before editing so we know the baseline failure, then run the relevant tests again after the fix. If a test conflicts with the README, update the test to match the product rule instead of changing code to satisfy the wrong expectation.

Acceptance:
- python -m pytest tests -q should pass.
- The implementation must satisfy README requirements.
- Briefly explain how you organized the work and how the project state changed while you worked.
"@
Write-Text $promptPath $prompt
$forbiddenPromptTerms = "(?i)taskspace|action map|\bmap\b|\bnode\b|subagent|spawn_agent|taskspace_control|\bparallel(ize)?\b|\bconcurrent(ly)?\b|\bsimultaneous(ly)?\b|\bdelegate\b|\bdelegation\b|\bmultiple agents?\b|\bmulti-agent\b|\bsplit .* agents?\b|\bfan[- ]?out\b"
$promptLeaksInternalConcepts = $prompt -match $forbiddenPromptTerms

if ($PlanOnly) {
    Write-Host "RunDir: $runDir"
    Write-Host "RepoDir: $repoDir"
    Write-Host "WhaleBin: $WhaleBin"
    Write-Host "Model: $Model"
    Write-Host "PromptPath: $promptPath"
    Write-Host "PromptLeaksInternalConcepts: $promptLeaksInternalConcepts"
    Write-Host "ReportPath: $reportPath"
    exit 0
}

if (-not (Test-Path $WhaleBin)) {
    throw "Whale binary not found: $WhaleBin"
}
$whaleVersion = (& $WhaleBin --version 2>&1) -join " "
$helpText = & $WhaleBin exec --help 2>&1
if (($helpText -join [Environment]::NewLine) -notmatch "--taskspace") {
    throw "Installed whale exec does not expose --taskspace."
}

$started = Get-Date
$execArgs = @(
    "exec",
    "--json",
    "--taskspace",
    "-m", $Model,
    "-C", $repoDir,
    "--dangerously-bypass-approvals-and-sandbox",
    "--output-last-message", $lastMessagePath,
    "-"
)
$execExitCode = Invoke-RealProcess $WhaleBin $execArgs $repoDir $jsonlPath $stderrPath $TimeoutSeconds $promptPath
$finished = Get-Date
$validationExitCode = Invoke-RealProcess "python" @("-m", "pytest", "tests", "-q") $repoDir $validationStdoutPath $validationStderrPath 120
$oracleStdoutPath = Join-Path $artifactDir "hidden-oracle.stdout.log"
$oracleStderrPath = Join-Path $artifactDir "hidden-oracle.stderr.log"
$oracleCode = @"
import sys
sys.path.insert(0, r'$((Join-Path $repoDir "src").Replace("'", "''"))')
from order_pipeline.parser import parse_order_line
from order_pipeline.pricing import apply_discount, add_shipping
from order_pipeline.invoice import invoice_total
assert parse_order_line(' SKU-1 , 2 , 19.50 ') == {'sku': 'sku-1', 'quantity': 2, 'unit_price': 19.5}
try:
    parse_order_line('sku-1,0,19.50')
except ValueError as exc:
    assert 'quantity' in str(exc).lower()
else:
    raise AssertionError('expected ValueError for non-positive quantity')
assert apply_discount(100, 'Premium') == 90
assert apply_discount(200, 'VIP') == 170
assert add_shipping(49.99) == 54.99
assert add_shipping(50) == 50
assert invoice_total([' SKU-1 , 2 , 20.00 ', 'sku-2,1,10.00'], 'Premium') == 50.0
assert invoice_total(['sku-1,3,25.00'], 'vip') == 63.75
print('hidden oracle passed')
"@
$oracleExitCode = Invoke-RealProcess "python" @("-c", $oracleCode) $repoDir $oracleStdoutPath $oracleStderrPath 120

$jsonlText = if (Test-Path $jsonlPath) { Get-Content -Raw -Encoding UTF8 $jsonlPath } else { "" }
$stderrText = if (Test-Path $stderrPath) { Get-Content -Raw -Encoding UTF8 $stderrPath } else { "" }
$validationStdout = if (Test-Path $validationStdoutPath) { Get-Content -Raw -Encoding UTF8 $validationStdoutPath } else { "" }
$lastMessage = if (Test-Path $lastMessagePath) { Get-Content -Raw -Encoding UTF8 $lastMessagePath } else { "" }
$threadId = Get-ThreadId $jsonlText
$rollout = Find-LatestRollout $started $threadId
$rolloutCopy = ""
$rolloutText = ""
if ($rollout) {
    $rolloutCopy = Join-Path $artifactDir "rollout.jsonl"
    Copy-Item -LiteralPath $rollout.FullName -Destination $rolloutCopy -Force
    $rolloutText = Get-Content -Raw -Encoding UTF8 $rolloutCopy
}

Push-Location $repoDir
try {
    git diff -- . | Set-Content -Encoding UTF8 $gitDiffPath
} finally {
    Pop-Location
}
$gitDiffText = if (Test-Path $gitDiffPath) { Get-Content -Raw -Encoding UTF8 $gitDiffPath } else { "" }

$obsJsonPath = Join-Path $artifactDir "action-map-observability.json"
$obsMdPath = Join-Path $artifactDir "action-map-observability.md"
$obsHtmlPath = Join-Path $artifactDir "action-map-observability.html"
$obsExitCode = 0
if ($rollout) {
    $exportScript = Join-Path $PSScriptRoot "export-action-map-observability.ps1"
    & $exportScript -RolloutPath $rolloutCopy -JsonlPath $jsonlPath -OutputDir $artifactDir -ArtifactRoot $repoDir | Out-Host
    $obsExitCode = $LASTEXITCODE
}
$obs = if (Test-Path $obsJsonPath) { Get-Content -Raw -Encoding UTF8 $obsJsonPath | ConvertFrom-Json } else { $null }
$graphHealth = Get-TaskspaceGraphHealth $obs
$toolCallArgs = @{}
foreach ($line in ($rolloutText -split "`r?`n")) {
    try { $evt = $line | ConvertFrom-Json } catch { continue }
    if ($evt.type -eq "response_item" -and $evt.payload.type -eq "function_call" -and $evt.payload.call_id) {
        $toolCallArgs[[string]$evt.payload.call_id] = [string]$evt.payload.arguments
    }
}

$mapCount = if ($obs) { @($obs.maps).Count } else { 0 }
$nodeCount = if ($obs) { @($obs.nodes).Count } else { 0 }
$nodeMetrics = if ($obs) { @($obs.nodes) } else { @() }
$agentCount = if ($obs) { @($obs.agents).Count } else { 0 }
$nodesWithResults = if ($obs) { @($obs.nodes | Where-Object { @($_.results).Count -gt 0 }).Count } else { 0 }
$completedNodes = if ($obs) { @($obs.nodes | Where-Object { $_.status -eq "completed" }).Count } else { 0 }
$titleText = if ($obs) { (@($obs.nodes | ForEach-Object { $_.title }) -join "`n").ToLowerInvariant() } else { "" }
$kindText = if ($obs) { (@($obs.nodes | ForEach-Object { $_.kind }) -join "`n").ToLowerInvariant() } else { "" }
$hasBoundaryNode = $titleText -match "boundary|scope|repo|inspection|inspect"
$hasParserNode = $titleText -match "parser|parse|order line|sku"
$hasPricingNode = $titleText -match "pricing|discount|invoice|shipping"
$hasImplementationNode = $titleText -match "implement|fix|change|implementation"
$hasValidationNode = $titleText -match "validat|regression|test|verify"
$hasInspectKind = $kindText -match "inspect_code_context"
$hasImplementationKind = $kindText -match "implement_solution"
$hasTestKind = $kindText -match "smoke_test|regression_test"
$unknownActionResultCount = if ($obs) { @($obs.nodes | ForEach-Object { @($_.results | Where-Object { $_.kind -eq "main_tool_call" -and $_.actionClass -eq "unknown" }) }).Count } else { 0 }
$parserInvestigationUsed = $false; $pricingInvestigationUsed = $false; $validationNodeHasPytestResult = $false; $implementationNodeHasSuccessfulEdit = $false; $editOutsideImplementationCount = 0; $subagentResultCount = 0
if ($obs) { foreach ($node in @($obs.nodes)) {
    $agentIds = @($node.agentThreads)
    $nodeSubagentResultCount = @($node.results | Where-Object { $agentIds -contains $_.sourceThreadId }).Count
    $subagentResultCount += $nodeSubagentResultCount
    if ($node.title -match "(?i)parser|parse|sku" -and $nodeSubagentResultCount -gt 0) { $parserInvestigationUsed = $true }
    if ($node.title -match "(?i)pricing|discount|invoice|shipping" -and $nodeSubagentResultCount -gt 0) { $pricingInvestigationUsed = $true }
    foreach ($result in @($node.results | Where-Object { $_.kind -eq "main_tool_call" -and $_.actionClass -eq "edit" })) {
        if ((Get-ObjectPropertyNames $result) -contains "success" -and $result.success -ne $true) { continue }
        if ([string]$node.kind -eq "implement_solution") { $implementationNodeHasSuccessfulEdit = $true } else { $editOutsideImplementationCount++ }
    }
    if ($node.title -match "(?i)validat|regression|test|verify") {
        foreach ($result in @($node.results | Where-Object { $_.kind -eq "main_tool_call" -and $_.actionClass -eq "test" })) {
            if ((Get-ObjectPropertyNames $result) -contains "success" -and $result.success -ne $true) {
                continue
            }
            $callId = [string]$result.callId
            $command = if ($callId -and $toolCallArgs.ContainsKey($callId)) { [string]$toolCallArgs[$callId] } else { "" }
            $combined = "$([string]$result.body)`n$command`n$([string]$result.preview)"
            if ($combined -match "python -m pytest tests -q" -and $combined -match "(?i)\bpassed\b") {
                $validationNodeHasPytestResult = $true
            }
        }
    }
} }
$blockedToolActionCount = if ($obs) {
    @($obs.nodes | ForEach-Object { @($_.blockedActions) }).Count
} else { 0 }
$unexpectedBlockedToolActionCount = Count-UnexpectedBlockedTaskspaceToolActions $obs
$failedTaskspaceToolResults = Count-FailedTaskspaceToolResults $obs
$unexpectedFailedTaskspaceToolResults = Count-UnexpectedFailedTaskspaceToolResults $obs
$failedCollabToolCalls = Count-FailedCollabToolCalls $obs
$unexpectedFailedCollabToolCalls = Count-UnexpectedFailedCollabToolCalls $obs
$problematicSuccessfulToolResults = Count-ProblematicSuccessfulToolResults $obs
$implementationOwnershipGap = Get-ImplementationOwnershipGap $obs $gitDiffText $toolCallArgs
$pytestOwnership = Get-PytestOwnership $obs $toolCallArgs
$validationNodeHasPytestResult = $pytestOwnership.Owned
$editResultsAfterFinalPytest = Count-EditResultsAfter $nodeMetrics $pytestOwnership.At
$unexpectedTaskspaceGateFailures = if ($obs) {
    @($obs.nodes | ForEach-Object {
            @($_.results | Where-Object {
                    $_.kind -eq "main_tool_call" -and
                    (Get-ObjectPropertyNames $_) -contains "success" -and
                    $_.success -eq $false -and
                    ([string]$_.body -match "TaskSpace (blocked|mode is active)|Call taskspace_control")
                })
        }).Count
} else { 0 }
$postHocEmptyTerminalNodes = if ($obs) { @($obs.nodes | Where-Object { $_.title -match "(?i)validat|final|synthesis" -and @($_.results).Count -eq 0 }).Count } else { 0 }
$leaseCreatedCount = Count-Matches $rolloutText '"lease_created"|LeaseCreated'
$leaseAttachedCount = Count-Matches $rolloutText '"lease_attached"|LeaseAttached'
$leaseReleasedCount = Count-Matches $rolloutText '"lease_released"|LeaseReleased'
$finishNodeCallCount = Count-Matches $rolloutText 'TaskSpace node finished:'
$finishNodeUnsupportedCount = Count-Matches $rolloutText 'unknown variant `finish_node`|unknown variant finish_node'
$spawnAgentCount = if ($obs) { @($obs.toolCalls | Where-Object { $_.tool -eq "spawn_agent" -and $_.status -eq "completed" }).Count } else { 0 }
$taskspaceControlCount = Count-Matches $rolloutText '"name":"taskspace_control"|"name"\s*:\s*"taskspace_control"'
$commandExecutionCount = Count-Matches $jsonlText '"type"\s*:\s*"command_execution"'
$taskRebornShellMisuseCount = Count-Matches ($stderrText + $jsonlText) "task-reborn.*not recognized|The term '/task-reborn'|The term 'task-reborn'"
$repoHead = ""; $repoStatus = ""
Push-Location $repoDir
try {
    $repoHead = (git rev-parse HEAD) -join "`n"
    $repoStatus = (git status --short) -join " "
} finally {
    Pop-Location
}

$firstCompletedAt = $null
$laterCreatedAfterCompletion = $false
if ($obs) {
    foreach ($event in @($obs.timeline)) {
        if ($event.kind -eq "node_status_changed" -and $event.details.currentStatus -eq "completed" -and $event.at) {
            $firstCompletedAt = [datetime]::Parse([string]$event.at)
            break
        }
    }
    if ($firstCompletedAt) {
        foreach ($event in @($obs.timeline)) {
            if ($event.kind -eq "node_status_changed" -and
                $event.details.previousStatus -eq "pending" -and
                $event.details.currentStatus -eq "ready" -and
                $event.at -and [datetime]::Parse([string]$event.at) -gt $firstCompletedAt) {
                $laterCreatedAfterCompletion = $true
                break
            }
        }
    }
}

$failures = New-Object System.Collections.Generic.List[string]
if ($promptLeaksInternalConcepts) { $failures.Add("natural growth-health prompt leaked TaskSpace/map/node/subagent concepts") }
if ($execExitCode -ne 0) { $failures.Add("whale exec exit code was $execExitCode") }
if ($validationExitCode -ne 0) { $failures.Add("post-run pytest exit code was $validationExitCode") }
if ($oracleExitCode -ne 0) { $failures.Add("hidden oracle exit code was $oracleExitCode") }
if ($validationStdout -notmatch "passed") { $failures.Add("pytest output did not contain a passing marker") }
if ([string]::IsNullOrWhiteSpace($gitDiffText)) { $failures.Add("repository diff is empty after the real agent run") }
if (-not $rollout) { $failures.Add("could not find the rollout for this thread") }
if ($rollout -and $obsExitCode -ne 0) { $failures.Add("observability export failed with exit code $obsExitCode") }
if ($mapCount -lt 1) { $failures.Add("no map was observed") }
if ($nodeCount -lt 4) { $failures.Add("map did not grow to at least 4 nodes; observed $nodeCount") }
if ($graphHealth.EdgeCount -lt 2) { $failures.Add("map did not create enough dependency edges; observed $($graphHealth.EdgeCount)") }
if ($graphHealth.OrderedEdgeCount -ne $graphHealth.EdgeCount) { $failures.Add("not every dependency edge had observable predecessor-complete and successor-work timestamps") }
if ($graphHealth.EdgeOrderViolationCount -gt 0) { $failures.Add("map dependency execution order was violated on $($graphHealth.EdgeOrderViolationCount) edge(s)") }
if ($graphHealth.ParallelInspectTrackCount -lt 2) { $failures.Add("expected at least 2 parallel inspect tracks with subagent ownership; observed $($graphHealth.ParallelInspectTrackCount)") }
if (-not $graphHealth.ParallelInspectTracksIndependent) { $failures.Add("parallel inspect tracks were not represented as independent graph tracks") }
if (-not $graphHealth.ImplementationDependsOnParallelInspectTracks) { $failures.Add("implementation node did not depend on all subagent-owned inspect tracks") }
if (-not $graphHealth.DirectImplementationDependsOnParallelInspectTracks) { $failures.Add("implementation node did not directly depend on all subagent-owned inspect tracks") }
if (-not $graphHealth.TestDependsOnImplementation) { $failures.Add("test/validation node did not depend on implementation node") }
if (-not $graphHealth.DirectTestDependsOnImplementation) { $failures.Add("test/validation node did not directly depend on implementation node") }
if ($graphHealth.OpenFinalSynthesisCount -gt 0) { $failures.Add("final synthesis node was left open: $($graphHealth.OpenFinalSynthesisCount)") }
if ($graphHealth.OpenLeafNodeCount -gt 0) { $failures.Add("open leaf nodes remained at the end of the run: $($graphHealth.OpenLeafNodeCount)") }
if ($agentCount -lt 2) { $failures.Add("expected at least 2 subagent leases; observed $agentCount agents") }
if ($spawnAgentCount -lt 2) { $failures.Add("expected at least 2 spawn_agent calls; observed $spawnAgentCount") }
if ($leaseCreatedCount -lt 2 -or $leaseAttachedCount -lt 2) { $failures.Add("expected at least 2 lease create/attach events") }
if ($leaseReleasedCount -lt 2) { $failures.Add("expected at least 2 lease releases") }
if ($subagentResultCount -lt 2) { $failures.Add("expected at least 2 subagent results written to nodes; observed $subagentResultCount") }
if ($nodesWithResults -lt 3) { $failures.Add("expected results on at least 3 nodes; observed $nodesWithResults") }
if ($completedNodes -lt 2) { $failures.Add("expected at least 2 completed nodes; observed $completedNodes") }
if ($finishNodeCallCount -lt 2) { $failures.Add("expected at least 2 finish_node calls; observed $finishNodeCallCount") }
if ($finishNodeUnsupportedCount -gt 0) { $failures.Add("runtime rejected finish_node as unsupported") }
if (-not $laterCreatedAfterCompletion) { $failures.Add("no follow-up node was created after an earlier node completed") }
if (-not ($hasInspectKind -and $hasImplementationKind -and $hasTestKind)) { $failures.Add("node kinds did not cover inspect/implementation/test structure") }
if (-not ($hasParserNode -and $hasPricingNode -and $hasImplementationNode -and $hasValidationNode)) {
    $failures.Add("node titles did not cover parser/pricing/implementation/validation scenario categories")
}
if (-not $implementationNodeHasSuccessfulEdit) { $failures.Add("implementation node did not own a successful edit action") }
if ($editOutsideImplementationCount -gt 0) { $failures.Add("observed $editOutsideImplementationCount successful edit action(s) outside implementation nodes") }
if (-not $validationNodeHasPytestResult) { $failures.Add("validation node did not own a passing pytest command result") }
if ($unexpectedBlockedToolActionCount -gt 0) { $failures.Add("unexpected blocked TaskSpace tool actions: $unexpectedBlockedToolActionCount") }
if ($unexpectedFailedTaskspaceToolResults -gt 0) { $failures.Add("unexpected failed taskspace-owned tool results: $unexpectedFailedTaskspaceToolResults") }
if ($unexpectedFailedCollabToolCalls -gt 0) { $failures.Add("unexpected failed collaboration tool calls: $unexpectedFailedCollabToolCalls") }
if ($implementationOwnershipGap.MissingCount -gt 0) { $failures.Add("changed paths not owned by successful implementation-node edits: $($implementationOwnershipGap.MissingPaths -join ', ')") }
if ($editResultsAfterFinalPytest -gt 0) { $failures.Add("edit action results occurred after the final owned pytest validation: $editResultsAfterFinalPytest") }
if ($unexpectedTaskspaceGateFailures -gt 0) { $failures.Add("unexpected TaskSpace gate/tool attribution failures: $unexpectedTaskspaceGateFailures") }
if ($unknownActionResultCount -gt 0) { $failures.Add("observed unknown taskspace action results: $unknownActionResultCount") }
if ($postHocEmptyTerminalNodes -gt 0) { $failures.Add("terminal validation/final nodes were created without results") }
if ($commandExecutionCount -lt 4) { $failures.Add("agent did not run enough real commands; observed $commandExecutionCount") }
if ($taskRebornShellMisuseCount -gt 0) { $failures.Add("agent attempted to run taskspace slash command as shell") }
$overall = if ($failures.Count -eq 0) { "PASS" } else { "FAIL" }

$report = New-Object System.Collections.Generic.List[string]
$report.Add("# Action Map Growth Health E2E Report")
$report.Add("")
foreach ($row in @(
    @("overall", $overall), @("scenario_id", $ScenarioId), @("run_dir", $runDir), @("repo_dir", $repoDir),
    @("whale_bin", $WhaleBin), @("model", $Model), @("started", $started.ToString("o")),
    @("finished", $finished.ToString("o")), @("thread_id", $threadId), @("exec_exit_code", $execExitCode),
    @("validation_exit_code", $validationExitCode), @("script_sha256", $scriptSha256),
    @("whale_sha256", $whaleSha256), @("whale_version", $whaleVersion), @("repo_head", $repoHead), @("repo_status", $repoStatus),
    @("rollout", $(if ($rollout) { $rollout.FullName } else { "" }))
)) { Add-ReportLine $report $row[0] $row[1] }
$report.Add("")
$report.Add("## Artifacts")
$report.Add("")
foreach ($row in @(@("jsonl", $jsonlPath), @("stderr", $stderrPath), @("last_message", $lastMessagePath), @("git_diff", $gitDiffPath), @("observability_json", $obsJsonPath), @("observability_md", $obsMdPath), @("observability_html", $obsHtmlPath))) { Add-ReportLine $report $row[0] $row[1] }
$report.Add("")
$report.Add("## Growth Health Metrics")
$report.Add("")
foreach ($row in @(
    @("prompt_leaks_internal_concepts", $promptLeaksInternalConcepts),
    @("maps", $mapCount), @("nodes", $nodeCount), @("agents", $agentCount),
    @("edges", $graphHealth.EdgeCount), @("ordered_edges", $graphHealth.OrderedEdgeCount),
    @("edge_order_violations", $graphHealth.EdgeOrderViolationCount),
    @("anchored_implementation_nodes", $graphHealth.AnchoredImplementationCount),
    @("parser_pricing_independent", $graphHealth.ParserPricingIndependent),
    @("implementation_depends_on_parser_and_pricing", $graphHealth.ImplementationDependsOnParserAndPricing),
    @("parallel_inspect_tracks", $graphHealth.ParallelInspectTrackCount),
    @("parallel_inspect_tracks_independent", $graphHealth.ParallelInspectTracksIndependent),
    @("implementation_depends_on_parallel_inspect_tracks", $graphHealth.ImplementationDependsOnParallelInspectTracks),
    @("direct_implementation_depends_on_parallel_inspect_tracks", $graphHealth.DirectImplementationDependsOnParallelInspectTracks),
    @("test_depends_on_implementation", $graphHealth.TestDependsOnImplementation),
    @("direct_test_depends_on_implementation", $graphHealth.DirectTestDependsOnImplementation),
    @("open_leaf_nodes", $graphHealth.OpenLeafNodeCount),
    @("open_final_synthesis_nodes", $graphHealth.OpenFinalSynthesisCount),
    @("completed_nodes", $completedNodes), @("nodes_with_results", $nodesWithResults),
    @("spawn_agent", $spawnAgentCount), @("taskspace_control", $taskspaceControlCount),
    @("finish_node_calls", $finishNodeCallCount), @("lease_created", $leaseCreatedCount),
    @("lease_attached", $leaseAttachedCount), @("lease_released", $leaseReleasedCount),
    @("subagent_results", $subagentResultCount),
    @("later_node_created_after_completion", $laterCreatedAfterCompletion),
    @("has_inspect_kind", $hasInspectKind), @("has_implementation_kind", $hasImplementationKind),
    @("has_test_kind", $hasTestKind),
    @("has_boundary_node", $hasBoundaryNode), @("has_parser_node", $hasParserNode),
    @("has_pricing_node", $hasPricingNode), @("has_implementation_node", $hasImplementationNode),
    @("has_validation_node", $hasValidationNode), @("parser_investigation_used", $parserInvestigationUsed),
    @("pricing_investigation_used", $pricingInvestigationUsed), @("implementation_node_has_successful_edit", $implementationNodeHasSuccessfulEdit),
    @("edit_outside_implementation", $editOutsideImplementationCount), @("blocked_tool_actions", $blockedToolActionCount),
    @("unexpected_blocked_tool_actions", $unexpectedBlockedToolActionCount),
    @("validation_node_has_pytest_result", $validationNodeHasPytestResult),
    @("failed_taskspace_tool_results", $failedTaskspaceToolResults),
    @("unexpected_failed_taskspace_tool_results", $unexpectedFailedTaskspaceToolResults),
    @("failed_collab_tool_calls", $failedCollabToolCalls), @("unexpected_failed_collab_tool_calls", $unexpectedFailedCollabToolCalls),
    @("problematic_successful_tool_results", $problematicSuccessfulToolResults),
    @("changed_paths_without_implementation_owner", $implementationOwnershipGap.MissingCount),
    @("edit_results_after_final_pytest", $editResultsAfterFinalPytest),
    @("unexpected_taskspace_gate_failures", $unexpectedTaskspaceGateFailures),
    @("unknown_action_results", $unknownActionResultCount),
    @("posthoc_empty_terminal_nodes", $postHocEmptyTerminalNodes), @("finish_node_unsupported", $finishNodeUnsupportedCount),
    @("hidden_oracle_exit_code", $oracleExitCode), @("real_command_execution", $commandExecutionCount),
    @("git_diff_bytes", $gitDiffText.Length)
)) { Add-ReportLine $report $row[0] $row[1] }
$report.Add("")
$report.Add("## Failures")
$report.Add("")
if ($failures.Count -eq 0) { $report.Add("None.") } else { foreach ($failure in $failures) { $report.Add("- $failure") } }
$report | Set-Content -Encoding UTF8 $reportPath

Write-Host "Report: $reportPath"
Write-Host "Observability: $obsMdPath"
Write-Host "JSONL: $jsonlPath"
Write-Host "LastMessage: $lastMessagePath"
Write-Host "Overall: $overall"
if ($overall -ne "PASS") { exit 1 }
exit 0
