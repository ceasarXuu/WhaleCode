. (Join-Path $PSScriptRoot "action-map-object-lib.ps1")

function Add-UniqueAuditValue {
    param([object]$List, [string]$Value)
    if (-not [string]::IsNullOrWhiteSpace($Value) -and -not $List.Contains($Value)) {
        $List.Add($Value)
    }
}

function Resolve-FinalArtifactPath {
    param([string]$ArtifactRef, [string]$ArtifactRoot)
    if ([string]::IsNullOrWhiteSpace($ArtifactRef)) { return "" }
    $candidates = New-Object System.Collections.Generic.List[string]
    $rootFull = ""
    if (-not [string]::IsNullOrWhiteSpace($ArtifactRoot)) {
        if (-not (Test-Path -LiteralPath $ArtifactRoot -PathType Container)) { return "" }
        $rootFull = [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath $ArtifactRoot).Path).TrimEnd("\", "/")
        if ([System.IO.Path]::IsPathRooted($ArtifactRef)) { $candidates.Add($ArtifactRef) }
        else { $candidates.Add((Join-Path $ArtifactRoot $ArtifactRef)) }
    }
    elseif ([System.IO.Path]::IsPathRooted($ArtifactRef)) {
        $candidates.Add($ArtifactRef)
    }
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            $resolved = [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath $candidate).Path)
            if ($rootFull) {
                $prefix = $rootFull + [System.IO.Path]::DirectorySeparatorChar
                if (-not $resolved.Equals($rootFull, [System.StringComparison]::OrdinalIgnoreCase) -and -not $resolved.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
                    return ""
                }
            }
            return $resolved
        }
    }
    return ""
}

function Get-FinalArtifactHash {
    param([string]$ResolvedPath)
    if ([string]::IsNullOrWhiteSpace($ResolvedPath)) { return "" }
    try { return (Get-FileHash -Algorithm SHA256 -LiteralPath $ResolvedPath).Hash.ToLowerInvariant() }
    catch { return "" }
}

function Get-EvidenceRefAuditId {
    param([object]$EvidenceRef)
    foreach ($field in @("resultId", "claimId", "factSourceId", "traceEventId", "artifactRef", "validatorRef")) {
        $value = [string](Get-ObjectField $EvidenceRef $field)
        if (-not [string]::IsNullOrWhiteSpace($value)) { return "$field=$value" }
    }
    return ($EvidenceRef | ConvertTo-Json -Compress -Depth 8)
}

function Add-FinalArtifactEdge {
    param([object]$Artifact, [string]$FromKind, [string]$FromId, [string]$ToKind, [string]$ToId, [string]$ValidityAtUse = "")
    if ([string]::IsNullOrWhiteSpace($FromId) -or [string]::IsNullOrWhiteSpace($ToId)) { return }
    $Artifact.dependencyEdges.Add([ordered]@{
        fromKind = $FromKind
        fromId = $FromId
        toKind = $ToKind
        toId = $ToId
        validityAtUse = $ValidityAtUse
    })
}

function Ensure-FinalArtifactRecord {
    param(
        [object]$Artifacts,
        [hashtable]$ArtifactById,
        [string]$ArtifactRef,
        [string]$TaskId,
        [string]$MapId,
        [string]$NodeId,
        [string]$ArtifactRoot
    )
    if ([string]::IsNullOrWhiteSpace($ArtifactRef)) { return $null }
    $artifactId = if ($TaskId) { "task:$TaskId|artifact:$ArtifactRef" } else { "artifact:$ArtifactRef" }
    if (-not $ArtifactById.ContainsKey($artifactId)) {
        $resolvedPath = Resolve-FinalArtifactPath $ArtifactRef $ArtifactRoot
        $ArtifactById[$artifactId] = [ordered]@{
            finalArtifactId = $artifactId
            finalArtifactPath = $ArtifactRef
            resolvedPath = $resolvedPath
            artifactHash = Get-FinalArtifactHash $resolvedPath
            artifactFound = -not [string]::IsNullOrWhiteSpace($resolvedPath)
            taskId = $TaskId
            mapId = $MapId
            nodeIds = New-Object System.Collections.Generic.List[string]
            resultIds = New-Object System.Collections.Generic.List[string]
            outputContractIds = New-Object System.Collections.Generic.List[string]
            claimIds = New-Object System.Collections.Generic.List[string]
            evidenceRefIds = New-Object System.Collections.Generic.List[string]
            factSourceIds = New-Object System.Collections.Generic.List[string]
            validatorRefs = New-Object System.Collections.Generic.List[string]
            sentinelIds = New-Object System.Collections.Generic.List[string]
            dependencyEdges = New-Object System.Collections.Generic.List[object]
        }
        $Artifacts.Add($ArtifactById[$artifactId])
    }
    $artifact = $ArtifactById[$artifactId]
    Add-UniqueAuditValue $artifact.nodeIds $NodeId
    return $artifact
}

