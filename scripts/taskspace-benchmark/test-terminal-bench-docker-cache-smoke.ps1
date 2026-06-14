param(
    [string]$RunRoot = "",
    [string]$BaseImage = "bash:5.2"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\terminal-bench-docker-cache-smoke" }
$runDir = Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff")
New-Item -ItemType Directory -Path $runDir -Force | Out-Null
$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { $script:failures.Add($Message) } }

docker pull $BaseImage | Out-Host
if ($LASTEXITCODE -ne 0) { throw "docker pull failed for $BaseImage" }
$baseDigest = (docker image inspect $BaseImage --format '{{index .RepoDigests 0}}')
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($baseDigest)) { throw "docker inspect did not return RepoDigest for $BaseImage" }

$taskDir = Join-Path $runDir "task"
$repoDir = Join-Path $runDir "repo"
$outputRoot = Join-Path $runDir "adapter-out"
$sourceVersion = "docker-cache-smoke-$([System.IO.Path]::GetFileName($runDir))"
New-Item -ItemType Directory -Path $taskDir, $repoDir -Force | Out-Null
@'
instruction: "Create hello.txt."
category: file-operations
'@ | Set-Content -LiteralPath (Join-Path $taskDir "task.yaml") -Encoding ASCII
@"
FROM $baseDigest
WORKDIR /app
"@ | Set-Content -LiteralPath (Join-Path $taskDir "Dockerfile") -Encoding ASCII
@"
#!/usr/bin/env bash
set -euo pipefail
echo "terminal-bench-cache-smoke"
"@ | Set-Content -LiteralPath (Join-Path $taskDir "run-tests.sh") -Encoding ASCII
"placeholder" | Set-Content -LiteralPath (Join-Path $repoDir "hello.txt") -Encoding UTF8

$adapterOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $taskDir -OutputRoot $outputRoot -SampleId "docker-cache-smoke" -SourceVersion $sourceVersion
$scenarioDir = [string]($adapterOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
Assert-True (-not [string]::IsNullOrWhiteSpace($scenarioDir)) "adapter did not return scenario_dir"
$scenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $scenarioDir "scenario.json") | ConvertFrom-Json
Assert-True ([bool]$scenario.external_benchmark.adapter_metadata.docker_image_cache.cache_eligible) "digest-pinned smoke fixture was not cache eligible"
$validator = Join-Path $scenarioDir "external-validator.ps1"

function Invoke-SmokeValidator {
    param([Parameter(Mandatory = $true)][string]$Name)
    $proofDir = Join-Path $runDir $Name
    New-Item -ItemType Directory -Path $proofDir -Force | Out-Null
    $oldCache = $env:TASKSPACE_DOCKER_IMAGE_CACHE
    $oldProof = $env:TASKSPACE_VALIDATION_ARTIFACT_DIR
    try {
        $env:TASKSPACE_DOCKER_IMAGE_CACHE = "1"
        $env:TASKSPACE_VALIDATION_ARTIFACT_DIR = $proofDir
        Push-Location $repoDir
        try { & powershell -NoProfile -ExecutionPolicy Bypass -File $validator | Tee-Object -FilePath (Join-Path $proofDir "validator.stdout.log") }
        finally { Pop-Location }
    } finally {
        if ($null -eq $oldCache) { Remove-Item Env:\TASKSPACE_DOCKER_IMAGE_CACHE -ErrorAction SilentlyContinue } else { $env:TASKSPACE_DOCKER_IMAGE_CACHE = $oldCache }
        if ($null -eq $oldProof) { Remove-Item Env:\TASKSPACE_VALIDATION_ARTIFACT_DIR -ErrorAction SilentlyContinue } else { $env:TASKSPACE_VALIDATION_ARTIFACT_DIR = $oldProof }
    }
    if ($LASTEXITCODE -ne 0) { throw "validator $Name exited $LASTEXITCODE" }
    $resultPath = Join-Path $proofDir "docker-build-result.json"
    Assert-True (Test-Path -LiteralPath $resultPath) "validator $Name did not write docker-build-result.json"
    $result = Get-Content -Raw -Encoding UTF8 -LiteralPath $resultPath | ConvertFrom-Json
    Assert-True ($result.PSObject.Properties.Name -contains "cache_lock_wait_ms") "validator $Name did not record cache lock wait"
    Assert-True (-not [string]::IsNullOrWhiteSpace([string]$result.cache_manifest_path) -and (Test-Path -LiteralPath ([string]$result.cache_manifest_path))) "validator $Name did not write docker cache manifest"
    $result
}

$first = Invoke-SmokeValidator "first"
$second = Invoke-SmokeValidator "second"
$firstStdout = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $runDir "first\validator.stdout.log")
$secondStdout = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $runDir "second\validator.stdout.log")
Assert-True (-not [bool]$first.cache_hit) "first validator run unexpectedly reported cache hit"
Assert-True ([bool]$second.cache_hit) "second validator run did not report cache hit"
Assert-True (@($second.phases | Where-Object { [string]$_.classification -eq "cache_hit" }).Count -eq 1) "second validator run did not classify build phase as cache_hit"
Assert-True ([string]$first.cache_key -eq [string]$second.cache_key) "cache key changed between smoke runs"
Assert-True ($firstStdout -match "terminal-bench-cache-smoke" -and $secondStdout -match "terminal-bench-cache-smoke") "validator smoke command output was missing"
Assert-True ($firstStdout -notmatch "No such file or directory" -and $secondStdout -notmatch "No such file or directory") "validator smoke command emitted shell file errors"

if ($failures.Count -gt 0) {
    Write-Host "Terminal-Bench Docker cache smoke: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "Terminal-Bench Docker cache smoke: PASS"
Write-Host "RunRoot: $runDir"
Write-Host "CacheImage: $($second.cache_image)"
