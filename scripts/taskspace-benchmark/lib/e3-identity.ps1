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
        [string[]]$ConfigOverride = @(),
        [bool]$EnableDockerImageCache = $false,
        [int]$MaxParallelSamples = 1,
        [int]$MaxParallelPairsPerSample = 1,
        [int]$MaxParallelValidationsPerPair = 1,
        [int]$MaxDockerConcurrency = 1,
        [int]$MaxModelConcurrency = 1,
        [string]$RunnerEntrypoint = "",
        [string]$RunnerScriptSha256 = "",
        [string]$ChildRunnerSha256 = "",
        [string]$TaskListSha256 = "",
        [string]$SampleSetId = "",
        [string]$RunSide = "both",
        [bool]$ScoringMode = $false
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
        execution_boundary = "docker_hard_boundary"
        config_override = @($ConfigOverride | Sort-Object)
        enable_docker_image_cache = $EnableDockerImageCache
        max_parallel_samples = $MaxParallelSamples
        max_parallel_pairs_per_sample = $MaxParallelPairsPerSample
        max_parallel_validations_per_pair = $MaxParallelValidationsPerPair
        max_docker_concurrency = $MaxDockerConcurrency
        max_model_concurrency = $MaxModelConcurrency
        runner_entrypoint = $RunnerEntrypoint
        runner_script_sha256 = $RunnerScriptSha256
        child_runner_sha256 = $ChildRunnerSha256
        task_list_sha256 = $TaskListSha256
        sample_set_id = $SampleSetId
        run_side = $RunSide
        scoring_mode = $ScoringMode
    }
    [pscustomobject]@{
        schema_version = 1
        profile = $profile
        profile_hash = Get-TaskspaceStableJsonHash $profile
    }
}

function Read-TaskspaceE3TaskList {
    param([Parameter(Mandatory = $true)][string]$Path)
    $raw = Get-Content -Raw -Encoding UTF8 -LiteralPath $Path
    if ($raw.TrimStart().StartsWith("[")) { return @($raw | ConvertFrom-Json) }
    @(Get-Content -Encoding UTF8 -LiteralPath $Path |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
        ForEach-Object { $_ | ConvertFrom-Json })
}

function Get-TaskspaceE3TaskSampleName {
    param($Task)
    if ($Task.PSObject.Properties.Name -contains "sample_id" -and -not [string]::IsNullOrWhiteSpace([string]$Task.sample_id)) {
        return [string]$Task.sample_id
    }
    if ($Task.PSObject.Properties.Name -contains "task_dir" -and -not [string]::IsNullOrWhiteSpace([string]$Task.task_dir)) {
        return Split-Path -Leaf ([string]$Task.task_dir)
    }
    ""
}

function Get-TaskspaceE3SampleSetDerivation {
    param(
        [Parameter(Mandatory = $true)][string]$Benchmark,
        [Parameter(Mandatory = $true)][string]$TaskListPath,
        [int]$Repeats = 0
    )
    $tasks = @(Read-TaskspaceE3TaskList $TaskListPath)
    $sampleNames = @($tasks | ForEach-Object { Get-TaskspaceE3TaskSampleName $_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $requiredP0 = @("processing-pipeline", "multi-source-data-merger", "recover-accuracy-log")
    $actualSorted = @($sampleNames | Sort-Object)
    $requiredSorted = @($requiredP0 | Sort-Object)
    $isFormalP0 =
        [string]$Benchmark -eq "terminal-bench" -and
        [int]$Repeats -eq 5 -and
        $sampleNames.Count -eq $requiredP0.Count -and
        (@(Compare-Object -ReferenceObject $requiredSorted -DifferenceObject $actualSorted).Count -eq 0)
    $sampleSetId = if ($isFormalP0) {
        "terminal-bench_E3-P0_3_5"
    } else {
        "$Benchmark`_E3-custom_$($sampleNames.Count)_$Repeats"
    }
    $missing = @($requiredP0 | Where-Object { $_ -notin $sampleNames })
    $unexpected = @($sampleNames | Where-Object { $_ -notin $requiredP0 })
    [pscustomobject]@{
        schema_version = 1
        sample_set_id = $sampleSetId
        formal_p0 = $isFormalP0
        benchmark = $Benchmark
        repeats = [int]$Repeats
        sample_count = [int]$sampleNames.Count
        sample_names = @($sampleNames)
        required_sample_names = @($requiredP0)
        missing_required_sample_names = @($missing)
        unexpected_sample_names = @($unexpected)
        derivation_method = "task_list_content"
        stable_code = if ($isFormalP0) { "" } else { "formal_p0_task_list_mismatch" }
    }
}
