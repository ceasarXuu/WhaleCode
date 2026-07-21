function New-R7SourceBinding {
    param([string]$Symbol, [string]$Path)
    [pscustomobject][ordered]@{
        symbol = $Symbol
        path = $Path
        sha256 = Get-R7Sha256File (Join-Path $script:R7RepoRoot $Path)
    }
}

function Copy-R7JsonValue {
    param($Value)
    (ConvertTo-R7CanonicalJson $Value) | ConvertFrom-Json -Depth 100
}

function New-R7TransitionFixture {
    param(
        [string]$Id,
        $RequestValue,
        $PreState,
        $ExpectedOutput,
        $PostState,
        [bool]$SchemaValid,
        [bool]$Accepted
    )
    [pscustomobject][ordered]@{
        id = $Id
        input = $RequestValue
        input_sha256 = Get-R7JsonValueHash $RequestValue
        pre_state = $PreState
        pre_state_sha256 = Get-R7JsonValueHash $PreState
        expected_output = $ExpectedOutput
        expected_output_sha256 = Get-R7JsonValueHash $ExpectedOutput
        post_state = $PostState
        post_state_sha256 = Get-R7JsonValueHash $PostState
        schema_valid = $SchemaValid
        accepted = $Accepted
    }
}

function New-R7TaskSpaceResult {
    param(
        [string]$Action,
        [string]$Status,
        [bool]$Success,
        [bool]$Committed,
        [int]$CanonicalRevision,
        $SubmittedRevision,
        $CommittedRevision,
        $Delta,
        [object[]]$Steps,
        $Error
    )
    [pscustomobject][ordered]@{
        schema_version = "TaskSpaceControlResultV2"
        action = $Action
        status = $Status
        success = $Success
        state_commit = $Committed
        partial_commit = $false
        canonical_revision = $CanonicalRevision
        submitted_expected_revision = $SubmittedRevision
        committed_revision = $CommittedRevision
        delta = $Delta
        steps = @($Steps)
        read = $null
        error = $Error
    }
}

function New-R7CommittedDelta {
    param([string]$MapId, [int]$Revision)
    [pscustomobject][ordered]@{
        map_id = $MapId
        committed_revision = $Revision
        graph_event_refs = @()
        node_detail_event_refs = @()
    }
}

