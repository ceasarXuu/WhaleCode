param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "adapters\external-benchmark-common.ps1")
. (Join-Path $PSScriptRoot "adapters\terminal-bench-remote-assets.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\terminal-bench-adapter-selftest" }
$runDir = Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff")
New-Item -ItemType Directory -Path $runDir -Force | Out-Null
$uvSeed = Join-Path $runDir "uv-cache-seed"
New-Item -ItemType Directory -Path $uvSeed -Force | Out-Null
"offline installer seed" | Set-Content -LiteralPath (Join-Path $uvSeed "install.sh") -Encoding ASCII
"offline archive seed" | Set-Content -LiteralPath (Join-Path $uvSeed "uv-x86_64-unknown-linux-gnu.tar.gz") -Encoding ASCII
$env:TASKSPACE_TBENCH_UV_CACHE_SOURCE = $uvSeed
$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { $script:failures.Add($Message) } }

$remoteTask = Join-Path $runDir "remote-asset"
New-Item -ItemType Directory -Path $remoteTask | Out-Null
@'
instruction: "Create hello.txt."
category: data-processing
'@ | Set-Content -LiteralPath (Join-Path $remoteTask "task.yaml") -Encoding UTF8
@'
FROM scratch
RUN curl -L -o /app/oewn.sqlite "https://huggingface.co/datasets/example/oewn.sqlite"
'@ | Set-Content -LiteralPath (Join-Path $remoteTask "Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $remoteTask "run-tests.sh") -Encoding UTF8
$remoteOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $remoteTask -OutputRoot (Join-Path $runDir "remote-out") -SampleId "remote" -SourceVersion "pinned"
$remoteScenarioDir = [string]($remoteOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$remoteScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $remoteScenarioDir "scenario.json") | ConvertFrom-Json
Assert-True (@($remoteScenario.external_benchmark.adapter_metadata.remote_assets).Count -eq 1) "remote asset URL was not recorded"
Assert-True (-not [bool]$remoteScenario.external_benchmark.validator_fidelity.e3_eligible) "remote asset scenario was E3 eligible without proof"
Assert-True ([bool]$remoteScenario.external_benchmark.adapter_metadata.e3_downgraded_until_remote_assets_proven) "remote asset downgrade metadata was not recorded"

$coveredTask = Join-Path $runDir "covered-uv-and-comment"
New-Item -ItemType Directory -Path $coveredTask | Out-Null
@'
instruction: "Create hello.txt."
category: data-processing
'@ | Set-Content -LiteralPath (Join-Path $coveredTask "task.yaml") -Encoding UTF8
@'
FROM scratch
# https://github.com/laude-institute/terminal-bench/packages
'@ | Set-Content -LiteralPath (Join-Path $coveredTask "Dockerfile") -Encoding UTF8
@'
#!/bin/sh
curl -LsSf https://astral.sh/uv/0.7.13/install.sh | sh
echo ok
'@ | Set-Content -LiteralPath (Join-Path $coveredTask "run-tests.sh") -Encoding UTF8
$coveredOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $coveredTask -OutputRoot (Join-Path $runDir "covered-out") -SampleId "covered" -SourceVersion "pinned"
$coveredScenarioDir = [string]($coveredOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$coveredScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $coveredScenarioDir "scenario.json") | ConvertFrom-Json
$coveredAssets = @($coveredScenario.external_benchmark.adapter_metadata.remote_assets)
Assert-True ($coveredAssets.Count -eq 1) "uv-covered scenario should record only the uv runtime URL, not comment URLs"
Assert-True ([string]$coveredAssets[0].asset_kind -eq "validator_dependency_cache") "uv-covered runtime dependency did not record validator dependency kind"
Assert-True (-not [bool]$coveredAssets[0].required_for_e3) "uv-covered runtime dependency should not be required as a task remote asset"
Assert-True ([string]$coveredAssets[0].injection_method -eq "covered_by_terminal_bench_uv_cache") "uv-covered runtime dependency did not record cache coverage"
Assert-True ((Test-Path -LiteralPath ([string]$coveredAssets[0].cache_path) -PathType Leaf)) "uv-covered runtime dependency did not point to a concrete cache file"
Assert-True ([int64]$coveredAssets[0].size_bytes -gt 0) "uv-covered runtime dependency did not record concrete cache size"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$coveredAssets[0].actual_sha256)) "uv-covered runtime dependency did not record actual sha"
Assert-True ([string]$coveredAssets[0].actual_sha256 -eq [string]$coveredAssets[0].expected_sha256) "uv-covered runtime dependency sha proof mismatch"
Assert-True ([bool]$coveredAssets[0].equivalence_proven) "uv-covered runtime dependency was not proof-marked"
$coveredFileSize = [int64](Get-Item -LiteralPath ([string]$coveredAssets[0].cache_path)).Length
Assert-True ([int64]$coveredAssets[0].size_bytes -eq $coveredFileSize) "uv-covered runtime dependency size did not match concrete cache file"
Assert-True (-not [bool]$coveredScenario.external_benchmark.adapter_metadata.e3_downgraded_until_remote_assets_proven) "uv-covered/comment-only scenario should not be downgraded by remote asset proof"

