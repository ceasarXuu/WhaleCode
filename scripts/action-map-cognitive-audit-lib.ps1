. (Join-Path $PSScriptRoot "action-map-object-lib.ps1")

function Convert-CognitiveState($CognitiveState) {
    if ($null -eq $CognitiveState) {
        $CognitiveState = [pscustomobject]@{}
    }
    return [ordered]@{
        successCriteria = @(Get-ObjectArray $CognitiveState.successCriteria)
        factSources = @(Get-ObjectArray $CognitiveState.factSources)
        outputContracts = @(Get-ObjectArray $CognitiveState.outputContracts)
        facts = @(Get-ObjectArray $CognitiveState.facts)
        assumptions = @(Get-ObjectArray $CognitiveState.assumptions)
        riskNotes = @(Get-ObjectArray $CognitiveState.riskNotes)
    }
}

function Convert-EvidencePackage($EvidencePackage) {
    if ($null -eq $EvidencePackage) {
        $EvidencePackage = [pscustomobject]@{}
    }
    $validity = [string]$EvidencePackage.validity
    if ([string]::IsNullOrWhiteSpace($validity)) {
        $validity = "unreviewed"
    }
    return [ordered]@{
        claims = @(Get-ObjectArray $EvidencePackage.claims)
        evidenceRefs = @(Get-ObjectArray $EvidencePackage.evidenceRefs)
        changedArtifacts = @(Get-ObjectArray $EvidencePackage.changedArtifacts)
        validatorRefs = @(Get-ObjectArray $EvidencePackage.validatorRefs)
        remainingUncertainty = @(Get-ObjectArray $EvidencePackage.remainingUncertainty)
        validity = $validity
        validityReason = [string]$EvidencePackage.validityReason
    }
}

function Update-ResultEvidenceDerivedFields {
    param([object]$Result)

    if (-not $Result) {
        return
    }
    if ($null -eq $Result.evidencePackage) {
        $Result.evidencePackage = Convert-EvidencePackage $null
    }
    $ep = $Result.evidencePackage
    $Result.validity = [string]$ep.validity
    $Result.claimCount = @(Get-ObjectArray $ep.claims).Count
    $Result.evidenceRefCount = @(Get-ObjectArray $ep.evidenceRefs).Count
    $Result.validatorRefCount = @(Get-ObjectArray $ep.validatorRefs).Count
}

function Add-AuditGateRecord {
    param(
        [System.Collections.Generic.List[object]]$GateRecords,
        [string]$GateId,
        [bool]$Pass,
        [string]$Expected,
        [string]$Observed,
        [object]$SubjectIds = @()
    )

    $GateRecords.Add([ordered]@{
        gateId = $GateId
        pass = $Pass
        expected = $Expected
        observed = $Observed
        subjectIds = @(Get-ObjectArray $SubjectIds)
    })
}

