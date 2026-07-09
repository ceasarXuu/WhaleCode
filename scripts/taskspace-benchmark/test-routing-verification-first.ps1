param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $PSScriptRoot "lib\scenario-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\routing-decision.ps1")
. (Join-Path $PSScriptRoot "lib\pair-report.ps1")
. (Join-Path $PSScriptRoot "lib\routing-report.ps1")

if (-not $RunRoot) {
    $RunRoot = Join-Path $repoRoot "target\routing-verification-first-selftest"
}
New-Item -ItemType Directory -Path $RunRoot -Force | Out-Null
$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { [void]$script:failures.Add($Message) }
}

$manifest = Read-TaskspaceScenarioManifest $repoRoot "count-call-stack"
$routing = New-TaskspaceRoutingDecision $manifest "Fix exact CLI output format."
Assert-True ([string]$routing.recommended_mode -eq "verification_first") "count-call-stack did not route to verification_first"
Assert-True ([bool]$routing.initial_constraints.must_read_validator_first) "verification_first did not require validator-first"
$routingPrompt = New-TaskspaceRoutingPrompt $routing
Assert-True ([string]::IsNullOrEmpty($routingPrompt)) "routing prompt should remain report-only and not inject model-visible strategy"

$reportDir = Join-Path $RunRoot "verification-routing-report"
New-Item -ItemType Directory -Path (Join-Path $reportDir "pair-001\left\artifacts\vprobe") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $reportDir "pair-001\right\artifacts") -Force | Out-Null
Write-TaskspaceJson $routing (Join-Path $reportDir "routing-decision.json")
Write-TaskspaceJson ([pscustomobject]@{
        logical_mode = "standard"
        business_success = $true
        nodes = 1
        spawn_agent_calls = 0
    }) (Join-Path $reportDir "pair-001\left\artifacts\metrics.json")
Write-TaskspaceJson ([pscustomobject]@{
        logical_mode = "taskspace"
        business_success = $true
        nodes = 2
        spawn_agent_calls = 0
        public_validation_skipped = $false
    }) (Join-Path $reportDir "pair-001\right\artifacts\metrics.json")
Write-TaskspaceJson ([pscustomobject]@{
        expected_format = "CALL_STACK_DEPTH=<positive integer>"
        local_checker = "scripts/validate.py"
    }) (Join-Path $reportDir "pair-001\left\artifacts\vprobe\expected-format-decision.json")

$summary = Write-TaskspaceSuiteRoutingSummary -RunDir ([string]$reportDir)
Assert-True (Test-Path -LiteralPath (Join-Path $reportDir "suite-routing-summary.json")) "suite-routing-summary.json was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $reportDir "pair-001\pair-routing-report.md")) "pair-routing-report.md was not written"
Assert-True ([int]$summary.verification_first_expected_format_count -eq 1) "expected-format evidence was not counted"
Assert-True (@($summary.routing_mistakes).Count -eq 0) "verification-first route with checker evidence was marked as mistake"

if ($failures.Count -gt 0) {
    Write-Error ("Verification-first routing self-test failed: " + (@($failures.ToArray()) -join "; "))
    exit 1
}
Write-Host "Verification-first routing self-test: PASS"
Write-Host "RunRoot: $RunRoot"
