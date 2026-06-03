param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "adapters\terminal-bench-uv-cache.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\terminal-bench-uv-cache-selftest" }
$runDir = Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff")
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { $script:failures.Add($Message) } }

$cacheRoot = Join-Path $runDir "cached"
New-Item -ItemType Directory -Force -Path (Join-Path $cacheRoot "_adapter-generated\uv-cache") | Out-Null
"installer" | Set-Content -LiteralPath (Join-Path $cacheRoot "_adapter-generated\uv-cache\install.sh") -Encoding ASCII
"archive" | Set-Content -LiteralPath (Join-Path $cacheRoot "_adapter-generated\uv-cache\uv-x86_64-unknown-linux-gnu.tar.gz") -Encoding ASCII
cmd /c exit 35
$cache = New-TerminalBenchUvCache $cacheRoot
Assert-True ([bool]$cache.enabled) "existing uv cache was disabled by stale LASTEXITCODE"

$curlWrapper = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $cache.root "bin\curl")
$aptWrapper = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $cache.root "bin\apt-get")
Assert-True ($curlWrapper.Contains("astral-sh/uv/releases/download/0.7.13/*x86_64-unknown-linux-gnu")) "curl wrapper did not match installer binary URL flexibly"
Assert-True ($aptWrapper -match "/usr/bin/apt-get update") "apt wrapper did not restore real update before non-curl installs"
Assert-True ($aptWrapper -match "/usr/bin/apt-get install") "apt wrapper dropped install subcommand on fallback"
Assert-True ($aptWrapper.Contains("[ -x /usr/bin/curl ]")) "apt wrapper did not require real system curl before short-circuiting curl install"

$task = Join-Path $runDir "task"
New-Item -ItemType Directory -Force -Path $task | Out-Null
@'
instruction: "Create hello.txt."
category: file-operations
'@ | Set-Content -LiteralPath (Join-Path $task "task.yaml") -Encoding UTF8
"FROM scratch" | Set-Content -LiteralPath (Join-Path $task "Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $task "run-tests.sh") -Encoding UTF8
$adapterOut = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $task -OutputRoot (Join-Path $runDir "adapter-out") -SampleId "uv-cache" -SourceVersion "pinned"
$scenarioDir = [string]($adapterOut | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$scenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $scenarioDir "scenario.json") | ConvertFrom-Json
$validator = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $scenarioDir "external-validator.ps1")
Assert-True ($scenario.external_benchmark.adapter_metadata.validator_dependency_cache.PSObject.Properties.Name -contains "apt_get_curl_short_circuit") "uv cache metadata did not record apt shim"
Assert-True ($validator -match "uv_cache_mount") "runtime proof manifest did not record uv cache mount"
Assert-True ($validator -match "uv_archive_sha256") "runtime proof manifest did not record uv archive hash"

if ($failures.Count -gt 0) {
    Write-Host "Terminal-Bench uv cache self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "Terminal-Bench uv cache self-test: PASS"
Write-Host "RunRoot: $runDir"
