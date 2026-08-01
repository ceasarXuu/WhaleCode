function Update-R7TestPhaseReference {
    param([string]$Id, [int]$InsertNumber)
    if ($null -eq $Id) {
        return $null
    }
    $number = [int]$Id.Substring(4)
    if ($number -ge $InsertNumber) {
        return "R71-{0:D2}" -f ($number + 1)
    }
    $Id
}

function Add-R7TestPlanPhaseAt {
    param($Plan, [int]$InsertNumber)
    $expanded = Copy-Plan $Plan
    foreach ($phase in @($expanded.phases)) {
        $phase.id = Update-R7TestPhaseReference ([string]$phase.id) $InsertNumber
        $phase.depends_on = @($phase.depends_on | ForEach-Object {
            Update-R7TestPhaseReference ([string]$_) $InsertNumber
        })
        if ($null -ne $phase.parent_diagnosis_id) {
            $phase.parent_diagnosis_id = Update-R7TestPhaseReference `
                ([string]$phase.parent_diagnosis_id) $InsertNumber
        }
        foreach ($repair in @($phase.spawned_repairs)) {
            $repair.phase_id = Update-R7TestPhaseReference `
                ([string]$repair.phase_id) $InsertNumber
        }
    }
    foreach ($property in @(
            "current_phase_id", "dynamic_cost_phase_id", "candidate_freeze_phase_id",
            "formal_evaluation_phase_id", "promotion_decision_phase_id"
        )) {
        $expanded.$property = Update-R7TestPhaseReference `
            ([string]$expanded.$property) $InsertNumber
    }
    foreach ($property in @(
            "nested_dispatch_boundary", "multi_patch_runtime_safety",
            "multi_patch_agent_behavior"
        )) {
        $expanded.route_role_phase_ids.$property = Update-R7TestPhaseReference `
            ([string]$expanded.route_role_phase_ids.$property) $InsertNumber
    }
    $expanded.held_out_sets.engineering.owner_phase_id = Update-R7TestPhaseReference `
        ([string]$expanded.held_out_sets.engineering.owner_phase_id) $InsertNumber
    $expanded.held_out_sets.promotion.owner_phase_id = Update-R7TestPhaseReference `
        ([string]$expanded.held_out_sets.promotion.owner_phase_id) $InsertNumber

    $newPhase = @'
{
  "id":"R71-00",
  "title":"正向插入夹具",
  "kind":"implementation",
  "severity":"high",
  "status":"planned",
  "root_ids":["R71-GI-003"],
  "depends_on":["R71-01"],
  "change_domain_key":"fixture.inserted_phase",
  "parent_diagnosis_id":null,
  "allowed_closure_outcomes":["implemented"],
  "closure_outcome":"pending",
  "evidence_artifact":null,
  "spawned_repairs":[],
  "failure_route":null,
  "acceptance_evidence_type":"insertion_fixture",
  "observability":{
    "mode":"artifact",
    "event_name":"r71_insertion_fixture",
    "required_fields":["fixture_id"]
  }
}
'@ | ConvertFrom-Json -Depth 100 -NoEnumerate
    $newPhase.id = "R71-{0:D2}" -f $InsertNumber
    $newPhase.observability.event_name = "r71_insertion_fixture_$InsertNumber"
    $before = @($expanded.phases | Where-Object {
        [int]([string]$_.id).Substring(4) -lt $InsertNumber
    })
    $after = @($expanded.phases | Where-Object {
        [int]([string]$_.id).Substring(4) -gt $InsertNumber
    })
    $expanded.phases = @($before) + @($newPhase) + @($after)
    $fixedCost = $expanded.phases | Where-Object {
        [string]$_.acceptance_evidence_type -eq "fixed_component_ledger"
    }
    $fixedCost.depends_on = @($fixedCost.depends_on) + @($newPhase.id)
    $expanded.phase_count = @($expanded.phases).Count
    $expanded
}

function New-R7TestEvidenceReference {
    param([string]$ArtifactType, [string]$Name, [string[]]$RequiredFields)
    $relativePath = "target/r7-execution-plan-selftest/$Name.json"
    $path = Join-Path $repoRoot $relativePath
    [void](New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path))
    $record = [ordered]@{}
    foreach ($field in $RequiredFields) {
        $record[$field] = "fixture"
    }
    [ordered]@{
        schema_version = "r71-phase-evidence-v1"
        artifact_type = $ArtifactType
        records = @($record)
    } | ConvertTo-Json -Compress |
        Set-Content -NoNewline -Encoding UTF8 -LiteralPath $path
    [pscustomobject]@{
        path = $relativePath
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
        artifact_type = $ArtifactType
        schema_path = "benchmarks/taskspace/r7/r7-phase-evidence-v1.schema.json"
        schema_version = "r71-phase-evidence-v1"
    }
}