function Get-R7TransitionFixtures {
    $map0 = [pscustomobject][ordered]@{map_id = $null; revision = 0; active_node = $null; nodes = @()}
    $map1 = [pscustomobject][ordered]@{map_id = "map-dev"; revision = 1; active_node = "work-1"; nodes = @("root", "work-1", "work-2", "finish")}
    $ready = [pscustomobject][ordered]@{map_id = "map-dev"; revision = 1; active_node = $null; nodes = @("root", "work-1", "finish")}
    $map2 = [pscustomobject][ordered]@{map_id = "map-dev"; revision = 2; active_node = "work-1"; nodes = @("root", "work-1", "finish")}
    $map3 = [pscustomobject][ordered]@{map_id = "map-dev"; revision = 3; active_node = "work-2"; nodes = @("root", "work-1", "work-2", "finish")}
    $initialize = [pscustomobject][ordered]@{
        action = "initialize_map"
        root = [pscustomobject]@{node_id = "root"; goal = "Repair the repository"}
        initial_work_node = [pscustomobject]@{node_id = "work-1"; goal = "Inspect the failure"}
        finish_identity = [pscustomobject]@{id = "finish"}
        additional_work_nodes = @([pscustomobject]@{node_id = "work-2"; goal = "Verify the repair"})
        edges = @(
            [pscustomobject]@{from = "root"; to = "work-1"},
            [pscustomobject]@{from = "work-1"; to = "work-2"},
            [pscustomobject]@{from = "work-2"; to = "finish"}
        )
        required_next_call = "ordinary_tool"
    }
    $bind = [pscustomobject][ordered]@{action = "bind_node"; expected_revision = 1; node_id = "work-1"; required_next_call = "ordinary_tool"}
    $continue = [pscustomobject][ordered]@{action = "complete_then_continue"; expected_revision = 2; current_node_id = "work-1"; next_node_id = "work-2"; required_next_call = "apply_patch"}
    $positive = @(
        New-R7TransitionFixture "initialize_with_sibling" $initialize $map0 (
            New-R7TaskSpaceResult "initialize_map" "committed" $true $true 1 $null 1 (
                New-R7CommittedDelta "map-dev" 1
            ) @([pscustomobject]@{kind = "map_initialized"; map_id = "map-dev"; revision = 1}) $null
        ) $map1 $true $true
        New-R7TransitionFixture "bind_with_sibling" $bind $ready (
            New-R7TaskSpaceResult "bind_node" "committed" $true $true 2 1 2 (
                New-R7CommittedDelta "map-dev" 2
            ) @([pscustomobject]@{kind = "node_bound"; map_id = "map-dev"; node_id = "work-1"; status = "running"; revision = 2}) $null
        ) $map2 $true $true
        New-R7TransitionFixture "complete_continue_with_patch" $continue $map2 (
            New-R7TaskSpaceResult "complete_then_continue" "committed" $true $true 3 2 3 (
                New-R7CommittedDelta "map-dev" 3
            ) @([pscustomobject]@{kind = "complete_then_continue"; map_id = "map-dev"; current_node_id = "work-1"; next_node_id = "work-2"; revision = 3}) $null
        ) $map3 $true $true
    )
    $missingSibling = New-R7TaskSpaceResult "bind_node" "protocol_failed" $false $false 1 1 $null $null @() (
        [pscustomobject]@{class = "protocol"; code = "TASKSPACE_REQUIRED_SIBLING_MISSING"; message = "required sibling missing"; actual = $null; expected = "ordinary_tool"}
    )
    $badArgs = New-R7TaskSpaceResult "bind_node" "argument_failed" $false $false 1 $null $null $null @() (
        [pscustomobject]@{class = "argument"; code = "TASKSPACE_INVALID_ARGUMENT"; message = "node_id is required"; actual = $null; expected = "non-empty node_id"}
    )
    $stale = New-R7TaskSpaceResult "complete_then_continue" "state_machine_failed" $false $false 2 1 $null $null @() (
        [pscustomobject]@{class = "state_machine"; code = "TASKSPACE_STALE_REVISION"; message = "revision mismatch"; actual = 2; expected = 1}
    )
    $negative = @(
        New-R7TransitionFixture "missing_required_sibling" $bind $ready $missingSibling $ready $true $false
        New-R7TransitionFixture "invalid_bind_arguments" ([pscustomobject]@{action = "bind_node"; expected_revision = 1; required_next_call = "ordinary_tool"}) $ready $badArgs $ready $false $false
        New-R7TransitionFixture "stale_revision" ([pscustomobject][ordered]@{action = "complete_then_continue"; expected_revision = 1; current_node_id = "work-1"; next_node_id = "work-2"; required_next_call = "ordinary_tool"}) $map2 $stale $map2 $true $false
    )
    [pscustomobject]@{positive = $positive; negative = $negative}
}

function Get-R7CarrierOutcomeSchema {
    @'
{
  "$schema":"https://json-schema.org/draft/2020-12/schema",
  "oneOf":[
    {"type":"object","properties":{"variant":{"const":"RejectedBeforeCommit"},"commit_state":{"const":"not_committed"},"tool_state":{"const":"not_dispatched"},"transition":{"type":"object"},"tool_result":{"type":"null"},"error":{"type":"object"}},"required":["variant","commit_state","tool_state","transition","tool_result","error"],"additionalProperties":false},
    {"type":"object","properties":{"variant":{"const":"CommittedNotExecuted"},"commit_state":{"const":"committed"},"tool_state":{"const":"not_started"},"transition":{"type":"object"},"tool_result":{"type":"null"},"error":{"type":"object"}},"required":["variant","commit_state","tool_state","transition","tool_result","error"],"additionalProperties":false},
    {"type":"object","properties":{"variant":{"const":"Executed"},"commit_state":{"const":"committed"},"tool_state":{"enum":["returned","failed","cancelled_after_start"]},"transition":{"type":"object"},"tool_result":{"type":["object","null"]},"error":{"type":["object","null"]}},"required":["variant","commit_state","tool_state","transition","tool_result","error"],"additionalProperties":false}
  ]
}
'@ | ConvertFrom-Json -Depth 100
}

