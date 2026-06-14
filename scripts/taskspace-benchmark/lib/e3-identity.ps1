$ErrorActionPreference = "Stop"

function Get-TaskspaceFileSha256 {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) { return "" }
    if (-not (Test-Path -LiteralPath $Path)) { return "" }
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-TaskspaceStableJsonHash {
    param($Value)
    $json = $Value | ConvertTo-Json -Depth 30 -Compress
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($json)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        ([System.BitConverter]::ToString($sha.ComputeHash($bytes)) -replace "-", "").ToLowerInvariant()
    } finally {
        $sha.Dispose()
    }
}

function New-TaskspaceE3ProfileIdentity {
    param(
        [string]$Benchmark = "",
        [string]$SourceVersion = "",
        [string]$Model = "",
        [int]$Repeats = 0,
        [int]$TimeoutSeconds = 0,
        [int]$ValidationTimeoutSeconds = 0,
        [int]$ValidationPretestTimeoutSeconds = 0,
        [int]$ValidationTestTimeoutSeconds = 0,
        [string]$SandboxMode = "",
        [string[]]$ConfigOverride = @(),
        [bool]$EnableDockerImageCache = $false,
        [int]$MaxParallelSamples = 1,
        [int]$MaxParallelPairsPerSample = 1,
        [int]$MaxParallelValidationsPerPair = 1,
        [int]$MaxDockerConcurrency = 1,
        [int]$MaxModelConcurrency = 1
    )
    $profile = [ordered]@{
        benchmark = $Benchmark
        source_version = $SourceVersion
        model = $Model
        repeats = $Repeats
        timeout_seconds = $TimeoutSeconds
        validation_timeout_seconds = $ValidationTimeoutSeconds
        validation_pretest_timeout_seconds = $ValidationPretestTimeoutSeconds
        validation_test_timeout_seconds = $ValidationTestTimeoutSeconds
        sandbox_mode = $SandboxMode
        config_override = @($ConfigOverride | Sort-Object)
        enable_docker_image_cache = $EnableDockerImageCache
        max_parallel_samples = $MaxParallelSamples
        max_parallel_pairs_per_sample = $MaxParallelPairsPerSample
        max_parallel_validations_per_pair = $MaxParallelValidationsPerPair
        max_docker_concurrency = $MaxDockerConcurrency
        max_model_concurrency = $MaxModelConcurrency
    }
    [pscustomobject]@{
        schema_version = 1
        profile = $profile
        profile_hash = Get-TaskspaceStableJsonHash $profile
    }
}