function Get-ArtifactRefsFromEvidenceRefs {
    param([object]$EvidenceRefs)
    $refs = New-Object System.Collections.Generic.List[string]
    foreach ($ref in @(Get-ObjectArray $EvidenceRefs)) {
        Add-UniqueAuditValue $refs ([string](Get-ObjectField $ref "artifactRef"))
    }
    return $refs
}

function Get-ResultArtifactRefs {
    param([object]$Result)
    $refs = New-Object System.Collections.Generic.List[string]
    $ep = Get-ObjectField $Result "evidencePackage"
    foreach ($artifact in @(Get-ObjectArray (Get-ObjectField $ep "changedArtifacts"))) {
        Add-UniqueAuditValue $refs ([string]$artifact)
    }
    foreach ($artifact in @(Get-ArtifactRefsFromEvidenceRefs (Get-ObjectField $ep "evidenceRefs"))) {
        Add-UniqueAuditValue $refs ([string]$artifact)
    }
    foreach ($claim in @(Get-ObjectArray (Get-ObjectField $ep "claims"))) {
        foreach ($artifact in @(Get-ArtifactRefsFromEvidenceRefs (Get-ObjectField $claim "evidenceRefs"))) {
            Add-UniqueAuditValue $refs ([string]$artifact)
        }
    }
    return $refs
}

function Get-OutputContractArtifactRefs {
    param([object]$Contract)
    $refs = New-Object System.Collections.Generic.List[string]
    foreach ($field in @("path", "pathOrArtifact", "artifactRef", "artifact")) {
        Add-UniqueAuditValue $refs ([string](Get-ObjectField $Contract $field))
    }
    foreach ($artifact in @(Get-ArtifactRefsFromEvidenceRefs (Get-ObjectField $Contract "evidenceRefs"))) {
        Add-UniqueAuditValue $refs ([string]$artifact)
    }
    return $refs
}

function Get-ResultIdsFromEvidenceRefs {
    param([object]$EvidenceRefs)
    $refs = New-Object System.Collections.Generic.List[string]
    foreach ($ref in @(Get-ObjectArray $EvidenceRefs)) {
        Add-UniqueAuditValue $refs ([string](Get-ObjectField $ref "resultId"))
    }
    return $refs
}

function Add-EvidenceRefToFinalArtifact {
    param([object]$Artifact, [string]$ParentKind, [string]$ParentId, [object]$EvidenceRef)
    $refId = Get-EvidenceRefAuditId $EvidenceRef
    Add-UniqueAuditValue $Artifact.evidenceRefIds $refId
    Add-FinalArtifactEdge $Artifact $ParentKind $ParentId "EvidenceRef" $refId
    foreach ($field in @("factSourceId", "validatorRef", "traceEventId", "resultId", "claimId", "artifactRef")) {
        $value = [string](Get-ObjectField $EvidenceRef $field)
        if ([string]::IsNullOrWhiteSpace($value)) { continue }
        if ($field -eq "factSourceId") { Add-UniqueAuditValue $Artifact.factSourceIds $value }
        if ($field -eq "validatorRef") { Add-UniqueAuditValue $Artifact.validatorRefs $value }
        Add-FinalArtifactEdge $Artifact "EvidenceRef" $refId $field $value
    }
}

