param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "adapters\external-benchmark-common.ps1")
. (Join-Path $PSScriptRoot "adapters\terminal-bench-remote-assets.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\terminal-bench-adapter-selftest" }
$runDir = Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff")
New-Item -ItemType Directory -Path $runDir -Force | Out-Null
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
