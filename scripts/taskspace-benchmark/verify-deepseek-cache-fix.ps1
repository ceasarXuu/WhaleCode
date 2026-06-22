param(
    [string]$Model = "deepseek-v4-flash",
    [string]$BaseUrl = "https://api.deepseek.com",
    [string]$OutputDir = "",
    [string]$ArtifactDir = "",
    [switch]$RunTaskspaceBenchmark,
    [string]$Scenario = "single-file-fast-fix",
    [int]$Repeats = 1,
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [int]$BenchmarkTimeoutSeconds = 900,
    [double]$MinOfficialSecondHitRate = 0.50,
    [double]$MinTaskspaceHitRate = 0.80,
    [double]$BaselineTaskspaceHitRate = 0.1386,
    [double]$MinTaskspaceImprovementRatio = 4.0,
    [switch]$SkipOfficialProbe,
    [switch]$AllowStaleWhaleBin
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path

function Get-NowStamp {
    (Get-Date).ToString("yyyyMMdd-HHmmss")
}

function Write-JsonFile {
    param($Value, [string]$Path)
    $parent = Split-Path -Parent $Path
    if ($parent -and -not (Test-Path -LiteralPath $parent)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    $Value | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Read-JsonFile {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $null
    }
    Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json
}

function Import-LocalEnvIfNeeded {
    param([string]$Name)
    if (-not [string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($Name))) {
        return
    }
    $envPath = Join-Path $repoRoot ".env.local"
    if (-not (Test-Path -LiteralPath $envPath -PathType Leaf)) { return }
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $envPath) {
        $trimmed = $line.Trim()
        if ([string]::IsNullOrWhiteSpace($trimmed) -or $trimmed.StartsWith("#")) { continue }
        $eq = $trimmed.IndexOf("=")
        if ($eq -lt 1) { continue }
        $key = $trimmed.Substring(0, $eq).Trim()
        if ($key -ne $Name) { continue }
        $value = $trimmed.Substring($eq + 1).Trim()
        if (($value.StartsWith('"') -and $value.EndsWith('"')) -or
            ($value.StartsWith("'") -and $value.EndsWith("'"))) {
            $value = $value.Substring(1, $value.Length - 2)
        }
        [Environment]::SetEnvironmentVariable($Name, $value, "Process")
        return
    }
}

function Get-UsageNumber {
    param($Object, [string[]]$Names)
    if ($null -eq $Object) { return $null }
    foreach ($name in $Names) {
        if ($Object.PSObject.Properties.Name -contains $name) {
            $value = $Object.$name
            if ($null -ne $value -and -not [string]::IsNullOrWhiteSpace([string]$value)) {
                try { return [int64]$value } catch { return $null }
            }
        }
    }
    $null
}

function Get-UsageStats {
    param($Usage)
    $input = Get-UsageNumber $Usage @("prompt_tokens", "input_tokens")
    $hit = Get-UsageNumber $Usage @("prompt_cache_hit_tokens", "cached_input_tokens", "cached_prompt_tokens")
    $miss = Get-UsageNumber $Usage @("prompt_cache_miss_tokens")
    if ($null -eq $hit -and $Usage -and $Usage.PSObject.Properties.Name -contains "prompt_tokens_details") {
        $hit = Get-UsageNumber $Usage.prompt_tokens_details @("cached_tokens")
    }
    if ($null -eq $hit -and $Usage -and $Usage.PSObject.Properties.Name -contains "input_tokens_details") {
        $hit = Get-UsageNumber $Usage.input_tokens_details @("cached_tokens")
    }
    if ($null -eq $miss -and $null -ne $input -and $null -ne $hit) {
        $miss = [Math]::Max(0, [int64]$input - [int64]$hit)
    }
    $denominator = if ($null -ne $hit -and $null -ne $miss) {
        [double]$hit + [double]$miss
    } elseif ($null -ne $input) {
        [double]$input
    } else {
        0.0
    }
    [pscustomobject]@{
        input_tokens = $input
        prompt_cache_hit_tokens = $hit
        prompt_cache_miss_tokens = $miss
        hit_rate = if ($denominator -gt 0 -and $null -ne $hit) {
            [Math]::Round([double]$hit / $denominator, 6)
        } else {
            $null
        }
        raw_usage = $Usage
    }
}

function New-StableProbeText {
    $lines = New-Object System.Collections.Generic.List[string]
    for ($i = 1; $i -le 260; $i++) {
        $n = $i.ToString("000")
        $lines.Add("CACHE_PROBE_STABLE_LINE_$n`: deterministic prefix content for DeepSeek context cache validation; keep this line byte-stable across requests.")
    }
    $lines -join "`n"
}

function Invoke-DeepSeekChatCompletion {
    param(
        [string]$ApiKey,
        [string]$Url,
        [string]$ModelName,
        [object[]]$Messages
    )
    $body = [ordered]@{
        model = $ModelName
        messages = @($Messages)
        temperature = 0
        max_tokens = 1
        stream = $false
    }
    $json = $body | ConvertTo-Json -Depth 20 -Compress
    Invoke-RestMethod `
        -Method Post `
        -Uri "$($Url.TrimEnd('/'))/chat/completions" `
        -Headers @{ Authorization = "Bearer $ApiKey" } `
        -ContentType "application/json" `
        -Body $json
}

function Invoke-OfficialCacheProbe {
    Import-LocalEnvIfNeeded "DEEPSEEK_API_KEY"
    $apiKey = [Environment]::GetEnvironmentVariable("DEEPSEEK_API_KEY")
    if ([string]::IsNullOrWhiteSpace($apiKey)) {
        throw "DEEPSEEK_API_KEY is required. Set it in the environment or .env.local."
    }
    $stable = New-StableProbeText
    $baseMessages = @(
        [ordered]@{ role = "system"; content = "You are a cache probe. Reply with OK." },
        [ordered]@{ role = "user"; content = "$stable`nReturn exactly OK." }
    )
    $extendedMessages = @(
        [ordered]@{ role = "system"; content = "You are a cache probe. Reply with OK." },
        [ordered]@{ role = "user"; content = "$stable`nReturn exactly OK." },
        [ordered]@{ role = "user"; content = "This is a short suffix after the identical prefix. Reply OK." }
    )
    $first = Invoke-DeepSeekChatCompletion $apiKey $BaseUrl $Model @($baseMessages)
    Start-Sleep -Milliseconds 500
    $second = Invoke-DeepSeekChatCompletion $apiKey $BaseUrl $Model @($baseMessages)
    Start-Sleep -Milliseconds 500
    $third = Invoke-DeepSeekChatCompletion $apiKey $BaseUrl $Model @($extendedMessages)
    $firstStats = Get-UsageStats $first.usage
    $secondStats = Get-UsageStats $second.usage
    $thirdStats = Get-UsageStats $third.usage
    $passed = (
        $null -ne $secondStats.hit_rate -and
        [double]$secondStats.hit_rate -ge $MinOfficialSecondHitRate -and
        $null -ne $secondStats.prompt_cache_hit_tokens
    )
    [pscustomobject]@{
        status = if ($passed) { "pass" } else { "fail" }
        model = $Model
        base_url = $BaseUrl
        method = "DeepSeek official usage.prompt_cache_hit_tokens / usage.prompt_cache_miss_tokens"
        min_second_hit_rate = $MinOfficialSecondHitRate
        first_request = $firstStats
        identical_second_request = $secondStats
        prefix_extension_third_request = $thirdStats
    }
}

