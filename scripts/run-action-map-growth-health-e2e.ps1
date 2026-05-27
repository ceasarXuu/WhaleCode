param(
    [string]$ScenarioId = "action-map-growth-health-order-pipeline",
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 1200,
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"

function New-Dir([string]$PathValue) {
    (New-Item -ItemType Directory -Force -Path $PathValue).FullName
}

function Count-Matches([string]$Text, [string]$Pattern) {
    ([regex]::Matches($Text, $Pattern)).Count
}

function Invoke-RealProcess {
    param(
        [string]$FilePath,
        [string[]]$ArgumentList,
        [string]$WorkingDirectory,
        [string]$StdoutPath,
        [string]$StderrPath,
        [int]$TimeoutSeconds,
        [string]$StdinPath = ""
    )
    $encoding = [System.Text.UTF8Encoding]::new($false)
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FilePath
    $startInfo.WorkingDirectory = $WorkingDirectory
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.StandardOutputEncoding = $encoding
    $startInfo.StandardErrorEncoding = $encoding
    if ($StdinPath) { $startInfo.RedirectStandardInput = $true }
    $startInfo.Arguments = (($ArgumentList | ForEach-Object {
        $arg = [string]$_
        if ($arg -match '[\s"]') { '"' + ($arg -replace '"', '\"') + '"' } else { $arg }
    }) -join " ")
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    [void]$process.Start()
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    if ($StdinPath) {
        $process.StandardInput.Write((Get-Content -Raw -Encoding UTF8 $StdinPath))
        $process.StandardInput.Close()
    }
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        try { $process.Kill($true) } catch { $process.Kill() }
        throw "Process timed out after $TimeoutSeconds seconds: $FilePath $($ArgumentList -join ' ')"
    }
    $stdoutTask.Wait()
    $stderrTask.Wait()
    $stdoutTask.Result | Set-Content -Encoding UTF8 $StdoutPath
    $stderrTask.Result | Set-Content -Encoding UTF8 $StderrPath
    $process.ExitCode
}

function Get-ThreadId([string]$JsonlText) {
    foreach ($line in ($JsonlText -split "`r?`n")) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try {
            $event = $line | ConvertFrom-Json
            if ($event.type -eq "thread.started" -and $event.thread_id) {
                return [string]$event.thread_id
            }
        } catch {
        }
    }
    ""
}

function Find-LatestRollout([datetime]$StartedAt, [string]$ThreadId) {
    $homes = @()
    if ($env:WHALE_HOME) { $homes += $env:WHALE_HOME }
    $homes += (Join-Path $env:USERPROFILE ".whale")
    foreach ($candidateHome in $homes | Select-Object -Unique) {
        if (-not (Test-Path $candidateHome)) { continue }
        $recent = Get-ChildItem -Path $candidateHome -Recurse -Filter "rollout-*.jsonl" -ErrorAction SilentlyContinue |
            Where-Object { $_.LastWriteTime -ge $StartedAt.AddMinutes(-2) } |
            Sort-Object LastWriteTime -Descending
        foreach ($candidate in $recent) {
            if (-not $ThreadId) { return $candidate }
            $raw = Get-Content -Raw -Encoding UTF8 $candidate.FullName -ErrorAction SilentlyContinue
            if ($raw -match [regex]::Escape($ThreadId)) { return $candidate }
        }
    }
    $null
}

function Add-ReportLine([System.Collections.Generic.List[string]]$Report, [string]$Key, $Value) {
    $Report.Add("- ${Key}: $Value")
}

function Write-Text([string]$PathValue, [string]$Text) {
    [System.IO.File]::WriteAllText($PathValue, $Text, [System.Text.UTF8Encoding]::new($false))
}

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
You are working in a real local code repository. Solve the order pipeline regressions through TaskSpace.

User task:
This project has related regressions across parser, pricing, and invoice integration. Use TaskSpace as the work driver. I want to observe whether the task map grows healthily as the work proceeds.