function Get-FinalArtifactAuditSummary {
    param(
        [object]$Tasks,
        [object]$Nodes,
        [object]$SentinelWarnings,
        [hashtable]$ResultById,
        [string]$ArtifactRoot = ""
    )
    $tasksArray = @(Get-ObjectArray $Tasks)
    $taskById = @{}
    foreach ($task in $tasksArray) { $taskById[[string](Get-ObjectField $task "id")] = $task }
    $artifacts = New-Object System.Collections.Generic.List[object]
    $artifactById = @{}
    $pendingArtifactContracts = New-Object System.Collections.Generic.List[object]
    foreach ($task in $tasksArray) {
        $taskId = [string](Get-ObjectField $task "id")
        foreach ($contract in @(Get-ObjectArray (Get-ObjectField (Get-ObjectField $task "cognitiveState") "outputContracts"))) {
            if ([string](Get-ObjectField $contract "kind") -ne "artifact") { continue }
            $refs = @(Get-OutputContractArtifactRefs $contract)
            if ($refs.Count -eq 0) {
                $pendingArtifactContracts.Add([ordered]@{
                    taskId = $taskId
                    contractId = [string](Get-ObjectField $contract "id")
                    resultIds = @(Get-ResultIdsFromEvidenceRefs (Get-ObjectField $contract "evidenceRefs"))
                })
                continue
            }
            foreach ($artifactRef in $refs) {
                $artifact = Ensure-FinalArtifactRecord $artifacts $artifactById ([string]$artifactRef) $taskId ([string](Get-ObjectField $task "activeMapId")) "" $ArtifactRoot
                Add-UniqueAuditValue $artifact.outputContractIds ([string](Get-ObjectField $contract "id"))
                Add-FinalArtifactEdge $artifact "OutputContract" ([string](Get-ObjectField $contract "id")) "FinalArtifact" $artifact.finalArtifactId
            }
        }
    }
    Add-ResultArtifactsToAudit $artifacts $artifactById $tasksArray $taskById $Nodes $ArtifactRoot
    $unsatisfiedContracts = New-Object System.Collections.Generic.List[string]
    foreach ($pending in @($pendingArtifactContracts.ToArray())) {
        $contractId = [string](Get-ObjectField $pending "contractId")
        $taskId = [string](Get-ObjectField $pending "taskId")
        $isSatisfied = @($artifacts.ToArray() | Where-Object {
                [string](Get-ObjectField $_ "taskId") -eq $taskId -and
                @((Get-ObjectField $_ "outputContractIds")) -contains $contractId
            }).Count -gt 0
        if (-not $isSatisfied) {
            Add-UniqueAuditValue $unsatisfiedContracts "$taskId/$contractId"
        }
    }
    return New-FinalArtifactAuditResult $artifacts $unsatisfiedContracts $ResultById $SentinelWarnings
}

function Add-ResultArtifactsToAudit {
    param($Artifacts, [hashtable]$ArtifactById, $TasksArray, [hashtable]$TaskById, $Nodes, [string]$ArtifactRoot)
    foreach ($node in @(Get-ObjectArray $Nodes)) {
        foreach ($result in @(Get-ObjectArray (Get-ObjectField $node "results"))) {
            $taskId = [string](Get-ObjectField $result "taskId")
            if (-not $taskId -and $TasksArray.Count -eq 1) { $taskId = [string](Get-ObjectField $TasksArray[0] "id") }
            $task = if ($taskId -and $TaskById.ContainsKey($taskId)) { $TaskById[$taskId] } else { $null }
            foreach ($artifactRef in @(Get-ResultArtifactRefs $result)) {
                $artifact = Ensure-FinalArtifactRecord $Artifacts $ArtifactById ([string]$artifactRef) $taskId ([string](Get-ObjectField $result "mapId")) ([string](Get-ObjectField $node "id")) $ArtifactRoot
                Add-UniqueAuditValue $artifact.resultIds ([string](Get-ObjectField $result "resultId"))
                Add-FinalArtifactEdge $artifact "Result" ([string](Get-ObjectField $result "resultId")) "FinalArtifact" $artifact.finalArtifactId ([string](Get-ObjectField $result "validity"))
                Add-LinkedContractsToArtifact $artifact $task ([string](Get-ObjectField $result "resultId")) ([string]$artifactRef)
                Add-ResultEvidenceToArtifact $artifact $result
            }
        }
    }
}

