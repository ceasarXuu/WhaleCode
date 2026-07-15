param(
    [string]$ScenarioId = "action-map-natural-user-order-pipeline",
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 1200,
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "action-map-real-user-e2e-lib.ps1")

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
$scriptSha256 = (Get-FileHash -Algorithm SHA256 $PSCommandPath).Hash; $libSha256 = (Get-FileHash -Algorithm SHA256 (Join-Path $PSScriptRoot "action-map-real-user-e2e-lib.ps1")).Hash
$whaleSha256 = if (Test-Path $WhaleBin) { (Get-FileHash -Algorithm SHA256 $WhaleBin).Hash } else { "" }
$workspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$repoHead = (git -C $workspaceRoot rev-parse HEAD 2>$null) -join "`n"

@'
from .invoice import invoice_total
from .parser import parse_order_line
from .pricing import add_shipping, apply_discount
'@ | ForEach-Object { Write-Text (Join-Path $srcDir "__init__.py") $_ }

@'
def parse_order_line(line):
    sku, quantity, unit_price = line.split(",")
    return {"sku": sku, "quantity": int(quantity), "unit_price": float(unit_price)}
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
    assert parse_order_line(" SKU-1 , 2 , 19.50 ") == {"sku": "sku-1", "quantity": 2, "unit_price": 19.50}

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
    assert invoice_total(["sku-1,3,25.00"], "vip") == 63.75
'@ | ForEach-Object { Write-Text (Join-Path $testDir "test_invoice.py") $_ }

@'
# Order Pipeline

Product rules:
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
    git config user.email "natural-user-e2e@example.local" | Out-Null
    git config user.name "Natural User E2E" | Out-Null
    git add . | Out-Null
    git commit -m "baseline order pipeline regressions" | Out-Null
} finally {
    Pop-Location
}

$prompt = @'
I just inherited this small order-pipeline project. The amount calculation and tests look inconsistent. Please handle it the way you would in a real project handoff: read the code and README first, find the root cause, and fix it.

The README is the source of product truth. If a test expectation conflicts with the README, update the test to match the README instead of changing the code to satisfy the wrong expectation. Please run the necessary tests at the end and briefly explain what you changed and why.
'@
Write-Text $promptPath $prompt
$forbiddenPromptTerms = Get-InternalOrchestrationLeakPattern
$promptLeakExcerpt = Get-RegexFirstMatchExcerpt $prompt $forbiddenPromptTerms
$promptLeaksInternalConcepts = -not [string]::IsNullOrWhiteSpace($promptLeakExcerpt)