Hard workflow requirements:
- Start with a concrete TaskSpace task and create a real map, not a generic plan.
- Do not use one generic "fix everything" node. Create and maintain separate nodes for boundary/repo inspection, parser investigation, pricing/integration investigation, implementation, and regression validation.
- Mark main-agent nodes complete with taskspace_control(action=finish_node) when their work is finished.
- Spawn at least two real investigation subagents, each bound to a concrete node: one for parser/tests and one for pricing/integration/tests.
- Subagents should investigate and report; the main agent decides and performs final edits.
- If a completed node cannot be reused, create a follow-up node and explain why in the node context.
- Read the repository files before making claims.
- If a test expectation conflicts with README requirements, update the test to match README instead of changing code to satisfy the incorrect test.
- Run a real failing validation before the fix and a real passing validation after the fix.
- Create and bind the validation node before the final python -m pytest tests -q command, so the validation node owns the validation command result.
- Make real code or test changes. Do not stop at a written plan.

Acceptance:
- python -m pytest tests -q should pass.
- The implementation must satisfy README requirements.
- The final answer must summarize the TaskSpace nodes that were created and how the map changed during the task.
"@
Write-Text $promptPath $prompt

if ($PlanOnly) {
    Write-Host "RunDir: $runDir"
    Write-Host "RepoDir: $repoDir"
    Write-Host "WhaleBin: $WhaleBin"
    Write-Host "Model: $Model"
    Write-Host "PromptPath: $promptPath"
    Write-Host "ReportPath: $reportPath"
    exit 0
}