function Add-LinkedContractsToArtifact {
    param([object]$Artifact, [object]$Task, [string]$ResultId, [string]$ArtifactRef)
    foreach ($contract in @(Get-ObjectArray (Get-ObjectField (Get-ObjectField $Task "cognitiveState") "outputContracts"))) {
        if ([string](Get-ObjectField $contract "kind") -eq "non_goal") { continue }
        $artifactRefs = @(Get-OutputContractArtifactRefs $contract)
        $resultRefs = @(Get-ResultIdsFromEvidenceRefs (Get-ObjectField $contract "evidenceRefs"))
        if ($artifactRefs.Count -gt 0 -and $artifactRefs -notcontains $ArtifactRef) { continue }
        if ($artifactRefs.Count -eq 0 -and $resultRefs -notcontains $ResultId) { continue }
        Add-UniqueAuditValue $Artifact.outputContractIds ([string](Get-ObjectField $contract "id"))
        Add-FinalArtifactEdge $Artifact "OutputContract" ([string](Get-ObjectField $contract "id")) "FinalArtifact" $Artifact.finalArtifactId
    }
}

function Add-ResultEvidenceToArtifact {
    param([object]$Artifact, [object]$Result)
    $ep = Get-ObjectField $Result "evidencePackage"
    foreach ($claim in @(Get-ObjectArray (Get-ObjectField $ep "claims"))) {
        $claimId = [string](Get-ObjectField $claim "id")
        Add-UniqueAuditValue $Artifact.claimIds $claimId
        Add-FinalArtifactEdge $Artifact "Result" ([string](Get-ObjectField $Result "resultId")) "Claim" $claimId ([string](Get-ObjectField $Result "validity"))
        foreach ($ref in @(Get-ObjectArray (Get-ObjectField $claim "evidenceRefs"))) {
            Add-EvidenceRefToFinalArtifact $Artifact "Claim" $claimId $ref
        }
    }
    foreach ($ref in @(Get-ObjectArray (Get-ObjectField $ep "evidenceRefs"))) {
        Add-EvidenceRefToFinalArtifact $Artifact "Result" ([string](Get-ObjectField $Result "resultId")) $ref
    }
    foreach ($validatorRef in @(Get-ObjectArray (Get-ObjectField $ep "validatorRefs"))) {
        Add-UniqueAuditValue $Artifact.validatorRefs ([string]$validatorRef)
        Add-FinalArtifactEdge $Artifact "Validator" ([string]$validatorRef) "Result" ([string](Get-ObjectField $Result "resultId"))
    }
}

