function Get-TaskspaceEvidenceRefCount {
    param([object]$Item)
    if ($null -eq $Item) { return 0 }
    $count = 0
    if ($Item.PSObject.Properties.Name -contains "evidenceRefs") {
        $count += @($Item.evidenceRefs).Count
    }
    if ($Item.PSObject.Properties.Name -contains "evidencePackage" -and $Item.evidencePackage) {
        $pkg = $Item.evidencePackage
        if ($pkg.PSObject.Properties.Name -contains "evidenceRefs") { $count += @($pkg.evidenceRefs).Count }
        if ($pkg.PSObject.Properties.Name -contains "validatorRefs") { $count += @($pkg.validatorRefs).Count }
        if ($pkg.PSObject.Properties.Name -contains "changedArtifacts") { $count += @($pkg.changedArtifacts).Count }
    }
    [int]$count
}

function New-TaskspaceManagedItem {
    param(
        [Parameter(Mandatory = $true)][string]$ItemType,
        [Parameter(Mandatory = $true)][string]$Id,
        [AllowEmptyString()][string]$MapId = "",
        [AllowEmptyString()][string]$TaskId = "",
        [AllowEmptyString()][string]$Status = "",
        [AllowEmptyString()][string]$Validity = "",
        [AllowEmptyString()][string]$Kind = "",
        [int]$EvidenceRefCount = 0,
        [int]$ValidatorRefCount = 0,
        [int]$ChangedArtifactCount = 0,
        [bool]$HasOutputRef = $false,
        [bool]$HasFailure = $false
    )
    $protectedReason = ""
    $retention = "retained"
    $salience = 0.35
    if ($ItemType -in @("success_criterion", "output_contract", "fact_source", "fact", "decision")) {
        $retention = "active"
        $salience = 0.80
        $protectedReason = "problem_ledger_active"
    } elseif ($ItemType -eq "node") {
        if ($Kind -in @("task_root", "finish")) {
            $retention = "retained"
            $salience = 0.85
            $protectedReason = "rooted_graph_skeleton"
        } elseif ($Status -in @("ready", "running", "blocked")) {
            $retention = "active"
            $salience = 0.75
            $protectedReason = "open_or_blocked_node"
        } elseif ($Status -eq "completed") {
            $retention = "retained"
            $salience = 0.45
        } else {
            $retention = "archived"
            $salience = 0.25
        }
    } elseif ($ItemType -eq "result") {
        if ($Validity -eq "accepted") {
            $retention = "retained"
            $salience = 0.65
            if ($EvidenceRefCount -gt 0 -or $ValidatorRefCount -gt 0 -or $ChangedArtifactCount -gt 0) {
                $protectedReason = "accepted_evidence"
                $salience = 0.85
            }
        } elseif ($Validity -in @("questioned", "invalid") -or $HasFailure) {
            $retention = "active"
            $salience = 0.90
            $protectedReason = "negative_or_failed_evidence"
        } elseif ($Kind -eq "main_tool_call") {
            $retention = if ($HasOutputRef) { "audit_only" } else { "archived" }
            $salience = if ($HasOutputRef) { 0.30 } else { 0.20 }
        } else {
            $retention = "archived"
            $salience = 0.25
        }
    }
    [pscustomobject]@{
        schema_version = "taskspace-map-managed-item-v1"
        item_type = $ItemType
        id = $Id
        map_id = $MapId
        task_id = $TaskId
        status = $Status
        validity = $Validity
        kind = $Kind
        retention_class = $retention
        base_salience = [Math]::Round([double]$salience, 4)
        protected_reason = $protectedReason
        evidence_ref_count = [int]$EvidenceRefCount
        validator_ref_count = [int]$ValidatorRefCount
        changed_artifact_count = [int]$ChangedArtifactCount
        has_output_ref = [bool]$HasOutputRef
    }
}

