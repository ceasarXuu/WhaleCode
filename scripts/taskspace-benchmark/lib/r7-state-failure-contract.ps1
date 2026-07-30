function Test-R7FailureStringArray {
    param($Value, [bool]$AllowEmpty = $true)
    if ($Value -isnot [System.Array]) { return $false }
    if (-not $AllowEmpty -and $Value.Count -eq 0) { return $false }
    foreach ($item in $Value) {
        if ($item -isnot [string] -or
            [string]::IsNullOrWhiteSpace([string]$item)) {
            return $false
        }
    }
    $true
}

function Get-R7FailureRawProperty {
    param($Object, [string]$Name)
    if ($null -eq $Object) { return $null }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) { return $null }
    ,$property.Value
}

function Get-R7StateViolationShapeError {
    param($Violation, [int]$Ordinal)
    $prefix = "state violation[$Ordinal]"
    if ($Violation -isnot [pscustomobject]) {
        return "$prefix must be an object"
    }
    $code = Get-R7JsonProperty $Violation "code"
    if ($code -isnot [string] -or
        [string]::IsNullOrWhiteSpace([string]$code)) {
        return "$prefix code must be a non-empty string"
    }
    if (-not (Test-R7FailureStringArray (
                Get-R7FailureRawProperty $Violation "subjects"
            ) $false)) {
        return "$prefix subjects must be a non-empty string array"
    }
    if ([string]$code -ne "node_state_invalid") { return "" }

    $nodeId = Get-R7JsonProperty $Violation "node_id"
    if ($nodeId -isnot [string] -or
        [string]::IsNullOrWhiteSpace([string]$nodeId)) {
        return "$prefix node_id must be a non-empty string"
    }
    $canonical = Get-R7JsonProperty $Violation "canonical_before_transaction"
    $candidate = Get-R7JsonProperty $Violation "rejected_candidate_at_violation"
    if ($canonical -isnot [pscustomobject]) {
        return "$prefix canonical_before_transaction must be an object"
    }
    if ($candidate -isnot [pscustomobject]) {
        return "$prefix rejected_candidate_at_violation must be an object"
    }
    if ((Get-R7JsonProperty $canonical "node_present") -isnot [bool]) {
        return "$prefix canonical node_present must be boolean"
    }
    $canonicalState = Get-R7JsonProperty $canonical "state"
    if ([bool]$canonical.node_present -and
        ($canonicalState -isnot [string] -or
            [string]::IsNullOrWhiteSpace([string]$canonicalState))) {
        return "$prefix canonical state must identify the present node state"
    }
    if (-not (Test-R7FailureStringArray (
                Get-R7FailureRawProperty $canonical "unsatisfied_predecessor_ids"
            ))) {
        return "$prefix canonical unsatisfied_predecessor_ids must be a string array"
    }
    if ((Get-R7JsonProperty $candidate "committed") -isnot [bool] -or
        [bool]$candidate.committed) {
        return "$prefix candidate committed must be boolean false"
    }
    $candidateState = Get-R7JsonProperty $candidate "state"
    if ($candidateState -isnot [string] -or
        [string]::IsNullOrWhiteSpace([string]$candidateState)) {
        return "$prefix candidate state must be a non-empty string"
    }
    if (-not (Test-R7FailureStringArray (
                Get-R7FailureRawProperty $candidate "allowed_states"
            ) $false)) {
        return "$prefix candidate allowed_states must be a non-empty string array"
    }
    if (-not (Test-R7FailureStringArray (
                Get-R7FailureRawProperty $candidate "unsatisfied_predecessor_ids"
            ))) {
        return "$prefix candidate unsatisfied_predecessor_ids must be a string array"
    }
    ""
}

function Get-R7StateFailureShapeError {
    param($Payload)
    $schemaVersion = [string](Get-R7JsonProperty $Payload "schema_version" "")
    $error = Get-R7JsonProperty $Payload "error"
    if ($error -isnot [pscustomobject] -or
        [string](Get-R7JsonProperty $error "class" "") -ne "state_machine") {
        return ""
    }
    $violations = if ($schemaVersion -eq "TaskSpaceControlResultV2") {
        $actual = Get-R7JsonProperty $error "actual"
        if ($actual -isnot [pscustomobject]) {
            return "$schemaVersion error.actual must be an object"
        }
        if ((Get-R7JsonProperty $error "expected") -isnot [pscustomobject]) {
            return "$schemaVersion error.expected must be an object"
        }
        $message = Get-R7JsonProperty $error "message"
        if ($message -isnot [string] -or
            [string]::IsNullOrWhiteSpace([string]$message)) {
            return "$schemaVersion error.message must be a non-empty string"
        }
        Get-R7FailureRawProperty $actual "violations"
    } else {
        Get-R7FailureRawProperty $error "violations"
    }
    if ($violations -isnot [System.Array] -or $violations.Count -eq 0) {
        return "$schemaVersion state failure violations must be a non-empty array"
    }
    for ($index = 0; $index -lt $violations.Count; $index++) {
        $shapeError = Get-R7StateViolationShapeError $violations[$index] $index
        if (-not [string]::IsNullOrWhiteSpace($shapeError)) { return $shapeError }
    }
    ""
}
