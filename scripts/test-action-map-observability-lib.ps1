param(
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "action-map-observability-lib.ps1")

if (-not $OutputDir) {
    $OutputDir = Join-Path $PSScriptRoot "..\target\test-reports\action-map-observability-lib"
}
[void](New-Item -ItemType Directory -Force -Path $OutputDir)

function Assert-Equal($Actual, $Expected, [string]$Message) {
    if ($Actual -ne $Expected) {
        throw "$Message. Expected '$Expected', got '$Actual'."
    }
}

$results = New-Object System.Collections.Generic.List[string]

try {
    $nodes = @{}
    $node = Ensure-Node $nodes "node-1" "Read source" "inspect_code_context"
    Add-Or-Update-NodeResult $node "2026-05-30T00:01:00Z" "result-1" "lease-1" "thread-1" "main_tool_call" "read"
    Add-Or-Update-NodeResult $node "2026-05-30T00:02:00Z" "result-1" "lease-1" "thread-1" "main_tool_call" "read" "Main tool call`ntool: shell_command`ncall_id: call-1`nsuccess: true`npreview:`nok"
    Assert-Equal ([string]$node.results[0].at) "2026-05-30T00:01:00Z" "result timestamp should preserve the first event time"
    Assert-Equal ([string]$node.results[0].callId) "call-1" "snapshot body should still enrich derived fields"
    $results.Add("preserve-existing-result-time: PASS")

    Add-Or-Update-NodeResult $node "" "result-2" "lease-2" "thread-1" "result" ""
    Add-Or-Update-NodeResult $node "2026-05-30T00:03:00Z" "result-2" "lease-2" "thread-1" "result" "" "done"
    Assert-Equal ([string]$node.results[1].at) "2026-05-30T00:03:00Z" "empty result timestamp should be filled later"
    $results.Add("fill-empty-result-time: PASS")

    $report = @("# Action Map Observability Lib Self-Test", "", "- overall: PASS") + ($results | ForEach-Object { "- $_" })
    $report | Set-Content -Encoding UTF8 (Join-Path $OutputDir "report.md")
    Write-Host "Report: $(Join-Path $OutputDir "report.md")"
    Write-Host "Overall: PASS"
} catch {
    $report = @("# Action Map Observability Lib Self-Test", "", "- overall: FAIL", "- error: $($_.Exception.Message)") + ($results | ForEach-Object { "- $_" })
    $report | Set-Content -Encoding UTF8 (Join-Path $OutputDir "report.md")
    Write-Host "Report: $(Join-Path $OutputDir "report.md")"
    Write-Host "Overall: FAIL"
    throw
}