$dockerUvTask = Join-Path $runDir "docker-uv"
New-Item -ItemType Directory -Path $dockerUvTask | Out-Null
@'
instruction: "Create hello.txt."
category: data-processing
'@ | Set-Content -LiteralPath (Join-Path $dockerUvTask "task.yaml") -Encoding UTF8
@'
FROM scratch
RUN curl -LsSf https://astral.sh/uv/0.7.13/install.sh | sh
'@ | Set-Content -LiteralPath (Join-Path $dockerUvTask "Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $dockerUvTask "run-tests.sh") -Encoding UTF8
$dockerUvOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $dockerUvTask -OutputRoot (Join-Path $runDir "docker-uv-out") -SampleId "docker-uv" -SourceVersion "pinned"
$dockerUvScenarioDir = [string]($dockerUvOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$dockerUvScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $dockerUvScenarioDir "scenario.json") | ConvertFrom-Json
$dockerUvAssets = @($dockerUvScenario.external_benchmark.adapter_metadata.remote_assets)
Assert-True ([string]$dockerUvAssets[0].asset_kind -eq "unknown_runtime_network_dependency") "Dockerfile uv curl-pipe should be classified as build/runtime network dependency"
Assert-True ([bool]$dockerUvAssets[0].required_for_e3) "Dockerfile uv URL should not be globally covered by validator uv cache"
Assert-True ([bool]$dockerUvScenario.external_benchmark.adapter_metadata.e3_downgraded_until_remote_assets_proven) "Dockerfile uv URL should downgrade until separately proven"

$registryTask = Join-Path $runDir "registry-url"
New-Item -ItemType Directory -Path $registryTask | Out-Null
@'
instruction: "Create hello.txt."
category: data-processing
'@ | Set-Content -LiteralPath (Join-Path $registryTask "task.yaml") -Encoding UTF8
@'
FROM scratch
ARG PACKAGE_INDEX=https://github.com/laude-institute/terminal-bench/packages
'@ | Set-Content -LiteralPath (Join-Path $registryTask "Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $registryTask "run-tests.sh") -Encoding UTF8
$registryOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $registryTask -OutputRoot (Join-Path $runDir "registry-out") -SampleId "registry" -SourceVersion "pinned"
$registryScenarioDir = [string]($registryOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$registryScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $registryScenarioDir "scenario.json") | ConvertFrom-Json
$registryAssets = @($registryScenario.external_benchmark.adapter_metadata.remote_assets)
Assert-True ([string]$registryAssets[0].asset_kind -eq "registry_or_source_endpoint") "Dockerfile ARG registry URL was not classified as endpoint metadata"
Assert-True (-not [bool]$registryAssets[0].required_for_e3) "Dockerfile ARG registry URL should not be treated as file remote asset"
Assert-True (-not [bool]$registryScenario.external_benchmark.adapter_metadata.e3_downgraded_until_remote_assets_proven) "Dockerfile ARG registry URL should not downgrade remote asset proof"

