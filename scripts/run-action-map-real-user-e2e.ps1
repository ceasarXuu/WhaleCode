param(
    [string]$ScenarioId = "action-map-real-user-cache-bugfix",
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 900,
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"

function Resolve-Dir([string]$PathValue) {
    $created = New-Item -ItemType Directory -Force -Path $PathValue
    return $created.FullName
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
    if (-not [string]::IsNullOrWhiteSpace($StdinPath)) {
        $startInfo.RedirectStandardInput = $true
    }
    $startInfo.Arguments = (($ArgumentList | ForEach-Object {
        $arg = [string]$_
        if ($arg -match '[\s"]') {
            '"' + ($arg -replace '"', '\"') + '"'
        } else {
            $arg
        }
    }) -join " ")

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    [void]$process.Start()

    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    if (-not [string]::IsNullOrWhiteSpace($StdinPath)) {
        $stdinText = Get-Content -Raw -Encoding UTF8 $StdinPath
        $process.StandardInput.Write($stdinText)
        $process.StandardInput.Close()
    }

    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        try {
            $process.Kill($true)
        }
        catch {
            $process.Kill()
        }
        throw "Process timed out after $TimeoutSeconds seconds: $FilePath $($ArgumentList -join ' ')"
    }

    $stdoutTask.Wait()
    $stderrTask.Wait()
    $stdoutTask.Result | Set-Content -Encoding UTF8 $StdoutPath
    $stderrTask.Result | Set-Content -Encoding UTF8 $StderrPath

    return $process.ExitCode
}

function Count-Matches([string]$Text, [string]$Pattern) {
    return ([regex]::Matches($Text, $Pattern)).Count
}

function Find-LatestRollout([datetime]$StartedAt, [string]$ThreadId) {
    $homeCandidates = @()
    if (-not [string]::IsNullOrWhiteSpace($env:WHALE_HOME)) {
        $homeCandidates += $env:WHALE_HOME
    }
    $homeCandidates += (Join-Path $env:USERPROFILE ".whale")

    foreach ($candidateHome in $homeCandidates | Select-Object -Unique) {
        if (-not (Test-Path $candidateHome)) {
            continue
        }
        $recent = Get-ChildItem -Path $candidateHome -Recurse -Filter "rollout-*.jsonl" -ErrorAction SilentlyContinue |
            Where-Object { $_.LastWriteTime -ge $StartedAt.AddMinutes(-2) } |
            Sort-Object LastWriteTime -Descending
        foreach ($candidate in $recent) {
            if ([string]::IsNullOrWhiteSpace($ThreadId)) {
                return $candidate
            }
            $raw = Get-Content -Raw -Encoding UTF8 $candidate.FullName -ErrorAction SilentlyContinue
            if ($raw -match [regex]::Escape($ThreadId)) {
                return $candidate
            }
        }
    }
    return $null
}

