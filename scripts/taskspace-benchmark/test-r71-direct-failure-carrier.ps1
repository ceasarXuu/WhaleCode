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
$phaseEvidenceSchemaPath = Join-Path $repoRoot (
    "benchmarks/taskspace/r7/r7-phase-evidence-v1.schema.json"
)
$phaseEvidenceSchema = Get-Content -Raw -Encoding UTF8 `
    -LiteralPath $phaseEvidenceSchemaPath
$weakEvidence =
    '{"schema_version":"r71-phase-evidence-v1",' +
    '"artifact_type":"strict_failure_carrier","records":[{"anything":1}]}'
Assert-R71DirectCarrier (
    -not ($weakEvidence |
        Test-Json -Schema $phaseEvidenceSchema -ErrorAction SilentlyContinue)
) "Strict failure evidence schema accepted a record without required facts"

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

$revisionMismatchObject = $validJson | ConvertFrom-Json -Depth 20
$revisionMismatchObject.canonical_revision = 3
$revisionMismatchObject.error.actual | Add-Member `
    -NotePropertyName canonical_revision `
    -NotePropertyValue 99
$revisionMismatch = Assert-InvalidCarrier `
    "actual canonical revision mismatch" `
    ($revisionMismatchObject | ConvertTo-Json -Compress -Depth 30)

$absentNodeObject =
    $stateMismatchObject | ConvertTo-Json -Depth 30 | ConvertFrom-Json -Depth 30
$absentNodeObject.error.code = "node_state_invalid"
$absentNodeObject.error.actual.violations[0].
    canonical_before_transaction.node_present = $false
$absentNodeObject.error.actual.violations[0].
    canonical_before_transaction.state = "waiting"
$absentNode = Assert-InvalidCarrier `
    "absent canonical node with state" `
    ($absentNodeObject | ConvertTo-Json -Compress -Depth 30)

$supplementalMismatchObject = [ordered]@{
    schema_version = "TaskSpaceResponseCommitFailureV3"
    status = "state_rejected"
    success = $false
    state_commit = $false
    canonical_revision = 4
    rejected_candidate_committed = $false
    executed_tool_call_count = 0
    failure_provenance = [ordered]@{
        scope = "provider_response"
        copy_group_id = "provider_response:control"
        zero_dispatch = $true
        affected_call_ids = @("control")
    }
    error = [ordered]@{
        class = "resource"
        code = "taskspace_canonical_store_unavailable"
        detail = "unavailable"
    }
}
$supplementalMismatch = Get-R7CallOutcome `
    -ToolSuccess $false `
    -Output ($supplementalMismatchObject | ConvertTo-Json -Compress -Depth 20) `
    -TrustedRuntimeCarrier
Assert-R71DirectCarrier (
    -not $supplementalMismatch.evidence_valid
) "Supplemental status/class contradiction remained valid"

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

$ordinaryMalformed = Get-R7CallOutcome `
    -ToolSuccess $false `
    -Output "{`"domain`":`nShell exit code: 7" `
    -ToolName "exec_command"
Assert-R71DirectCarrier (
    $ordinaryMalformed.evidence_valid -and
    $ordinaryMalformed.failure_class -eq "ordinary_tool" -and
    $ordinaryMalformed.failure_code -eq "shell_exit_7"
) "Malformed ordinary JSON bypassed the ordinary Tool classifier"

$domainCall = ConvertTo-R7CallDescriptor `
    -CallId "domain-success" `
    -ToolName "mcp__domain__lookup" `
    -Arguments "{}"
$domainObserved = Get-R7ResponseItemOutcome ([pscustomobject]@{
        type = "function_call_output"
        call_id = "domain-success"
        output = '{"success":false,"business_status":"declined"}'
    })
$domainCalls = @{ "domain-success" = $domainCall }
Apply-R7ObservedOutcome $domainCalls $domainObserved
Assert-R71DirectCarrier (
    $domainCall.success -and $domainCall.evidence_valid
) "Ordinary domain success field was treated as Tool transport status"

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

$invalidPrepareJson = $responseCommitJson.Replace(
    '"action":"execute"',
    '"action":"finish_map"'
)
$controlCall = ConvertTo-R7CallDescriptor `
    -CallId "prepare-control" `
    -ToolName "taskspace_control" `
    -Arguments '{"action":"execute"}'