$argFileTask = Join-Path $runDir "arg-file-url"
New-Item -ItemType Directory -Path $argFileTask | Out-Null
@'
instruction: "Create hello.txt."
category: data-processing
'@ | Set-Content -LiteralPath (Join-Path $argFileTask "task.yaml") -Encoding UTF8
@'
FROM scratch
ARG ASSET_URL=https://example.invalid/asset.tar.gz
RUN curl -L -o /app/asset.tar.gz "$ASSET_URL"
'@ | Set-Content -LiteralPath (Join-Path $argFileTask "Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $argFileTask "run-tests.sh") -Encoding UTF8
$argFileOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $argFileTask -OutputRoot (Join-Path $runDir "arg-file-out") -SampleId "arg-file" -SourceVersion "pinned"
$argFileScenarioDir = [string]($argFileOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$argFileScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $argFileScenarioDir "scenario.json") | ConvertFrom-Json
$argFileAssets = @($argFileScenario.external_benchmark.adapter_metadata.remote_assets)
Assert-True ([string]$argFileAssets[0].asset_kind -eq "unknown_runtime_network_dependency") "Dockerfile ARG file URL should fail closed"
Assert-True ([bool]$argFileAssets[0].required_for_e3) "Dockerfile ARG file URL should require E3 proof"
Assert-True ([bool]$argFileScenario.external_benchmark.adapter_metadata.e3_downgraded_until_remote_assets_proven) "Dockerfile ARG file URL should downgrade remote asset proof"

$unknownPipeTask = Join-Path $runDir "unknown-curl-pipe"
New-Item -ItemType Directory -Path $unknownPipeTask | Out-Null
@'
instruction: "Create hello.txt."
category: data-processing
'@ | Set-Content -LiteralPath (Join-Path $unknownPipeTask "task.yaml") -Encoding UTF8
"FROM scratch" | Set-Content -LiteralPath (Join-Path $unknownPipeTask "Dockerfile") -Encoding UTF8
"curl -LsSf https://example.invalid/install.sh | sh" | Set-Content -LiteralPath (Join-Path $unknownPipeTask "run-tests.sh") -Encoding UTF8
$unknownPipeOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $unknownPipeTask -OutputRoot (Join-Path $runDir "unknown-pipe-out") -SampleId "unknown-pipe" -SourceVersion "pinned"
$unknownPipeScenarioDir = [string]($unknownPipeOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$unknownPipeScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $unknownPipeScenarioDir "scenario.json") | ConvertFrom-Json
$unknownPipeAssets = @($unknownPipeScenario.external_benchmark.adapter_metadata.remote_assets)
Assert-True ([string]$unknownPipeAssets[0].asset_kind -eq "unknown_runtime_network_dependency") "non-uv curl-pipe should be classified as unknown runtime dependency"
Assert-True ([bool]$unknownPipeAssets[0].required_for_e3) "non-uv curl-pipe should fail closed"
Assert-True ([bool]$unknownPipeScenario.external_benchmark.adapter_metadata.e3_downgraded_until_remote_assets_proven) "non-uv curl-pipe should downgrade remote asset proof"

