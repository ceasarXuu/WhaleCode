param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("deepswe", "terminal-bench")]
    [string]$Benchmark,
    [Parameter(Mandatory = $true)][string]$TaskDir,
    [string]$SampleId = "",
    [Parameter(Mandatory = $true)][string]$SourceVersion,
    [int]$Repeats = 5,
    [string]$RunRoot = "",
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 900,
    [ValidateSet("bypass", "full-auto", "workspace-write")]
    [string]$SandboxMode = "full-auto",
    [string[]]$ConfigOverride = @('model_reasoning_effort="max"'),
    [string]$AuditReviewRoot = "",
    [switch]$AllowStaleWhaleBin,
    [switch]$PlanOnly
)

$ErrorActionPreference = "Stop"
if ($Repeats -lt 5) { throw "E3 external benchmark requires Repeats >= 5." }
if ([string]::IsNullOrWhiteSpace($SourceVersion)) { throw "SourceVersion must pin the external benchmark source revision." }
if (-not $RunRoot) { $RunRoot = Join-Path ([System.IO.Path]::GetTempPath()) "whale-e3-external-bench-runs" }

$runner = Join-Path $PSScriptRoot "run-taskspace-external-benchmark.ps1"
$args = @(
    "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $runner,
    "-Benchmark", $Benchmark,
    "-TaskDir", $TaskDir,
    "-SourceVersion", $SourceVersion,
    "-Repeats", $Repeats,
    "-RunRoot", $RunRoot,
    "-WhaleBin", $WhaleBin,
    "-Model", $Model,
    "-TimeoutSeconds", $TimeoutSeconds,
    "-SandboxMode", $SandboxMode,
    "-EnableAggregate"
)
if (-not [string]::IsNullOrWhiteSpace($SampleId)) { $args += @("-SampleId", $SampleId) }
foreach ($override in @($ConfigOverride)) { $args += @("-ConfigOverride", $override) }
if (-not [string]::IsNullOrWhiteSpace($AuditReviewRoot)) { $args += @("-AuditReviewRoot", $AuditReviewRoot) }
if ($AllowStaleWhaleBin) { $args += "-AllowStaleWhaleBin" }
if ($PlanOnly) { $args += "-PlanOnly" }

Write-Host "E3ExternalMode: True"
Write-Host "MinimumRepeats: 5"
Write-Host "RequestedRepeats: $Repeats"
& powershell @args
exit $LASTEXITCODE