function Invoke-TaskspaceBenchmarkRun {
    $runRoot = Join-Path $repoRoot ("target\deepseek-cache-fix-validation\benchmark-" + (Get-NowStamp))
    $runner = Join-Path $PSScriptRoot "run-taskspace-benchmark.ps1"
    $runnerParams = @{
        Scenario = $Scenario
        Repeats = $Repeats
        RunRoot = $runRoot
        WhaleBin = $WhaleBin
        Model = $Model
        TimeoutSeconds = $BenchmarkTimeoutSeconds
        AllowNonE2Result = $true
    }
    if ($AllowStaleWhaleBin) { $runnerParams.AllowStaleWhaleBin = $true }
    $logPath = Join-Path $OutputDir "taskspace-benchmark.stdout.log"
    $oldGitConfigCount = $env:GIT_CONFIG_COUNT
    $oldGitConfigKey0 = $env:GIT_CONFIG_KEY_0
    $oldGitConfigValue0 = $env:GIT_CONFIG_VALUE_0
    try {
        $env:GIT_CONFIG_COUNT = "1"
        $env:GIT_CONFIG_KEY_0 = "core.autocrlf"
        $env:GIT_CONFIG_VALUE_0 = "false"
        & $runner @runnerParams *>&1 | Tee-Object -FilePath $logPath
        $exit = $LASTEXITCODE
    } finally {
        if ($null -eq $oldGitConfigCount) { Remove-Item Env:\GIT_CONFIG_COUNT -ErrorAction SilentlyContinue } else { $env:GIT_CONFIG_COUNT = $oldGitConfigCount }
        if ($null -eq $oldGitConfigKey0) { Remove-Item Env:\GIT_CONFIG_KEY_0 -ErrorAction SilentlyContinue } else { $env:GIT_CONFIG_KEY_0 = $oldGitConfigKey0 }
        if ($null -eq $oldGitConfigValue0) { Remove-Item Env:\GIT_CONFIG_VALUE_0 -ErrorAction SilentlyContinue } else { $env:GIT_CONFIG_VALUE_0 = $oldGitConfigValue0 }
    }
    $latest = Get-ChildItem -LiteralPath (Join-Path $runRoot $Scenario) -Directory -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    [pscustomobject]@{
        exit_code = $exit
        run_root = $runRoot
        artifact_dir = if ($latest) { $latest.FullName } else { "" }
        stdout_log = $logPath
    }
}