$cachedTask = Join-Path $runDir "cached-asset"
$cachedOut = Join-Path $runDir "cached-out"
$cachedUrl = "https://huggingface.co/datasets/example/oewn.sqlite"
$cachedBytes = [System.Text.Encoding]::UTF8.GetBytes("cached-db")
$cachedSha = Get-TerminalBenchStringSha256 "cached-db"
$cachedKey = Get-TerminalBenchStringSha256 $cachedUrl
$cachedPath = Join-Path $cachedOut (Join-Path "_asset-cache" (Join-Path "remote-cached" (Join-Path $cachedKey "oewn.sqlite")))
New-Item -ItemType Directory -Path (Split-Path -Parent $cachedPath) -Force | Out-Null
[System.IO.File]::WriteAllBytes($cachedPath, $cachedBytes)
New-Item -ItemType Directory -Path $cachedTask | Out-Null
@'
instruction: "Create hello.txt."
category: data-processing
'@ | Set-Content -LiteralPath (Join-Path $cachedTask "task.yaml") -Encoding UTF8
@"
FROM scratch
RUN curl -L -o /app/oewn.sqlite "$cachedUrl"
RUN echo "$cachedSha  /app/oewn.sqlite" | sha256sum -c -
"@ | Set-Content -LiteralPath (Join-Path $cachedTask "Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $cachedTask "run-tests.sh") -Encoding UTF8
$cachedOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $cachedTask -OutputRoot $cachedOut -SampleId "remote-cached" -SourceVersion "pinned"
$cachedScenarioDir = [string]($cachedOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$cachedScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $cachedScenarioDir "scenario.json") | ConvertFrom-Json
$cachedAsset = @($cachedScenario.external_benchmark.adapter_metadata.remote_assets)[0]
Assert-True ([bool]$cachedAsset.equivalence_proven) "cached remote asset equivalence was not proven"
Assert-True ([string]$cachedAsset.injection_method -eq "dockerfile_copy_rewrite") "cached remote asset was not injected"
Assert-True ((Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $cachedScenarioDir "fixture\Dockerfile")) -match "COPY .wra") "cached remote asset Dockerfile was not rewritten"

