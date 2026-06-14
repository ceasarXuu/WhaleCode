$ErrorActionPreference = "Stop"

function Invoke-TaskspaceGateCommand {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$Command,
        [int]$TimeoutSeconds = 120
    )
    $started = Get-Date
    $job = Start-Job -ScriptBlock {
        param([string]$WorkDir, [string]$InnerCommand)
        Set-Location -LiteralPath $WorkDir
        $output = & powershell -NoProfile -ExecutionPolicy Bypass -Command $InnerCommand 2>&1
        [pscustomobject]@{
            exit_code = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
            output = @($output | ForEach-Object { [string]$_ })
        }
    } -ArgumentList $RepoRoot, $Command
    $timedOut = -not (Wait-Job -Job $job -Timeout $TimeoutSeconds)
    if ($timedOut) {
        Stop-Job -Job $job -ErrorAction SilentlyContinue | Out-Null
        Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
        return [pscustomobject]@{ command = $Command; exit_code = 124; timed_out = $true; duration_ms = [int64](((Get-Date) - $started).TotalMilliseconds); output_tail = @("timeout after $TimeoutSeconds seconds") }
    }
    $result = Receive-Job -Job $job | Select-Object -First 1
    Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
    [pscustomobject]@{
        command = $Command
        exit_code = if ($result) { [int]$result.exit_code } else { 1 }
        timed_out = $false
        duration_ms = [int64](((Get-Date) - $started).TotalMilliseconds)
        output_tail = @($result.output | Select-Object -Last 20)
    }
}

function New-TaskspaceE3StartGateMarkdown {
    param($Gate)
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# TaskSpace E3 Start Gate")
    $lines.Add("")
    $lines.Add("- status: $($Gate.status)")
    $lines.Add("- run_validity: $($Gate.run_validity)")
    $lines.Add("- first_failure_artifact: $($Gate.first_failure_artifact)")
    $lines.Add("")
    $lines.Add("## Gates")
    foreach ($gateRow in @($Gate.gates)) {
        $lines.Add("- $($gateRow.name): $($gateRow.status) $(if ($gateRow.reason) { '(' + $gateRow.reason + ')' } else { '' })")
    }
    $lines.Add("")
    $lines.Add("## Self Tests")
    if (@($Gate.self_tests).Count -eq 0) { $lines.Add("- skipped") } else {
        foreach ($test in @($Gate.self_tests)) { $lines.Add("- `$($test.command)`: exit=$($test.exit_code) timeout=$($test.timed_out)") }
    }
    @($lines.ToArray())
}

function Invoke-TaskspaceE3StartGate {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$BenchmarkRoot,
        [Parameter(Mandatory = $true)][string]$OutputDir,
        [string]$Scenario = "",
        [string]$ScenarioPath = "",
        [string]$RunRoot = "",
        [string]$TaskListPath = "",
        [switch]$RunSelfTests,
        [string[]]$SelfTestCommands = @(
            ".\scripts\taskspace-benchmark\test-e3-score-validity.ps1",
            ".\scripts\taskspace-benchmark\test-terminal-bench-uv-cache-harness.ps1",
            ".\scripts\taskspace-benchmark\test-e3-harness-guardrails.ps1",
            ".\scripts\taskspace-benchmark\test-e3-proof-harness.ps1",
            ".\scripts\taskspace-benchmark\test-harness.ps1"
        )
    )
    New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
    if ([string]::IsNullOrWhiteSpace($RunRoot)) { $RunRoot = Join-Path $OutputDir "runs" }
    $RunRoot = [System.IO.Path]::GetFullPath($RunRoot)
    $paths = New-Object System.Collections.Generic.List[string]
    $paths.Add($OutputDir); $paths.Add($RunRoot); $paths.Add($RepoRoot)
    if ($TaskListPath) { $paths.Add($TaskListPath) }
    $manifest = $null
    $manifestHealth = $null
    if (-not [string]::IsNullOrWhiteSpace($Scenario) -or -not [string]::IsNullOrWhiteSpace($ScenarioPath)) {
        $manifest = Read-TaskspaceScenarioManifest $RepoRoot $Scenario $ScenarioPath
        $paths.Add($manifest.ScenarioRoot)
        $manifestHealth = Get-TaskspaceHarnessHealth $manifest $RunRoot $manifest.ScenarioRoot
    }
    $diskHealth = New-TaskspaceDiskHealth @($paths.ToArray()) "e3_start_gate"
    $gates = New-Object System.Collections.Generic.List[object]
    $gates.Add([pscustomobject]@{ name = "disk_preflight"; status = [string]$diskHealth.status; reason = if ([string]$diskHealth.status -eq "pass") { "" } else { "disk_space_low" } })
    $dockerFailures = @($diskHealth.docker_storage_checks | Where-Object { [string]$_.status -eq "fail" })
    $gates.Add([pscustomobject]@{ name = "docker_storage"; status = if ($dockerFailures.Count -eq 0) { "pass" } else { "fail" }; reason = if ($dockerFailures.Count -eq 0) { "" } else { "docker_storage_low" } })
    if ($manifestHealth) {
        $pathFailures = @($manifestHealth.findings | Where-Object { [string]$_.stable_code -in @("relative_materialized_path", "path_unresolvable", "uv_cache_missing", "validator_source_missing") })
        $gates.Add([pscustomobject]@{ name = "path_contract"; status = if ($pathFailures.Count -eq 0) { "pass" } else { "fail" }; reason = if ($pathFailures.Count -eq 0) { "" } else { [string]$pathFailures[0].stable_code } })
    } else {
        $gates.Add([pscustomobject]@{ name = "path_contract"; status = "skipped"; reason = "no_scenario_manifest" })
    }
    $selfTests = @()
    if ($RunSelfTests) {
        $selfTests = @($SelfTestCommands | ForEach-Object { Invoke-TaskspaceGateCommand $RepoRoot $_ 180 })
        $gates.Add([pscustomobject]@{ name = "cheap_self_tests"; status = if (@($selfTests | Where-Object { [int]$_.exit_code -ne 0 }).Count -eq 0) { "pass" } else { "fail" }; reason = "" })
    } else {
        $gates.Add([pscustomobject]@{ name = "cheap_self_tests"; status = "skipped"; reason = "RunSelfTests not set" })
    }
    $failed = @($gates.ToArray() | Where-Object { [string]$_.status -eq "fail" })
    $jsonPath = Join-Path $OutputDir "e3-start-gate.json"
    $markdownPath = Join-Path $OutputDir "e3-start-gate.md"
    $gate = [pscustomobject]@{
        schema_version = 1
        status = if ($failed.Count -eq 0) { "pass" } else { "fail" }
        run_validity = if ($failed.Count -eq 0) { "valid" } else { "invalid_harness" }
        exit_code = if ($failed.Count -eq 0) { 0 } else { 3 }
        gates = @($gates.ToArray())
        self_tests = @($selfTests)
        disk_health = $diskHealth
        manifest_health = $manifestHealth
        first_failure_artifact = if ($failed.Count -eq 0) { "" } else { $jsonPath }
        generated_at = (Get-Date).ToString("o")
    }
    $gate | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $jsonPath -Encoding UTF8
    New-TaskspaceE3StartGateMarkdown $gate | Set-Content -LiteralPath $markdownPath -Encoding UTF8
    $gate | Add-Member -NotePropertyName json_path -NotePropertyValue $jsonPath -Force
    $gate | Add-Member -NotePropertyName markdown_path -NotePropertyValue $markdownPath -Force
    $gate
}