function New-R7OutcomeFixture {
    param([string]$Id, [string]$Variant, [string]$CommitState, [string]$ToolState, $ToolResult, $Error)
    $request = [pscustomobject][ordered]@{scenario = $Id; transition_id = "transition-$Id"; tool_call_id = "tool-$Id"}
    $output = [pscustomobject][ordered]@{
        variant = $Variant
        commit_state = $CommitState
        tool_state = $ToolState
        transition = [pscustomobject]@{id = "transition-$Id"; revision = if ($CommitState -eq "committed") { 2 } else { 1 }}
        tool_result = $ToolResult
        error = $Error
    }
    [pscustomobject][ordered]@{
        id = $Id
        variant = $Variant
        input = $request
        input_sha256 = Get-R7JsonValueHash $request
        output = $output
        output_sha256 = Get-R7JsonValueHash $output
        commit_state = $CommitState
        tool_state = $ToolState
    }
}

function Get-R7OutcomeFixtures {
    @(
        New-R7OutcomeFixture "rejected_before_commit" "RejectedBeforeCommit" "not_committed" "not_dispatched" $null ([pscustomobject]@{code = "TASKSPACE_REQUIRED_SIBLING_MISSING"})
        New-R7OutcomeFixture "authorization_denied" "CommittedNotExecuted" "committed" "not_started" $null ([pscustomobject]@{code = "TOOL_AUTHORIZATION_DENIED"})
        New-R7OutcomeFixture "ordinary_returned" "Executed" "committed" "returned" ([pscustomobject]@{content = "ok"}) $null
        New-R7OutcomeFixture "ordinary_failed" "Executed" "committed" "failed" ([pscustomobject]@{content = "failed"; is_error = $true}) ([pscustomobject]@{code = "TOOL_FAILED"})
        New-R7OutcomeFixture "cancelled_after_start" "Executed" "committed" "cancelled_after_start" $null ([pscustomobject]@{code = "TOOL_CANCELLED"})
        New-R7OutcomeFixture "mcp_fields_absent" "Executed" "committed" "returned" ([pscustomobject]@{}) $null
        New-R7OutcomeFixture "mcp_fields_null" "Executed" "committed" "returned" ([pscustomobject]@{content = $null; structured_content = $null; is_error = $null; meta = $null}) $null
        New-R7OutcomeFixture "mcp_image_payload" "Executed" "committed" "returned" ([pscustomobject]@{content = @([pscustomobject]@{type = "image"; data = "fixture"}); structured_content = [pscustomobject]@{kind = "image"}; is_error = $false; meta = [pscustomobject]@{mime = "image/png"}}) $null
    )
}

function New-R7CarrierSpec {
    param([string]$WireApi, [string]$ToolSpec, $BusinessSchema, $BusinessFixture, $Transition, $Parser)
    $decoratedSchema = Copy-R7JsonValue $BusinessSchema
    $decoratedSchema.properties | Add-Member -NotePropertyName "taskspace_transition" -NotePropertyValue $Transition
    $decoratedFixture = Copy-R7JsonValue $BusinessFixture
    $decoratedFixture | Add-Member -NotePropertyName "taskspace_transition" -NotePropertyValue ([pscustomobject]@{action = "bind_node"; expected_revision = 1; node_id = "work-1"; required_next_call = "ordinary_tool"})
    [pscustomobject][ordered]@{
        wire_api = $WireApi
        tool_spec = $ToolSpec
        business_schema = $BusinessSchema
        business_schema_sha256 = Get-R7JsonValueHash $BusinessSchema
        decorated_schema = $decoratedSchema
        decorated_schema_sha256 = Get-R7JsonValueHash $decoratedSchema
        business_fixture = $BusinessFixture
        decorated_fixture = $decoratedFixture
        parser = $Parser
        reserved_field_removed_before_handler = $true
    }
}