function Get-CognitiveAuditSummary {
    param(
        [object]$Tasks,
        [object]$Nodes,
        [object]$SentinelWarnings,
        [object]$Timeline
    )

    $results = New-Object System.Collections.Generic.List[object]
    $resultById = @{}
    foreach ($node in @(Get-ObjectArray $Nodes)) {
        foreach ($result in @(Get-ObjectArray (Get-ObjectField $node "results"))) {
            if ($null -ne $result) {
                $results.Add($result)
                $resultId = [string](Get-ObjectField $result "resultId")
                if ($resultId) {
                    $resultById[$resultId] = $result
                }
            }
        }
    }

    $outputContracts = 0
    $factSources = 0
    foreach ($task in @(Get-ObjectArray $Tasks)) {
        $cognitive = Get-ObjectField $task "cognitiveState"
        $outputContracts += @(Get-ObjectArray (Get-ObjectField $cognitive "outputContracts")).Count
        $factSources += @(Get-ObjectArray (Get-ObjectField $cognitive "factSources")).Count
    }

    $acceptedResults = New-Object System.Collections.Generic.List[object]
    $questionedOrInvalidResults = New-Object System.Collections.Generic.List[object]
    $acceptedMissingEvidence = New-Object System.Collections.Generic.List[object]
    $resultsWithClaimEvidence = New-Object System.Collections.Generic.List[object]
    foreach ($result in @(Get-ObjectArray $results)) {
        $validity = [string](Get-ObjectField $result "validity")
        $claimCount = [int](Get-ObjectField $result "claimCount")
        $evidenceRefCount = [int](Get-ObjectField $result "evidenceRefCount")
        if ($validity -eq "accepted") {
            $acceptedResults.Add($result)
            if ($claimCount -le 0 -or $evidenceRefCount -le 0) {
                $acceptedMissingEvidence.Add($result)
            }
        }
        if ($validity -in @("questioned", "invalid")) {
            $questionedOrInvalidResults.Add($result)
        }
        if ($claimCount -gt 0 -and $evidenceRefCount -gt 0) {
            $resultsWithClaimEvidence.Add($result)
        }
    }

    $badFactSourceRefs = 0
    $activeFactMissingEvidenceRefs = New-Object System.Collections.Generic.List[string]
    $activeFactMissingJoinableSource = New-Object System.Collections.Generic.List[string]
    $activeFactInvalidSource = New-Object System.Collections.Generic.List[string]
    $activeFactCount = 0
    foreach ($task in @(Get-ObjectArray $Tasks)) {
        $taskId = [string](Get-ObjectField $task "id")
        $cognitive = Get-ObjectField $task "cognitiveState"
        $taskSourceById = @{}
        foreach ($source in @(Get-ObjectArray (Get-ObjectField $cognitive "factSources"))) {
            $sourceId = [string](Get-ObjectField $source "id")
            if ($sourceId) {
                $taskSourceById[$sourceId] = $source
            }
        }
        foreach ($fact in @(Get-ObjectArray (Get-ObjectField $cognitive "facts"))) {
            $activeFactCount++
            $factId = [string](Get-ObjectField $fact "id")
            if (-not $factId) {
                $factId = [string](Get-ObjectField $fact "statement")
            }
            $factSubject = "$taskId/$factId"
            $refs = @(Get-ObjectArray (Get-ObjectField $fact "evidenceRefs"))
            if ($refs.Count -eq 0) {
                $activeFactMissingEvidenceRefs.Add($factSubject)
                $activeFactMissingJoinableSource.Add($factSubject)
                continue
            }
            $hasJoinableSource = $false
            foreach ($ref in $refs) {
                $sourceId = [string](Get-ObjectField $ref "factSourceId")
                if (-not $sourceId -or -not $taskSourceById.ContainsKey($sourceId)) {
                    if ($sourceId) {
                        $activeFactMissingJoinableSource.Add("$factSubject->$sourceId")
                    }
                    continue
                }
                $provenance = [string](Get-ObjectField $taskSourceById[$sourceId] "provenance")
                if ($provenance -in @("generated_for_test_only", "inferred", "unknown")) {
                    $badFactSourceRefs++
                    $activeFactInvalidSource.Add("$factSubject->$sourceId")
                }
                else {
                    $hasJoinableSource = $true
                }
            }
            if (-not $hasJoinableSource) {
                $activeFactMissingJoinableSource.Add($factSubject)
            }
        }
    }

    $validityTransitionEvents = @(Get-ObjectArray $Timeline | Where-Object { [string](Get-ObjectField $_ "kind") -eq "result_validity_changed" })
    $orphanValidityTransitions = New-Object System.Collections.Generic.List[string]
    foreach ($event in $validityTransitionEvents) {
        $details = Get-ObjectField $event "details"
        $resultId = [string](Get-ObjectField $details "resultId")
        if ($resultId -and -not $resultById.ContainsKey($resultId)) {
            $orphanValidityTransitions.Add($resultId)
        }
    }
    $validityTransitions = $validityTransitionEvents.Count
    $cognitiveUpdates = @(Get-ObjectArray $Timeline | Where-Object { [string](Get-ObjectField $_ "kind") -eq "cognitive_state_updated" }).Count
    $activeSentinelWarnings = @(Get-ObjectArray $SentinelWarnings | Where-Object { [string]$_.status -ne "cleared" }).Count

    $gateRecords = New-Object System.Collections.Generic.List[object]
    Add-AuditGateRecord $gateRecords "required_output_contract_missing" ($outputContracts -gt 0) "at least one output contract" "outputContracts=$outputContracts"
    Add-AuditGateRecord $gateRecords "required_fact_source_missing" ($factSources -gt 0) "at least one fact source" "factSources=$factSources"
    Add-AuditGateRecord $gateRecords "result_claims_evidence_missing" ($results.Count -eq 0 -or $resultsWithClaimEvidence.Count -gt 0) "when results exist, at least one result has claims and evidence" "results=$($results.Count), resultWithClaimEvidence=$($resultsWithClaimEvidence.Count)"
    Add-AuditGateRecord $gateRecords "accepted_result_missing_evidence" ($acceptedMissingEvidence.Count -eq 0) "accepted results include claims and evidence" "acceptedMissingEvidence=$($acceptedMissingEvidence.Count)" @($acceptedMissingEvidence | ForEach-Object { [string](Get-ObjectField $_ "resultId") })
    Add-AuditGateRecord $gateRecords "active_fact_source_missing" ($activeFactMissingEvidenceRefs.Count -eq 0 -and $activeFactMissingJoinableSource.Count -eq 0) "each active fact has a joinable factSourceId evidence ref" "missingEvidenceRefs=$($activeFactMissingEvidenceRefs.Count), missingJoinableSource=$($activeFactMissingJoinableSource.Count)" @($activeFactMissingJoinableSource.ToArray())
    Add-AuditGateRecord $gateRecords "self_generated_data_leakage" ($badFactSourceRefs -eq 0) "active facts do not rely on generated/inferred/unknown sources" "invalidSourceRefs=$badFactSourceRefs" @($activeFactInvalidSource.ToArray())
    Add-AuditGateRecord $gateRecords "result_validity_transition_missing" ($results.Count -eq 0 -or $validityTransitions -gt 0) "when results exist, at least one result validity transition is recorded" "results=$($results.Count), validityTransitions=$validityTransitions"
    Add-AuditGateRecord $gateRecords "orphan_result_validity_transition" ($orphanValidityTransitions.Count -eq 0) "validity transition result ids join to snapshot results" "orphanTransitions=$($orphanValidityTransitions.Count)" @($orphanValidityTransitions.ToArray())

    $failures = @($gateRecords.ToArray() | Where-Object { $_.pass -ne $true } | ForEach-Object { [string]$_.gateId })

    return [ordered]@{
        auditSchemaVersion = "taskspace-cognitive-audit-v1"
        auditScope = "mvp-structural-subset"
        fullMvpHardGateImplemented = $false
        promotionNotInMvp = $true
        structuralGatePassed = ($failures.Count -eq 0)
        hardGatePassed = ($failures.Count -eq 0)
        hardGateFailures = @($failures)
        gateRecords = @($gateRecords.ToArray())
        unsupportedMvpGateIds = @(
            "questioned_or_invalid_result_in_cognitive_state_update",
            "questioned_or_invalid_final_artifact_dependency",
            "sentinel_warning_uncleared_for_final_artifact",
            "audit_why_chain_missing",
            "final_artifact_hash_missing"
        )
        metrics = [ordered]@{
            outputContractCount = $outputContracts
            factSourceCount = $factSources
            activeFactCount = $activeFactCount
            resultCount = $results.Count
            acceptedResultCount = $acceptedResults.Count
            questionedOrInvalidResultCount = $questionedOrInvalidResults.Count
            acceptedResultMissingEvidenceCount = $acceptedMissingEvidence.Count
            resultWithClaimEvidenceCount = $resultsWithClaimEvidence.Count
            activeFactMissingEvidenceRefCount = $activeFactMissingEvidenceRefs.Count
            activeFactMissingJoinableSourceCount = $activeFactMissingJoinableSource.Count
            invalidFactSourceReferenceCount = $badFactSourceRefs
            orphanResultValidityTransitionCount = $orphanValidityTransitions.Count
            resultValidityTransitionCount = $validityTransitions
            cognitiveStateUpdateCount = $cognitiveUpdates
            activeSentinelWarningCount = $activeSentinelWarnings
        }
        reportOnly = [ordered]@{
            promotionTrigger = $false
            promotionLatencyMs = $null
            collapseRate = $null
        }
    }
}
