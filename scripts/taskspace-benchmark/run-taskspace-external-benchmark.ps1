param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("deepswe", "terminal-bench")]
    [string]$Benchmark,
    [Parameter(Mandatory = $true)][string]$TaskDir,
    [string]$SampleId = "",
    [string]$SourceVersion = "",
    [int]$Repeats = 1,
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if ([string]::IsNullOrWhiteSpace($SourceVersion)) { throw "SourceVersion must pin the external benchmark source revision." }
if (-not $RunRoot) { $RunRoot = Join-Path ([System.IO.Path]::GetTempPath()) "whale-external-bench-runs" }
$scenarioRoot = Join-Path $RunRoot "materialized-scenarios"
$adapter = switch ($Benchmark) {
    "deepswe" { Join-Path $PSScriptRoot "adapters\deepswe-adapter.ps1" }
    "terminal-bench" { Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1" }
}
$materialized = & $adapter -TaskDir $TaskDir -OutputRoot $scenarioRoot -SampleId $SampleId -SourceVersion $SourceVersion
$scenarioDir = [string]($materialized | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
if ([string]::IsNullOrWhiteSpace($scenarioDir)) { throw "Adapter did not return a scenario_dir." }
$runner = Join-Path $repoRoot "scripts\taskspace-benchmark\run-taskspace-benchmark.ps1"
$args = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $runner, "-ScenarioPath", $scenarioDir, "-Repeats", $Repeats, "-WhaleBin", $WhaleBin, "-Model", $Model, "-RunRoot", (Join-Path $RunRoot "runs"), "-AllowNonE2Result")
if ($PlanOnly) { $args += "-PlanOnly" }
& powershell @args