$siblingCall = ConvertTo-R7CallDescriptor `
    -CallId "tool-1" `
    -ToolName "exec_command" `
    -Arguments '{"cmd":"true"}'
$prepareCalls = @{
    "prepare-control" = $controlCall
    "tool-1" = $siblingCall
}
$invalidPrepareObserved = Get-R7ResponseItemOutcome `
    ([pscustomobject]@{
        type = "function_call_output"
        call_id = "prepare-control"
        output = $invalidPrepareJson
    }) `
    ([pscustomobject]@{ toolSuccess = $true })
Apply-R7ObservedOutcome $prepareCalls $invalidPrepareObserved
Assert-R71DirectCarrier (
    -not $controlCall.evidence_valid -and
    [string]::IsNullOrWhiteSpace([string]$siblingCall.expected_reservation_id)
) "Invalid response-prepare carrier mutated sibling attribution"

$duplicatePrepareJson = $responseCommitJson.Replace(
    '"status":"accepted"',
    '"status":"accepted","Status":"accepted"'
)
$duplicateControl = ConvertTo-R7CallDescriptor `
    -CallId "duplicate-control" `
    -ToolName "taskspace_control" `
    -Arguments '{"action":"execute"}'
$duplicateSibling = ConvertTo-R7CallDescriptor `
    -CallId "tool-1" `
    -ToolName "exec_command" `
    -Arguments '{"cmd":"true"}'
$duplicateCalls = @{
    "duplicate-control" = $duplicateControl
    "tool-1" = $duplicateSibling
}
$duplicateObserved = Get-R7ResponseItemOutcome `
    ([pscustomobject]@{
        type = "function_call_output"
        call_id = "duplicate-control"
        output = $duplicatePrepareJson
    }) `
    ([pscustomobject]@{ toolSuccess = $true })
Apply-R7ObservedOutcome $duplicateCalls $duplicateObserved
Assert-R71DirectCarrier (
    $duplicateControl.parse_status -eq "duplicate_failure_json_property" -and
    [string]::IsNullOrWhiteSpace([string]$duplicateSibling.expected_reservation_id)
) "Duplicate response-prepare carrier bypassed strict diagnostics"

if (-not [string]::IsNullOrWhiteSpace($EvidencePath)) {
    function New-R71ProductionCallRow {
        param(
            [string]$CallId,
            [string]$ToolName,
            [string]$Output,
            $ToolSuccess,
            [string]$ControlArguments = "{}"
        )
        $call = ConvertTo-R7CallDescriptor `
            -CallId $CallId `
            -ToolName $ToolName `
            -Arguments $ControlArguments
        $calls = @{ $CallId = $call }
        $observed = Get-R7ResponseItemOutcome `
            ([pscustomobject]@{
                type = "function_call_output"
                call_id = $CallId
                output = $Output
            }) `
            ([pscustomobject]@{ toolSuccess = $ToolSuccess })
        Apply-R7ObservedOutcome $calls $observed
        $call | Add-Member -NotePropertyName source_sha256 -NotePropertyValue (
            Get-R7Sha256Hex $Output
        )
        $call
    }
    $mismatchedResponseCommitJson = $responseCommitJson.Replace(
        '"action":"execute"',
        '"action":"reopen_map"'
    )
    $evidenceExecuteArgs =
        '{"action":"execute","expected_revision":1,' +
        '"actions":[{"node_id":"work","tool":"exec_command"}]}'
    $evidenceCalls = @(
        New-R71ProductionCallRow `
            "valid-direct-failure" "taskspace_control" $validJson $false
        New-R71ProductionCallRow `
            "duplicate-json" "taskspace_control" $duplicateRoot $false
        New-R71ProductionCallRow `
            "outer-inner-mismatch" "taskspace_control" $validJson $true
        New-R71ProductionCallRow `
            "ordinary-schema-isolation" "exec_command" $ordinaryJson $false
        New-R71ProductionCallRow `
            "response-request-mismatch" `
            "taskspace_control" `
            $mismatchedResponseCommitJson `
            $true `
            $evidenceExecuteArgs
        New-R71ProductionCallRow `
            "missing-transport-status" `
            "taskspace_control" `
            $validJson `
            $null
    )
    $records = @(
        $evidenceCalls | ForEach-Object {
            [ordered]@{
                request_id = "r71-01-fixture"
                call_id = [string]$_.call_id
                carrier_schema = [string]$_.carrier_schema
                parse_status = [string]$_.parse_status
                reason_code = [string]$_.reason_code
                source_kind = "production_call_row"
                source_sha256 = [string]$_.source_sha256
            }
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
