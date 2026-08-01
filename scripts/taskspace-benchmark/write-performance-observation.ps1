param(
    [Parameter(Mandatory = $true)][string]$RunRoot,
    [string]$OutputDirectory = "",
    [string]$ReportBaseName = "performance-observation"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "lib/performance-observation.ps1")

$result = Write-TaskspacePerformanceObservation `
    -RunRoot $RunRoot `
    -OutputDirectory $OutputDirectory `
    -ReportBaseName $ReportBaseName

Write-Host "PerformanceObservationJson: $($result.json_path)"
Write-Host "PerformanceObservationMarkdown: $($result.markdown_path)"
Write-Host "PerformanceObservationEvents: $($result.event_log_path)"
