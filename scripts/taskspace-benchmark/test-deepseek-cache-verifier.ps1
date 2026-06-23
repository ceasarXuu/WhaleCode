param(
    [string]$RunRoot = (Join-Path (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path "target\deepseek-cache-verifier-selftest")
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$failures = New-Object System.Collections.Generic.List[string]

function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { [void]$failures.Add($Message) }
}

function Write-Json {
    param($Value, [string]$Path)
    $parent = Split-Path -Parent $Path
    if ($parent -and -not (Test-Path -LiteralPath $parent)) {
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
    }
    $Value | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $Path -Encoding UTF8
}

function New-CacheVerifierFixture {
    param(
        [string]$Name,
        [double]$Request2PlusHitRate = 0.96,
        [double]$TraceCoverage = 1.0,
        [int]$CacheUsageMissingCount = 0,
        [int]$NativeToolsSchemaHotPathCount = 0,
        [int]$ToolFreeActionContractCount = 2
    )

    $dir = Join-Path $RunRoot $Name
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
    Write-Json ([pscustomobject]@{ modes = [pscustomobject]@{} }) (Join-Path $dir "token-summary.json")
    Write-Json ([pscustomobject]@{
            schema_version = "TaskSpaceProviderCacheTraceSummaryV1"
            provider_request_count = 2
            trace_coverage = $TraceCoverage
            cache_usage_missing_count = $CacheUsageMissingCount
            native_tools_schema_hot_path_count = $NativeToolsSchemaHotPathCount
            tool_free_action_contract_count = $ToolFreeActionContractCount
            unknown_or_unclassified_count = 0
            request_2_plus_count = 1
            request_2_plus_cached_input_tokens = [int64]($Request2PlusHitRate * 1000)
            request_2_plus_uncached_input_tokens = [int64]((1.0 - $Request2PlusHitRate) * 1000)
            request_2_plus_hit_rate = $Request2PlusHitRate
        }) (Join-Path $dir "provider-cache-trace-summary.json")

    $rightArtifacts = Join-Path $dir "pair-001\right\artifacts"
    New-Item -ItemType Directory -Path $rightArtifacts -Force | Out-Null
    Write-Json ([pscustomobject]@{
            logical_mode = "taskspace"
            exec_exit_code = 1
            business_success = $false
        }) (Join-Path $rightArtifacts "metrics.json")
    Write-Json ([pscustomobject]@{
            rollout_trace = [pscustomobject]@{
                model_request_count = 2
                input_tokens = 1000
                cached_input_tokens = 960
                output_tokens = 10
            }
        }) (Join-Path $rightArtifacts "request-summary.json")
    $dir
}

if (Test-Path -LiteralPath $RunRoot) {
    $resolved = (Resolve-Path -LiteralPath $RunRoot).Path
    $targetRoot = (Resolve-Path -LiteralPath (Join-Path $repoRoot "target")).Path
    if (-not $resolved.StartsWith($targetRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to clean test output outside target: $resolved"
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
New-Item -ItemType Directory -Path $RunRoot -Force | Out-Null

$verifier = Join-Path $PSScriptRoot "verify-deepseek-cache-fix.ps1"

$cachePassTaskFail = New-CacheVerifierFixture "cache-pass-task-fail"
$outPass = Join-Path $RunRoot "out-pass"
& powershell -NoProfile -ExecutionPolicy Bypass -File $verifier -SkipOfficialProbe -ArtifactDir $cachePassTaskFail -OutputDir $outPass *> $null
Assert-True ($LASTEXITCODE -eq 0) "cache verifier should pass cache-valid artifacts even when task business_success=false"
$passReport = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $outPass "deepseek-cache-fix-verification.json") | ConvertFrom-Json
Assert-True ([string]$passReport.status -eq "pass") "cache-valid artifact did not produce pass status"
Assert-True (-not ($passReport.taskspace_validation.PSObject.Properties.Name -contains "taskspace_run_success")) "cache verifier still reports taskspace_run_success as a cache gate"
Assert-True (-not ($passReport.taskspace_validation.taskspace.PSObject.Properties.Name -contains "business_success")) "cache verifier still reports business_success in taskspace cache stats"

$nativeToolsHotPath = New-CacheVerifierFixture "native-tools-hot-path" -NativeToolsSchemaHotPathCount 1
$outNative = Join-Path $RunRoot "out-native"
& powershell -NoProfile -ExecutionPolicy Bypass -File $verifier -SkipOfficialProbe -ArtifactDir $nativeToolsHotPath -OutputDir $outNative *> $null
Assert-True ($LASTEXITCODE -eq 3) "cache verifier should fail native tools schema hot-path artifacts"
$nativeReport = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $outNative "deepseek-cache-fix-verification.json") | ConvertFrom-Json
Assert-True ([string]$nativeReport.taskspace_validation.status -eq "fail") "native tools hot-path artifact did not fail taskspace cache validation"

$missingUsage = New-CacheVerifierFixture "missing-cache-usage" -CacheUsageMissingCount 1
$outMissingUsage = Join-Path $RunRoot "out-missing-usage"
& powershell -NoProfile -ExecutionPolicy Bypass -File $verifier -SkipOfficialProbe -ArtifactDir $missingUsage -OutputDir $outMissingUsage *> $null
Assert-True ($LASTEXITCODE -eq 3) "cache verifier should fail artifacts with missing provider cache usage"

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}

Write-Host "deepseek cache verifier selftest passed"
