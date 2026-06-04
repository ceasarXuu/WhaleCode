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

function Assert-Contains($Items, [string]$Expected, [string]$Message) {
    if (@($Items) -notcontains $Expected) {
        throw "$Message. Missing '$Expected'."
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

    $evidencePackage = [pscustomobject]@{
        claims = @([pscustomobject]@{
                id = "claim-1"
                statement = "validator passed"
                evidenceRefs = @([pscustomobject]@{ resultId = "result-3"; validatorRef = "pytest" })
            })
        evidenceRefs = @([pscustomobject]@{ resultId = "result-3"; validatorRef = "pytest" })
        changedArtifacts = @("src/app.py")
        validatorRefs = @("pytest")
        remainingUncertainty = @()
        validity = "accepted"
        validityReason = "validator passed"
    }
    Add-Or-Update-NodeResult $node "2026-05-30T00:04:00Z" "result-3" "lease-3" "thread-1" "result" "test" "validated" $evidencePackage
    Assert-Equal ([string]$node.results[2].validity) "accepted" "accepted result validity should be derived from evidence package"
    Assert-Equal ([int]$node.results[2].claimCount) 1 "accepted result claim count should be derived"
    Assert-Equal ([int]$node.results[2].evidenceRefCount) 1 "accepted result evidence count should be derived"
    $results.Add("result-evidence-package-derived-fields: PASS")

    $tasks = New-Object System.Collections.Generic.List[object]
    $taskById = @{}
    $cognitiveState = [pscustomobject]@{
        outputContracts = @([pscustomobject]@{ id = "contract-1"; kind = "artifact"; description = "write patched source"; evidenceRefs = @([pscustomobject]@{ resultId = "result-3" }) })
        factSources = @([pscustomobject]@{ id = "source-1"; provenance = "observed_from_environment"; description = "pytest output"; evidenceRefs = @([pscustomobject]@{ validatorRef = "pytest" }) })
        facts = @([pscustomobject]@{ id = "fact-1"; statement = "tests passed"; evidenceRefs = @([pscustomobject]@{ factSourceId = "source-1" }) })
        assumptions = @()
        riskNotes = @()
        successCriteria = @("validator passes")
    }
    [void](Ensure-Task $tasks $taskById "task-1" "Fix app" "Repair failing validator" "active" "thread-1" "map-1" @("map-1") $cognitiveState)
    $timeline = New-Object System.Collections.Generic.List[object]
    Add-TimelineEvent $timeline "2026-05-30T00:05:00Z" "cognitive_state_updated" "contract recorded" ([pscustomobject]@{})
    Add-TimelineEvent $timeline "2026-05-30T00:06:00Z" "result_validity_changed" "result accepted" ([pscustomobject]@{})
    $audit = Get-CognitiveAuditSummary @($tasks.ToArray()) @($nodes.Values) @() @($timeline.ToArray())
    Assert-Equal ([string]$audit.auditSchemaVersion) "taskspace-cognitive-audit-v1" "audit schema version should be explicit"
    Assert-Equal ([bool]$audit.hardGatePassed) $true "complete cognitive audit chain should pass structural gates"
    Assert-Equal ([int]$audit.metrics.outputContractCount) 1 "audit should count output contracts"
    Assert-Equal ([int]$audit.metrics.acceptedResultCount) 1 "audit should count accepted results"
    $results.Add("cognitive-audit-complete-chain: PASS")

    $leakyState = [pscustomobject]@{
        outputContracts = @([pscustomobject]@{ id = "contract-2"; kind = "artifact"; description = "write output"; evidenceRefs = @([pscustomobject]@{ resultId = "result-3" }) })
        factSources = @([pscustomobject]@{ id = "source-2"; provenance = "generated_for_test_only"; description = "self generated fixture"; evidenceRefs = @([pscustomobject]@{ resultId = "result-3" }) })
        facts = @([pscustomobject]@{ id = "fact-2"; statement = "fixture is production data"; evidenceRefs = @([pscustomobject]@{ factSourceId = "source-2" }) })
        assumptions = @()
        riskNotes = @()
        successCriteria = @()
    }
    $leakyTasks = New-Object System.Collections.Generic.List[object]
    $leakyTaskById = @{}
    [void](Ensure-Task $leakyTasks $leakyTaskById "task-2" "Bad source" "Reject generated fact" "active" "thread-1" "map-1" @("map-1") $leakyState)
    $leakyAudit = Get-CognitiveAuditSummary @($leakyTasks.ToArray()) @($nodes.Values) @() @($timeline.ToArray())
    Assert-Equal ([bool]$leakyAudit.hardGatePassed) $false "generated_for_test_only active fact source should fail audit"
    Assert-Contains $leakyAudit.hardGateFailures "self_generated_data_leakage" "generated source leakage should be reported"
    $results.Add("cognitive-audit-generated-source-leakage: PASS")

    $unsourcedState = [pscustomobject]@{
        outputContracts = @([pscustomobject]@{ id = "contract-3"; kind = "artifact"; description = "write output"; evidenceRefs = @([pscustomobject]@{ resultId = "result-3" }) })
        factSources = @([pscustomobject]@{ id = "source-3"; provenance = "observed_from_environment"; description = "validator output"; evidenceRefs = @([pscustomobject]@{ resultId = "result-3" }) })
        facts = @([pscustomobject]@{ id = "fact-3"; statement = "tests passed"; evidenceRefs = @() })
        assumptions = @()
        riskNotes = @()
        successCriteria = @()
    }
    $unsourcedTasks = New-Object System.Collections.Generic.List[object]
    $unsourcedTaskById = @{}
    [void](Ensure-Task $unsourcedTasks $unsourcedTaskById "task-3" "Unsourced fact" "Reject unsourced fact" "active" "thread-1" "map-1" @("map-1") $unsourcedState)
    $unsourcedAudit = Get-CognitiveAuditSummary @($unsourcedTasks.ToArray()) @($nodes.Values) @() @($timeline.ToArray())
    Assert-Equal ([bool]$unsourcedAudit.hardGatePassed) $false "active fact without evidence refs should fail audit"
    Assert-Contains $unsourcedAudit.hardGateFailures "active_fact_source_missing" "unsourced active fact should be reported"
    $results.Add("cognitive-audit-unsourced-active-fact: PASS")

    $unknownSourceState = [pscustomobject]@{
        outputContracts = @([pscustomobject]@{ id = "contract-4"; kind = "artifact"; description = "write output"; evidenceRefs = @([pscustomobject]@{ resultId = "result-3" }) })
        factSources = @([pscustomobject]@{ id = "source-4"; provenance = "observed_from_environment"; description = "validator output"; evidenceRefs = @([pscustomobject]@{ resultId = "result-3" }) })
        facts = @([pscustomobject]@{ id = "fact-4"; statement = "tests passed"; evidenceRefs = @([pscustomobject]@{ factSourceId = "missing-source" }) })
        assumptions = @()
        riskNotes = @()
        successCriteria = @()
    }
    $unknownSourceTasks = New-Object System.Collections.Generic.List[object]
    $unknownSourceTaskById = @{}
    [void](Ensure-Task $unknownSourceTasks $unknownSourceTaskById "task-4" "Unknown source" "Reject missing source" "active" "thread-1" "map-1" @("map-1") $unknownSourceState)
    $unknownSourceAudit = Get-CognitiveAuditSummary @($unknownSourceTasks.ToArray()) @($nodes.Values) @() @($timeline.ToArray())
    Assert-Equal ([bool]$unknownSourceAudit.hardGatePassed) $false "active fact with unknown factSourceId should fail audit"
    Assert-Contains $unknownSourceAudit.hardGateFailures "active_fact_source_missing" "unknown fact source should be reported"
    $results.Add("cognitive-audit-unknown-active-fact-source: PASS")

    $crossTaskAState = [pscustomobject]@{
        outputContracts = @([pscustomobject]@{ id = "contract-5"; kind = "artifact"; description = "write output"; evidenceRefs = @([pscustomobject]@{ resultId = "result-3" }) })
        factSources = @()
        facts = @([pscustomobject]@{ id = "fact-5"; statement = "tests passed"; evidenceRefs = @([pscustomobject]@{ factSourceId = "shared-source" }) })
        assumptions = @()
        riskNotes = @()
        successCriteria = @()
    }
    $crossTaskBState = [pscustomobject]@{
        outputContracts = @()
        factSources = @([pscustomobject]@{ id = "shared-source"; provenance = "observed_from_environment"; description = "belongs to another task"; evidenceRefs = @([pscustomobject]@{ resultId = "result-3" }) })
        facts = @()
        assumptions = @()
        riskNotes = @()
        successCriteria = @()
    }
    $crossTasks = New-Object System.Collections.Generic.List[object]
    $crossTaskById = @{}
    [void](Ensure-Task $crossTasks $crossTaskById "task-a" "Task A" "Reject cross-task source" "active" "thread-1" "map-1" @("map-1") $crossTaskAState)
    [void](Ensure-Task $crossTasks $crossTaskById "task-b" "Task B" "Owns source" "active" "thread-1" "map-2" @("map-2") $crossTaskBState)
    $crossAudit = Get-CognitiveAuditSummary @($crossTasks.ToArray()) @($nodes.Values) @() @($timeline.ToArray())
    Assert-Equal ([bool]$crossAudit.hardGatePassed) $false "active fact should not join to a fact source owned by another task"
    Assert-Contains $crossAudit.hardGateFailures "active_fact_source_missing" "cross-task source mismatch should be reported"
    $crossGate = @($crossAudit.gateRecords | Where-Object { $_.gateId -eq "active_fact_source_missing" } | Select-Object -First 1)
    if (@($crossGate.subjectIds) -notcontains "task-a/fact-5->shared-source") {
        throw "cross-task source mismatch should include task/fact/source subject id."
    }
    $results.Add("cognitive-audit-cross-task-source-mismatch: PASS")

    $fixtureDir = Join-Path $OutputDir "black-box-fixture"
    [void](New-Item -ItemType Directory -Force -Path $fixtureDir)
    $rolloutPath = Join-Path $fixtureDir "rollout.jsonl"
    $jsonlPath = Join-Path $fixtureDir "whale-exec.jsonl"
    $exportDir = Join-Path $fixtureDir "export"
    $snapshotEvent = [ordered]@{
        timestamp = "2026-05-30T00:07:00Z"
        payload = [ordered]@{
            type = "snapshot_updated"
            snapshot = [ordered]@{
                tasks = @([ordered]@{
                        id = "task-1"
                        title = "Fix app"
                        objective = "Repair failing validator"
                        status = "active"
                        ownerSessionId = "thread-1"
                        activeMapId = "map-1"
                        mapIds = @("map-1")
                        cognitiveState = $cognitiveState
                    })
                maps = @([ordered]@{
                        id = "map-1"
                        title = "Fix app"
                        ownerSessionId = "thread-1"
                        createdFrom = $null
                        edges = @()
                        nodes = @([ordered]@{ id = "node-1"; title = "Read source"; kind = "inspect_code_context"; status = "completed" })
                        results = @([ordered]@{
                                id = "result-3"
                                nodeId = "node-1"
                                assignmentId = "lease-3"
                                sourceThreadId = "thread-1"
                                kind = "result"
                                actionClass = "test"
                                body = "validated"
                                evidencePackage = $evidencePackage
                            })
                    })
                maintenanceBarriers = @()
                sentinelWarnings = @([ordered]@{
                        id = "sentinel-1"
                        sentinelType = "validator_failure"
                        status = "cleared"
                        severity = "warning"
                        taskId = "task-1"
                        mapId = "map-1"
                        nodeId = "node-1"
                        resultId = "result-3"
                        traceEventIds = @("trace-1")
                        reason = "fixture warning"
                        clearanceAction = "FixApplied"
                        createdAtMs = "1"
                    })
            }
        }
    }
    $validityEvent = [ordered]@{ timestamp = "2026-05-30T00:08:00Z"; payload = [ordered]@{ type = "result_validity_changed"; resultId = "result-3"; validity = "accepted" } }
    @(
        ($snapshotEvent | ConvertTo-Json -Depth 30 -Compress),
        ($validityEvent | ConvertTo-Json -Depth 30 -Compress)
    ) | Set-Content -LiteralPath $rolloutPath -Encoding UTF8
    "" | Set-Content -LiteralPath $jsonlPath -Encoding UTF8
    & (Join-Path $PSScriptRoot "export-action-map-observability.ps1") -RolloutPath $rolloutPath -JsonlPath $jsonlPath -OutputDir $exportDir | Out-Null
    $exportJson = Get-Content -LiteralPath (Join-Path $exportDir "action-map-observability.json") -Raw -Encoding UTF8 | ConvertFrom-Json
    Assert-Equal ([bool]$exportJson.cognitiveAudit.structuralGatePassed) $true "black-box fixture should pass structural gate"
    Assert-Equal ([int]$exportJson.summary.inputParseErrors) 0 "black-box fixture should have no parse errors"
    $html = Get-Content -LiteralPath (Join-Path $exportDir "action-map-observability.html") -Raw -Encoding UTF8
    $match = [regex]::Match($html, '(?s)<script type="application/json" id="trace-data">(.*?)</script>')
    if (-not $match.Success) {
        throw "HTML report trace-data script was not found."
    }
    if ($match.Groups[1].Value -match "&quot;") {
        throw "HTML report should not entity-encode JSON quotes in trace-data."
    }
    [void]($match.Groups[1].Value | ConvertFrom-Json)
    $markdown = Get-Content -LiteralPath (Join-Path $exportDir "action-map-observability.md") -Raw -Encoding UTF8
    foreach ($needle in @("## Source", "### Gate Records", "## Result Evidence", "## Sentinel Warnings", "Known Missing / Future Work")) {
        if ($markdown -notmatch [regex]::Escape($needle)) {
            throw "Markdown report did not contain '$needle'."
        }
    }
    $results.Add("black-box-export-report-html-parse: PASS")

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
