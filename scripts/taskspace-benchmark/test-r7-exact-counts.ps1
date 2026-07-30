$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "lib/r7-five-layer-trace-analysis.ps1")

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

$large = [int64]9007199254740992
$requests = @(
    [pscustomobject]@{
        primary_failure_class = "none"
        evidence_health = "valid"
        invalid_evidence_count = $large
        sibling_failure_copy_count = $large
        input_tokens = [int64]0
        cached_input_tokens = [int64]0
        output_tokens = [int64]0
        reasoning_output_tokens = [int64]0
        total_tokens = [int64]0
        receipt_before = $false
        receipt_original_role = ""
        receipt_wire_role = ""
    },
    [pscustomobject]@{
        primary_failure_class = "none"
        evidence_health = "valid"
        invalid_evidence_count = [int64]1
        sibling_failure_copy_count = [int64]1
        input_tokens = [int64]0
        cached_input_tokens = [int64]0
        output_tokens = [int64]0
        reasoning_output_tokens = [int64]0
        total_tokens = [int64]0
        receipt_before = $false
        receipt_original_role = ""
        receipt_wire_role = ""
    }
)
$summary = Get-R7RequestObservabilitySummary $requests
$expected = [int64]9007199254740993
Assert-True (
    $summary.invalid_evidence_call_count -is [int64] -and
    $summary.invalid_evidence_call_count -eq $expected
) "Invalid evidence count lost Int64 precision"
Assert-True (
    $summary.sibling_failure_copy_count -is [int64] -and
    $summary.sibling_failure_copy_count -eq $expected
) "Sibling failure copy count lost Int64 precision"
Assert-True (
    $summary.provider_requests -is [int64] -and
    $summary.primary_failure_counts.none -is [int64]
) "Request summary counts are not Int64 facts"

foreach ($invalid in @(
        @([double]1.5),
        @("1")
    )) {
    $rejected = $false
    try {
        Get-R7ExactInt64Sum $invalid "invalid_fixture" | Out-Null
    } catch {
        $rejected = $true
    }
    Assert-True $rejected "Exact count accepted a non-integer fact"
}

$overflowRejected = $false
try {
    Get-R7ExactInt64Sum @([int64]::MaxValue, [int64]1) "overflow_fixture" |
        Out-Null
} catch {
    $overflowRejected = $true
}
Assert-True $overflowRejected "Exact count silently overflowed Int64"

Write-Output "R7 exact count contract passed."