function Get-TaskspaceMapManagedItems {
    param([Parameter(Mandatory = $true)]$Observability)
    $items = New-Object System.Collections.Generic.List[object]
    foreach ($task in @($Observability.tasks)) {
        $items.Add((New-TaskspaceManagedItem "task" ([string]$task.id) ([string]$task.activeMapId) ([string]$task.id) ([string]$task.status) "" "" 0 0 0 $false $false))
        $ledger = $task.problemLedger
        if ($ledger) {
            foreach ($row in @($ledger.successCriteria)) {
                $items.Add((New-TaskspaceManagedItem "success_criterion" ([string]$row.id) ([string]$task.activeMapId) ([string]$task.id) ([string]$row.status) "" ([string]$row.kind) (Get-TaskspaceEvidenceRefCount $row) 0 0 $false $false))
            }
            foreach ($row in @($ledger.knownFacts)) {
                $items.Add((New-TaskspaceManagedItem "fact" ([string]$row.id) ([string]$task.activeMapId) ([string]$task.id) "" "" "" (Get-TaskspaceEvidenceRefCount $row) 0 0 $false $false))
            }
            foreach ($row in @($ledger.decisions)) {
                $items.Add((New-TaskspaceManagedItem "decision" ([string]$row.id) ([string]$task.activeMapId) ([string]$task.id) "" "" "" (Get-TaskspaceEvidenceRefCount $row) 0 0 $false $false))
            }
        }
        $state = $task.cognitiveState
        if ($state) {
            foreach ($row in @($state.outputContracts)) {
                $items.Add((New-TaskspaceManagedItem "output_contract" ([string]$row.id) ([string]$task.activeMapId) ([string]$task.id) "" "" ([string]$row.kind) (Get-TaskspaceEvidenceRefCount $row) 0 0 $false $false))
            }
            foreach ($row in @($state.factSources)) {
                $items.Add((New-TaskspaceManagedItem "fact_source" ([string]$row.id) ([string]$task.activeMapId) ([string]$task.id) "" "" ([string]$row.provenance) (Get-TaskspaceEvidenceRefCount $row) 0 0 $false $false))
            }
            foreach ($row in @($state.facts)) {
                $items.Add((New-TaskspaceManagedItem "fact" ([string]$row.id) ([string]$task.activeMapId) ([string]$task.id) "" "" "" (Get-TaskspaceEvidenceRefCount $row) 0 0 $false $false))
            }
        }
    }
    foreach ($node in @($Observability.nodes)) {
        $items.Add((New-TaskspaceManagedItem "node" ([string]$node.id) ([string]$node.mapId) ([string]$node.taskId) ([string]$node.status) "" ([string]$node.kind) 0 0 0 $false $false))
        foreach ($result in @($node.results)) {
            $pkg = $result.evidencePackage
            $validatorCount = if ($pkg -and $pkg.PSObject.Properties.Name -contains "validatorRefs") { @($pkg.validatorRefs).Count } else { 0 }
            $changedCount = if ($pkg -and $pkg.PSObject.Properties.Name -contains "changedArtifacts") { @($pkg.changedArtifacts).Count } else { 0 }
            $artifactRefs = @($result.artifactRefs)
            $hasOutputRef = @($artifactRefs | Where-Object { [string]$_ -match "^output-ref://" }).Count -gt 0
            $hasFailure = ($result.PSObject.Properties.Name -contains "success" -and $false -eq [bool]$result.success)
            $items.Add((New-TaskspaceManagedItem "result" ([string]$result.resultId) ([string]$result.mapId) ([string]$result.taskId) "" ([string]$result.validity) ([string]$result.kind) (Get-TaskspaceEvidenceRefCount $result) $validatorCount $changedCount $hasOutputRef $hasFailure))
        }
    }
    @($items.ToArray() | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.id) })
}

function New-TaskspaceMapCompactionEvents {
    param([object[]]$ManagedItems)
    $events = New-Object System.Collections.Generic.List[object]
    foreach ($item in @($ManagedItems)) {
        if ([string]$item.retention_class -in @("archived", "audit_only")) {
            $events.Add([pscustomobject]@{
                schema_version = "taskspace-compaction-event-v1"
                event_kind = if ([string]$item.retention_class -eq "audit_only") { "archive_to_audit_only" } else { "archive_from_active_projection" }
                item_type = [string]$item.item_type
                item_id = [string]$item.id
                retention_class = [string]$item.retention_class
                base_salience = [double]$item.base_salience
                physical_delete = $false
                reason = if ([bool]$item.has_output_ref) { "large_output_kept_by_artifact_ref" } else { "low_salience_unprotected_item" }
            })
        }
    }
    @($events.ToArray())
}

