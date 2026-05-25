param(
    [string]$Model = "deepseek-v4-pro"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RunId = Get-Date -Format "yyyyMMdd-HHmmss"
$RunRoot = Join-Path $env:TEMP "whale-deepseek-reasoning-replay-e2e-$RunId"
$EventsPath = Join-Path $RunRoot "events.jsonl"
$StderrPath = Join-Path $RunRoot "stderr.log"
$LastMessagePath = Join-Path $RunRoot "last-message.txt"
$ReportPath = Join-Path $RunRoot "report.md"

New-Item -ItemType Directory -Force -Path $RunRoot | Out-Null
Set-Content -LiteralPath (Join-Path $RunRoot "README.md") -Value "# Whale DeepSeek replay E2E`n" -Encoding UTF8

$Whale = Get-Command whale -ErrorAction Stop
if ($Whale.Source -notmatch "\\.whale\\bin\\whale\.exe$") {
    throw "Expected installed whale from ~/.whale/bin, got: $($Whale.Source)"
}

$Help = & whale exec --help
if (($Help -join [Environment]::NewLine) -notmatch "--json") {
    throw "Installed whale exec does not expose --json."
}

$Prompt = @(
    "This is a real regression test.",
    "In the same model turn, start exactly two shell commands in parallel:",
    "1. Get-Location",
    "2. Get-ChildItem",
    "After both tool results return, continue the model turn and summarize the current directory in one short sentence."
) -join "`n"

$Args = @(
    "exec",
    "--json",
    "--dangerously-bypass-approvals-and-sandbox",
    "-C", $RunRoot,
    "-m", $Model,
    "-o", $LastMessagePath,
    "-"
)

$StartedAt = Get-Date
$ProcessInfo = [System.Diagnostics.ProcessStartInfo]::new()
$ProcessInfo.FileName = $Whale.Source
$ProcessInfo.Arguments = ($Args | ForEach-Object { '"' + ($_ -replace '"', '\"') + '"' }) -join " "
$ProcessInfo.UseShellExecute = $false
$ProcessInfo.RedirectStandardInput = $true
$ProcessInfo.RedirectStandardOutput = $true
$ProcessInfo.RedirectStandardError = $true
$Process = [System.Diagnostics.Process]::new()
$Process.StartInfo = $ProcessInfo
[void]$Process.Start()
$Process.StandardInput.Write($Prompt)
$Process.StandardInput.Close()
$StdoutText = $Process.StandardOutput.ReadToEnd()
$StderrText = $Process.StandardError.ReadToEnd()
$Process.WaitForExit()
$ExitCode = $Process.ExitCode
$EndedAt = Get-Date
Set-Content -LiteralPath $EventsPath -Encoding UTF8 -Value $StdoutText
Set-Content -LiteralPath $StderrPath -Encoding UTF8 -Value $StderrText

$Failures = [System.Collections.Generic.List[string]]::new()
if ($ExitCode -ne 0) {
    $Failures.Add("whale exec exit code was $ExitCode")
}

$RawEvents = if (Test-Path -LiteralPath $EventsPath) {
    Get-Content -LiteralPath $EventsPath -Encoding UTF8
} else {
    @()
}
$Stderr = if (Test-Path -LiteralPath $StderrPath) {
    Get-Content -LiteralPath $StderrPath -Encoding UTF8
} else {
    @()
}
$CombinedText = (@($RawEvents) + @($Stderr)) -join "`n"
if ($CombinedText -match "reasoning_content|insufficient tool messages|invalid_request_error") {
    $Failures.Add("output contains API history/reasoning replay error")
}

$Events = @()
foreach ($Line in $RawEvents) {
    $Trimmed = $Line.Trim()
    if (-not $Trimmed.StartsWith("{")) {
        continue
    }
    try {
        $Events += ($Trimmed | ConvertFrom-Json)
    } catch {
        $Failures.Add("failed to parse JSON event line: $Trimmed")
    }
}

$CommandStarted = @($Events | Where-Object {
    $_.type -eq "item.started" -and $_.item.type -eq "command_execution"
})
$CommandCompleted = @($Events | Where-Object {
    $_.type -eq "item.completed" -and $_.item.type -eq "command_execution"
})
$AgentMessages = @($Events | Where-Object {
    $_.type -eq "item.completed" -and $_.item.type -eq "agent_message"
})

if ($CommandStarted.Count -lt 2) {
    $Failures.Add("expected at least two command executions to start, got $($CommandStarted.Count)")
}
if ($CommandCompleted.Count -lt 2) {
    $Failures.Add("expected at least two command executions to complete, got $($CommandCompleted.Count)")
}
if ($AgentMessages.Count -lt 1) {
    $Failures.Add("expected an agent message after tool outputs")
}

if ($CommandStarted.Count -ge 2 -and $CommandCompleted.Count -ge 1) {
    $FirstCompletionIndex = [Array]::IndexOf($Events, $CommandCompleted[0])
    $StartedBeforeFirstCompletion = @($CommandStarted | Where-Object {
        [Array]::IndexOf($Events, $_) -ge 0 -and [Array]::IndexOf($Events, $_) -lt $FirstCompletionIndex
    })
    if ($StartedBeforeFirstCompletion.Count -lt 2) {
        $Failures.Add("expected two commands to be started before the first command completed")
    }
}

if (-not (Test-Path -LiteralPath $LastMessagePath)) {
    $Failures.Add("last-message file was not written")
} else {
    $LastMessage = Get-Content -LiteralPath $LastMessagePath -Encoding UTF8 -Raw
    if ([string]::IsNullOrWhiteSpace($LastMessage)) {
        $Failures.Add("last-message file is empty")
    }
}

$Report = [System.Collections.Generic.List[string]]::new()
$Report.Add("# DeepSeek Reasoning Replay E2E")
$Report.Add("")
$Report.Add("- started_at: $($StartedAt.ToString("o"))")
$Report.Add("- ended_at: $($EndedAt.ToString("o"))")
$Report.Add("- run_root: $RunRoot")
$Report.Add("- whale: $($Whale.Source)")
$Report.Add("- model: $Model")
$Report.Add("- exit_code: $ExitCode")
$Report.Add("- command_started: $($CommandStarted.Count)")
$Report.Add("- command_completed: $($CommandCompleted.Count)")
$Report.Add("- agent_messages: $($AgentMessages.Count)")
$Report.Add("- events: $EventsPath")
$Report.Add("- stderr: $StderrPath")
$Report.Add("- last_message: $LastMessagePath")
$Report.Add("")
if ($Failures.Count -eq 0) {
    $Report.Add("status: PASS")
} else {
    $Report.Add("status: FAIL")
    $Report.Add("")
    foreach ($Failure in $Failures) {
        $Report.Add("- $Failure")
    }
}

Set-Content -LiteralPath $ReportPath -Encoding UTF8 -Value $Report
Get-Content -LiteralPath $ReportPath -Encoding UTF8

if ($Failures.Count -gt 0) {
    throw "DeepSeek reasoning replay E2E failed. Report: $ReportPath"
}
