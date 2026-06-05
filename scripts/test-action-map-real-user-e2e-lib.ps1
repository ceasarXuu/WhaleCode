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
    $leakPattern = Get-InternalOrchestrationLeakPattern
    foreach ($leakyText in @(
            "I delegated two independent evidence tracks to parallel explorers.",
            "I used fan-out to split the work across agents.",
            "The subagent result was accepted.",
            "The action map moved node-1 into final_synthesis."
        )) {
        if ([string]::IsNullOrWhiteSpace((Get-RegexFirstMatchExcerpt $leakyText $leakPattern))) {
            throw "Expected orchestration leak to be detected in: $leakyText"
        }
    }
    Assert-Equal (Get-RegexFirstMatchExcerpt "I fixed the parser and ran tests." $leakPattern) "" "ordinary final summary should not leak"
    Assert-Equal (Get-RegexFirstMatchExcerpt "I used the README to map every function and test expectation." $leakPattern) "" "ordinary map verb should not leak"
    Assert-Equal (Get-RegexFirstMatchExcerpt "I fixed concurrent request handling and parallel test execution support." $leakPattern) "" "ordinary concurrency terms should not leak"
    $results.Add("orchestration-leak-filter: PASS")

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
                outputPreview = 'TaskSpace node `node-3` is already held by an active lease; wait for release or choose another ready node.'
            },
            [pscustomobject]@{
                tool = "spawn_agent"
                status = "failed"
                promptPreview = "Inspect follow-up"
                outputPreview = "TaskSpace blocked spawn_agent for inspect node `node-6` because a completed narrow inspect node already exists and only one follow-up inspect track is available."
            },
            [pscustomobject]@{
                tool = "spawn_agent"
                status = "failed"
                promptPreview = "Inspect tests"
                outputPreview = "transport error"
            }
        )
    }
    Assert-Equal (Count-FailedCollabToolCalls $obs) 4 "all failed collab calls should still be counted"
    Assert-Equal (Count-UnexpectedFailedCollabToolCalls $obs) 1 "recovered runtime gates should not count as unexpected"
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