function Write-TaskspaceMapManagementArtifacts {
    param(
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [AllowEmptyString()][string]$ObservabilityJsonPath = ""
    )
    $summaryPath = Join-Path $ArtifactDir "map-management-summary.json"
    $eventsPath = Join-Path $ArtifactDir "compaction-events.jsonl"
    if ([string]::IsNullOrWhiteSpace($ObservabilityJsonPath) -or -not (Test-Path -LiteralPath $ObservabilityJsonPath)) {
        $summary = [pscustomobject]@{
            schema_version = "taskspace-map-management-summary-v1"
            availability = "source_missing"
            source_path = $ObservabilityJsonPath
            total_item_count = 0
            retention_coverage_ratio = $null
            salience_coverage_ratio = $null
            protected_item_count = 0
            protected_miss_count = 0
            archived_item_count = 0
            audit_only_item_count = 0
            semantic_replacement_rate = $null
            compaction_event_count = 0
            events_path = $eventsPath
            items = @()
        }
        Write-TaskspaceJson $summary $summaryPath
        Set-Content -LiteralPath $eventsPath -Encoding UTF8 -Value ""
        return $summary
    }
    $obs = Get-Content -Raw -Encoding UTF8 -LiteralPath $ObservabilityJsonPath | ConvertFrom-Json
    $items = @(Get-TaskspaceMapManagedItems $obs)
    $events = @(New-TaskspaceMapCompactionEvents $items)
    if ($events.Count -gt 0) {
        @($events | ForEach-Object { $_ | ConvertTo-Json -Compress -Depth 8 }) | Set-Content -LiteralPath $eventsPath -Encoding UTF8
    } else {
        Set-Content -LiteralPath $eventsPath -Encoding UTF8 -Value ""
    }
    $total = @($items).Count
    $retentionCovered = @($items | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.retention_class) }).Count
    $salienceCovered = @($items | Where-Object { $null -ne $_.base_salience }).Count
    $protected = @($items | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.protected_reason) })
    $archived = @($items | Where-Object { [string]$_.retention_class -eq "archived" }).Count
    $auditOnly = @($items | Where-Object { [string]$_.retention_class -eq "audit_only" }).Count
    $summary = [pscustomobject]@{
        schema_version = "taskspace-map-management-summary-v1"
        availability = "measured"
        source_path = $ObservabilityJsonPath
        total_item_count = [int]$total
        retention_coverage_ratio = if ($total -gt 0) { [Math]::Round([double]$retentionCovered / [double]$total, 4) } else { $null }
        salience_coverage_ratio = if ($total -gt 0) { [Math]::Round([double]$salienceCovered / [double]$total, 4) } else { $null }
        protected_item_count = [int]@($protected).Count
        protected_miss_count = [int]@($protected | Where-Object { [string]$_.retention_class -in @("archived", "audit_only") }).Count
        archived_item_count = [int]$archived
        audit_only_item_count = [int]$auditOnly
        semantic_replacement_rate = if ($total -gt 0) { [Math]::Round([double]($archived + $auditOnly) / [double]$total, 4) } else { $null }
        compaction_event_count = [int]@($events).Count
        events_path = $eventsPath
        items = @($items)
    }
    Write-TaskspaceJson $summary $summaryPath
    $summary
}

function Write-TaskspaceSuiteMapManagementSummary {
    param([Parameter(Mandatory = $true)][string]$RootDir)
    $summaryPath = Join-Path $RootDir "suite-map-management-summary.json"
    $files = @(Get-ChildItem -LiteralPath $RootDir -Filter "map-management-summary.json" -Recurse -ErrorAction SilentlyContinue | Sort-Object FullName)
    $rows = New-Object System.Collections.Generic.List[object]
    foreach ($file in $files) {
        try {
            $summary = Get-Content -Raw -Encoding UTF8 -LiteralPath $file.FullName | ConvertFrom-Json
            if ([string]$summary.availability -ne "measured") { continue }
            $rows.Add([pscustomobject]@{
                path = $file.FullName
                total_item_count = [int]$summary.total_item_count
                retention_coverage_ratio = [double]$summary.retention_coverage_ratio
                salience_coverage_ratio = [double]$summary.salience_coverage_ratio
                protected_item_count = [int]$summary.protected_item_count
                protected_miss_count = [int]$summary.protected_miss_count
                archived_item_count = [int]$summary.archived_item_count
                audit_only_item_count = [int]$summary.audit_only_item_count
                semantic_replacement_rate = [double]$summary.semantic_replacement_rate
                compaction_event_count = [int]$summary.compaction_event_count
            })
        } catch {}
    }
    $count = [int]$rows.Count
    $totalItems = 0
    $protectedItems = 0
    $protectedMiss = 0
    $archived = 0
    $auditOnly = 0
    $compaction = 0
    foreach ($row in @($rows.ToArray())) {
        $totalItems += [int]$row.total_item_count
        $protectedItems += [int]$row.protected_item_count
        $protectedMiss += [int]$row.protected_miss_count
        $archived += [int]$row.archived_item_count
        $auditOnly += [int]$row.audit_only_item_count
        $compaction += [int]$row.compaction_event_count
    }
    $minRetention = $null
    $minSalience = $null
    if ($count -gt 0) {
        $retentionMeasure = $rows | Measure-Object -Property retention_coverage_ratio -Minimum
        $salienceMeasure = $rows | Measure-Object -Property salience_coverage_ratio -Minimum
        $minRetention = [double]$retentionMeasure.Minimum
        $minSalience = [double]$salienceMeasure.Minimum
    }
    $summary = [pscustomobject]@{
        schema_version = "taskspace-suite-map-management-summary-v1"
        availability = if ($count -gt 0) { "measured" } else { "source_missing" }
        source_summary_count = [int]$count
        total_item_count = [int]$totalItems
        min_retention_coverage_ratio = $minRetention
        min_salience_coverage_ratio = $minSalience
        protected_item_count = [int]$protectedItems
        protected_miss_count = [int]$protectedMiss
        archived_item_count = [int]$archived
        audit_only_item_count = [int]$auditOnly
        semantic_replacement_rate = if ($totalItems -gt 0) { [Math]::Round([double]($archived + $auditOnly) / [double]$totalItems, 4) } else { $null }
        compaction_event_count = [int]$compaction
        summaries = @($rows.ToArray())
    }
    Write-TaskspaceJson $summary $summaryPath
    $summary
}