function Get-ModeHitRate {
    param($TokenSummary, [string]$Mode)
    if ($null -eq $TokenSummary -or -not ($TokenSummary.PSObject.Properties.Name -contains "modes")) {
        return $null
    }
    $modeObj = $TokenSummary.modes.$Mode
    if ($null -eq $modeObj) { return $null }
    $input = Get-UsageNumber $modeObj @("input_tokens")
    $cached = Get-UsageNumber $modeObj @("cached_input_tokens")
    $uncached = Get-UsageNumber $modeObj @("uncached_input_tokens")
    $denominator = if ($null -ne $cached -and $null -ne $uncached) {
        [double]$cached + [double]$uncached
    } elseif ($null -ne $input) {
        [double]$input
    } else {
        0.0
    }
    [pscustomobject]@{
        model_request_count = Get-UsageNumber $modeObj @("model_request_count")
        input_tokens = $input
        cached_input_tokens = $cached
        uncached_input_tokens = $uncached
        hit_rate = if ($denominator -gt 0 -and $null -ne $cached) {
            [Math]::Round([double]$cached / $denominator, 6)
        } else {
            $null
        }
    }
}

function Get-SideRolloutHitRate {
    param([string]$Dir, [string]$Mode)
    $metricFiles = @(Get-ChildItem -LiteralPath $Dir -Recurse -Filter "metrics.json" -File -ErrorAction SilentlyContinue)
    foreach ($metricFile in $metricFiles) {
        $metrics = Read-JsonFile $metricFile.FullName
        if ($null -eq $metrics) { continue }
        if (-not ($metrics.PSObject.Properties.Name -contains "logical_mode")) { continue }
        if ([string]$metrics.logical_mode -ne $Mode) { continue }
        $requestPath = if ($metrics.PSObject.Properties.Name -contains "request_summary_path") {
            [string]$metrics.request_summary_path
        } else {
            Join-Path $metricFile.DirectoryName "request-summary.json"
        }
        $request = Read-JsonFile $requestPath
        if ($null -eq $request -or -not ($request.PSObject.Properties.Name -contains "rollout_trace")) {
            continue
        }
        $trace = $request.rollout_trace
        $input = Get-UsageNumber $trace @("input_tokens")
        $cached = Get-UsageNumber $trace @("cached_input_tokens")
        $output = Get-UsageNumber $trace @("output_tokens")
        if ($null -eq $input -or $null -eq $cached) { continue }
        $uncached = [Math]::Max(0, [int64]$input - [int64]$cached)
        return [pscustomobject]@{
            source = "side_request_summary_rollout_trace"
            request_summary_path = $requestPath
            metrics_path = $metricFile.FullName
            exec_exit_code = Get-UsageNumber $metrics @("exec_exit_code")
            business_success = if ($metrics.PSObject.Properties.Name -contains "business_success") { [bool]$metrics.business_success } else { $null }
            model_request_count = Get-UsageNumber $trace @("model_request_count")
            input_tokens = $input
            output_tokens = $output
            cached_input_tokens = $cached
            uncached_input_tokens = $uncached
            hit_rate = if ([double]$input -gt 0) { [Math]::Round([double]$cached / [double]$input, 6) } else { $null }
        }
    }
    $null
}

