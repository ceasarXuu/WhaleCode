param(
    [string]$LedgerPath = (
        Join-Path (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path `
            "benchmarks/whale-agent-run-ledger.json"
    )
)

$ErrorActionPreference = "Stop"

function Assert-Ledger {
    param(
        [bool]$Condition,
        [string]$Message
    )

    if (-not $Condition) {
        throw $Message
    }
}

function Test-NonnegativeInteger {
    param([object]$Value)

    $isInteger = $Value -is [byte] -or
        $Value -is [int16] -or
        $Value -is [int32] -or
        $Value -is [int64] -or
        $Value -is [uint16] -or
        $Value -is [uint32] -or
        $Value -is [uint64]

    return $isInteger -and [decimal]$Value -ge 0
}

Assert-Ledger (Test-Path -LiteralPath $LedgerPath -PathType Leaf) `
    "Whale Agent run ledger is missing: $LedgerPath"

$ledger = Get-Content -Raw -Encoding UTF8 -LiteralPath $LedgerPath |
    ConvertFrom-Json -Depth 100

Assert-Ledger ($ledger.schema_version -ceq "whale-agent-run-ledger-v1") `
    "Whale Agent run ledger schema_version drifted"
Assert-Ledger ($null -ne $ledger.entries) "Whale Agent run ledger entries are missing"

$recordIds = [System.Collections.Generic.HashSet[string]]::new(
    [System.StringComparer]::Ordinal
)

foreach ($entry in @($ledger.entries)) {
    $id = [string]$entry.record_id
    Assert-Ledger (-not [string]::IsNullOrWhiteSpace($id)) "Ledger record_id is empty"
    Assert-Ledger ($recordIds.Add($id)) "Duplicate ledger record_id: $id"
    Assert-Ledger (-not [string]::IsNullOrWhiteSpace([string]$entry.reason)) `
        "$id reason is empty"

    $execution = $entry.execution
    foreach ($field in @(
        "batch_count",
        "repeats_per_arm_per_sample",
        "planned_sample_runs",
        "actual_sample_runs",
        "api_requests"
    )) {
        Assert-Ledger (Test-NonnegativeInteger $execution.$field) `
            "$id execution.$field is not a nonnegative integer"
    }

    $tokens = $entry.tokens
    foreach ($field in @("input", "cached_input", "uncached_input", "output")) {
        Assert-Ledger (Test-NonnegativeInteger $tokens.$field) `
            "$id tokens.$field is not a nonnegative integer"
    }
    Assert-Ledger (
        [int64]$tokens.input -eq
        ([int64]$tokens.cached_input + [int64]$tokens.uncached_input)
    ) "$id input token identity is inconsistent"

    $isLargeRun = [int64]$execution.planned_sample_runs -gt 3
    $isHistorical = [string]$entry.record_type -ceq "historical_aggregate"
    if ($isLargeRun -and -not $isHistorical) {
        Assert-Ledger ($entry.authorization.required -eq $true) `
            "$id large run does not require authorization"
        Assert-Ledger ([string]$entry.authorization.status -ceq "granted") `
            "$id large run has no granted budget"
        Assert-Ledger (
            -not [string]::IsNullOrWhiteSpace([string]$entry.authorization.reference)
        ) "$id large run has no authorization reference"
    }

    $cost = $entry.monetary_cost
    Assert-Ledger (
        @("planned", "estimated", "actual", "unavailable") -ccontains [string]$cost.status
    ) "$id monetary cost status is invalid"
    if ([string]$cost.status -cin @("estimated", "actual")) {
        Assert-Ledger ($null -ne $cost.amount -and [decimal]$cost.amount -ge 0) `
            "$id settled monetary amount is missing"
        Assert-Ledger ($null -ne $cost.pricing_snapshot) `
            "$id settled monetary cost has no pricing snapshot"
        Assert-Ledger (-not [string]::IsNullOrWhiteSpace([string]$cost.formula)) `
            "$id settled monetary cost has no formula"
    }

    if ([string]$entry.status -cne "planned") {
        Assert-Ledger (-not [string]::IsNullOrWhiteSpace([string]$entry.started_at)) `
            "$id settled record has no started_at"
        Assert-Ledger (-not [string]::IsNullOrWhiteSpace([string]$entry.ended_at)) `
            "$id settled record has no ended_at"
    }
}

Write-Host ("Whale Agent run ledger passed: {0} entries" -f $recordIds.Count)
