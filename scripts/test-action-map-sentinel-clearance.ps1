param(
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "action-map-observability-lib.ps1")

if (-not $OutputDir) {
    $OutputDir = Join-Path $PSScriptRoot "..\target\test-reports\action-map-sentinel-clearance"
}
[void](New-Item -ItemType Directory -Force -Path $OutputDir)

function Assert-Equal($Actual, $Expected, [string]$Message) {
    if ($Actual -ne $Expected) { throw "$Message. Expected '$Expected', got '$Actual'." }
}

function Assert-Contains($Items, [string]$Expected, [string]$Message) {
    if (@($Items) -notcontains $Expected) { throw "$Message. Missing '$Expected'." }
}

function New-ClearEvent([string]$SentinelId, [string]$Action, [string]$At = "2026-06-05T00:04:00Z", [string]$TaskId = "task-1", [string]$MapId = "map-1", [string]$NodeId = "node-1", [string]$ResultId = "result-1") {
    [ordered]@{
        at = $At
        kind = "sentinel_warning_cleared"
        summary = "clear $SentinelId"
        details = [ordered]@{
            sentinelId = $SentinelId
            clearAction = $Action
            taskId = $TaskId
            mapId = $MapId
            nodeId = $NodeId
            resultId = $ResultId
            clearedBy = "main-agent"
            clearedAtMs = "4"
            clearEventIds = @("clear-trace-1")
        }
    }
}

function New-TestContext([string]$OutputDir) {
    $artifactRoot = Join-Path $OutputDir "artifact-root"
    [void](New-Item -ItemType Directory -Force -Path (Join-Path $artifactRoot "src"))
    "print('ok')" | Set-Content -LiteralPath (Join-Path $artifactRoot "src\app.py") -Encoding UTF8
    $evidencePackage = [pscustomobject]@{
        claims = @([pscustomobject]@{ id = "claim-1"; statement = "validator passed"; evidenceRefs = @([pscustomobject]@{ resultId = "result-1"; validatorRef = "pytest" }) })
        evidenceRefs = @([pscustomobject]@{ resultId = "result-1"; validatorRef = "pytest" })
        changedArtifacts = @("src/app.py")
        validatorRefs = @("pytest")
        remainingUncertainty = @()
        validity = "accepted"
        validityReason = "pytest passed"
    }
    $nodes = @{}
    Add-Or-Update-NodeResult (Ensure-Node $nodes "node-1" "Implement app" "implement_solution") "2026-06-05T00:01:00Z" "result-1" "lease-1" "thread-1" "result" "edit" "validated" $evidencePackage "map-1" "task-1"
    $cognitiveState = [pscustomobject]@{
        outputContracts = @([pscustomobject]@{ id = "contract-1"; kind = "artifact"; evidenceRefs = @([pscustomobject]@{ resultId = "result-1" }) })
        factSources = @([pscustomobject]@{ id = "source-1"; provenance = "observed_from_environment"; description = "pytest"; evidenceRefs = @([pscustomobject]@{ validatorRef = "pytest" }) })
        facts = @([pscustomobject]@{ id = "fact-1"; statement = "validator passed"; evidenceRefs = @([pscustomobject]@{ factSourceId = "source-1" }) })
        assumptions = @()
        riskNotes = @()
        successCriteria = @()
    }
    $tasks = New-Object System.Collections.Generic.List[object]
    $taskById = @{}
    [void](Ensure-Task $tasks $taskById "task-1" "Fix app" "Produce final artifact" "active" "thread-1" "map-1" @("map-1") $cognitiveState)
    $timeline = New-Object System.Collections.Generic.List[object]
    Add-TimelineEvent $timeline "2026-06-05T00:02:00Z" "cognitive_state_updated" "contract recorded" ([pscustomobject]@{})
    Add-TimelineEvent $timeline "2026-06-05T00:03:00Z" "result_validity_changed" "result accepted" ([pscustomobject]@{ resultId = "result-1" })
    [pscustomobject]@{ ArtifactRoot = $artifactRoot; Tasks = @($tasks.ToArray()); Nodes = @($nodes.Values); Timeline = @($timeline.ToArray()) }
}

function Invoke-TestAudit($Context, $Warnings, $ExtraTimeline = @()) {
    Get-CognitiveAuditSummary $Context.Tasks $Context.Nodes @($Warnings) @($Context.Timeline + @($ExtraTimeline)) $Context.ArtifactRoot
}

$results = New-Object System.Collections.Generic.List[string]

