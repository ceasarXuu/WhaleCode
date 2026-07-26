param(
    [string]$Model = "deepseek-v4-flash",
    [string]$Endpoint = "https://api.deepseek.com/chat/completions",
    [ValidateRange(1, 10)][int]$Repeat = 3,
    [string]$RunRoot = "",
    [int]$TimeoutSeconds = 900,
    [switch]$ForceImageBuild
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/bootstrap.ps1") -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot

function Import-DeepSeekCredential {
    if (-not [string]::IsNullOrWhiteSpace([string]$env:DEEPSEEK_API_KEY)) { return }
    $envPath = Join-Path $repoRoot ".env.local"
    if (-not (Test-Path -LiteralPath $envPath -PathType Leaf)) {
        throw "DEEPSEEK_API_KEY is missing and .env.local does not exist."
    }
    foreach ($line in Get-Content -LiteralPath $envPath) {
        if ($line -match '^\s*DEEPSEEK_API_KEY\s*=\s*(.+?)\s*$') {
            $value = $Matches[1].Trim().Trim('"').Trim("'")
            if (-not [string]::IsNullOrWhiteSpace($value)) {
                $env:DEEPSEEK_API_KEY = $value
                return
            }
        }
    }
    throw "DEEPSEEK_API_KEY is missing from .env.local."
}

$commit = (& git -C $repoRoot rev-parse HEAD).Trim()
$runId = (Get-Date).ToUniversalTime().ToString("yyyyMMdd-HHmmss-fff")
if ([string]::IsNullOrWhiteSpace($RunRoot)) {
    $RunRoot = Join-Path $repoRoot "target/r7-a2-b0-provider-wire/$commit/$runId"
} elseif (-not [IO.Path]::IsPathRooted($RunRoot)) {
    $RunRoot = Join-Path $repoRoot $RunRoot
}
$RunRoot = New-Dir $RunRoot
$contract = Read-TaskspaceContainerContract $repoRoot
$image = Resolve-TaskspaceContainerImage $repoRoot $contract -ForceBuild:$ForceImageBuild
$identity = @{
    run_id = $runId
    sample_id = "r7-a2-b0-provider-wire"
    pair_id = "probe"
    side = "right"
    logical_mode = "taskspace"
}
$environment = @{ PYTHONDONTWRITEBYTECODE = "1" }

$testResult = Invoke-TaskspaceContainerRole `
    -Role validator `
    -Image $image `
    -Contract $contract `
    -WorkspaceDir $repoRoot `
    -ArtifactDir $RunRoot `
    -Command @(
        "python3", "-m", "pytest", "-q", "-p", "no:cacheprovider",
        "/workspace/scripts/taskspace-benchmark/test_r7_a2_b0_provider_wire_probe.py",
        "/workspace/scripts/taskspace-benchmark/test_r7_a2_b0_result.py"
    ) `
    -TimeoutSeconds 180 `
    -Identity $identity `
    -Environment $environment `
    -WorkspaceReadOnly
if ($testResult.exit_code -ne 0) {
    throw "A2-B0 deterministic probe tests failed: $($testResult.stderr_path)"
}

Import-DeepSeekCredential
$secretPath = New-TaskspaceContainerSecret $RunRoot ([string]$env:DEEPSEEK_API_KEY)
try {
    $liveResult = Invoke-TaskspaceContainerRole `
        -Role agent `
        -Image $image `
        -Contract $contract `
        -WorkspaceDir $repoRoot `
        -ArtifactDir $RunRoot `
        -Command @(
            "python3",
            "/workspace/scripts/taskspace-benchmark/r7_a2_b0_provider_wire_probe.py",
            "--model", $Model,
            "--endpoint", $Endpoint,
            "--repeat", [string]$Repeat,
            "--output", "/artifacts/provider-wire-result.json",
            "--raw-dir", "/artifacts/raw-provider",
            "--repo-commit", $commit
        ) `
        -TimeoutSeconds $TimeoutSeconds `
        -Identity $identity `
        -SecretPath $secretPath `
        -Environment $environment `
        -WorkspaceReadOnly
} finally {
    Remove-TaskspaceContainerSecret $secretPath
}

$resultPath = Join-Path $RunRoot "provider-wire-result.json"
if (-not (Test-Path -LiteralPath $resultPath -PathType Leaf)) {
    throw "A2-B0 live result is missing: $resultPath"
}
$result = Get-Content -Raw -Encoding UTF8 -LiteralPath $resultPath | ConvertFrom-Json -Depth 80
$manifestPath = Join-Path $RunRoot "b0-run-manifest.json"
Write-TaskspaceContainerJson ([pscustomobject]@{
        schema_version = 1
        phase = "R7.1-A2-B0"
        status = if ($liveResult.exit_code -eq 0 -and [string]$result.decision.overall -eq "proceed") { "completed" } else { "paused" }
        generated_at = (Get-Date).ToUniversalTime().ToString("o")
        repo_commit = $commit
        model = $Model
        endpoint = $Endpoint
        repeat_per_scenario = $Repeat
        execution = "docker"
        image_ref = [string]$image.image_ref
        image_digest = [string]$image.image_digest
        docker_server_version = [string]$image.docker_server_version
        deterministic_test_exit_code = [int]$testResult.exit_code
        live_probe_exit_code = [int]$liveResult.exit_code
        result_path = $resultPath
        result_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $resultPath).Hash.ToLowerInvariant()
        raw_trace_root = Join-Path $RunRoot "raw-provider"
        container_lifecycle_path = [string]$liveResult.lifecycle_path
    }) $manifestPath

Write-Output "A2B0RunRoot: $RunRoot"
Write-Output "A2B0Manifest: $manifestPath"
Write-Output "A2B0Result: $resultPath"
Write-Output "A2B0Decision: $($result.decision.overall)"
if ($liveResult.exit_code -ne 0 -or [string]$result.decision.overall -ne "proceed") {
    exit 2
}
