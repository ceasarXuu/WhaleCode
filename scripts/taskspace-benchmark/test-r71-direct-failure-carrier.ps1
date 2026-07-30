param([string]$EvidencePath = "")

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "lib/r7-five-layer-trace-analysis.ps1")

function Assert-R71DirectCarrier {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Get-TestControlFailureJson {
    param(
        [string]$Action = "execute",
        [string]$ExpectedAction = "execute"
    )
    [ordered]@{
        schema_version = "TaskSpaceControlResultV2"
        action = $Action
        status = "argument_failed"
        success = $false
        state_commit = $false
        partial_commit = $false
        canonical_revision = 0
        submitted_expected_revision = $null
        committed_revision = $null
        delta = $null
        steps = @()
        read = $null
        error = [ordered]@{
            class = "argument"
            code = "TASKSPACE_INVALID_ARGUMENT"
            message = "invalid arguments"
            actual = [ordered]@{ action = $Action }
            expected = [ordered]@{ action = $ExpectedAction }
        }
    } | ConvertTo-Json -Compress -Depth 20
}

function Assert-InvalidCarrier {
    param(
        [string]$Name,
        [string]$Output,
        [bool]$ToolSuccess = $false,
        [string]$ToolName = "taskspace_control"
    )
    $result = Get-R7CallOutcome `
        -ToolSuccess $ToolSuccess `
        -Output $Output `
        -ToolName $ToolName
    Assert-R71DirectCarrier (-not $result.evidence_valid) `
        "$Name remained valid"
    Assert-R71DirectCarrier (
        $result.failure_class -eq "evidence_unclassified" -and
        -not [string]::IsNullOrWhiteSpace([string]$result.parse_status) -and
        -not [string]::IsNullOrWhiteSpace([string]$result.failure_code)
    ) "$Name did not return stable failure evidence"
    $result
}

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$resultContractPath = Join-Path $repoRoot (
    "benchmarks/taskspace/r7/five-layer-taskspace-result-v3.contract.json"
)
$resultContract = Get-Content -Raw -Encoding UTF8 -LiteralPath $resultContractPath |
    ConvertFrom-Json -Depth 20
Assert-R71DirectCarrier (
    (@($resultContract.accepted_actions) -join "`n") -eq
        (@(Get-R7ControlActionNames) -join "`n")
) "Direct failure action allowlist drifted from the result contract"

$validJson = Get-TestControlFailureJson
$valid = Get-R7CallOutcome `
    -ToolSuccess $false `
    -Output $validJson `
    -ToolName "taskspace_control"
Assert-R71DirectCarrier (
    $valid.evidence_valid -and
    $valid.failure_code -eq "TASKSPACE_INVALID_ARGUMENT" -and
    $valid.state_commit -eq $false
) "Valid direct control failure was rejected"

$duplicateRoot = $validJson.Replace(
    '{"schema_version":"TaskSpaceControlResultV2"',
    '{"schema_version":"benign","schema_version":"TaskSpaceControlResultV2"'
)
$duplicateNested = $validJson.Replace(
    '"code":"TASKSPACE_INVALID_ARGUMENT"',
    '"code":"benign","code":"TASKSPACE_INVALID_ARGUMENT"'
)
$duplicateCase = $validJson.Replace(
    '"success":false',
    '"Success":true,"success":false'
)
$duplicateResults = @(
    Assert-InvalidCarrier "duplicate root property" $duplicateRoot
    Assert-InvalidCarrier "duplicate nested property" $duplicateNested
    Assert-InvalidCarrier "case-insensitive duplicate property" $duplicateCase
)
foreach ($result in $duplicateResults) {
    Assert-R71DirectCarrier (
        $result.parse_status -eq "duplicate_failure_json_property"
    ) "Duplicate JSON did not expose its mechanical parse reason"
}

$invalidAction = Assert-InvalidCarrier `
    "unsupported action" `
    (Get-TestControlFailureJson "dance" "dance")
$expectedMismatch = Assert-InvalidCarrier `
    "expected action mismatch" `
    (Get-TestControlFailureJson "execute" "read_map")

$outerMismatch = Assert-InvalidCarrier `
    "outer success overrides inner failure" `
    $validJson `
    $true
Assert-R71DirectCarrier (
    $outerMismatch.parse_status -eq "outer_inner_success_mismatch" -and
    $outerMismatch.state_commit -eq $false
) "Outer/inner mismatch did not preserve the inner state fact"

$innerSuccess = $validJson.
    Replace('"success":false', '"success":true').
    Replace('"state_commit":false', '"state_commit":true')
$innerMismatch = Assert-InvalidCarrier `
    "outer failure overrides inner success" `
    $innerSuccess
Assert-R71DirectCarrier (
    $innerMismatch.parse_status -eq "outer_inner_success_mismatch"
) "Inverse outer/inner mismatch was not rejected"

$stateMismatchObject = $validJson | ConvertFrom-Json -Depth 20
$stateMismatchObject.status = "state_machine_failed"
$stateMismatchObject.error.class = "state_machine"
$stateMismatchObject.error.code = "different_code"
$stateMismatchObject.error.actual = [pscustomobject]@{
    violations = @(
        [pscustomobject]@{
            code = "node_state_invalid"
            subjects = @("node")
            node_id = "work"
            canonical_before_transaction = [pscustomobject]@{
                node_present = $true
                state = "waiting"
                unsatisfied_predecessor_ids = @("predecessor")
            }
            rejected_candidate_at_violation = [pscustomobject]@{
                committed = $false
                state = "completed"
                allowed_states = @("ready", "in_flight")
                unsatisfied_predecessor_ids = @()
            }
        }
    )
}
$stateMismatch = Assert-InvalidCarrier `
    "state error code mismatch" `
    ($stateMismatchObject | ConvertTo-Json -Compress -Depth 30)

$ordinaryJson =
    '{"schema_version":"OrdinaryExecResultV1","metadata":' +
    '{"execution_outcome":"exited","shell_exit_code":1}}'
$ordinary = Get-R7CallOutcome `
    -ToolSuccess $false `
    -Output $ordinaryJson `
    -ToolName "exec_command"
Assert-R71DirectCarrier (
    $ordinary.evidence_valid -and
    $ordinary.failure_class -eq "ordinary_tool" -and
    $ordinary.failure_code -eq "shell_exit_1"
) "Ordinary Tool schema was misclassified as a TaskSpace carrier"

$spoofedControl = Assert-InvalidCarrier `
    "TaskSpace schema on ordinary Tool" `
    $validJson `
    $false `
    "exec_command"
Assert-R71DirectCarrier (
    $spoofedControl.failure_code -eq "taskspace_failure_untrusted_carrier"
) "Reserved TaskSpace schema was accepted from an ordinary Tool"

$responseCommitJson =
    '{"schema_version":"TaskSpaceResponseCommitV1","status":"accepted",' +
    '"success":true,"state_commit":true,"map_id":"map-1",' +
    '"action":"execute","revision_before":1,"revision_after":2,' +
    '"reserved_actions":[{"call_index":0,"call_id":"tool-1",' +
    '"node_id":"work","tool":"exec_command",' +
    '"reservation_id":"reservation:tool-1"}]}'
$responseCommit = Get-R7CallOutcome `
    -ToolSuccess $true `
    -Output $responseCommitJson `
    -ToolName "taskspace_control"
Assert-R71DirectCarrier (
    $responseCommit.evidence_valid -and
    $responseCommit.success -and
    $responseCommit.state_commit
) "Valid response-prepare success carrier was rejected"
$malformedResponseCommit = Assert-InvalidCarrier `
    "incomplete response-prepare carrier" `
    ($responseCommitJson.Replace('"map_id":"map-1",', "")) `
    $true
$trustedMalformedResponseCommit = Get-R7CallOutcome `
    -ToolSuccess $true `
    -Output ($responseCommitJson.Replace('"map_id":"map-1",', "")) `
    -TrustedRuntimeCarrier
Assert-R71DirectCarrier (
    -not $trustedMalformedResponseCommit.evidence_valid
) "Trusted carrier path bypassed response-prepare shape validation"
$spoofedResponseCommit = Assert-InvalidCarrier `
    "response-prepare schema on ordinary Tool" `
    $responseCommitJson `
    $true `
    "exec_command"
Assert-R71DirectCarrier (
    $spoofedResponseCommit.failure_code -eq
        "taskspace_failure_untrusted_carrier"
) "Reserved response-prepare schema was accepted from an ordinary Tool"

if (-not [string]::IsNullOrWhiteSpace($EvidencePath)) {
    $records = @(
        [ordered]@{
            request_id = "r71-01-fixture"
            call_id = "valid-direct-failure"
            carrier_schema = "TaskSpaceControlResultV2"
            parse_status = [string]$valid.parse_status
            reason_code = [string]$valid.failure_code
        }
        [ordered]@{
            request_id = "r71-01-fixture"
            call_id = "duplicate-json"
            carrier_schema = "untrusted_duplicate_json"
            parse_status = [string]$duplicateResults[0].parse_status
            reason_code = [string]$duplicateResults[0].failure_code
        }
        [ordered]@{
            request_id = "r71-01-fixture"
            call_id = "outer-inner-mismatch"
            carrier_schema = "TaskSpaceControlResultV2"
            parse_status = [string]$outerMismatch.parse_status
            reason_code = [string]$outerMismatch.failure_code
        }
        [ordered]@{
            request_id = "r71-01-fixture"
            call_id = "ordinary-schema-isolation"
            carrier_schema = "OrdinaryExecResultV1"
            parse_status = [string]$ordinary.parse_status
            reason_code = [string]$ordinary.failure_code
        }
    )
    $parent = Split-Path -Parent $EvidencePath
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    [ordered]@{
        schema_version = "r71-phase-evidence-v1"
        artifact_type = "strict_failure_carrier"
        records = $records
    } | ConvertTo-Json -Depth 20 |
        Set-Content -Encoding UTF8 -LiteralPath $EvidencePath
}

Write-Output "R71-01 direct failure carrier contract passed."