try {
    $context = New-TestContext $OutputDir
    $activeWarning = [pscustomobject]@{ at = "2026-06-05T00:01:00Z"; id = "sentinel-1"; status = "active"; taskId = "task-1"; mapId = "map-1"; resultId = "result-1"; nodeId = "node-1"; clearanceAction = ""; traceEventIds = @("trace-1") }
    $activeAudit = Invoke-TestAudit $context @($activeWarning)
    Assert-Equal ([bool]$activeAudit.hardGatePassed) $false "active sentinel should fail final artifact audit"
    Assert-Contains $activeAudit.hardGateFailures "sentinel_warning_uncleared_for_final_artifact" "active sentinel should be reported"
    $results.Add("active-sentinel-fails: PASS")

    foreach ($action in @("FixApplied", "RiskAcceptedByMainAgent", "ContractRevised")) {
        $audit = Invoke-TestAudit $context @($activeWarning) @(New-ClearEvent "sentinel-1" $action)
        Assert-Equal ([bool]$audit.hardGatePassed) $true "valid clear action $action should clear sentinel"
    }
    $results.Add("valid-clear-events-pass: PASS")

    $invalidActionAudit = Invoke-TestAudit $context @($activeWarning) @(New-ClearEvent "sentinel-1" "LooksFine")
    Assert-Equal ([bool]$invalidActionAudit.hardGatePassed) $false "invalid clear action should not clear sentinel"
    Assert-Contains $invalidActionAudit.hardGateFailures "sentinel_warning_uncleared_for_final_artifact" "invalid clear action should be reported as uncleared"
    $results.Add("invalid-clear-action-fails: PASS")

    $wrongIdAudit = Invoke-TestAudit $context @($activeWarning) @(New-ClearEvent "sentinel-other" "FixApplied")
    Assert-Equal ([bool]$wrongIdAudit.hardGatePassed) $false "clear event for another sentinel should not clear this warning"
    $results.Add("wrong-clear-id-fails: PASS")

    $wrongContextAudit = Invoke-TestAudit $context @($activeWarning) @(New-ClearEvent "sentinel-1" "FixApplied" "2026-06-05T00:04:00Z" "task-other" "map-1" "node-1" "result-1")
    Assert-Equal ([bool]$wrongContextAudit.hardGatePassed) $false "clear event with mismatched task context should not clear this warning"
    $results.Add("wrong-clear-context-fails: PASS")

    $earlyClearAudit = Invoke-TestAudit $context @($activeWarning) @(New-ClearEvent "sentinel-1" "FixApplied" "2026-06-04T23:59:00Z")
    Assert-Equal ([bool]$earlyClearAudit.hardGatePassed) $false "clear event before warning should not clear later active warning"
    $results.Add("early-clear-event-fails: PASS")

    $snapshotCleared = [pscustomobject]@{ id = "sentinel-2"; status = "cleared"; taskId = "task-1"; mapId = "map-1"; resultId = "result-1"; nodeId = "node-1"; clearanceAction = "ContractRevised"; traceEventIds = @("trace-2") }
    $snapshotAudit = Invoke-TestAudit $context @($snapshotCleared)
    Assert-Equal ([bool]$snapshotAudit.hardGatePassed) $true "snapshot-cleared sentinel with valid action should pass"
    $results.Add("snapshot-cleared-valid-action-passes: PASS")

    $snapshotClearedWithInstruction = [pscustomobject]@{
        id = "sentinel-3"
        status = "cleared"
        taskId = "task-1"
        mapId = "map-1"
        resultId = "result-1"
        nodeId = "node-1"
        clearanceAction = "Run a successful validator, revise the contract, or explicitly accept the risk before final artifact audit."
        clearAction = "FixApplied"
        traceEventIds = @("trace-3", "trace-4")
        clearedAtMs = "4"
    }
    $snapshotInstructionAudit = Invoke-TestAudit $context @($snapshotClearedWithInstruction)
    Assert-Equal ([bool]$snapshotInstructionAudit.hardGatePassed) $true "snapshot-cleared sentinel should use clearAction instead of clearance instructions"
    $results.Add("snapshot-cleared-clear-action-passes: PASS")

    $directAudit = Get-FinalArtifactAuditSummary $context.Tasks $context.Nodes @() @{"result-1" = $context.Nodes[0].results[0]} $context.ArtifactRoot
    Assert-Equal ([bool]$directAudit.hardGatePassed) $true "legacy five-argument direct final artifact audit call should keep ArtifactRoot as the fifth argument"
    Assert-Equal ([bool]$directAudit.finalArtifacts[0].artifactFound) $true "legacy direct call should still resolve final artifact under ArtifactRoot"
    $results.Add("legacy-direct-final-artifact-audit-call: PASS")

    $report = @("# Action Map Sentinel Clearance Self-Test", "", "- overall: PASS") + ($results | ForEach-Object { "- $_" })
    $report | Set-Content -Encoding UTF8 (Join-Path $OutputDir "report.md")
    Write-Host "Report: $(Join-Path $OutputDir "report.md")"
    Write-Host "Overall: PASS"
} catch {
    $report = @("# Action Map Sentinel Clearance Self-Test", "", "- overall: FAIL", "- error: $($_.Exception.Message)") + ($results | ForEach-Object { "- $_" })
    $report | Set-Content -Encoding UTF8 (Join-Path $OutputDir "report.md")
    Write-Host "Report: $(Join-Path $OutputDir "report.md")"
    Write-Host "Overall: FAIL"
    throw
}