function New-R7OracleScenario {
    param([string]$Id, $Request, [string[]]$Events, $Outcome)
    $sourcePath = "third_party/codex-cli/codex-rs/core/src/tools/sequence_tests.rs"
    $trace = [pscustomobject][ordered]@{events = $Events; outcome = $Outcome}
    [pscustomobject][ordered]@{
        id = $Id
        fixture_path = $sourcePath
        fixture_sha256 = Get-R7Sha256File (Join-Path $script:R7RepoRoot $sourcePath)
        request_fixture = $Request
        request_fixture_sha256 = Get-R7JsonValueHash $Request
        expected_trace = $trace
        expected_outcome_sha256 = Get-R7JsonValueHash $trace
    }
}

function New-R7ExecutableArtifactFixtures {
    param($Closure)
    $control = Read-R7StrictJson (Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/five-layer-taskspace-control-v2.schema.json")
    $transitionInput = Copy-R7JsonValue $control.provider_tool.function.parameters
    $transitionInput | Add-Member -NotePropertyName '$schema' -NotePropertyValue "https://json-schema.org/draft/2020-12/schema"
    $transitionInput | Add-Member -NotePropertyName oneOf -NotePropertyValue @($transitionInput.anyOf)
    $transitionInput.psobject.Properties.Remove("anyOf")
    $resultSchema = Read-R7StrictJson (Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/five-layer-taskspace-result-v2.schema.json")
    $fixtures = Get-R7TransitionFixtures
    $parser = New-R7SourceBinding "build_tool_call" "third_party/codex-cli/codex-rs/core/src/tools/router.rs"
    $functionSchema = [pscustomobject][ordered]@{'$schema' = "https://json-schema.org/draft/2020-12/schema"; type = "object"; properties = [pscustomobject]@{path = [pscustomobject]@{type = "string"}}; required = @("path"); additionalProperties = $false}
    $namespaceSchema = [pscustomobject][ordered]@{'$schema' = "https://json-schema.org/draft/2020-12/schema"; type = "object"; properties = [pscustomobject]@{query = [pscustomobject]@{type = "string"}}; required = @("query"); additionalProperties = $false}
    $l4 = [pscustomobject][ordered]@{
        schema_version = 2
        artifact_role = "l4_schema"
        reserved_field = "taskspace_transition"
        collision_policy = "reject_capability_epoch"
        carrier_specs = @(
            New-R7CarrierSpec "responses" "Function" $functionSchema ([pscustomobject]@{path = "src/lib.rs"}) $transitionInput $parser
            New-R7CarrierSpec "deepseek_chat" "Function" $functionSchema ([pscustomobject]@{path = "src/lib.rs"}) $transitionInput $parser
            New-R7CarrierSpec "responses" "Namespace" $namespaceSchema ([pscustomobject]@{query = "workspace"}) $transitionInput $parser
        )
        standard_identity = [pscustomobject][ordered]@{
            tool_schema = New-R7SourceBinding "create_taskspace_control_tool" "third_party/codex-cli/codex-rs/tools/src/taskspace_tool.rs"
            wire_mapper = New-R7SourceBinding "chat_tools_from_responses_tools" "third_party/codex-cli/codex-rs/codex-api/src/endpoint/responses.rs"
            handler = New-R7SourceBinding "TaskSpaceControlHandler" "third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control.rs"
        }
    }
    $transition = [pscustomobject][ordered]@{schema_version = 2; artifact_role = "transition_schema"; schema_id = "r7-taskspace-transition-v2"; input_schema = $transitionInput; output_schema = $resultSchema; positive_fixtures = @($fixtures.positive); negative_fixtures = @($fixtures.negative)}
    $typed = [pscustomobject][ordered]@{schema_version = 2; artifact_role = "typed_outcome"; outcome_type = "TaskSpaceCarrierOutcome"; variants = @("RejectedBeforeCommit", "CommittedNotExecuted", "Executed"); deferred_authorization_variants = @("NotRequired", "Requested", "Approved", "Denied"); mcp_presence_fields = @("content", "structured_content", "is_error", "meta"); outcome_schema = Get-R7CarrierOutcomeSchema; fixtures = @(Get-R7OutcomeFixtures)}
    $oracles = @(
        New-R7OracleScenario "empty_map_initialize" ([pscustomobject]@{control = $fixtures.positive[0].input; sibling = [pscustomobject]@{tool = "list_dir"}}) @("preflight.accepted", "taskspace.committed", "tool.started", "tool.returned") $typed.fixtures[2].output
        New-R7OracleScenario "ready_bind" ([pscustomobject]@{control = $fixtures.positive[1].input; sibling = [pscustomobject]@{tool = "list_dir"}}) @("preflight.accepted", "taskspace.committed", "tool.started", "tool.returned") $typed.fixtures[2].output
        New-R7OracleScenario "complete_continue" ([pscustomobject]@{control = $fixtures.positive[2].input; sibling = [pscustomobject]@{tool = "apply_patch"}}) @("preflight.accepted", "taskspace.committed", "tool.started", "tool.returned") $typed.fixtures[2].output
        New-R7OracleScenario "standalone_negative" ([pscustomobject]@{control = $fixtures.negative[0].input; sibling = $null}) @("preflight.rejected") $typed.fixtures[0].output
        New-R7OracleScenario "argument_failure" ([pscustomobject]@{control = $fixtures.negative[1].input; sibling = [pscustomobject]@{tool = "list_dir"}}) @("arguments.rejected") $typed.fixtures[0].output
        New-R7OracleScenario "commit_tool_failure" ([pscustomobject]@{control = $fixtures.positive[1].input; sibling = [pscustomobject]@{tool = "list_dir"; result = "failed"}}) @("preflight.accepted", "taskspace.committed", "tool.started", "tool.failed") $typed.fixtures[3].output
        New-R7OracleScenario "code_mode_carrier" ([pscustomobject]@{control = $fixtures.positive[1].input; sibling = [pscustomobject]@{tool = "exec"; nested = "list_dir"}}) @("preflight.accepted", "taskspace.committed", "code_mode.started", "tool.returned") $typed.fixtures[2].output
    )
    $oracle = [pscustomobject][ordered]@{schema_version = 2; artifact_role = "carrier_protocol_oracle"; oracle_version = 3; scenarios = $oracles}
    $matrix = [pscustomobject][ordered]@{schema_version = 2; artifact_role = "capability_matrix"; entry_closure_sha256 = Get-R7JsonValueHash $Closure; entries = @($Closure.entries)}
    $head = Get-R7GitLine @("rev-parse", "HEAD")
    $router = "third_party/codex-cli/codex-rs/core/src/tools/router.rs"
    $routerHash = Get-R7Sha256File (Join-Path $script:R7RepoRoot $router)
    $rollback = [pscustomobject][ordered]@{
        schema_version = 2
        artifact_role = "rollback_manifest"
        baseline_commit = $head
        baseline_authority_sha256 = Get-R7Sha256File (Join-Path $script:R7RepoRoot $script:R7AuthorityPath)
        baseline_production_sha256 = Get-R7Sha256File (Join-Path $script:R7RepoRoot $script:R7ProductionPath)
        changed_paths = @($router)
        changed_path_inventory = @([pscustomobject]@{path = $router; rollback_action = "preserve"; candidate_sha256 = $routerHash; git_mode = "100644"})
        restore_targets = @(
            [pscustomobject]@{path = $script:R7AuthorityPath; sha256 = Get-R7Sha256File (Join-Path $script:R7RepoRoot $script:R7AuthorityPath); git_mode = "100644"},
            [pscustomobject]@{path = $script:R7ProductionPath; sha256 = Get-R7Sha256File (Join-Path $script:R7RepoRoot $script:R7ProductionPath); git_mode = "100644"}
        )
        commands = @("pwsh test-r7-continuous-action-candidate.ps1 -CandidateId id", "pwsh set-r7-continuous-action-candidate-status.ps1 -CandidateId id")
    }
    [ordered]@{l4_schema = $l4; transition_schema = $transition; typed_outcome = $typed; carrier_protocol_oracle = $oracle; entry_closure = $Closure; capability_matrix = $matrix; rollback_manifest = $rollback}
}

function Assert-R7GeneratedArtifactSemantics {
    param($Fixtures, [string]$ScratchRoot)
    [System.IO.Directory]::CreateDirectory($ScratchRoot) | Out-Null
    foreach ($carrier in @($Fixtures.l4_schema.carrier_specs)) {
        $label = "$($carrier.wire_api)-$($carrier.tool_spec)"
        if ((Get-R7JsonValueHash $carrier.business_schema) -cne [string]$carrier.business_schema_sha256) { throw "R7_FIXTURE_L4_BUSINESS_HASH label=$label" }
        if ((Get-R7JsonValueHash $carrier.decorated_schema) -cne [string]$carrier.decorated_schema_sha256) { throw "R7_FIXTURE_L4_DECORATED_HASH label=$label" }
        $businessPath = Join-Path $ScratchRoot "l4-business-$label.json"
        $decoratedPath = Join-Path $ScratchRoot "l4-decorated-$label.json"
        Write-R7JsonFile $businessPath $carrier.business_schema
        Write-R7JsonFile $decoratedPath $carrier.decorated_schema
        if (-not (($carrier.business_fixture | ConvertTo-Json -Depth 100 -Compress) | Test-Json -SchemaFile $businessPath -ErrorAction SilentlyContinue)) { throw "R7_FIXTURE_L4_BUSINESS_INVALID label=$label" }
        if (-not (($carrier.decorated_fixture | ConvertTo-Json -Depth 100 -Compress) | Test-Json -SchemaFile $decoratedPath -ErrorAction SilentlyContinue)) { throw "R7_FIXTURE_L4_DECORATED_INVALID label=$label" }
    }
    $inputPath = Join-Path $ScratchRoot "transition-input-schema.json"
    $outputPath = Join-Path $ScratchRoot "transition-output-schema.json"
    Write-R7JsonFile $inputPath $Fixtures.transition_schema.input_schema
    Write-R7JsonFile $outputPath $Fixtures.transition_schema.output_schema
    foreach ($fixture in @($Fixtures.transition_schema.positive_fixtures) + @($Fixtures.transition_schema.negative_fixtures)) {
        $valid = [bool](($fixture.input | ConvertTo-Json -Depth 100 -Compress) | Test-Json -SchemaFile $inputPath -ErrorAction SilentlyContinue)
        if ($valid -ne [bool]$fixture.schema_valid) { throw "R7_FIXTURE_TRANSITION_SCHEMA id=$($fixture.id)" }
        if (-not (($fixture.expected_output | ConvertTo-Json -Depth 100 -Compress) | Test-Json -SchemaFile $outputPath -ErrorAction SilentlyContinue)) { throw "R7_FIXTURE_TRANSITION_OUTPUT id=$($fixture.id)" }
    }
    $outcomePath = Join-Path $ScratchRoot "typed-outcome-schema.json"
    Write-R7JsonFile $outcomePath $Fixtures.typed_outcome.outcome_schema
    foreach ($fixture in @($Fixtures.typed_outcome.fixtures)) {
        if (-not (($fixture.output | ConvertTo-Json -Depth 100 -Compress) | Test-Json -SchemaFile $outcomePath -ErrorAction SilentlyContinue)) { throw "R7_FIXTURE_OUTCOME_INVALID id=$($fixture.id)" }
    }
    foreach ($scenario in @($Fixtures.carrier_protocol_oracle.scenarios)) {
        if ((Get-R7JsonValueHash $scenario.request_fixture) -cne [string]$scenario.request_fixture_sha256) { throw "R7_FIXTURE_ORACLE_REQUEST id=$($scenario.id)" }
        if ((Get-R7JsonValueHash $scenario.expected_trace) -cne [string]$scenario.expected_outcome_sha256) { throw "R7_FIXTURE_ORACLE_TRACE id=$($scenario.id)" }
    }
}
