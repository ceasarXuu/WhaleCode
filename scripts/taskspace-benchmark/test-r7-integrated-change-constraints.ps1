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

Assert-ExactIds @($contract.architectural_constraints) "C" 16
Assert-ExactIds @($contract.regression_invariants) "R" 20
Assert-ExactIds @($contract.candidate_gates) "G" 13
Assert-ExactIds @($contract.rejected_directions) "D" 10

$documentPath = Join-Path $repoRoot ([string]$contract.governing_document.path)
$documentHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $documentPath).Hash.ToLowerInvariant()
Assert-Contract ($documentHash -eq [string]$contract.governing_document.sha256) "Integrated constraint document hash drifted"

$closed = @($contract.regression_invariants | Where-Object { [string]$_.status -eq "closed" })
$open = @($contract.regression_invariants | Where-Object { [string]$_.status -eq "open" })
Assert-Contract ($closed.Count -eq 18) "Exactly 18 historical regressions must remain closed"
Assert-Contract (($open.id -join ",") -eq "R-19,R-20") "R-19 and R-20 must remain open until the role-separated compact wire passes the full gate"

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
Assert-Contract ([string]$costRegression.required_behavior -match "55578") "Current schema cost baseline is missing"
$roleRegression = @($contract.regression_invariants | Where-Object { [string]$_.id -eq "R-20" })[0]
Assert-Contract ([string]$roleRegression.required_behavior -match "structurally distinct") "Initialization role-structure invariant is missing"

$document = Get-Content -Raw -Encoding UTF8 -LiteralPath $documentPath
foreach ($id in @(
    @($contract.architectural_constraints | ForEach-Object { [string]$_.id })
    @($contract.regression_invariants | ForEach-Object { [string]$_.id })
)) {
    Assert-Contract ($document.Contains($id)) "Governing document does not contain $id"
}

Write-Output "R7 integrated change constraints: PASS"