function Test-MissingModeUsage {
    param($ModeStats)
    if ($null -eq $ModeStats) { return $true }
    if (-not ($ModeStats.PSObject.Properties.Name -contains "hit_rate") -or $null -eq $ModeStats.hit_rate) { return $true }
    if (-not ($ModeStats.PSObject.Properties.Name -contains "model_request_count") -or $null -eq $ModeStats.model_request_count) { return $true }
    try { return ([int64]$ModeStats.model_request_count -eq 0) } catch { return $true }
}

function Test-TaskspaceArtifact {
    param([string]$Dir)
    if ([string]::IsNullOrWhiteSpace($Dir)) {
        return [pscustomobject]@{ status = "not_run"; artifact_dir = ""; reason = "no artifact dir supplied" }
    }
    $tokenPath = Join-Path $Dir "token-summary.json"
    $summary = Read-JsonFile $tokenPath
    if ($null -eq $summary) {
        return [pscustomobject]@{ status = "fail"; artifact_dir = $Dir; reason = "token-summary.json missing"; token_summary_path = $tokenPath }
    }
    $standard = Get-ModeHitRate $summary "standard"
    $taskspace = Get-ModeHitRate $summary "taskspace"
    if (Test-MissingModeUsage $standard) {
        $fallbackStandard = Get-SideRolloutHitRate $Dir "standard"
        if ($fallbackStandard) { $standard = $fallbackStandard }
    }
    if (Test-MissingModeUsage $taskspace) {
        $fallbackTaskspace = Get-SideRolloutHitRate $Dir "taskspace"
        if ($fallbackTaskspace) { $taskspace = $fallbackTaskspace }
    }
    $cacheTraceSummaryPath = Join-Path $Dir "provider-cache-trace-summary.json"
    $cacheTraceSummary = Read-JsonFile $cacheTraceSummaryPath
    $improvementRatio = if ($taskspace -and $null -ne $taskspace.hit_rate -and $BaselineTaskspaceHitRate -gt 0) {
        [Math]::Round([double]$taskspace.hit_rate / $BaselineTaskspaceHitRate, 4)
    } else {
        $null
    }
    $passed = (
        $taskspace -and
        $null -ne $taskspace.hit_rate -and
        [double]$taskspace.hit_rate -ge $MinTaskspaceHitRate -and
        $null -ne $improvementRatio -and
        [double]$improvementRatio -ge $MinTaskspaceImprovementRatio
    )
    [pscustomobject]@{
        status = if ($passed) { "pass" } else { "fail" }
        artifact_dir = $Dir
        token_summary_path = $tokenPath
        min_taskspace_hit_rate = $MinTaskspaceHitRate
        baseline_taskspace_hit_rate = $BaselineTaskspaceHitRate
        min_taskspace_improvement_ratio = $MinTaskspaceImprovementRatio
        standard = $standard
        taskspace = $taskspace
        provider_cache_trace_summary_path = $cacheTraceSummaryPath
        provider_cache_trace_summary = $cacheTraceSummary
        taskspace_improvement_ratio = $improvementRatio
    }
}