if ([string]::IsNullOrWhiteSpace($RunRoot)) {
    $RunRoot = Join-Path $PSScriptRoot "..\target\real-user-e2e"
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
$runDir = Resolve-Dir (Join-Path $RunRoot "$ScenarioId\$stamp")
$repoDir = Resolve-Dir (Join-Path $runDir "repo")
$artifactDir = Resolve-Dir (Join-Path $runDir "artifacts")
$srcDir = Resolve-Dir (Join-Path $repoDir "src")
$testDir = Resolve-Dir (Join-Path $repoDir "tests")

$promptPath = Join-Path $artifactDir "user-prompt.txt"
$jsonlPath = Join-Path $artifactDir "whale-exec.jsonl"
$stderrPath = Join-Path $artifactDir "whale-exec.stderr.log"
$lastMessagePath = Join-Path $artifactDir "last-message.md"
$validationStdoutPath = Join-Path $artifactDir "validation.stdout.log"
$validationStderrPath = Join-Path $artifactDir "validation.stderr.log"
$reportPath = Join-Path $artifactDir "report.md"

$cachePy = Join-Path $srcDir "cache.py"
$testPy = Join-Path $testDir "test_cache.py"

@'
def cache_key(namespace, key):
    return f"{namespace}:{key.lower()}"
'@ | Set-Content -Encoding UTF8 $cachePy

@'
from src.cache import cache_key

def test_cache_key_normalizes_key():
    assert cache_key("Users", "ABC") == "Users:abc"

def test_cache_key_normalizes_namespace():
    assert cache_key("Users", "ABC") == "users:abc"
'@ | Set-Content -Encoding UTF8 $testPy

@'
# Real user E2E sandbox

This repository contains a small cache key regression. The namespace should be
normalized the same way as the key.
'@ | Set-Content -Encoding UTF8 (Join-Path $repoDir "README.md")

Push-Location $repoDir
try {
    git init | Out-Null
    git config user.email "real-user-e2e@example.local" | Out-Null
    git config user.name "Real User E2E" | Out-Null
    git add . | Out-Null
    git commit -m "baseline cache regression sandbox" | Out-Null
}
finally {
    Pop-Location
}

$prompt = @"
You are working in a real local code repository. Solve the problem through the real WhaleCode workflow.

User task:
This project has a cache-key regression. Use Action Map experiment mode to organize the work. First spawn a subagent to investigate the failing boundary and relevant files. Then the main agent should decide the fix, edit code or tests, run validation, and report the commands it actually ran.

Hard requirements:
- Read the repository files before making claims.
- Spawn a real subagent for investigation.
- Make a real code or test change.
- Run a real validation command.
- Do not stop at a written plan.
"@
$prompt | Set-Content -Encoding UTF8 $promptPath

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
if (($helpText -join [Environment]::NewLine) -notmatch "--map-mode") {
    throw "Installed whale exec does not expose --map-mode. Build and install the current tree before running this real-user E2E."
}
if (($helpText -join [Environment]::NewLine) -notmatch "--map-restart") {
    throw "Installed whale exec does not expose --map-restart. Build and install the current tree before running this real-user E2E."
}

$started = Get-Date
$execArgs = @(
    "exec",
    "--json",
    "--map-mode", "experiment",
    "--map-restart",
    "-m", $Model,
    "-C", $repoDir,
    "--dangerously-bypass-approvals-and-sandbox",
    "--output-last-message", $lastMessagePath,
    "-"
)

$execExitCode = Invoke-RealProcess `
    -FilePath $WhaleBin `
    -ArgumentList $execArgs `
    -WorkingDirectory $repoDir `
    -StdoutPath $jsonlPath `
    -StderrPath $stderrPath `
    -TimeoutSeconds $TimeoutSeconds `
    -StdinPath $promptPath
$finished = Get-Date

$validationExitCode = Invoke-RealProcess `
    -FilePath "python" `
    -ArgumentList @("-c", "from src.cache import cache_key; assert cache_key('Users','ABC') == 'users:abc'; print('cache validation passed')") `
    -WorkingDirectory $repoDir `
    -StdoutPath $validationStdoutPath `
    -StderrPath $validationStderrPath `
    -TimeoutSeconds 60

$jsonlText = if (Test-Path $jsonlPath) { Get-Content -Raw -Encoding UTF8 $jsonlPath } else { "" }
$stderrText = if (Test-Path $stderrPath) { Get-Content -Raw -Encoding UTF8 $stderrPath } else { "" }
$validationStdout = if (Test-Path $validationStdoutPath) { Get-Content -Raw -Encoding UTF8 $validationStdoutPath } else { "" }
$lastMessage = if (Test-Path $lastMessagePath) { Get-Content -Raw -Encoding UTF8 $lastMessagePath } else { "" }

$threadId = ""
foreach ($line in ($jsonlText -split "`r?`n")) {
    if ([string]::IsNullOrWhiteSpace($line)) {
        continue
    }
    try {
        $event = $line | ConvertFrom-Json
        if ($event.type -eq "thread.started" -and $event.thread_id) {
            $threadId = [string]$event.thread_id
            break
        }
    }
    catch {
    }
}

$rollout = Find-LatestRollout $started $threadId
$rolloutText = ""
if ($rollout) {
    $rolloutCopy = Join-Path $artifactDir "rollout.jsonl"
    Copy-Item -LiteralPath $rollout.FullName -Destination $rolloutCopy -Force
    $rolloutText = Get-Content -Raw -Encoding UTF8 $rolloutCopy
}

$gitDiffPath = Join-Path $artifactDir "git-diff.patch"
Push-Location $repoDir
try {
    git diff -- . | Set-Content -Encoding UTF8 $gitDiffPath
}
finally {
    Pop-Location
}
$gitDiffText = if (Test-Path $gitDiffPath) { Get-Content -Raw -Encoding UTF8 $gitDiffPath } else { "" }

$observabilityHtmlPath = Join-Path $artifactDir "action-map-observability.html"
$observabilityMarkdownPath = Join-Path $artifactDir "action-map-observability.md"
$observabilityJsonPath = Join-Path $artifactDir "action-map-observability.json"
$observabilityExitCode = 0
if ($rollout) {
    $exportScript = Join-Path $PSScriptRoot "export-action-map-observability.ps1"
    & $exportScript -RolloutPath $rolloutCopy -JsonlPath $jsonlPath -OutputDir $artifactDir | Out-Host
    $observabilityExitCode = $LASTEXITCODE
}

$threadStartedCount = Count-Matches $jsonlText '"type"\s*:\s*"thread\.started"'
$turnCompletedCount = Count-Matches $jsonlText '"type"\s*:\s*"turn\.completed"'
$commandExecutionCount = Count-Matches $jsonlText '"type"\s*:\s*"command_execution"'
$fileChangeCount = Count-Matches $jsonlText '"type"\s*:\s*"file_change"'
$spawnAgentCount = Count-Matches $jsonlText '"tool"\s*:\s*"spawn_agent"'
$agentMessageCount = Count-Matches $jsonlText '"type"\s*:\s*"agent_message"'
$mapModeChangedCount = Count-Matches $rolloutText '"mode_changed"|ModeChanged|map_runtime_mode|MapRuntime'
$mapNodeCount = Count-Matches $rolloutText '"map_created"|"node_status_changed"|"lease_created"|"lease_attached"|"node_result_recorded"|"lease_released"|MapCreated|NodeStatusChanged|LeaseCreated|LeaseAttached|NodeResultRecorded|LeaseReleased'
$mapCreatedCount = Count-Matches $rolloutText '"map_created"|MapCreated'
$leaseCreatedCount = Count-Matches $rolloutText '"lease_created"|LeaseCreated'
$leaseAttachedCount = Count-Matches $rolloutText '"lease_attached"|LeaseAttached'
$mapCompletionCount = Count-Matches $rolloutText '"node_result_recorded"|"lease_released"|NodeResultRecorded|LeaseReleased'
$mapRestartShellMisuseCount = Count-Matches ($stderrText + $jsonlText) "map-restart.*not recognized|The term '/map-restart'|The term 'map-restart'"
$rolloutRecordErrorCount = Count-Matches $stderrText "failed to record rollout items"

$overall = "PASS"
$failures = New-Object System.Collections.Generic.List[string]
if ($execExitCode -ne 0) { $failures.Add("whale exec exit code was $execExitCode") }
if ($validationExitCode -ne 0) { $failures.Add("post-run validation exit code was $validationExitCode") }
if ($threadStartedCount -lt 1) { $failures.Add("no thread.started event in real exec JSONL") }
if ($turnCompletedCount -lt 1) { $failures.Add("no turn.completed event in real exec JSONL") }
if ($commandExecutionCount -lt 1) { $failures.Add("agent did not run a command") }
if ([string]::IsNullOrWhiteSpace($gitDiffText)) { $failures.Add("repository diff is empty after the real agent run") }
if ($spawnAgentCount -lt 1) { $failures.Add("agent did not call spawn_agent") }
if ($agentMessageCount -lt 1) { $failures.Add("agent did not produce an agent message") }
if ($validationStdout -notmatch "cache validation passed") { $failures.Add("cache validation marker was not printed") }
if (-not $rollout) { $failures.Add("could not find the real rollout for this thread") }
if ($rollout -and $mapModeChangedCount -lt 1) { $failures.Add("rollout does not show map runtime mode evidence") }
if ($rollout -and $mapNodeCount -lt 1) { $failures.Add("rollout does not show real map/node/lease runtime evidence") }
if ($rollout -and $mapCreatedCount -lt 1) { $failures.Add("rollout does not show map_created") }
if ($rollout -and $leaseCreatedCount -lt 1) { $failures.Add("rollout does not show lease_created") }
if ($rollout -and $leaseAttachedCount -lt 1) { $failures.Add("rollout does not show lease_attached") }
if ($rollout -and $mapCompletionCount -lt 1) { $failures.Add("rollout does not show node_result_recorded or lease_released") }
if ($mapRestartShellMisuseCount -gt 0) { $failures.Add("agent attempted to run /map-restart as a shell command") }
if ($rolloutRecordErrorCount -gt 0) { $failures.Add("runtime reported rollout persistence errors") }
if ($rollout -and $observabilityExitCode -ne 0) { $failures.Add("action map observability export failed with exit code $observabilityExitCode") }
if ($rollout -and -not (Test-Path $observabilityHtmlPath)) { $failures.Add("action map observability HTML was not generated") }
if ($failures.Count -gt 0) { $overall = "FAIL" }

$report = New-Object System.Collections.Generic.List[string]
$report.Add("# Action Map Real User E2E Report")
$report.Add("")
$report.Add("- overall: $overall")
$report.Add("- scenario_id: $ScenarioId")
$report.Add("- run_dir: $runDir")
$report.Add("- repo_dir: $repoDir")
$report.Add("- whale_bin: $WhaleBin")
$report.Add("- model: $Model")
$report.Add("- command: $WhaleBin $($execArgs -join ' ')")
$report.Add("- started: $($started.ToString("o"))")
$report.Add("- finished: $($finished.ToString("o"))")
$report.Add("- thread_id: $threadId")
$report.Add("- exec_exit_code: $execExitCode")
$report.Add("- validation_exit_code: $validationExitCode")
$report.Add("- rollout: $($rollout.FullName)")
$report.Add("")
$report.Add("## Evidence Counts")
$report.Add("")
$report.Add("- thread_started: $threadStartedCount")
$report.Add("- turn_completed: $turnCompletedCount")
$report.Add("- command_execution: $commandExecutionCount")
$report.Add("- file_change: $fileChangeCount")
$report.Add("- git_diff_bytes: $($gitDiffText.Length)")
$report.Add("- spawn_agent: $spawnAgentCount")
$report.Add("- agent_message: $agentMessageCount")
$report.Add("- map_runtime_evidence: $mapModeChangedCount")
$report.Add("- map_node_evidence: $mapNodeCount")
$report.Add("- map_created: $mapCreatedCount")
$report.Add("- lease_created: $leaseCreatedCount")
$report.Add("- lease_attached: $leaseAttachedCount")
$report.Add("- map_completion_or_release: $mapCompletionCount")
$report.Add("- map_restart_shell_misuse: $mapRestartShellMisuseCount")
$report.Add("- rollout_record_errors: $rolloutRecordErrorCount")
$report.Add("")
$report.Add("## Failures")
$report.Add("")
if ($failures.Count -eq 0) {
    $report.Add("None.")
} else {
    foreach ($failure in $failures) {
        $report.Add("- $failure")
    }
}
$report.Add("")
$report.Add("## Artifacts")
$report.Add("")
$report.Add("- prompt: $promptPath")
$report.Add("- exec_jsonl: $jsonlPath")
$report.Add("- exec_stderr: $stderrPath")
$report.Add("- last_message: $lastMessagePath")
$report.Add("- validation_stdout: $validationStdoutPath")
$report.Add("- validation_stderr: $validationStderrPath")
$report.Add("- git_diff: $gitDiffPath")
$report.Add("- action_map_observability_html: $observabilityHtmlPath")
$report.Add("- action_map_observability_md: $observabilityMarkdownPath")
$report.Add("- action_map_observability_json: $observabilityJsonPath")
$report.Add("")
$report.Add("## Last Message Preview")
$report.Add("")
if ([string]::IsNullOrWhiteSpace($lastMessage)) {
    $report.Add("(empty)")
} else {
    $preview = $lastMessage
    if ($preview.Length -gt 4000) {
        $preview = $preview.Substring(0, 4000)
    }
    $report.Add('```text')
    $report.Add($preview)
    $report.Add('```')
}
$report.Add("")
$report.Add("## Stderr Preview")
$report.Add("")
if ([string]::IsNullOrWhiteSpace($stderrText)) {
    $report.Add("(empty)")
} else {
    $preview = $stderrText
    if ($preview.Length -gt 4000) {
        $preview = $preview.Substring(0, 4000)
    }
    $report.Add('```text')
    $report.Add($preview)
    $report.Add('```')
}

$report | Set-Content -Encoding UTF8 $reportPath

Write-Host "Report: $reportPath"
Write-Host "JSONL: $jsonlPath"
Write-Host "LastMessage: $lastMessagePath"
Write-Host "Overall: $overall"

if ($overall -ne "PASS") {
    exit 1
}
exit 0