if (-not (Test-Path $WhaleBin)) {
    throw "Whale binary not found: $WhaleBin"
}
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
    & $exportScript -RolloutPath $rolloutCopy -JsonlPath $jsonlPath -OutputDir $artifactDir | Out-Host
    $obsExitCode = $LASTEXITCODE
}
$obs = if (Test-Path $obsJsonPath) { Get-Content -Raw -Encoding UTF8 $obsJsonPath | ConvertFrom-Json } else { $null }
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
$nodesWithResults = if ($obs) { @($obs.nodes | Where-Object { @($_.results).Count -gt 0 }).Count } else { 0 }
$completedNodes = if ($obs) { @($obs.nodes | Where-Object { $_.status -eq "completed" }).Count } else { 0 }
$titleText = if ($obs) { (@($obs.nodes | ForEach-Object { $_.title }) -join "`n").ToLowerInvariant() } else { "" }
$hasBoundaryNode = $titleText -match "boundary|scope|repo|inspection|inspect"
$hasParserNode = $titleText -match "parser|parse|order line|sku"
$hasPricingNode = $titleText -match "pricing|discount|invoice|shipping"
$hasImplementationNode = $titleText -match "implement|fix|change|implementation"
$hasValidationNode = $titleText -match "validat|regression|test|verify"
$parserInvestigationUsed = $false; $pricingInvestigationUsed = $false; $validationNodeHasPytestResult = $false; $implementationNodeHasMainTools = $false
if ($obs) { foreach ($node in @($obs.nodes)) {
    $agentIds = @($node.agentThreads)
    $subagentResultCount = @($node.results | Where-Object { $agentIds -contains $_.sourceThreadId }).Count
    $mainToolCount = @($node.results | Where-Object { $_.kind -eq "main_tool_call" }).Count
    if ($node.title -match "(?i)parser|parse|sku" -and $subagentResultCount -gt 0) { $parserInvestigationUsed = $true }
    if ($node.title -match "(?i)pricing|discount|invoice|shipping" -and $subagentResultCount -gt 0) { $pricingInvestigationUsed = $true }
    if ($node.title -match "(?i)validat|regression|test|verify") {
        foreach ($line in ($rolloutText -split "`r?`n")) {
            if ($line -notmatch ('"nodeId"\s*:\s*"' + [regex]::Escape([string]$node.id) + '"')) { continue }
            try { $evt = $line | ConvertFrom-Json } catch { continue }
            foreach ($map in @($evt.payload.snapshot.maps)) { foreach ($result in @($map.results)) {
                if ($result.nodeId -eq $node.id -and $result.kind -eq "main_tool_call" -and $result.body -match 'call_id:\s*(call_[A-Za-z0-9_]+)') {
                    $callId = $Matches[1]
                    if ($result.body -match 'passed' -and $toolCallArgs[$callId] -match 'python -m pytest tests -q') { $validationNodeHasPytestResult = $true }
                }
            } }
        }
    }
    if ($node.title -match "(?i)implement|fix|change|implementation" -and $mainToolCount -gt 0) { $implementationNodeHasMainTools = $true }
} }
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
if ($execExitCode -ne 0) { $failures.Add("whale exec exit code was $execExitCode") }
if ($validationExitCode -ne 0) { $failures.Add("post-run pytest exit code was $validationExitCode") }
if ($oracleExitCode -ne 0) { $failures.Add("hidden oracle exit code was $oracleExitCode") }
if ($validationStdout -notmatch "passed") { $failures.Add("pytest output did not contain a passing marker") }
if ([string]::IsNullOrWhiteSpace($gitDiffText)) { $failures.Add("repository diff is empty after the real agent run") }
if (-not $rollout) { $failures.Add("could not find the rollout for this thread") }
if ($rollout -and $obsExitCode -ne 0) { $failures.Add("observability export failed with exit code $obsExitCode") }
if ($mapCount -lt 1) { $failures.Add("no map was observed") }
if ($nodeCount -lt 4) { $failures.Add("map did not grow to at least 4 nodes; observed $nodeCount") }
if ($agentCount -lt 2) { $failures.Add("expected at least 2 subagent leases; observed $agentCount agents") }
if ($spawnAgentCount -lt 2) { $failures.Add("expected at least 2 spawn_agent calls; observed $spawnAgentCount") }
if ($leaseCreatedCount -lt 2 -or $leaseAttachedCount -lt 2) { $failures.Add("expected at least 2 lease create/attach events") }
if ($leaseReleasedCount -lt 2) { $failures.Add("expected at least 2 lease releases") }
if ($nodesWithResults -lt 3) { $failures.Add("expected results on at least 3 nodes; observed $nodesWithResults") }
if ($completedNodes -lt 2) { $failures.Add("expected at least 2 completed nodes; observed $completedNodes") }
if ($finishNodeCallCount -lt 2) { $failures.Add("expected at least 2 finish_node calls; observed $finishNodeCallCount") }
if ($finishNodeUnsupportedCount -gt 0) { $failures.Add("runtime rejected finish_node as unsupported") }
if (-not $laterCreatedAfterCompletion) { $failures.Add("no follow-up node was created after an earlier node completed") }
if (-not ($hasBoundaryNode -and $hasParserNode -and $hasPricingNode -and $hasImplementationNode -and $hasValidationNode)) {
    $failures.Add("node titles did not cover boundary/parser/pricing/implementation/validation categories")
}
if (-not $parserInvestigationUsed) { $failures.Add("parser investigation node did not receive a subagent result") }
if (-not $pricingInvestigationUsed) { $failures.Add("pricing/invoice investigation node did not receive a subagent result") }
if (-not $implementationNodeHasMainTools) { $failures.Add("implementation node did not own main-agent tool results") }
if (-not $validationNodeHasPytestResult) { $failures.Add("validation node did not own a passing pytest command result") }
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
    @("validation_exit_code", $validationExitCode), @("script_sha256", $scriptSha256), @("rollout", $rollout.FullName)
)) { Add-ReportLine $report $row[0] $row[1] }
$report.Add("")
$report.Add("## Growth Health Metrics")
$report.Add("")
foreach ($row in @(
    @("maps", $mapCount), @("nodes", $nodeCount), @("agents", $agentCount),
    @("completed_nodes", $completedNodes), @("nodes_with_results", $nodesWithResults),
    @("spawn_agent", $spawnAgentCount), @("taskspace_control", $taskspaceControlCount),
    @("finish_node_calls", $finishNodeCallCount), @("lease_created", $leaseCreatedCount),
    @("lease_attached", $leaseAttachedCount), @("lease_released", $leaseReleasedCount),
    @("later_node_created_after_completion", $laterCreatedAfterCompletion),
    @("has_boundary_node", $hasBoundaryNode), @("has_parser_node", $hasParserNode),
    @("has_pricing_node", $hasPricingNode), @("has_implementation_node", $hasImplementationNode),
    @("has_validation_node", $hasValidationNode), @("parser_investigation_used", $parserInvestigationUsed),
    @("pricing_investigation_used", $pricingInvestigationUsed), @("implementation_node_has_main_tools", $implementationNodeHasMainTools),
    @("validation_node_has_pytest_result", $validationNodeHasPytestResult),
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