function New-FinalArtifactAuditResult {
    param($Artifacts, $ContractWithoutArtifact, [hashtable]$ResultById, $SentinelWarnings)
    $missingWhyChain = New-Object System.Collections.Generic.List[string]
    $missingHash = New-Object System.Collections.Generic.List[string]
    $badResultDependencies = New-Object System.Collections.Generic.List[string]
    $nonAcceptedDependencies = New-Object System.Collections.Generic.List[string]
    $unclearedSentinels = New-Object System.Collections.Generic.List[string]
    foreach ($subject in $ContractWithoutArtifact) { Add-UniqueAuditValue $missingWhyChain $subject }
    foreach ($artifact in @($Artifacts.ToArray())) {
        if ([string]::IsNullOrWhiteSpace([string]$artifact.artifactHash)) { Add-UniqueAuditValue $missingHash ([string]$artifact.finalArtifactId) }
        if ($artifact.outputContractIds.Count -eq 0 -or $artifact.resultIds.Count -eq 0 -or $artifact.claimIds.Count -eq 0 -or $artifact.evidenceRefIds.Count -eq 0 -or ($artifact.validatorRefs.Count + $artifact.factSourceIds.Count) -eq 0) {
            Add-UniqueAuditValue $missingWhyChain ([string]$artifact.finalArtifactId)
        }
        foreach ($resultId in @($artifact.resultIds)) {
            if ($ResultById.ContainsKey($resultId)) {
                $validity = [string](Get-ObjectField $ResultById[$resultId] "validity")
                if ($validity -ne "accepted") { Add-UniqueAuditValue $nonAcceptedDependencies "$($artifact.finalArtifactId)->${resultId}:$validity" }
                if ($validity -in @("questioned", "invalid")) { Add-UniqueAuditValue $badResultDependencies "$($artifact.finalArtifactId)->$resultId" }
            }
            else {
                Add-UniqueAuditValue $nonAcceptedDependencies "$($artifact.finalArtifactId)->${resultId}:missing"
            }
            Add-UnclearedSentinelSubjects $artifact $resultId $SentinelWarnings $unclearedSentinels
        }
    }
    $gateRecords = New-Object System.Collections.Generic.List[object]
    Add-AuditGateRecord $gateRecords "audit_why_chain_missing" ($missingWhyChain.Count -eq 0) "final artifacts have output contract, result, claim, evidence, and validator/fact source joins" "missingWhyChain=$($missingWhyChain.Count)" @($missingWhyChain.ToArray())
    Add-AuditGateRecord $gateRecords "final_artifact_hash_missing" ($missingHash.Count -eq 0) "each final artifact path resolves to a SHA-256 hash" "missingHash=$($missingHash.Count)" @($missingHash.ToArray())
    Add-AuditGateRecord $gateRecords "non_accepted_final_artifact_dependency" ($nonAcceptedDependencies.Count -eq 0) "final artifact dependencies use accepted results only" "nonAcceptedDependencies=$($nonAcceptedDependencies.Count)" @($nonAcceptedDependencies.ToArray())
    Add-AuditGateRecord $gateRecords "questioned_or_invalid_final_artifact_dependency" ($badResultDependencies.Count -eq 0) "final artifact dependencies do not use questioned/invalid results" "badDependencies=$($badResultDependencies.Count)" @($badResultDependencies.ToArray())
    Add-AuditGateRecord $gateRecords "sentinel_warning_uncleared_for_final_artifact" ($unclearedSentinels.Count -eq 0) "sentinel warnings affecting final artifacts are cleared" "unclearedSentinels=$($unclearedSentinels.Count)" @($unclearedSentinels.ToArray())
    $failures = @($gateRecords.ToArray() | Where-Object { $_.pass -ne $true } | ForEach-Object { [string]$_.gateId })
    return [ordered]@{
        hardGatePassed = ($failures.Count -eq 0)
        hardGateFailures = @($failures)
        gateRecords = @($gateRecords.ToArray())
        finalArtifacts = @($Artifacts.ToArray())
        metrics = [ordered]@{
            finalArtifactCount = $Artifacts.Count
            finalArtifactMissingHashCount = $missingHash.Count
            finalArtifactMissingWhyChainCount = $missingWhyChain.Count
            nonAcceptedFinalArtifactDependencyCount = $nonAcceptedDependencies.Count
            questionedOrInvalidFinalArtifactDependencyCount = $badResultDependencies.Count
            unclearedFinalArtifactSentinelCount = $unclearedSentinels.Count
        }
    }
}

function Add-UnclearedSentinelSubjects {
    param([object]$Artifact, [string]$ResultId, $SentinelWarnings, [object]$UnclearedSentinels)
    foreach ($warning in @(Get-ObjectArray $SentinelWarnings)) {
        if ([string](Get-ObjectField $warning "status") -eq "cleared") { continue }
        if ([string](Get-ObjectField $warning "resultId") -eq $ResultId -or @($Artifact.nodeIds) -contains [string](Get-ObjectField $warning "nodeId")) {
            Add-UniqueAuditValue $Artifact.sentinelIds ([string](Get-ObjectField $warning "id"))
            Add-UniqueAuditValue $UnclearedSentinels "$($Artifact.finalArtifactId)->$([string](Get-ObjectField $warning 'id'))"
        }
    }
}