$inlineTask = Join-Path $runDir "inline"
New-Item -ItemType Directory -Path $inlineTask | Out-Null
@'
instruction: "Fix the inline instruction case."
category: software-engineering
'@ | Set-Content -LiteralPath (Join-Path $inlineTask "task.yaml") -Encoding UTF8
"FROM scratch" | Set-Content -LiteralPath (Join-Path $inlineTask "Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $inlineTask "run-tests.sh") -Encoding UTF8
$inlineOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $inlineTask -OutputRoot (Join-Path $runDir "inline-out") -SampleId "inline" -SourceVersion "pinned"
$inlineScenarioDir = [string]($inlineOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
Assert-True ((Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $inlineScenarioDir "prompt.txt")) -match "Fix the inline instruction case") "inline instruction was not extracted"
$parseErrors = $null
[System.Management.Automation.Language.Parser]::ParseFile((Join-Path $inlineScenarioDir "external-validator.ps1"), [ref]$null, [ref]$parseErrors) | Out-Null
Assert-True (@($parseErrors).Count -eq 0) "generated Terminal-Bench validator PowerShell did not parse"

$bomTask = Join-Path $runDir "bom-crlf-script"
New-Item -ItemType Directory -Path $bomTask | Out-Null
@'
instruction: "Create hello.txt."
category: data-processing
'@ | Set-Content -LiteralPath (Join-Path $bomTask "task.yaml") -Encoding UTF8
"FROM scratch" | Set-Content -LiteralPath (Join-Path $bomTask "Dockerfile") -Encoding UTF8
[System.IO.File]::WriteAllText((Join-Path $bomTask "run-tests.sh"), "#!/usr/bin/env bash`r`necho ok`r`n", [System.Text.UTF8Encoding]::new($true))
$bomOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $bomTask -OutputRoot (Join-Path $runDir "bom-out") -SampleId "bom" -SourceVersion "pinned"
$bomScenarioDir = [string]($bomOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$normalizedScriptPath = Join-Path $bomScenarioDir "external-validator-source\run-tests.sh"
$normalizedBytes = [System.IO.File]::ReadAllBytes($normalizedScriptPath)
$normalizedText = [System.Text.Encoding]::UTF8.GetString($normalizedBytes)
Assert-True (-not ($normalizedBytes.Length -ge 3 -and $normalizedBytes[0] -eq 0xEF -and $normalizedBytes[1] -eq 0xBB -and $normalizedBytes[2] -eq 0xBF)) "validator run-tests.sh retained UTF-8 BOM"
Assert-True ($normalizedText -notmatch "`r") "validator run-tests.sh retained CRLF/CR line endings"

$cacheTask = Join-Path $runDir "docker-cache"
New-Item -ItemType Directory -Path $cacheTask | Out-Null
@'
instruction: "Create hello.txt."
category: file-operations
'@ | Set-Content -LiteralPath (Join-Path $cacheTask "task.yaml") -Encoding UTF8
"FROM scratch" | Set-Content -LiteralPath (Join-Path $cacheTask "Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $cacheTask "run-tests.sh") -Encoding UTF8
$cacheOutputRoot = Join-Path $runDir "docker-cache-out"
$cacheOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $cacheTask -OutputRoot $cacheOutputRoot -SampleId "docker-cache" -SourceVersion "pinned"
$cacheScenarioDir = [string]($cacheOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$cacheScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $cacheScenarioDir "scenario.json") | ConvertFrom-Json
$cacheKey = [string]$cacheScenario.external_benchmark.adapter_metadata.docker_image_cache.cache_key
Assert-True (-not [string]::IsNullOrWhiteSpace($cacheKey)) "docker image cache key was not recorded"
Assert-True (-not [bool]$cacheScenario.external_benchmark.adapter_metadata.docker_image_cache.cache_eligible) "floating Dockerfile base image should not be cache eligible"
Assert-True ([string]$cacheScenario.external_benchmark.adapter_metadata.docker_image_cache.cache_bypass_reason -eq "dockerfile_base_image_not_digest_pinned") "floating Dockerfile cache bypass reason was not recorded"
$cacheValidator = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $cacheScenarioDir "external-validator.ps1")
Assert-True ($cacheValidator -match [regex]::Escape('$cacheEnabled = ([string]$env:TASKSPACE_DOCKER_IMAGE_CACHE -eq "1" -and $cacheEligible)')) "generated validator did not gate docker cache behind env opt-in and eligibility"
Assert-True ($cacheValidator -match [regex]::Escape('$cacheEligible = $false')) "generated validator did not disable cache for floating Dockerfile base image"
"FROM alpine@sha256:0000000000000000000000000000000000000000000000000000000000000000" | Set-Content -LiteralPath (Join-Path $cacheTask "Dockerfile") -Encoding UTF8
$cachePinnedOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $cacheTask -OutputRoot (Join-Path $runDir "docker-cache-pinned-out") -SampleId "docker-cache" -SourceVersion "pinned"
$cachePinnedScenarioDir = [string]($cachePinnedOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$cachePinnedScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $cachePinnedScenarioDir "scenario.json") | ConvertFrom-Json
Assert-True ([bool]$cachePinnedScenario.external_benchmark.adapter_metadata.docker_image_cache.cache_eligible) "digest-pinned Dockerfile base image should be cache eligible"
$pinnedCache = $cachePinnedScenario.external_benchmark.adapter_metadata.docker_image_cache
Assert-True ([int]$pinnedCache.schema_version -eq 2 -and [string]$pinnedCache.cache_schema_version -eq "terminal-bench-image-cache-v2") "docker cache metadata did not record v2 schema"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$pinnedCache.validator_source_sha256)) "docker cache metadata did not include validator source hash"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$pinnedCache.adapter_sha256)) "docker cache metadata did not include adapter hash"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$pinnedCache.uv_install_sha256) -and -not [string]::IsNullOrWhiteSpace([string]$pinnedCache.uv_archive_sha256)) "docker cache metadata did not include uv cache hashes"
Assert-True ([string]$pinnedCache.docker_platform -eq "default" -and [string]$pinnedCache.docker_network_mode -eq "default" -and [string]$pinnedCache.docker_build_environment_mode -eq "host-proxy-forwarded") "docker cache metadata did not include platform/network/env proof fields"
$cachePinnedValidator = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $cachePinnedScenarioDir "external-validator.ps1")
Assert-True ($cacheValidator -match [regex]::Escape('Invoke-DockerOutput -Arguments @("image", "inspect", $cacheImage)')) "generated validator did not inspect cache image before build"
Assert-True ($cachePinnedValidator -match [regex]::Escape('Invoke-Docker -Arguments @("build", "--pull", "-t", $cacheImage, $fixtureDockerPath)')) "generated validator did not build stable cache image on miss"
Assert-True ($cachePinnedValidator -match [regex]::Escape('"cache_hit"')) "generated validator did not record cache hit classification"
Assert-True ($cachePinnedValidator -match [regex]::Escape('Invoke-WithDockerCacheLock')) "generated validator did not wrap Docker cache inspect/build with cache lock"
Assert-True ($cachePinnedValidator -match [regex]::Escape('cache_lock_wait_ms = [int64]$script:TaskspaceDockerCacheLockWaitMs')) "generated validator did not record Docker cache lock wait"
Assert-True ($cachePinnedValidator -match [regex]::Escape('$cacheManifestPath = Join-Path $proofDir "docker-cache-manifest.json"')) "generated validator did not write Docker cache manifest path"
Assert-True ($cachePinnedValidator -match [regex]::Escape('validator_source_sha256 = $validatorSourceSha256')) "generated validator cache manifest did not include validator source hash"
Assert-True ($cachePinnedValidator -match [regex]::Escape('adapter_sha256 = $adapterSha256')) "generated validator cache manifest did not include adapter hash"
"echo changed" | Set-Content -LiteralPath (Join-Path $cacheTask "run-tests.sh") -Encoding UTF8
$cacheValidatorChangedOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $cacheTask -OutputRoot (Join-Path $runDir "docker-cache-validator-changed-out") -SampleId "docker-cache" -SourceVersion "pinned"
$cacheValidatorChangedScenarioDir = [string]($cacheValidatorChangedOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$cacheValidatorChangedScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $cacheValidatorChangedScenarioDir "scenario.json") | ConvertFrom-Json
Assert-True ([string]$cacheValidatorChangedScenario.external_benchmark.adapter_metadata.docker_image_cache.cache_key -ne [string]$pinnedCache.cache_key) "docker cache key did not change after validator source mutation"
"echo ok" | Set-Content -LiteralPath (Join-Path $cacheTask "run-tests.sh") -Encoding UTF8
$cacheSourceChangedOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $cacheTask -OutputRoot (Join-Path $runDir "docker-cache-source-changed-out") -SampleId "docker-cache" -SourceVersion "pinned-v2"
$cacheSourceChangedScenarioDir = [string]($cacheSourceChangedOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$cacheSourceChangedScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $cacheSourceChangedScenarioDir "scenario.json") | ConvertFrom-Json
Assert-True ([string]$cacheSourceChangedScenario.external_benchmark.adapter_metadata.docker_image_cache.cache_key -ne [string]$pinnedCache.cache_key) "docker cache key did not change after SourceVersion mutation"
"FROM scratch`nLABEL changed=true" | Set-Content -LiteralPath (Join-Path $cacheTask "Dockerfile") -Encoding UTF8
$cacheChangedOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $cacheTask -OutputRoot (Join-Path $runDir "docker-cache-changed-out") -SampleId "docker-cache" -SourceVersion "pinned"
$cacheChangedScenarioDir = [string]($cacheChangedOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$cacheChangedScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $cacheChangedScenarioDir "scenario.json") | ConvertFrom-Json
Assert-True ([string]$cacheChangedScenario.external_benchmark.adapter_metadata.docker_image_cache.cache_key -ne $cacheKey) "docker cache key did not change after Dockerfile mutation"

$foldedTask = Join-Path $runDir "folded"
New-Item -ItemType Directory -Path $foldedTask | Out-Null
@'
instruction: >
  Read one line
  then write the file.
category: file-operations
'@ | Set-Content -LiteralPath (Join-Path $foldedTask "task.yaml") -Encoding UTF8
"FROM scratch" | Set-Content -LiteralPath (Join-Path $foldedTask "Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $foldedTask "run-tests.sh") -Encoding UTF8
$foldedOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $foldedTask -OutputRoot (Join-Path $runDir "folded-out") -SampleId "folded" -SourceVersion "pinned"
$foldedScenarioDir = [string]($foldedOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
Assert-True ((Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $foldedScenarioDir "prompt.txt")) -match "Read one line then write the file") "folded instruction was not extracted"

if ($failures.Count -gt 0) {
    Write-Host "Terminal-Bench adapter self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "Terminal-Bench adapter self-test: PASS"
Write-Host "RunRoot: $runDir"