function Write-MarkdownReport {
    param($Result, [string]$Path)
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add("# DeepSeek Cache Fix Verification")
    $lines.Add("")
    $lines.Add("- generated_at: $($Result.generated_at)")
    $lines.Add("- status: $($Result.status)")
    $lines.Add("- model: $($Result.model)")
    $lines.Add("- official_probe: $($Result.official_probe.status)")
    $lines.Add("- taskspace_validation: $($Result.taskspace_validation.status)")
    if ($Result.official_probe -and $Result.official_probe.identical_second_request) {
        $lines.Add("- official_second_hit_rate: $($Result.official_probe.identical_second_request.hit_rate)")
        $lines.Add("- official_second_hit_tokens: $($Result.official_probe.identical_second_request.prompt_cache_hit_tokens)")
        $lines.Add("- official_second_miss_tokens: $($Result.official_probe.identical_second_request.prompt_cache_miss_tokens)")
    }
    if ($Result.taskspace_validation -and $Result.taskspace_validation.taskspace) {
        $lines.Add("- taskspace_hit_rate: $($Result.taskspace_validation.taskspace.hit_rate)")
        $lines.Add("- taskspace_cached_input_tokens: $($Result.taskspace_validation.taskspace.cached_input_tokens)")
        $lines.Add("- taskspace_uncached_input_tokens: $($Result.taskspace_validation.taskspace.uncached_input_tokens)")
        $lines.Add("- taskspace_improvement_ratio: $($Result.taskspace_validation.taskspace_improvement_ratio)")
    }
    if ($Result.taskspace_validation -and $Result.taskspace_validation.provider_cache_trace_summary) {
        $trace = $Result.taskspace_validation.provider_cache_trace_summary
        $lines.Add("- cache_trace_coverage: $($trace.trace_coverage)")
        $lines.Add("- request_2_plus_hit_rate: $($trace.request_2_plus_hit_rate)")
        $lines.Add("- native_tools_schema_hot_path_count: $($trace.native_tools_schema_hot_path_count)")
        $lines.Add("- tool_free_action_contract_count: $($trace.tool_free_action_contract_count)")
        $lines.Add("- cache_trace_summary: $($Result.taskspace_validation.provider_cache_trace_summary_path)")
    }
    $lines.Add("")
    $lines.Add("## Interpretation")
    if ($Result.status -eq "pass") {
        $lines.Add("DeepSeek official cache fields are present and TaskSpace cache-hit criteria passed.")
    } elseif ($Result.status -eq "official_only_pass") {
        $lines.Add("DeepSeek official cache fields are present. TaskSpace benchmark validation was not requested or no artifact was supplied.")
    } else {
        $lines.Add("Verification did not pass. Inspect the JSON report for failed thresholds and raw usage fields.")
    }
    $lines | Set-Content -LiteralPath $Path -Encoding UTF8
}

if (-not $OutputDir) {
    $OutputDir = Join-Path $repoRoot ("target\deepseek-cache-fix-validation\verify-" + (Get-NowStamp))
}
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

$official = [pscustomobject]@{ status = "skipped"; reason = "SkipOfficialProbe was set" }
if (-not $SkipOfficialProbe) {
    $official = Invoke-OfficialCacheProbe
}

$benchmark = $null
$artifactToAnalyze = $ArtifactDir
if ($RunTaskspaceBenchmark) {
    $benchmark = Invoke-TaskspaceBenchmarkRun
    $artifactToAnalyze = [string]$benchmark.artifact_dir
}
$taskspaceValidation = Test-TaskspaceArtifact $artifactToAnalyze

$officialPass = ([string]$official.status -in @("pass", "skipped"))
$taskspaceRequested = ($RunTaskspaceBenchmark -or -not [string]::IsNullOrWhiteSpace($ArtifactDir))
$taskspacePass = if ($taskspaceRequested) { [string]$taskspaceValidation.status -eq "pass" } else { $true }
$status = if ($officialPass -and $taskspacePass -and $taskspaceRequested) {
    "pass"
} elseif ($officialPass -and -not $taskspaceRequested) {
    "official_only_pass"
} else {
    "fail"
}

$result = [pscustomobject]@{
    schema_version = "deepseek-cache-fix-verification-v1"
    generated_at = (Get-Date).ToString("o")
    repo_root = $repoRoot
    model = $Model
    status = $status
    official_probe = $official
    benchmark_run = $benchmark
    taskspace_validation = $taskspaceValidation
}

$jsonPath = Join-Path $OutputDir "deepseek-cache-fix-verification.json"
$mdPath = Join-Path $OutputDir "deepseek-cache-fix-verification.md"
Write-JsonFile $result $jsonPath
Write-MarkdownReport $result $mdPath

Write-Host "Status: $status"
Write-Host "Report: $mdPath"
Write-Host "Json: $jsonPath"
if ($benchmark -and $benchmark.artifact_dir) { Write-Host "ArtifactDir: $($benchmark.artifact_dir)" }

if ([string]$official.status -eq "fail") { exit 2 }
if ($taskspaceRequested -and [string]$taskspaceValidation.status -ne "pass") { exit 3 }
if ($status -eq "fail") { exit 1 }
exit 0
