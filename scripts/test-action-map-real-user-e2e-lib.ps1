param(
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "action-map-real-user-e2e-lib.ps1")

if (-not $OutputDir) {
    $OutputDir = Join-Path $PSScriptRoot "..\target\test-reports\action-map-real-user-e2e-lib"
}
[void](New-Item -ItemType Directory -Force -Path $OutputDir)

function Assert-Equal($Actual, $Expected, [string]$Message) {
    if ($Actual -ne $Expected) { throw "$Message. Expected '$Expected', got '$Actual'." }
}

$results = New-Object System.Collections.Generic.List[string]

try {
    $obs = [pscustomobject]@{
        toolCalls = @(
            [pscustomobject]@{
                tool = "spawn_agent"
                status = "failed"
                promptPreview = "Inspect implementation"
                outputPreview = 'TaskSpace node `node-2` is completed; create or choose an open ready node for the subagent.'
            },
            [pscustomobject]@{
                tool = "spawn_agent"
                status = "failed"
                promptPreview = "Inspect tests"
                outputPreview = "transport error"
            }
        )
    }
    Assert-Equal (Count-FailedCollabToolCalls $obs) 2 "all failed collab calls should still be counted"
    Assert-Equal (Count-UnexpectedFailedCollabToolCalls $obs) 1 "recovered stale spawn should not count as unexpected"
    $results.Add("unexpected-failed-collab-filter: PASS")

    $report = @("# Action Map Real User E2E Lib Self-Test", "", "- overall: PASS") + ($results | ForEach-Object { "- $_" })
    $report | Set-Content -Encoding UTF8 (Join-Path $OutputDir "report.md")
    Write-Host "Report: $(Join-Path $OutputDir "report.md")"
    Write-Host "Overall: PASS"
} catch {
    $report = @("# Action Map Real User E2E Lib Self-Test", "", "- overall: FAIL", "- error: $($_.Exception.Message)") + ($results | ForEach-Object { "- $_" })
    $report | Set-Content -Encoding UTF8 (Join-Path $OutputDir "report.md")
    Write-Host "Report: $(Join-Path $OutputDir "report.md")"
    Write-Host "Overall: FAIL"
    throw
}
