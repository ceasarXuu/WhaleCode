$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$contractPath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-integrated-change-constraints-v1.json"
$schemaPath = Join-Path $repoRoot "benchmarks/taskspace/r7/five-layer-integrated-change-constraints-v1.schema.json"

function Assert-Contract {
    param(
        [bool]$Condition,
        [string]$Message
    )
    if (-not $Condition) {
        throw $Message
    }
}

function Assert-ExactIds {
    param(
        [object[]]$Items,
        [string]$Prefix,
        [int]$Count
    )
    $actual = @($Items | ForEach-Object { [string]$_.id })
    $expected = @(1..$Count | ForEach-Object { "{0}-{1:D2}" -f $Prefix, $_ })
    Assert-Contract (($actual -join ",") -eq ($expected -join ",")) "$Prefix ids are incomplete, duplicated, or out of order"
}

$raw = Get-Content -Raw -Encoding UTF8 -LiteralPath $contractPath
Assert-Contract ($raw | Test-Json -SchemaFile $schemaPath -ErrorAction Stop) "Integrated constraint contract does not match its schema"
$contract = $raw | ConvertFrom-Json -Depth 40

Assert-ExactIds @($contract.architectural_constraints) "C" 19
Assert-ExactIds @($contract.regression_invariants) "R" 25
Assert-ExactIds @($contract.candidate_gates) "G" 19
Assert-ExactIds @($contract.rejected_directions) "D" 13

$documentPath = Join-Path $repoRoot ([string]$contract.governing_document.path)
$documentHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $documentPath).Hash.ToLowerInvariant()
Assert-Contract ($documentHash -eq [string]$contract.governing_document.sha256) "Integrated constraint document hash drifted"

$closed = @($contract.regression_invariants | Where-Object { [string]$_.status -eq "closed" })
$open = @($contract.regression_invariants | Where-Object { [string]$_.status -eq "open" })
Assert-Contract ($closed.Count -eq 20) "Exactly 20 historical regressions must remain closed"
Assert-Contract (($open.id -join ",") -eq "R-10,R-19,R-22,R-24,R-25") "R-10, R-19, R-22, R-24, and R-25 must remain open until all continuation gates pass"

$knownIds = @{}
foreach ($item in @($contract.architectural_constraints) + @($contract.regression_invariants)) {
    $knownIds[[string]$item.id] = $true
}
foreach ($direction in @($contract.rejected_directions)) {
    foreach ($violation in @($direction.violates)) {
        Assert-Contract $knownIds.ContainsKey([string]$violation) "Rejected direction $($direction.id) references unknown constraint $violation"
    }
}

$dynamicSchema = @($contract.rejected_directions | Where-Object { [string]$_.id -eq "D-01" })[0]
Assert-Contract (@($dynamicSchema.violates) -contains "C-06") "Dynamic schema rejection must be bound to immutable capability epochs"

$costRegression = @($contract.regression_invariants | Where-Object { [string]$_.id -eq "R-19" })[0]
Assert-Contract ([string]$costRegression.problem -match "46926") "Current schema cost baseline is missing"
$roleRegression = @($contract.regression_invariants | Where-Object { [string]$_.id -eq "R-20" })[0]
Assert-Contract ([string]$roleRegression.required_behavior -match "structurally distinct") "Initialization role-structure invariant is missing"
Assert-Contract ([string]$roleRegression.status -eq "closed") "Role-separated initialization regression must remain closed"
$continuousRegression = @($contract.regression_invariants | Where-Object { [string]$_.id -eq "R-10" })[0]
Assert-Contract ([string]$continuousRegression.required_behavior -match "one response") "Continuous-action response grammar gate is missing"
$ordinaryToolRegression = @($contract.regression_invariants | Where-Object { [string]$_.id -eq "R-13" })[0]
Assert-Contract ([string]$ordinaryToolRegression.required_behavior -match "byte-identical native schema") "Ordinary Tool fidelity gate is missing"
$subagentRegression = @($contract.regression_invariants | Where-Object { [string]$_.id -eq "R-21" })[0]
Assert-Contract ([string]$subagentRegression.required_behavior -match "same persisted canonical Map") "TaskSpace subagent persistent handoff gate is missing"
$operationDriftRegression = @($contract.regression_invariants | Where-Object { [string]$_.id -eq "R-22" })[0]
Assert-Contract ([string]$operationDriftRegression.required_behavior -match "multi-Patch") "Map-request operation drift gate is missing"
$mapOwnershipRegression = @($contract.regression_invariants | Where-Object { [string]$_.id -eq "R-23" })[0]
Assert-Contract ([string]$mapOwnershipRegression.required_behavior -match "independently persisted Map Store") "Persistent canonical Map ownership gate is missing"
$actionOwnershipRegression = @($contract.regression_invariants | Where-Object { [string]$_.id -eq "R-24" })[0]
Assert-Contract ([string]$actionOwnershipRegression.required_behavior -match "declares node_id per ordinary action") "Agent-authored per-action ownership gate is missing"
$derivedLifecycleRegression = @($contract.regression_invariants | Where-Object { [string]$_.id -eq "R-25" })[0]
Assert-Contract ([string]$derivedLifecycleRegression.required_behavior -match "derives Waiting, Ready, InFlight, Blocked, Completed") "Fact-derived lifecycle gate is missing"

$document = Get-Content -Raw -Encoding UTF8 -LiteralPath $documentPath
foreach ($id in @(
    @($contract.architectural_constraints | ForEach-Object { [string]$_.id })
    @($contract.regression_invariants | ForEach-Object { [string]$_.id })
)) {
    Assert-Contract ($document.Contains($id)) "Governing document does not contain $id"
}

Write-Output "R7 integrated change constraints: PASS"
