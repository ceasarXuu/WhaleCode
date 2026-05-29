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

function New-Dir([string]$PathValue) { (New-Item -ItemType Directory -Force -Path $PathValue).FullName }
function Write-Text([string]$PathValue, [string]$Text) { [System.IO.File]::WriteAllText($PathValue, $Text, [System.Text.UTF8Encoding]::new($false)) }
function Count-Matches([string]$Text, [string]$Pattern) { ([regex]::Matches($Text, $Pattern)).Count }
function Get-ObjectPropertyNames($Value) {
    if ($null -eq $Value) { return @() }
    @($Value.PSObject.Properties.Name)
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
            if ($event.type -eq "thread.started" -and $event.thread_id) { return [string]$event.thread_id }
        } catch {}
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

function Get-SuccessfulTaskspaceOrdering([string]$RolloutText) {
    $firstBinding = $null
    $firstOrdinary = $null
    $pending = @{}
    foreach ($line in ($RolloutText -split "`r?`n")) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $evt = $line | ConvertFrom-Json } catch { continue }
        if (-not $firstBinding -and $evt.type -eq "event_msg" -and [string]$evt.payload.type -eq "lease_created") {
            $firstBinding = [string]$evt.timestamp
        }
        if ($evt.type -eq "response_item" -and $evt.payload.type -eq "function_call") {
            $name = [string]$evt.payload.name
            if ($name -match "^(shell_command|apply_patch|spawn_agent)$" -and $evt.payload.call_id) {
                $pending[[string]$evt.payload.call_id] = [pscustomobject]@{
                    Timestamp = [string]$evt.timestamp
                    Tool = $name
                }
            }
        }
        if (-not $firstOrdinary -and $evt.type -eq "response_item" -and $evt.payload.type -eq "function_call_output") {
            $callId = [string]$evt.payload.call_id
            if ($pending.ContainsKey($callId)) {
                $output = [string]$evt.payload.output
                if ($output -notmatch "TaskSpace (mode is active|blocked this tool call)|Call taskspace_control") {
                    $firstOrdinary = $pending[$callId]
                }
            }
        }
    }
    $before = $false
    if ($firstOrdinary -and -not $firstBinding) { $before = $true }
    elseif ($firstOrdinary -and $firstBinding) { $before = ([datetime]$firstOrdinary.Timestamp) -lt ([datetime]$firstBinding) }
    [pscustomobject]@{
        FirstBindingTimestamp = if ($firstBinding) { $firstBinding } else { "" }
        FirstOrdinaryToolTimestamp = if ($firstOrdinary) { $firstOrdinary.Timestamp } else { "" }
        FirstOrdinaryTool = if ($firstOrdinary) { $firstOrdinary.Tool } else { "" }
        OrdinaryToolBeforeBinding = $before
    }
}