if ($PlanOnly -or $PSBoundParameters.ContainsKey("PlanOnly")) {
    Write-Host "RunDir: $runDir"
    Write-Host "RepoDir: $repoDir"
    Write-Host "WhaleBin: $WhaleBin"
    Write-Host "Model: $Model"
    Write-Host "PromptPath: $promptPath"
    Write-Host "PromptLeaksInternalConcepts: $promptLeaksInternalConcepts"
    Write-Host "ReportPath: $reportPath"
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
    raise AssertionError('expected ValueError')
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
$threadId = Get-ThreadId $jsonlText
$rollout = Find-LatestRollout $started $threadId
$rolloutText = ""
$rolloutCopy = ""
if ($rollout) {
    $rolloutCopy = Join-Path $artifactDir "rollout.jsonl"
    Copy-Item -LiteralPath $rollout.FullName -Destination $rolloutCopy -Force
    $rolloutText = Get-Content -Raw -Encoding UTF8 $rolloutCopy
}

Push-Location $repoDir
try { git diff -- . | Set-Content -Encoding UTF8 $gitDiffPath } finally { Pop-Location }
$gitDiffText = if (Test-Path $gitDiffPath) { Get-Content -Raw -Encoding UTF8 $gitDiffPath } else { "" }

$obsJsonPath = Join-Path $artifactDir "action-map-observability.json"
$obsMdPath = Join-Path $artifactDir "action-map-observability.md"
$obsHtmlPath = Join-Path $artifactDir "action-map-observability.html"
$obsExitCode = 0
if ($rollout) {
    $exportScript = Join-Path $PSScriptRoot "export-action-map-observability.ps1"
    & $exportScript -RolloutPath $rolloutCopy -JsonlPath $jsonlPath -OutputDir $artifactDir -WhalePath $WhaleBin -ArtifactRoot $repoDir | Out-Host
    $obsExitCode = $LASTEXITCODE
}
$obs = if (Test-Path $obsJsonPath) { Get-Content -Raw -Encoding UTF8 $obsJsonPath | ConvertFrom-Json } else { $null }
$lastMessage = if (Test-Path $lastMessagePath) { Get-Content -Raw -Encoding UTF8 $lastMessagePath } else { "" }
$finalOutputLeakExcerpt = Get-RegexFirstMatchExcerpt $lastMessage $forbiddenPromptTerms
$finalOutputLeaksInternalConcepts = -not [string]::IsNullOrWhiteSpace($finalOutputLeakExcerpt)
$cognitiveAudit = if ($obs) { $obs.cognitiveAudit } else { $null }
$cognitiveHardGatePassed = if ($cognitiveAudit) { [bool]$cognitiveAudit.hardGatePassed } else { $false }
$cognitiveHardGateFailures = if ($cognitiveAudit) { @($cognitiveAudit.hardGateFailures) } else { @() }
$outputContractCount = if ($obs) { [int]$obs.summary.outputContracts } else { 0 }
$factSourceCount = if ($obs) { [int]$obs.summary.factSources } else { 0 }
$acceptedResultCount = if ($obs) { [int]$obs.summary.acceptedResults } else { 0 }
$finalArtifactCount = if ($obs) { [int]$obs.summary.finalArtifacts } else { 0 }
$toolCallArgs = @{}
foreach ($line in ($rolloutText -split "`r?`n")) {
    try { $evt = $line | ConvertFrom-Json } catch { continue }
    if ($evt.type -eq "response_item" -and $evt.payload.type -eq "function_call" -and $evt.payload.call_id) {
        $toolCallArgs[[string]$evt.payload.call_id] = [string]$evt.payload.arguments
    }
}

$mapCount = if ($obs) { @($obs.maps).Count } else { 0 }
$nodeCount = if ($obs) { @($obs.nodes).Count } else { 0 }
$agentCount = if ($obs) { @($obs.agents).Count } else { 0 }
$completedNodes = if ($obs) { @($obs.nodes | Where-Object { $_.status -eq "completed" }).Count } else { 0 }
$nodesWithResults = if ($obs) { @($obs.nodes | Where-Object { @($_.results).Count -gt 0 }).Count } else { 0 }
$nodeMetrics = if ($obs) { @($obs.nodes) } else { @() }
$titleText = if ($obs) { (@($obs.nodes | ForEach-Object { $_.title }) -join "`n").ToLowerInvariant() } else { "" }
$hasBoundaryNode = $titleText -match "boundary|scope|repo|inspect|read"
$hasParserNode = $titleText -match "parser|parse|sku"
$hasPricingNode = $titleText -match "pricing|discount|invoice|shipping"
$hasImplementationNode = $titleText -match "implement|fix|change"
$hasValidationNode = $titleText -match "validat|regression|test|verify"
$titleCoverageComplete = $hasBoundaryNode -and $hasParserNode -and $hasPricingNode -and $hasImplementationNode -and $hasValidationNode
$kindText = (@($nodeMetrics | ForEach-Object { $_.kind }) -join "`n").ToLowerInvariant()
$hasInspectKind = $kindText -match "inspect_code_context"
$hasImplementationKind = $kindText -match "implement_solution"
$hasTestKind = $kindText -match "smoke_test|regression_test"
$implementationNodeHasMainTools = if ($obs) { @($obs.nodes | Where-Object { $_.title -match "(?i)implement|fix|change" -and @($_.results | Where-Object { $_.kind -eq "main_tool_call" }).Count -gt 0 }).Count -gt 0 } else { $false }
$implementationKindHasEdit = @($nodeMetrics | Where-Object {
        $_.kind -eq "implement_solution" -and @($_.results | Where-Object {
                $_.kind -eq "main_tool_call" -and $_.actionClass -eq "edit" -and ((Get-ObjectPropertyNames $_) -notcontains "success" -or $_.success -eq $true)
            }).Count -gt 0
    }).Count -gt 0
$testKindHasTest = @($nodeMetrics | Where-Object {
        ($_.kind -eq "smoke_test" -or $_.kind -eq "regression_test") -and @($_.results | Where-Object {
                $_.kind -eq "main_tool_call" -and $_.actionClass -eq "test" -and ((Get-ObjectPropertyNames $_) -notcontains "success" -or $_.success -eq $true)
            }).Count -gt 0
    }).Count -gt 0
$editResultsOutsideImplementation = @($nodeMetrics | Where-Object { $_.kind -ne "implement_solution" } | ForEach-Object { @($_.results | Where-Object { $_.kind -eq "main_tool_call" -and $_.actionClass -eq "edit" }) }).Count
$testResultsOutsideDiagnosticOrTestNode = @($nodeMetrics | Where-Object { $_.kind -ne "inspect_code_context" -and $_.kind -ne "smoke_test" -and $_.kind -ne "regression_test" } | ForEach-Object { @($_.results | Where-Object { $_.kind -eq "main_tool_call" -and $_.actionClass -eq "test" }) }).Count
$unknownActionResults = @($nodeMetrics | ForEach-Object { @($_.results | Where-Object { $_.kind -eq "main_tool_call" -and ([string]::IsNullOrWhiteSpace([string]$_.actionClass) -or $_.actionClass -eq "unknown") }) }).Count
$failedTaskspaceToolResults = Count-FailedTaskspaceToolResults $obs
$unexpectedFailedTaskspaceToolResults = Count-UnexpectedFailedTaskspaceToolResults $obs
$unexpectedFailedToolBudget = 0
$failedCollabToolCalls = Count-FailedCollabToolCalls $obs
$unexpectedFailedCollabToolCalls = Count-UnexpectedFailedCollabToolCalls $obs
$problematicSuccessfulToolResults = Count-ProblematicSuccessfulToolResults $obs
$implementationOwnershipGap = Get-ImplementationOwnershipGap $obs $gitDiffText $toolCallArgs
$blockedActionCount = 0
foreach ($node in $nodeMetrics) {
    if ($null -ne $node.blockedActions) {
        $blockedActionCount += @($node.blockedActions).Count
    }
}
$pytestOwnership = Get-PytestOwnership $obs $toolCallArgs
$validationNodeHasPytestResult = $pytestOwnership.Owned
$editResultsAfterFinalPytest = Count-EditResultsAfter $nodeMetrics $pytestOwnership.At
$finishNodeCallCount = Count-Matches $rolloutText 'TaskSpace node finished:'
$spawnAgentCount = if ($obs) { @($obs.toolCalls | Where-Object { $_.tool -eq "spawn_agent" -and $_.status -eq "completed" }).Count } else { 0 }
$expectedFinishNodeCalls = [Math]::Max(1, $completedNodes - $spawnAgentCount - 1)
$commandStats = Get-CommandStats $jsonlText
$taskspaceOrdering = Get-SuccessfulTaskspaceOrdering $rolloutText
$postHocEmptyTerminalNodes = if ($obs) { @($obs.nodes | Where-Object { $_.title -match "(?i)validat|final|synthesis" -and @($_.results).Count -eq 0 }).Count } else { 0 }
$taskspaceControlCount = Count-Matches $rolloutText '"name":"taskspace_control"|"name"\s*:\s*"taskspace_control"'
$realWorkItemCount = @($nodeMetrics | ForEach-Object {
        @($_.results | Where-Object {
                $_.kind -eq "main_tool_call" -and
                ($_.toolName -eq "shell_command" -or $_.toolName -eq "apply_patch") -and
                ((Get-ObjectPropertyNames $_) -notcontains "success" -or $_.success -eq $true)
            })
    }).Count

$failures = New-Object System.Collections.Generic.List[string]
if ($promptLeaksInternalConcepts) { $failures.Add("natural user prompt leaked internal orchestration concepts: $promptLeakExcerpt") }
if ($finalOutputLeaksInternalConcepts) { $failures.Add("final user output leaked internal orchestration concepts: $finalOutputLeakExcerpt") }
if ($execExitCode -ne 0) { $failures.Add("whale exec exit code was $execExitCode") }
if ($validationExitCode -ne 0) { $failures.Add("post-run pytest exit code was $validationExitCode") }
if ($oracleExitCode -ne 0) { $failures.Add("hidden oracle exit code was $oracleExitCode") }
if ([string]::IsNullOrWhiteSpace($gitDiffText)) { $failures.Add("repository diff is empty after the real agent run") }
if (-not $rollout) { $failures.Add("could not find rollout for this thread") }
if ($rollout -and $obsExitCode -ne 0) { $failures.Add("observability export failed with exit code $obsExitCode") }
if ($rollout -and $obsExitCode -eq 0 -and -not $cognitiveHardGatePassed) { $failures.Add("cognitive hard gate failed: $($cognitiveHardGateFailures -join ', ')") }
if ($taskspaceOrdering.OrdinaryToolBeforeBinding) { $failures.Add("ordinary tool was called before taskspace task/node binding") }
if ($mapCount -lt 1) { $failures.Add("no task map was observed") }
if ($nodeCount -lt 4) { $failures.Add("natural task did not grow to at least 4 nodes; observed $nodeCount") }
if ($nodesWithResults -lt 3) { $failures.Add("expected results on at least 3 nodes; observed $nodesWithResults") }
if ($completedNodes -lt 2) { $failures.Add("expected at least 2 completed nodes; observed $completedNodes") }
if ($finishNodeCallCount -lt $expectedFinishNodeCalls) { $failures.Add("expected at least $expectedFinishNodeCalls direct finish_node calls after subagent/final auto-completions; observed $finishNodeCallCount") }
if (-not ($hasInspectKind -and $hasImplementationKind -and $hasTestKind)) { $failures.Add("node kinds did not cover inspect/implementation/test phases") }
if (-not $implementationKindHasEdit) { $failures.Add("implement_solution node did not own an edit action result") }
if (-not $testKindHasTest) { $failures.Add("smoke_test/regression_test node did not own a test action result") }
if ($editResultsOutsideImplementation -gt 0) { $failures.Add("edit action results were recorded outside implement_solution nodes: $editResultsOutsideImplementation") }
if ($testResultsOutsideDiagnosticOrTestNode -gt 0) { $failures.Add("test action results were recorded outside inspect/smoke/regression nodes: $testResultsOutsideDiagnosticOrTestNode") }
if ($unknownActionResults -gt 0) { $failures.Add("unknown action results were recorded: $unknownActionResults") }
if ($unexpectedFailedTaskspaceToolResults -gt 0) { $failures.Add("unexpected failed taskspace-owned tool results: $unexpectedFailedTaskspaceToolResults") }
if ($unexpectedFailedCollabToolCalls -gt 0) { $failures.Add("unexpected failed collaboration tool calls: $unexpectedFailedCollabToolCalls") }
if ($implementationOwnershipGap.MissingCount -gt 0) { $failures.Add("changed paths not owned by successful implementation-node edits: $($implementationOwnershipGap.MissingPaths -join ', ')") }
if (-not $commandStats.AgentRanPassingPytest) { $failures.Add("agent did not run a passing pytest command") }
if (-not $validationNodeHasPytestResult) { $failures.Add("pytest passed, but was not owned by a smoke/regression node") }
if ($editResultsAfterFinalPytest -gt 0) { $failures.Add("edit action results occurred after the final owned pytest validation: $editResultsAfterFinalPytest") }
if ($postHocEmptyTerminalNodes -gt 0) { $failures.Add("terminal validation/final nodes were created without results") }
if ($commandStats.Completed -lt 3) { $failures.Add("agent did not run enough real shell commands; observed $($commandStats.Completed)") }
if ($realWorkItemCount -lt 5) { $failures.Add("agent did not produce enough successful real shell/patch work items; observed $realWorkItemCount") }
$overall = if ($failures.Count -eq 0) { "PASS" } else { "FAIL" }

$report = New-Object System.Collections.Generic.List[string]
$report.Add("# Natural User TaskSpace E2E Report")
$report.Add("")
foreach ($row in @(
    @("overall", $overall), @("scenario_id", $ScenarioId), @("run_dir", $runDir), @("repo_dir", $repoDir),
    @("whale_bin", $WhaleBin), @("model", $Model), @("started", $started.ToString("o")),
    @("finished", $finished.ToString("o")), @("thread_id", $threadId), @("exec_exit_code", $execExitCode),
    @("validation_exit_code", $validationExitCode), @("script_sha256", $scriptSha256), @("e2e_lib_sha256", $libSha256),
    @("whale_sha256", $whaleSha256), @("whale_version", $whaleVersion), @("repo_head", $repoHead),
    @("rollout", $(if ($rollout) { $rollout.FullName } else { "" }))
)) { Add-ReportLine $report $row[0] $row[1] }
$report.Add("")
$report.Add("## Natural Prompt Guard")
$report.Add("")
Add-ReportLine $report "prompt_leaks_internal_concepts" $promptLeaksInternalConcepts
Add-ReportLine $report "final_output_leaks_internal_concepts" $finalOutputLeaksInternalConcepts
Add-ReportLine $report "prompt_leak_excerpt" $promptLeakExcerpt
Add-ReportLine $report "final_output_leak_excerpt" $finalOutputLeakExcerpt
Add-ReportLine $report "forbidden_prompt_terms" $forbiddenPromptTerms
$report.Add("")
$report.Add("## Growth Metrics")
$report.Add("")
foreach ($row in @(
    @("maps", $mapCount), @("nodes", $nodeCount), @("agents", $agentCount), @("spawn_agent", $spawnAgentCount),
    @("output_contracts", $outputContractCount), @("fact_sources", $factSourceCount),
    @("accepted_results", $acceptedResultCount), @("cognitive_hard_gate_passed", $cognitiveHardGatePassed),
    @("cognitive_hard_gate_failures", $($cognitiveHardGateFailures -join ", ")),
    @("final_artifacts", $finalArtifactCount),
    @("completed_nodes", $completedNodes), @("nodes_with_results", $nodesWithResults),
    @("taskspace_control", $taskspaceControlCount), @("finish_node_calls", $finishNodeCallCount),
    @("expected_finish_node_calls", $expectedFinishNodeCalls),
    @("has_boundary_node", $hasBoundaryNode), @("has_parser_node", $hasParserNode), @("has_pricing_node", $hasPricingNode),
    @("has_implementation_node", $hasImplementationNode), @("has_validation_node", $hasValidationNode),
    @("title_coverage_complete", $titleCoverageComplete), @("has_inspect_kind", $hasInspectKind),
    @("has_implementation_kind", $hasImplementationKind), @("has_test_kind", $hasTestKind),
    @("implementation_node_has_main_tools", $implementationNodeHasMainTools),
    @("implementation_kind_has_edit", $implementationKindHasEdit), @("test_kind_has_test", $testKindHasTest),
    @("edit_results_outside_implementation", $editResultsOutsideImplementation),
    @("test_results_outside_diagnostic_or_test_node", $testResultsOutsideDiagnosticOrTestNode),
    @("unknown_action_results", $unknownActionResults), @("blocked_action_count", $blockedActionCount),
    @("failed_taskspace_tool_results", $failedTaskspaceToolResults),
    @("unexpected_failed_taskspace_tool_results", $unexpectedFailedTaskspaceToolResults),
    @("unexpected_failed_tool_budget", $unexpectedFailedToolBudget),
    @("failed_collab_tool_calls", $failedCollabToolCalls),
    @("unexpected_failed_collab_tool_calls", $unexpectedFailedCollabToolCalls),
    @("problematic_successful_tool_results", $problematicSuccessfulToolResults),
    @("changed_paths_without_implementation_owner", $implementationOwnershipGap.MissingCount),
    @("agent_ran_passing_pytest", $commandStats.AgentRanPassingPytest),
    @("pytest_owned_by_validation_node", $validationNodeHasPytestResult),
    @("edit_results_after_final_pytest", $editResultsAfterFinalPytest),
    @("posthoc_empty_terminal_nodes", $postHocEmptyTerminalNodes), @("hidden_oracle_exit_code", $oracleExitCode),
    @("unique_completed_command_executions", $commandStats.Completed), @("failed_command_executions", $commandStats.Failed),
    @("successful_real_shell_or_patch_work_items", $realWorkItemCount),
    @("agent_pytest_command_count", $commandStats.PytestCount), @("git_diff_bytes", $gitDiffText.Length),
    @("first_taskspace_binding_timestamp", $taskspaceOrdering.FirstBindingTimestamp),
    @("first_taskspace_binding_evidence", $taskspaceOrdering.FirstBindingEvidence),
    @("first_ordinary_tool_timestamp", $taskspaceOrdering.FirstOrdinaryToolTimestamp),
    @("first_ordinary_tool", $taskspaceOrdering.FirstOrdinaryTool),
    @("ordinary_tool_before_taskspace_binding", $taskspaceOrdering.OrdinaryToolBeforeBinding),
    @("pytest_owner_node_id", $pytestOwnership.NodeId), @("pytest_owner_node_kind", $pytestOwnership.NodeKind),
    @("pytest_owner_node_title", $pytestOwnership.NodeTitle),
    @("pytest_owner_result_id", $pytestOwnership.ResultId), @("pytest_owner_call_id", $pytestOwnership.CallId),
    @("pytest_owner_command", $pytestOwnership.Command)
)) { Add-ReportLine $report $row[0] $row[1] }
$report.Add("")
$report.Add("## Failures")
$report.Add("")
if ($failures.Count -eq 0) { $report.Add("None.") } else { foreach ($failure in $failures) { $report.Add("- $failure") } }
$report.Add("")
$report.Add("## Artifacts")
$report.Add("")
foreach ($row in @(
    @("prompt", $promptPath), @("exec_jsonl", $jsonlPath), @("exec_stderr", $stderrPath), @("rollout_copy", $rolloutCopy),
    @("last_message", $lastMessagePath), @("validation_stdout", $validationStdoutPath), @("validation_stderr", $validationStderrPath),
    @("hidden_oracle_stdout", $oracleStdoutPath), @("hidden_oracle_stderr", $oracleStderrPath), @("git_diff", $gitDiffPath),
    @("action_map_observability_json", $obsJsonPath), @("action_map_observability_md", $obsMdPath), @("action_map_observability_html", $obsHtmlPath)
)) { Add-ReportLine $report $row[0] $row[1] }
$report | Set-Content -Encoding UTF8 $reportPath

Write-Host "Report: $reportPath"
Write-Host "Observability: $obsMdPath"
Write-Host "JSONL: $jsonlPath"
Write-Host "LastMessage: $lastMessagePath"
Write-Host "Overall: $overall"
if ($overall -ne "PASS") { exit 1 }
exit 0
