param(
    [string]$ScenarioId = "action-map-natural-multi-agent-order-pipeline",
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 1800,
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
$oraclePath = Join-Path $artifactDir "hidden_oracle.py"
$oracleStdoutPath = Join-Path $artifactDir "hidden-oracle.stdout.log"
$oracleStderrPath = Join-Path $artifactDir "hidden-oracle.stderr.log"
$gitDiffPath = Join-Path $artifactDir "git-diff.patch"
$reportPath = Join-Path $artifactDir "report.md"
$scriptSha256 = (Get-FileHash -Algorithm SHA256 $PSCommandPath).Hash; $libSha256 = (Get-FileHash -Algorithm SHA256 (Join-Path $PSScriptRoot "action-map-real-user-e2e-lib.ps1")).Hash
$whaleSha256 = if (Test-Path $WhaleBin) { (Get-FileHash -Algorithm SHA256 $WhaleBin).Hash } else { "" }

Write-Text (Join-Path $srcDir "__init__.py") @'
from .invoice import invoice_total
from .parser import parse_order_line
from .pricing import add_shipping, apply_discount
from .summary import summarize_order
'@
Write-Text (Join-Path $srcDir "parser.py") @'
def parse_order_line(line):
    sku, quantity, unit_price = line.split(",")
    return {"sku": sku, "quantity": int(quantity), "unit_price": float(unit_price)}
'@
Write-Text (Join-Path $srcDir "pricing.py") @'
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
'@
Write-Text (Join-Path $srcDir "invoice.py") @'
from .parser import parse_order_line
from .pricing import add_shipping, apply_discount

def invoice_total(lines, customer_tier="standard"):
    items = [parse_order_line(line) for line in lines]
    subtotal = sum(item["quantity"] * item["unit_price"] for item in items)
    discounted = apply_discount(subtotal, customer_tier)
    return round(add_shipping(discounted), 2)
'@
Write-Text (Join-Path $srcDir "summary.py") @'
from .parser import parse_order_line

def summarize_order(lines):
    items = [parse_order_line(line) for line in lines]
    return {"count": len(items), "skus": [item["sku"] for item in items]}
'@
Write-Text (Join-Path $testDir "test_parser.py") @'
from order_pipeline.parser import parse_order_line

def test_parse_order_line_normalizes_sku_and_numbers():
    assert parse_order_line(" SKU-1 , 2 , 19.50 ") == {"sku": "sku-1", "quantity": 2, "unit_price": 19.50}

def test_parse_order_line_rejects_non_positive_quantity():
    try:
        parse_order_line("sku-1,0,19.50")
    except ValueError as exc:
        assert "quantity" in str(exc).lower()
    else:
        raise AssertionError("expected ValueError")
'@
Write-Text (Join-Path $testDir "test_pricing.py") @'
from order_pipeline.pricing import add_shipping, apply_discount

def test_premium_discount_is_percent_and_case_insensitive():
    assert apply_discount(100, "Premium") == 90

def test_vip_discount_is_percent_and_case_insensitive():
    assert apply_discount(200, "VIP") == 170

def test_shipping_uses_discounted_total():
    assert add_shipping(49.99) == 54.99
    assert add_shipping(50) == 50
'@
Write-Text (Join-Path $testDir "test_invoice_summary.py") @'
from order_pipeline.invoice import invoice_total
from order_pipeline.summary import summarize_order

def test_invoice_total_combines_parser_discount_and_shipping():
    lines = [" SKU-1 , 2 , 20.00 ", "sku-2,1,10.00"]
    assert invoice_total(lines, "Premium") == 50.0

def test_invoice_total_vip_large_order_gets_free_shipping():
    assert invoice_total(["sku-1,3,25.00"], "vip") == 63.75

def test_summary_uses_normalized_skus():
    assert summarize_order([" SKU-1 , 2 , 20.00 ", "SKU-2,1,10.00"]) == {"count": 2, "skus": ["sku-1", "sku-2"]}
'@
Write-Text (Join-Path $repoDir "README.md") @'
# Order Pipeline

Product rules:
- SKU values must be trimmed and lowercased.
- Quantity must be a positive integer.
- Premium customers receive 10 percent off, case-insensitive.
- VIP customers receive 15 percent off, case-insensitive.
- Shipping is added only when the discounted total is below 50.
- Summary output must reuse normalized parsed item data.
'@
Write-Text (Join-Path $repoDir "pyproject.toml") @'
[tool.pytest.ini_options]
pythonpath = ["src"]
'@

Push-Location $repoDir
try {
    git init | Out-Null
    git config user.email "natural-multi-agent-e2e@example.local" | Out-Null
    git config user.name "Natural Multi Agent E2E" | Out-Null
    git add . | Out-Null
    git commit -m "baseline order pipeline handoff" | Out-Null
} finally {
    Pop-Location
}

$prompt = @'
I inherited this small order-pipeline project and need you to treat it like a real handoff. Parser behavior, pricing discounts, invoice totals, and summary output all look connected, and some tests may be wrong relative to the README.

Please inspect the README, tests, and implementation before editing. Separate product truth from broken expectations, organize the work in whatever way best fits the project, then integrate the findings yourself and make the final changes.

Run the current tests before editing so we know the baseline failure, then run the relevant tests again after the fix. Briefly explain how you organized the work and what changed.
'@
Write-Text $promptPath $prompt
$forbiddenPromptTerms = Get-InternalOrchestrationLeakPattern
$forbiddenFinalTerms = $forbiddenPromptTerms
$promptLeakExcerpt = Get-RegexFirstMatchExcerpt $prompt $forbiddenPromptTerms
$promptLeaksInternalConcepts = -not [string]::IsNullOrWhiteSpace($promptLeakExcerpt)

if ($PlanOnly) {
    Write-Host "RunDir: $runDir"
    Write-Host "RepoDir: $repoDir"
    Write-Host "PromptPath: $promptPath"
    Write-Host "PromptLeaksInternalConcepts: $promptLeaksInternalConcepts"
    exit 0
}

if (-not (Test-Path $WhaleBin)) { throw "Whale binary not found: $WhaleBin" }
$whaleVersion = (& $WhaleBin --version 2>&1) -join " "
$helpText = & $WhaleBin exec --help 2>&1
if (($helpText -join [Environment]::NewLine) -notmatch "--taskspace") { throw "Whale exec does not expose --taskspace." }

$started = Get-Date
$execArgs = @("exec", "--json", "--taskspace", "-m", $Model, "-C", $repoDir, "--dangerously-bypass-approvals-and-sandbox", "--output-last-message", $lastMessagePath, "-")
$execExitCode = Invoke-RealProcess $WhaleBin $execArgs $repoDir $jsonlPath $stderrPath $TimeoutSeconds $promptPath
$finished = Get-Date

$validationExitCode = Invoke-RealProcess "python" @("-m", "pytest", "tests", "-q") $repoDir $validationStdoutPath $validationStderrPath 120
Write-Text $oraclePath @"
import sys
sys.path.insert(0, r'$((Join-Path $repoDir "src").Replace("'", "''"))')
from order_pipeline.parser import parse_order_line
from order_pipeline.pricing import apply_discount, add_shipping
from order_pipeline.invoice import invoice_total
from order_pipeline.summary import summarize_order

assert parse_order_line(' SKU-1 , 2 , 19.50 ') == {'sku': 'sku-1', 'quantity': 2, 'unit_price': 19.5}
try:
    parse_order_line('sku-1,0,19.50')
except ValueError as exc:
    assert 'quantity' in str(exc).lower()
else:
    raise AssertionError('expected ValueError')
assert apply_discount(100, 'Premium') == 90
assert apply_discount(200, 'VIP') == 170
assert add_shipping(49.99) == 54.99
assert add_shipping(50) == 50
assert invoice_total([' SKU-1 , 2 , 20.00 ', 'sku-2,1,10.00'], 'Premium') == 50.0
assert invoice_total(['sku-1,3,25.00'], 'vip') == 63.75
assert summarize_order([' SKU-1 , 2 , 20.00 ', 'SKU-2,1,10.00']) == {'count': 2, 'skus': ['sku-1', 'sku-2']}
print('hidden oracle passed')
"@
$oracleExitCode = Invoke-RealProcess "python" @($oraclePath) $repoDir $oracleStdoutPath $oracleStderrPath 120
Push-Location $repoDir
try {
    $gitDiffText = git diff -- src tests README.md pyproject.toml
    $gitDiffText | Set-Content -Encoding UTF8 $gitDiffPath
} finally {
    Pop-Location
}

$jsonlText = Get-Content -Raw -Encoding UTF8 $jsonlPath
$stderrText = Get-Content -Raw -Encoding UTF8 $stderrPath
$validationStdout = Get-Content -Raw -Encoding UTF8 $validationStdoutPath
$lastMessage = if (Test-Path $lastMessagePath) { Get-Content -Raw -Encoding UTF8 $lastMessagePath } else { "" }
$finalOutputLeakExcerpt = Get-RegexFirstMatchExcerpt $lastMessage $forbiddenFinalTerms
$finalOutputLeaksInternalConcepts = -not [string]::IsNullOrWhiteSpace($finalOutputLeakExcerpt)
$threadId = Get-ThreadId $jsonlText
$rollout = Find-LatestRollout $started $threadId
$obsExitCode = -1
$obs = $null
$rolloutText = ""
$obsDir = Join-Path $artifactDir "observability"
$obsJsonPath = Join-Path $obsDir "action-map-observability.json"; $obsMdPath = Join-Path $obsDir "action-map-observability.md"; $obsHtmlPath = Join-Path $obsDir "action-map-observability.html"
if ($rollout) {
    $rolloutText = Get-Content -Raw -Encoding UTF8 $rollout.FullName
    Copy-Item -LiteralPath $rollout.FullName -Destination (Join-Path $artifactDir "rollout.jsonl") -Force
    $exportScript = Join-Path $PSScriptRoot "export-action-map-observability.ps1"
    $obsStdout = Join-Path $artifactDir "observability.stdout.log"
    $obsStderr = Join-Path $artifactDir "observability.stderr.log"
    $obsExitCode = Invoke-RealProcess "powershell" @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $exportScript, "-RolloutPath", $rollout.FullName, "-JsonlPath", $jsonlPath, "-OutputDir", $obsDir, "-WhalePath", $WhaleBin, "-ThreadId", $threadId, "-ArtifactRoot", $repoDir) $repoDir $obsStdout $obsStderr 180
    $obsJson = Join-Path $obsDir "action-map-observability.json"
    if (Test-Path $obsJson) { $obs = Get-Content -Raw -Encoding UTF8 $obsJson | ConvertFrom-Json }
}
$toolCallArgs = @{}
foreach ($line in ($rolloutText -split "`r?`n")) {
    try { $evt = $line | ConvertFrom-Json } catch { continue }
    if ($evt.type -eq "response_item" -and $evt.payload.type -eq "function_call" -and $evt.payload.call_id) {
        $toolCallArgs[[string]$evt.payload.call_id] = [string]$evt.payload.arguments
    }
}

$ordering = Get-SuccessfulTaskspaceOrdering $rolloutText
$graphHealth = Get-TaskspaceGraphHealth $obs
$cognitiveAudit = if ($obs) { $obs.cognitiveAudit } else { $null }
$cognitiveHardGatePassed = if ($cognitiveAudit) { [bool]$cognitiveAudit.hardGatePassed } else { $false }
$cognitiveHardGateFailures = if ($cognitiveAudit) { @($cognitiveAudit.hardGateFailures) } else { @() }
$outputContractCount = if ($obs) { [int]$obs.summary.outputContracts } else { 0 }
$factSourceCount = if ($obs) { [int]$obs.summary.factSources } else { 0 }
$acceptedResultCount = if ($obs) { [int]$obs.summary.acceptedResults } else { 0 }
$finalArtifactCount = if ($obs) { [int]$obs.summary.finalArtifacts } else { 0 }
$mapCount = if ($obs) { @($obs.maps).Count } else { 0 }
$nodeCount = if ($obs) { @($obs.nodes).Count } else { 0 }
$agentCount = if ($obs) { @($obs.agents).Count } else { 0 }
$nodesWithResults = if ($obs) { @($obs.nodes | Where-Object { @($_.results).Count -gt 0 }).Count } else { 0 }
$completedNodes = if ($obs) { @($obs.nodes | Where-Object { $_.status -eq "completed" }).Count } else { 0 }
$spawnAgentCount = if ($obs) { @($obs.toolCalls | Where-Object { $_.tool -eq "spawn_agent" -and $_.status -eq "completed" }).Count } else { 0 }
$blockedToolActionCount = if ($obs) { @($obs.nodes | ForEach-Object { @($_.blockedActions) }).Count } else { 0 }
$unexpectedBlockedToolActionCount = Count-UnexpectedBlockedTaskspaceToolActions $obs
$implementationNodeHasSuccessfulEdit = $false
$editOutsideImplementationCount = 0
if ($obs) {
    foreach ($node in @($obs.nodes)) {
        foreach ($result in @($node.results | Where-Object { $_.kind -eq "main_tool_call" -and $_.actionClass -eq "edit" })) {
            if ((Get-ObjectPropertyNames $result) -contains "success" -and $result.success -ne $true) {
                continue
            }
            if ([string]$node.kind -eq "implement_solution") {
                $implementationNodeHasSuccessfulEdit = $true
            } else {
                $editOutsideImplementationCount++
            }
        }
    }
}
$failedTaskspaceToolResults = Count-FailedTaskspaceToolResults $obs
$unexpectedFailedTaskspaceToolResults = Count-UnexpectedFailedTaskspaceToolResults $obs
$unexpectedFailedToolBudget = 0
$failedCollabToolCalls = Count-FailedCollabToolCalls $obs
$unexpectedFailedCollabToolCalls = Count-UnexpectedFailedCollabToolCalls $obs
$problematicSuccessfulToolResults = Count-ProblematicSuccessfulToolResults $obs
$implementationOwnershipGap = Get-ImplementationOwnershipGap $obs $gitDiffText $toolCallArgs
$unexpectedTaskspaceGateFailures = if ($obs) {
    @($obs.nodes | ForEach-Object {
            @($_.results | Where-Object {
                    $_.kind -eq "main_tool_call" -and
                    (Get-ObjectPropertyNames $_) -contains "success" -and
                    $_.success -eq $false
                })
        }).Count
} else { 0 }
$subagentResultCount = if ($obs) {
    @($obs.nodes | ForEach-Object {
            $agentIds = @($_.agentThreads)
            @($_.results | Where-Object { $agentIds -contains $_.sourceThreadId })
        }).Count
} else { 0 }
$testNodeHasPassingPytest = $false
if ($obs) {
    foreach ($node in @($obs.nodes | Where-Object { [string]$_.kind -match "smoke_test|regression_test" })) {
        foreach ($result in @($node.results | Where-Object { $_.kind -eq "main_tool_call" -and $_.actionClass -eq "test" })) {
            if ((Get-ObjectPropertyNames $result) -contains "success" -and $result.success -eq $false) { continue }
            if ($result.success -eq $true) { $testNodeHasPassingPytest = $true }
        }
    }
}
$commandExecutionCount = Count-Matches $jsonlText '"type"\s*:\s*"command_execution"'
$repoHead = ""
$repoStatus = ""
Push-Location $repoDir
try {
    $repoHead = (git rev-parse HEAD) -join "`n"
    $repoStatus = (git status --short) -join " "
} finally {
    Pop-Location
}

$failures = New-Object System.Collections.Generic.List[string]
if ($promptLeaksInternalConcepts) { $failures.Add("natural user prompt leaked internal orchestration concepts: $promptLeakExcerpt") }
if ($finalOutputLeaksInternalConcepts) { $failures.Add("final user output leaked internal orchestration concepts: $finalOutputLeakExcerpt") }
if ($execExitCode -ne 0) { $failures.Add("whale exec exit code was $execExitCode") }
if ($validationExitCode -ne 0) { $failures.Add("post-run pytest exit code was $validationExitCode") }
if ($oracleExitCode -ne 0) { $failures.Add("hidden oracle exit code was $oracleExitCode") }
if ($validationStdout -notmatch "passed") { $failures.Add("pytest output did not contain a passing marker") }
if ([string]::IsNullOrWhiteSpace($gitDiffText)) { $failures.Add("repository diff is empty after the real agent run") }
if (-not $rollout) { $failures.Add("could not find the rollout for this thread") }
if ($rollout -and $obsExitCode -ne 0) { $failures.Add("observability export failed with exit code $obsExitCode") }
if ($rollout -and $obsExitCode -eq 0 -and -not $cognitiveHardGatePassed) { $failures.Add("cognitive hard gate failed: $($cognitiveHardGateFailures -join ', ')") }
if ($ordering.OrdinaryToolBeforeBinding) { $failures.Add("ordinary tool succeeded before first TaskSpace binding") }
if ($mapCount -lt 1) { $failures.Add("no map was observed") }
if ($nodeCount -lt 5) { $failures.Add("map did not grow to at least 5 nodes; observed $nodeCount") }
if ($graphHealth.EdgeCount -lt 2) { $failures.Add("map did not create enough dependency edges; observed $($graphHealth.EdgeCount)") }
if ($graphHealth.OrderedEdgeCount -ne $graphHealth.EdgeCount) { $failures.Add("not every dependency edge had observable predecessor-complete and successor-work timestamps") }
if ($graphHealth.EdgeOrderViolationCount -gt 0) { $failures.Add("map dependency execution order was violated on $($graphHealth.EdgeOrderViolationCount) edge(s)") }
if ($graphHealth.ParallelInspectTrackCount -lt 2) { $failures.Add("expected at least 2 parallel inspect tracks with subagent ownership; observed $($graphHealth.ParallelInspectTrackCount)") }
if (-not $graphHealth.ParallelInspectTracksIndependent) { $failures.Add("parallel inspect tracks were not represented as independent graph tracks") }
if (-not $graphHealth.DirectImplementationDependsOnParallelInspectTracks) { $failures.Add("implementation node did not directly depend on all subagent-owned inspect tracks") }
if (-not $graphHealth.DirectTestDependsOnImplementation) { $failures.Add("test node did not directly depend on implementation node") }
if ($graphHealth.OpenFinalSynthesisCount -gt 0) { $failures.Add("final synthesis node was left open: $($graphHealth.OpenFinalSynthesisCount)") }
if ($graphHealth.OpenLeafNodeCount -gt 0) { $failures.Add("open leaf nodes remained at the end of the run: $($graphHealth.OpenLeafNodeCount)") }
if ($agentCount -lt 2) { $failures.Add("expected at least 2 subagent leases; observed $agentCount agents") }
if ($spawnAgentCount -lt 2) { $failures.Add("expected at least 2 successful spawn_agent calls; observed $spawnAgentCount") }
if ($subagentResultCount -lt 2) { $failures.Add("expected at least 2 subagent results written to nodes") }
if ($nodesWithResults -lt 3) { $failures.Add("expected results on at least 3 nodes; observed $nodesWithResults") }
if ($completedNodes -lt 2) { $failures.Add("expected at least 2 completed nodes; observed $completedNodes") }
if (-not $implementationNodeHasSuccessfulEdit) { $failures.Add("implementation node did not own a successful edit action") }
if ($editOutsideImplementationCount -gt 0) { $failures.Add("observed $editOutsideImplementationCount successful edit action(s) outside implementation nodes") }
if (-not $testNodeHasPassingPytest) { $failures.Add("test node did not own a passing pytest result") }
if ($unexpectedBlockedToolActionCount -gt 0) { $failures.Add("unexpected blocked TaskSpace tool actions: $unexpectedBlockedToolActionCount") }
if ($unexpectedFailedTaskspaceToolResults -gt 0) { $failures.Add("unexpected failed taskspace-owned tool results: $unexpectedFailedTaskspaceToolResults") }
if ($unexpectedFailedCollabToolCalls -gt 0) { $failures.Add("unexpected failed collaboration tool calls: $unexpectedFailedCollabToolCalls") }
if ($implementationOwnershipGap.MissingCount -gt 0) { $failures.Add("changed paths not owned by successful implementation-node edits: $($implementationOwnershipGap.MissingPaths -join ', ')") }
if ($unexpectedTaskspaceGateFailures -gt 0) { $failures.Add("unexpected TaskSpace gate/tool attribution failures: $unexpectedTaskspaceGateFailures") }
if ($commandExecutionCount -lt 4) { $failures.Add("agent did not run enough real commands; observed $commandExecutionCount") }

$overall = if ($failures.Count -eq 0) { "PASS" } else { "FAIL" }
$report = New-Object System.Collections.Generic.List[string]
$report.Add("# Action Map Natural Multi-Agent E2E Report")
$report.Add("")
foreach ($row in @(
    @("overall", $overall), @("scenario_id", $ScenarioId), @("run_dir", $runDir), @("repo_dir", $repoDir),
    @("whale_bin", $WhaleBin), @("model", $Model), @("started", $started.ToString("o")),
    @("finished", $finished.ToString("o")), @("thread_id", $threadId), @("exec_exit_code", $execExitCode),
    @("validation_exit_code", $validationExitCode), @("oracle_exit_code", $oracleExitCode), @("script_sha256", $scriptSha256), @("e2e_lib_sha256", $libSha256),
    @("whale_sha256", $whaleSha256), @("whale_version", $whaleVersion), @("repo_head", $repoHead), @("repo_status", $repoStatus),
    @("rollout", $(if ($rollout) { $rollout.FullName } else { "" }))
)) { Add-ReportLine $report $row[0] $row[1] }
$report.Add(""); $report.Add("## Artifacts"); $report.Add("")
foreach ($row in @(@("jsonl", $jsonlPath), @("stderr", $stderrPath), @("last_message", $lastMessagePath), @("git_diff", $gitDiffPath), @("observability_json", $obsJsonPath), @("observability_md", $obsMdPath), @("observability_html", $obsHtmlPath))) { Add-ReportLine $report $row[0] $row[1] }
$report.Add("")
$report.Add("## Metrics")
$report.Add("")
foreach ($row in @(
    @("prompt_leaks_internal_concepts", $promptLeaksInternalConcepts), @("final_output_leaks_internal_concepts", $finalOutputLeaksInternalConcepts),
    @("prompt_leak_excerpt", $promptLeakExcerpt), @("final_output_leak_excerpt", $finalOutputLeakExcerpt), @("forbidden_terms", $forbiddenFinalTerms), @("maps", $mapCount), @("nodes", $nodeCount),
    @("output_contracts", $outputContractCount), @("fact_sources", $factSourceCount),
    @("accepted_results", $acceptedResultCount), @("cognitive_hard_gate_passed", $cognitiveHardGatePassed),
    @("cognitive_hard_gate_failures", $($cognitiveHardGateFailures -join ", ")),
    @("final_artifacts", $finalArtifactCount),
    @("edges", $graphHealth.EdgeCount), @("ordered_edges", $graphHealth.OrderedEdgeCount),
    @("edge_order_violations", $graphHealth.EdgeOrderViolationCount),
    @("anchored_implementation_nodes", $graphHealth.AnchoredImplementationCount),
    @("parallel_inspect_tracks", $graphHealth.ParallelInspectTrackCount),
    @("parallel_inspect_tracks_independent", $graphHealth.ParallelInspectTracksIndependent),
    @("implementation_depends_on_parallel_inspect_tracks", $graphHealth.ImplementationDependsOnParallelInspectTracks),
    @("direct_implementation_depends_on_parallel_inspect_tracks", $graphHealth.DirectImplementationDependsOnParallelInspectTracks),
    @("test_depends_on_implementation", $graphHealth.TestDependsOnImplementation),
    @("direct_test_depends_on_implementation", $graphHealth.DirectTestDependsOnImplementation),
    @("open_leaf_nodes", $graphHealth.OpenLeafNodeCount),
    @("open_final_synthesis_nodes", $graphHealth.OpenFinalSynthesisCount),
    @("agents", $agentCount), @("spawn_agent_calls", $spawnAgentCount), @("subagent_results", $subagentResultCount),
    @("nodes_with_results", $nodesWithResults), @("completed_nodes", $completedNodes),
    @("blocked_tool_actions", $blockedToolActionCount),
    @("unexpected_blocked_tool_actions", $unexpectedBlockedToolActionCount),
    @("implementation_node_has_successful_edit", $implementationNodeHasSuccessfulEdit),
    @("edit_outside_implementation", $editOutsideImplementationCount),
    @("failed_taskspace_tool_results", $failedTaskspaceToolResults),
    @("unexpected_failed_taskspace_tool_results", $unexpectedFailedTaskspaceToolResults),
    @("unexpected_failed_tool_budget", $unexpectedFailedToolBudget),
    @("failed_collab_tool_calls", $failedCollabToolCalls), @("unexpected_failed_collab_tool_calls", $unexpectedFailedCollabToolCalls),
    @("problematic_successful_tool_results", $problematicSuccessfulToolResults),
    @("changed_paths_without_implementation_owner", $implementationOwnershipGap.MissingCount),
    @("unexpected_taskspace_gate_failures", $unexpectedTaskspaceGateFailures),
    @("test_node_has_passing_pytest", $testNodeHasPassingPytest),
    @("ordinary_before_binding", $ordering.OrdinaryToolBeforeBinding), @("command_executions", $commandExecutionCount)
)) { Add-ReportLine $report $row[0] $row[1] }
$report.Add("")
$report.Add("## Failures")
$report.Add("")
if ($failures.Count -eq 0) { $report.Add("- none") } else { foreach ($failure in $failures) { $report.Add("- $failure") } }
$report | Set-Content -Encoding UTF8 $reportPath
Write-Host "Report: $reportPath"
Write-Host "Overall: $overall"
if ($overall -ne "PASS") { exit 1 }
