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
        validityReason = "validator | passed`nclean"
    }
    Add-Or-Update-NodeResult $node "2026-05-30T00:04:00Z" "result-3" "lease-3" "thread-1" "result" "test" "validated" $evidencePackage
    Assert-Equal ([string]$node.results[2].validity) "accepted" "accepted result validity should be derived from evidence package"
    Assert-Equal ([int]$node.results[2].claimCount) 1 "accepted result claim count should be derived"
    Assert-Equal ([int]$node.results[2].evidenceRefCount) 1 "accepted result evidence count should be derived"
    $results.Add("result-evidence-package-derived-fields: PASS")

    $artifactRoot = Join-Path $OutputDir "artifact-root"
    $artifactSrcDir = Join-Path $artifactRoot "src"
    [void](New-Item -ItemType Directory -Force -Path $artifactSrcDir)
    "print('ok')" | Set-Content -LiteralPath (Join-Path $artifactSrcDir "app.py") -Encoding UTF8

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
    $audit = Get-CognitiveAuditSummary @($tasks.ToArray()) @($nodes.Values) @() @($timeline.ToArray()) $artifactRoot
    Assert-Equal ([string]$audit.auditSchemaVersion) "taskspace-cognitive-audit-v1" "audit schema version should be explicit"
    Assert-Equal ([bool]$audit.hardGatePassed) $true "complete cognitive audit chain should pass hard gates"
    Assert-Equal ([bool]$audit.fullMvpHardGateImplemented) $true "final-artifact MVP hard gates should be implemented"
    Assert-Equal ([int]@($audit.unsupportedMvpGateIds).Count) 0 "implemented MVP audit should not list unsupported gates"
    Assert-Equal ([int]$audit.metrics.outputContractCount) 1 "audit should count output contracts"
    Assert-Equal ([int]$audit.metrics.acceptedResultCount) 1 "audit should count accepted results"
    Assert-Equal ([int]$audit.metrics.finalArtifactCount) 1 "audit should derive final artifact from changedArtifacts"
    if ([string]::IsNullOrWhiteSpace([string]$audit.finalArtifacts[0].artifactHash)) {
        throw "final artifact should include a SHA-256 artifact hash."
    }
    $results.Add("cognitive-audit-complete-chain: PASS")

    Add-Or-Update-NodeResult $node "2026-05-30T00:06:05Z" "result-extra" "lease-extra" "thread-1" "result" "test" "same artifact extra result" $evidencePackage
    foreach ($contractCase in @(
            [pscustomobject]@{ name = "path-only"; contract = [pscustomobject]@{ id = "contract-path-only"; kind = "artifact"; artifactRef = "src/app.py"; evidenceRefs = @() } },
            [pscustomobject]@{ name = "path-and-result"; contract = [pscustomobject]@{ id = "contract-path-result"; kind = "artifact"; artifactRef = "src/app.py"; evidenceRefs = @([pscustomobject]@{ resultId = "result-3" }) } }
        )) {
        $caseTasks = New-Object System.Collections.Generic.List[object]
        $caseTaskById = @{}
        $caseState = [pscustomobject]@{ outputContracts = @($contractCase.contract); factSources = $cognitiveState.factSources; facts = $cognitiveState.facts; assumptions = @(); riskNotes = @(); successCriteria = @("validator passes") }
        [void](Ensure-Task $caseTasks $caseTaskById "task-$($contractCase.name)" "Contract $($contractCase.name)" "Accept valid contract join" "active" "thread-1" "map-1" @("map-1") $caseState)
        $caseAudit = Get-CognitiveAuditSummary @($caseTasks.ToArray()) @($nodes.Values) @() @($timeline.ToArray()) $artifactRoot
        Assert-Equal ([bool]$caseAudit.hardGatePassed) $true "valid $($contractCase.name) artifact contract should pass audit"
    }
    $results.Add("cognitive-audit-contract-positive-joins: PASS")

    $invalidNodes = @{}
    $invalidNode = Ensure-Node $invalidNodes "node-invalid" "Invalid result" "implement_patch"
    $invalidEvidence = [pscustomobject]@{
        claims = @([pscustomobject]@{ id = "claim-invalid"; statement = "bad output"; evidenceRefs = @([pscustomobject]@{ resultId = "result-invalid"; validatorRef = "pytest" }) })
        evidenceRefs = @([pscustomobject]@{ resultId = "result-invalid"; validatorRef = "pytest" })
        changedArtifacts = @("src/app.py")
        validatorRefs = @("pytest")
        remainingUncertainty = @("validator failed")
        validity = "invalid"
        validityReason = "validator failed"
    }
    Add-Or-Update-NodeResult $invalidNode "2026-05-30T00:06:30Z" "result-invalid" "lease-invalid" "thread-1" "result" "implement" "bad output" $invalidEvidence "map-1" "task-invalid"
    $invalidState = [pscustomobject]@{
        outputContracts = @([pscustomobject]@{ id = "contract-invalid"; kind = "artifact"; description = "write output"; evidenceRefs = @([pscustomobject]@{ resultId = "result-invalid" }) })
        factSources = @([pscustomobject]@{ id = "source-invalid"; provenance = "observed_from_environment"; description = "validator output"; evidenceRefs = @([pscustomobject]@{ validatorRef = "pytest" }) })
        facts = @([pscustomobject]@{ id = "fact-invalid"; statement = "bad result is true"; evidenceRefs = @([pscustomobject]@{ resultId = "result-invalid" }) })
        assumptions = @()
        riskNotes = @()
        successCriteria = @()
    }
    $invalidTasks = New-Object System.Collections.Generic.List[object]
    $invalidTaskById = @{}
    [void](Ensure-Task $invalidTasks $invalidTaskById "task-invalid" "Invalid dependency" "Reject invalid result as dependency" "active" "thread-1" "map-1" @("map-1") $invalidState)
    $invalidAudit = Get-CognitiveAuditSummary @($invalidTasks.ToArray()) @($invalidNodes.Values) @() @($timeline.ToArray()) $artifactRoot
    Assert-Equal ([bool]$invalidAudit.hardGatePassed) $false "invalid result should not anchor active facts or final artifacts"
    Assert-Contains $invalidAudit.hardGateFailures "questioned_or_invalid_result_in_cognitive_state_update" "invalid result in active fact should be reported"
    Assert-Contains $invalidAudit.hardGateFailures "questioned_or_invalid_final_artifact_dependency" "invalid result in final artifact dependency should be reported"
    $results.Add("cognitive-audit-invalid-result-dependencies: PASS")

    $unreviewedNodes = @{}
    $unreviewedNode = Ensure-Node $unreviewedNodes "node-unreviewed" "Unreviewed result" "implement_patch"
    $unreviewedEvidence = [pscustomobject]@{
        claims = @([pscustomobject]@{ id = "claim-unreviewed"; statement = "output exists"; evidenceRefs = @([pscustomobject]@{ resultId = "result-unreviewed"; validatorRef = "pytest" }) })
        evidenceRefs = @([pscustomobject]@{ resultId = "result-unreviewed"; validatorRef = "pytest" })
        changedArtifacts = @("src/app.py")
        validatorRefs = @("pytest")
        remainingUncertainty = @("not reviewed")
        validity = "unreviewed"
        validityReason = "not reviewed"
    }
    Add-Or-Update-NodeResult $unreviewedNode "2026-05-30T00:06:40Z" "result-unreviewed" "lease-unreviewed" "thread-1" "result" "implement" "output" $unreviewedEvidence "map-1" "task-unreviewed"
    $unreviewedState = [pscustomobject]@{
        outputContracts = @([pscustomobject]@{ id = "contract-unreviewed"; kind = "artifact"; description = "write output"; evidenceRefs = @([pscustomobject]@{ resultId = "result-unreviewed" }) })
        factSources = @([pscustomobject]@{ id = "source-unreviewed"; provenance = "observed_from_environment"; description = "validator output"; evidenceRefs = @([pscustomobject]@{ validatorRef = "pytest" }) })
        facts = @([pscustomobject]@{ id = "fact-unreviewed"; statement = "validator exists"; evidenceRefs = @([pscustomobject]@{ factSourceId = "source-unreviewed" }) })
        assumptions = @()
        riskNotes = @()
        successCriteria = @()
    }
    $unreviewedTasks = New-Object System.Collections.Generic.List[object]
    $unreviewedTaskById = @{}
    [void](Ensure-Task $unreviewedTasks $unreviewedTaskById "task-unreviewed" "Unreviewed dependency" "Reject unreviewed final dependency" "active" "thread-1" "map-1" @("map-1") $unreviewedState)
    $unreviewedTimeline = New-Object System.Collections.Generic.List[object]
    Add-TimelineEvent $unreviewedTimeline "2026-05-30T00:06:41Z" "result_validity_changed" "result still unreviewed" ([pscustomobject]@{ resultId = "result-unreviewed" })
    $unreviewedAudit = Get-CognitiveAuditSummary @($unreviewedTasks.ToArray()) @($unreviewedNodes.Values) @() @($unreviewedTimeline.ToArray()) $artifactRoot
    Assert-Equal ([bool]$unreviewedAudit.hardGatePassed) $false "unreviewed result should not anchor final artifacts"
    Assert-Contains $unreviewedAudit.hardGateFailures "non_accepted_final_artifact_dependency" "unreviewed final artifact dependency should be reported"
    $results.Add("cognitive-audit-unreviewed-final-artifact-dependency: PASS")

    $orphanNodes = @{}
    $orphanNode = Ensure-Node $orphanNodes "node-orphan" "One artifact" "implement_patch"
    Add-Or-Update-NodeResult $orphanNode "2026-05-30T00:06:45Z" "result-good" "lease-good" "thread-1" "result" "implement" "output" $evidencePackage "map-1" "task-orphan"
    $orphanState = [pscustomobject]@{
        outputContracts = @(
            [pscustomobject]@{ id = "contract-good"; kind = "artifact"; description = "write app.py"; evidenceRefs = @([pscustomobject]@{ resultId = "result-good" }) },
            [pscustomobject]@{ id = "contract-orphan"; kind = "artifact"; description = "write missing.txt"; artifactRef = "missing.txt"; evidenceRefs = @() }
        )
        factSources = @([pscustomobject]@{ id = "source-orphan"; provenance = "observed_from_environment"; description = "validator output"; evidenceRefs = @([pscustomobject]@{ validatorRef = "pytest" }) })
        facts = @([pscustomobject]@{ id = "fact-orphan"; statement = "tests passed"; evidenceRefs = @([pscustomobject]@{ factSourceId = "source-orphan" }) })
        assumptions = @()
        riskNotes = @()
        successCriteria = @()
    }
    $orphanTasks = New-Object System.Collections.Generic.List[object]
    $orphanTaskById = @{}
    [void](Ensure-Task $orphanTasks $orphanTaskById "task-orphan" "Orphan contract" "Reject orphan artifact contract" "active" "thread-1" "map-1" @("map-1") $orphanState)
    $orphanAudit = Get-CognitiveAuditSummary @($orphanTasks.ToArray()) @($orphanNodes.Values) @() @($timeline.ToArray()) $artifactRoot
    Assert-Equal ([bool]$orphanAudit.hardGatePassed) $false "orphan artifact contract should fail why-chain audit"
    Assert-Contains $orphanAudit.hardGateFailures "audit_why_chain_missing" "orphan artifact contract should be reported"
    $orphanGate = @($orphanAudit.gateRecords | Where-Object { $_.gateId -eq "audit_why_chain_missing" } | Select-Object -First 1)
    if (@($orphanGate.subjectIds) -notcontains "task:task-orphan|artifact:missing.txt") {
        throw "orphan artifact contract should include task/contract subject id."
    }
    $results.Add("cognitive-audit-orphan-artifact-contract: PASS")

    $mismatchNodes = @{}
    $mismatchNode = Ensure-Node $mismatchNodes "node-mismatch" "Mismatched contract" "implement_patch"
    $mismatchEvidence = [pscustomobject]@{ claims = @([pscustomobject]@{ id = "claim-mismatch"; statement = "validator passed"; evidenceRefs = @([pscustomobject]@{ resultId = "result-path"; validatorRef = "pytest" }) }); evidenceRefs = @([pscustomobject]@{ resultId = "result-path"; validatorRef = "pytest" }); changedArtifacts = @("src/app.py"); validatorRefs = @("pytest"); remainingUncertainty = @(); validity = "accepted"; validityReason = "validator passed" }
    Add-Or-Update-NodeResult $mismatchNode "2026-05-30T00:06:47Z" "result-path" "lease-path" "thread-1" "result" "implement" "output" $mismatchEvidence "map-1" "task-mismatch"
    $mismatchTasks = New-Object System.Collections.Generic.List[object]
    $mismatchTaskById = @{}
    [void](Ensure-Task $mismatchTasks $mismatchTaskById "task-mismatch" "Mismatched contract" "Reject contract result mismatch" "active" "thread-1" "map-1" @("map-1") ([pscustomobject]@{
                outputContracts = @([pscustomobject]@{ id = "contract-mismatch"; kind = "artifact"; artifactRef = "src/app.py"; evidenceRefs = @([pscustomobject]@{ resultId = "result-other"; artifactRef = "src/app.py" }) })
                factSources = @([pscustomobject]@{ id = "source-mismatch"; provenance = "observed_from_environment"; description = "validator"; evidenceRefs = @([pscustomobject]@{ validatorRef = "pytest" }) })
                facts = @([pscustomobject]@{ id = "fact-mismatch"; statement = "tests passed"; evidenceRefs = @([pscustomobject]@{ factSourceId = "source-mismatch" }) })
                assumptions = @(); riskNotes = @(); successCriteria = @()
            }))
    $mismatchAudit = Get-CognitiveAuditSummary @($mismatchTasks.ToArray()) @($mismatchNodes.Values) @() @($timeline.ToArray()) $artifactRoot
    Assert-Equal ([bool]$mismatchAudit.hardGatePassed) $false "contract artifact path and resultId mismatch should fail audit"
    Assert-Contains $mismatchAudit.hardGateFailures "output_contract_result_mismatch" "contract result mismatch should be reported"
    $results.Add("cognitive-audit-contract-result-mismatch: PASS")

    $missingHashAudit = Get-CognitiveAuditSummary @($tasks.ToArray()) @($nodes.Values) @() @($timeline.ToArray()) (Join-Path $OutputDir "missing-artifact-root")
    Assert-Equal ([bool]$missingHashAudit.hardGatePassed) $false "missing artifact hash should fail final artifact audit"
    Assert-Contains $missingHashAudit.hardGateFailures "final_artifact_hash_missing" "missing artifact hash should be reported"
    $results.Add("cognitive-audit-final-artifact-hash-missing: PASS")

    $outsidePath = Join-Path $OutputDir "outside-root.txt"
    "outside" | Set-Content -LiteralPath $outsidePath -Encoding UTF8
    $outsideEvidence = [pscustomobject]@{
        claims = @([pscustomobject]@{ id = "claim-outside"; statement = "outside output"; evidenceRefs = @([pscustomobject]@{ resultId = "result-outside"; validatorRef = "pytest" }) })
        evidenceRefs = @([pscustomobject]@{ resultId = "result-outside"; validatorRef = "pytest" })
        changedArtifacts = @($outsidePath)
        validatorRefs = @("pytest")
        remainingUncertainty = @()
        validity = "accepted"
        validityReason = "validator passed"
    }
    $outsideNodes = @{}
    $outsideNode = Ensure-Node $outsideNodes "node-outside" "Outside artifact" "implement_patch"
    Add-Or-Update-NodeResult $outsideNode "2026-05-30T00:06:50Z" "result-outside" "lease-outside" "thread-1" "result" "implement" "outside output" $outsideEvidence "map-1" "task-outside"
    $outsideTasks = New-Object System.Collections.Generic.List[object]
    $outsideTaskById = @{}
    [void](Ensure-Task $outsideTasks $outsideTaskById "task-outside" "Outside artifact" "Reject outside artifact root" "active" "thread-1" "map-1" @("map-1") ([pscustomobject]@{
            outputContracts = @([pscustomobject]@{ id = "contract-outside"; kind = "artifact"; description = "write outside"; evidenceRefs = @([pscustomobject]@{ resultId = "result-outside" }) })
            factSources = @([pscustomobject]@{ id = "source-outside"; provenance = "observed_from_environment"; description = "validator"; evidenceRefs = @([pscustomobject]@{ validatorRef = "pytest" }) })
            facts = @([pscustomobject]@{ id = "fact-outside"; statement = "validator exists"; evidenceRefs = @([pscustomobject]@{ factSourceId = "source-outside" }) })
            assumptions = @(); riskNotes = @(); successCriteria = @()
        }))
    $outsideAudit = Get-CognitiveAuditSummary @($outsideTasks.ToArray()) @($outsideNodes.Values) @() @($timeline.ToArray()) $artifactRoot
    Assert-Equal ([bool]$outsideAudit.hardGatePassed) $false "artifact outside ArtifactRoot should not be hashed"
    Assert-Contains $outsideAudit.hardGateFailures "final_artifact_hash_missing" "outside artifact root should be reported as missing hash"
    $results.Add("cognitive-audit-artifact-root-containment: PASS")

    $traversalEvidence = [pscustomobject]@{ claims = @([pscustomobject]@{ id = "claim-traversal"; statement = "outside output"; evidenceRefs = @([pscustomobject]@{ resultId = "result-traversal"; validatorRef = "pytest" }) }); evidenceRefs = @([pscustomobject]@{ resultId = "result-traversal"; validatorRef = "pytest" }); changedArtifacts = @("..\outside-root.txt"); validatorRefs = @("pytest"); remainingUncertainty = @(); validity = "accepted"; validityReason = "validator passed" }
    $traversalNodes = @{}
    Add-Or-Update-NodeResult (Ensure-Node $traversalNodes "node-traversal" "Traversal artifact" "implement_patch") "2026-05-30T00:06:51Z" "result-traversal" "lease-traversal" "thread-1" "result" "implement" "outside output" $traversalEvidence "map-1" "task-traversal"
    $traversalTasks = New-Object System.Collections.Generic.List[object]
    $traversalTaskById = @{}
    [void](Ensure-Task $traversalTasks $traversalTaskById "task-traversal" "Traversal artifact" "Reject traversal artifact root" "active" "thread-1" "map-1" @("map-1") ([pscustomobject]@{ outputContracts = @([pscustomobject]@{ id = "contract-traversal"; kind = "artifact"; evidenceRefs = @([pscustomobject]@{ resultId = "result-traversal" }) }); factSources = @([pscustomobject]@{ id = "source-traversal"; provenance = "observed_from_environment"; description = "validator"; evidenceRefs = @([pscustomobject]@{ validatorRef = "pytest" }) }); facts = @([pscustomobject]@{ id = "fact-traversal"; statement = "validator exists"; evidenceRefs = @([pscustomobject]@{ factSourceId = "source-traversal" }) }); assumptions = @(); riskNotes = @(); successCriteria = @() }))
    $traversalAudit = Get-CognitiveAuditSummary @($traversalTasks.ToArray()) @($traversalNodes.Values) @() @($timeline.ToArray()) $artifactRoot
    Assert-Equal ([bool]$traversalAudit.hardGatePassed) $false "artifact path traversal outside ArtifactRoot should not be hashed"
    Assert-Contains $traversalAudit.hardGateFailures "final_artifact_hash_missing" "path traversal outside root should be reported as missing hash"
    $results.Add("cognitive-audit-artifact-root-traversal-containment: PASS")

    $unclearedWarning = [pscustomobject]@{ id = "sentinel-open"; status = "active"; resultId = "result-3"; nodeId = "node-1" }
    $unclearedAudit = Get-CognitiveAuditSummary @($tasks.ToArray()) @($nodes.Values) @($unclearedWarning) @($timeline.ToArray()) $artifactRoot
    Assert-Equal ([bool]$unclearedAudit.hardGatePassed) $false "uncleared sentinel affecting final artifact should fail audit"
    Assert-Contains $unclearedAudit.hardGateFailures "sentinel_warning_uncleared_for_final_artifact" "uncleared sentinel should be reported"
    $results.Add("cognitive-audit-final-artifact-uncleared-sentinel: PASS")

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

    $crossResultNodes = @{}
    $crossResultNode = Ensure-Node $crossResultNodes "node-task-b" "Task B result" "inspect_code_context"
    Add-Or-Update-NodeResult $crossResultNode "2026-05-30T00:06:55Z" "result-task-b" "lease-task-b" "thread-1" "result" "test" "validated" $evidencePackage "map-b" "task-b"
    $crossResultTasks = New-Object System.Collections.Generic.List[object]
    $crossResultTaskById = @{}
    [void](Ensure-Task $crossResultTasks $crossResultTaskById "task-a" "Task A" "Reject cross-task accepted result" "active" "thread-1" "map-a" @("map-a") ([pscustomobject]@{
            outputContracts = @([pscustomobject]@{ id = "contract-a"; kind = "artifact"; description = "write output"; evidenceRefs = @([pscustomobject]@{ resultId = "result-task-b" }) })
            factSources = @([pscustomobject]@{ id = "source-a"; provenance = "observed_from_environment"; description = "validator"; evidenceRefs = @([pscustomobject]@{ validatorRef = "pytest" }) })
            facts = @([pscustomobject]@{ id = "fact-cross-result"; statement = "task b result belongs here"; evidenceRefs = @([pscustomobject]@{ resultId = "result-task-b" }) })
            assumptions = @(); riskNotes = @(); successCriteria = @()
        }))
    [void](Ensure-Task $crossResultTasks $crossResultTaskById "task-b" "Task B" "Owns result" "active" "thread-1" "map-b" @("map-b") ([pscustomobject]@{
            outputContracts = @(); factSources = @(); facts = @(); assumptions = @(); riskNotes = @(); successCriteria = @()
        }))
    $crossResultAudit = Get-CognitiveAuditSummary @($crossResultTasks.ToArray()) @($crossResultNodes.Values) @() @($timeline.ToArray()) $artifactRoot
    Assert-Equal ([bool]$crossResultAudit.hardGatePassed) $false "active fact should not join to an accepted result owned by another task"
    Assert-Contains $crossResultAudit.hardGateFailures "questioned_or_invalid_result_in_cognitive_state_update" "cross-task result source should be reported"
    $results.Add("cognitive-audit-cross-task-accepted-result-source: PASS")

    $samePathNodes = @{}
    $samePathA = Ensure-Node $samePathNodes "node-same-a" "Task A artifact" "implement_patch"
    $samePathB = Ensure-Node $samePathNodes "node-same-b" "Task B artifact" "implement_patch"
    Add-Or-Update-NodeResult $samePathA "2026-05-30T00:06:56Z" "result-same-a" "lease-same-a" "thread-1" "result" "implement" "validated" $evidencePackage "map-a" "task-same-a"
    $evidencePackageB = [pscustomobject]@{
        claims = @([pscustomobject]@{ id = "claim-b"; statement = "validator passed"; evidenceRefs = @([pscustomobject]@{ resultId = "result-same-b"; validatorRef = "pytest" }) })
        evidenceRefs = @([pscustomobject]@{ resultId = "result-same-b"; validatorRef = "pytest" })
        changedArtifacts = @("src/app.py")
        validatorRefs = @("pytest")
        remainingUncertainty = @()
        validity = "accepted"
        validityReason = "validator passed"
    }
    Add-Or-Update-NodeResult $samePathB "2026-05-30T00:06:57Z" "result-same-b" "lease-same-b" "thread-1" "result" "implement" "validated" $evidencePackageB "map-b" "task-same-b"
    $samePathTasks = New-Object System.Collections.Generic.List[object]
    $samePathById = @{}
    [void](Ensure-Task $samePathTasks $samePathById "task-same-a" "Same path A" "Do not merge artifacts" "active" "thread-1" "map-a" @("map-a") ([pscustomobject]@{ outputContracts = @([pscustomobject]@{ id = "contract-a"; kind = "artifact"; description = "write A"; evidenceRefs = @([pscustomobject]@{ resultId = "result-same-a" }) }); factSources = @([pscustomobject]@{ id = "source-a"; provenance = "observed_from_environment"; description = "validator"; evidenceRefs = @([pscustomobject]@{ validatorRef = "pytest" }) }); facts = @([pscustomobject]@{ id = "fact-a"; statement = "tests passed"; evidenceRefs = @([pscustomobject]@{ factSourceId = "source-a" }) }); assumptions = @(); riskNotes = @(); successCriteria = @() }))
    [void](Ensure-Task $samePathTasks $samePathById "task-same-b" "Same path B" "Do not merge artifacts" "active" "thread-1" "map-b" @("map-b") ([pscustomobject]@{ outputContracts = @([pscustomobject]@{ id = "contract-b"; kind = "artifact"; description = "write B"; evidenceRefs = @([pscustomobject]@{ resultId = "result-same-b" }) }); factSources = @([pscustomobject]@{ id = "source-b"; provenance = "observed_from_environment"; description = "validator"; evidenceRefs = @([pscustomobject]@{ validatorRef = "pytest" }) }); facts = @([pscustomobject]@{ id = "fact-b"; statement = "tests passed"; evidenceRefs = @([pscustomobject]@{ factSourceId = "source-b" }) }); assumptions = @(); riskNotes = @(); successCriteria = @() }))
    $samePathAudit = Get-CognitiveAuditSummary @($samePathTasks.ToArray()) @($samePathNodes.Values) @() @($timeline.ToArray()) $artifactRoot
    Assert-Equal ([int]$samePathAudit.metrics.finalArtifactCount) 2 "same relative artifact path in two tasks should not merge"
    $results.Add("cognitive-audit-same-path-multi-task-artifacts: PASS")

    $fixtureDir = Join-Path $OutputDir "black-box-fixture"
    [void](New-Item -ItemType Directory -Force -Path $fixtureDir)
    [void](New-Item -ItemType Directory -Force -Path (Join-Path $fixtureDir "src"))
    "print('ok')" | Set-Content -LiteralPath (Join-Path $fixtureDir "src\app.py") -Encoding UTF8
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
                        title = "Fix | app"
                        objective = "Repair failing validator`nwith evidence"
                        status = "active"
                        ownerSessionId = "thread-1"
                        activeMapId = "map-1"
                        mapIds = @("map-1")
                        cognitiveState = $cognitiveState
                    })
                maps = @([ordered]@{
                        id = "map-1"
                        taskId = "task-1"
                        title = "Fix | app"
                        ownerSessionId = "thread-1"
                        createdFrom = $null
                        edges = @()
                        nodes = @([ordered]@{ id = "node-1"; title = "Read | source"; kind = "inspect_code_context"; status = "completed" })
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
                        reason = "fixture | warning`ncleared"
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
    & (Join-Path $PSScriptRoot "export-action-map-observability.ps1") -RolloutPath $rolloutPath -JsonlPath $jsonlPath -OutputDir $exportDir -ArtifactRoot $fixtureDir | Out-Null
    $exportJson = Get-Content -LiteralPath (Join-Path $exportDir "action-map-observability.json") -Raw -Encoding UTF8 | ConvertFrom-Json
    Assert-Equal ([bool]$exportJson.cognitiveAudit.structuralGatePassed) $true "black-box fixture should pass structural gate"
    Assert-Equal ([bool]$exportJson.cognitiveAudit.hardGatePassed) $true "black-box fixture should pass final artifact hard gate"
    Assert-Equal ([int]$exportJson.summary.inputParseErrors) 0 "black-box fixture should have no parse errors"
    Assert-Equal ([int]$exportJson.summary.finalArtifacts) 1 "black-box fixture should export final artifact count"
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
    foreach ($needle in @(
            "## Source",
            "- artifact root:",
            "### Gate Records",
            "## Final Artifacts",
            "## Result Evidence",
            "## Sentinel Warnings",
            "Known Missing / Future Work",
            '`changedArtifacts` / `artifactRef`'
        )) {
        if ($markdown -notmatch [regex]::Escape($needle)) {
            throw "Markdown report did not contain '$needle'."
        }
    }
    foreach ($escaped in @("Fix \| app", "Repair failing validator<br>with evidence", "Read \| source", "fixture \| warning<br>cleared", "validator \| passed<br>clean")) {
        if ($markdown -notmatch [regex]::Escape($escaped)) {
            throw "Markdown report did not escape table content '$escaped'."
        }
    }
    foreach ($ch in $markdown.ToCharArray()) {
        $code = [int][char]$ch
        if ($code -lt 32 -and $code -notin @(9, 10, 13)) {
            throw "Markdown report contains unexpected control character code $code."
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