function Add-ReportLine([System.Collections.Generic.List[string]]$Report, [string]$Key, $Value) {
    $Report.Add("- ${Key}: $Value")
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
$oraclePath = Join-Path $artifactDir "hidden_oracle.py"
$oracleStdoutPath = Join-Path $artifactDir "hidden-oracle.stdout.log"
$oracleStderrPath = Join-Path $artifactDir "hidden-oracle.stderr.log"
$gitDiffPath = Join-Path $artifactDir "git-diff.patch"
$reportPath = Join-Path $artifactDir "report.md"
$scriptSha256 = (Get-FileHash -Algorithm SHA256 $PSCommandPath).Hash
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
I inherited this small order-pipeline project and I need you to treat it like a real handoff. The parsing, pricing, invoice total, and summary behavior seem connected, so please inspect the README, tests, and implementation, separate product truth from broken expectations, and fix the project.

Several checks can be investigated independently before you make the final change. Please parallelize the independent parser and pricing/invoice/summary investigations where that speeds up the handoff, then integrate the findings yourself, run the relevant tests, and briefly explain the fix.
'@
Write-Text $promptPath $prompt
$forbiddenPromptTerms = "taskspace|action map|\bmap\b|\bnode\b|subagent|spawn_agent|taskspace_control"
$promptLeaksInternalConcepts = $prompt -match $forbiddenPromptTerms

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
    $obsExitCode = Invoke-RealProcess "powershell" @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $exportScript, "-RolloutPath", $rollout.FullName, "-JsonlPath", $jsonlPath, "-OutputDir", $obsDir) $repoDir $obsStdout $obsStderr 180
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
$problematicSuccessfulToolResults = Count-ProblematicSuccessfulToolResults $obs
$implementationOwnershipGap = Get-ImplementationOwnershipGap $obs $gitDiffText $toolCallArgs
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
            $combined = "$([string]$result.body)`n$([string]$result.preview)"
            if ($combined -match "pytest" -and $combined -match "(?i)\bpassed\b") { $testNodeHasPassingPytest = $true }
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
if ($promptLeaksInternalConcepts) { $failures.Add("natural user prompt leaked internal TaskSpace terms") }
if ($execExitCode -ne 0) { $failures.Add("whale exec exit code was $execExitCode") }
if ($validationExitCode -ne 0) { $failures.Add("post-run pytest exit code was $validationExitCode") }
if ($oracleExitCode -ne 0) { $failures.Add("hidden oracle exit code was $oracleExitCode") }
if ($validationStdout -notmatch "passed") { $failures.Add("pytest output did not contain a passing marker") }
if ([string]::IsNullOrWhiteSpace($gitDiffText)) { $failures.Add("repository diff is empty after the real agent run") }
if (-not $rollout) { $failures.Add("could not find the rollout for this thread") }
if ($rollout -and $obsExitCode -ne 0) { $failures.Add("observability export failed with exit code $obsExitCode") }
if ($ordering.OrdinaryToolBeforeBinding) { $failures.Add("ordinary tool succeeded before first TaskSpace binding") }
if ($mapCount -lt 1) { $failures.Add("no map was observed") }
if ($nodeCount -lt 5) { $failures.Add("map did not grow to at least 5 nodes; observed $nodeCount") }
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
if ($failedCollabToolCalls -gt 0) { $failures.Add("unexpected failed collaboration tool calls: $failedCollabToolCalls") }
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
    @("validation_exit_code", $validationExitCode), @("oracle_exit_code", $oracleExitCode), @("script_sha256", $scriptSha256),
    @("whale_sha256", $whaleSha256), @("whale_version", $whaleVersion), @("repo_head", $repoHead), @("repo_status", $repoStatus),
    @("rollout", $(if ($rollout) { $rollout.FullName } else { "" }))
)) { Add-ReportLine $report $row[0] $row[1] }
$report.Add(""); $report.Add("## Artifacts"); $report.Add("")
foreach ($row in @(@("jsonl", $jsonlPath), @("stderr", $stderrPath), @("last_message", $lastMessagePath), @("git_diff", $gitDiffPath), @("observability_json", $obsJsonPath), @("observability_md", $obsMdPath), @("observability_html", $obsHtmlPath))) { Add-ReportLine $report $row[0] $row[1] }
$report.Add("")
$report.Add("## Metrics")
$report.Add("")
foreach ($row in @(
    @("prompt_leaks_internal_concepts", $promptLeaksInternalConcepts), @("maps", $mapCount), @("nodes", $nodeCount),
    @("agents", $agentCount), @("spawn_agent_calls", $spawnAgentCount), @("subagent_results", $subagentResultCount),
    @("nodes_with_results", $nodesWithResults), @("completed_nodes", $completedNodes),
    @("blocked_tool_actions", $blockedToolActionCount),
    @("unexpected_blocked_tool_actions", $unexpectedBlockedToolActionCount),
    @("implementation_node_has_successful_edit", $implementationNodeHasSuccessfulEdit),
    @("edit_outside_implementation", $editOutsideImplementationCount),
    @("failed_taskspace_tool_results", $failedTaskspaceToolResults),
    @("unexpected_failed_taskspace_tool_results", $unexpectedFailedTaskspaceToolResults),
    @("unexpected_failed_tool_budget", $unexpectedFailedToolBudget),
    @("failed_collab_tool_calls", $failedCollabToolCalls),
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
