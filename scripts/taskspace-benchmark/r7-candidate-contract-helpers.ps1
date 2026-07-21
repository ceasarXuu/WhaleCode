function Assert-CandidateActivationTargets {
    param([object]$Candidate, [object]$ActiveAuthority)
    $l4Targets = @($Candidate.activation_targets.L4)
    $l5Targets = @($Candidate.activation_targets.L5)
    Assert-Equal $l4Targets.Count 1 "Candidate must declare exactly one L4 activation target"
    Assert-Equal $l5Targets.Count 3 "Candidate must declare exactly three L5 activation targets"
    $l4 = $l4Targets[0]
    Assert-Equal ([string]$l4.artifact_role) "l4_schema" "Candidate L4 activation role drifted"
    Assert-Equal ([string]$l4.authority_layer) "L4" "Candidate L4 authority layer drifted"
    Assert-Equal ([string]$l4.implementation_status) "active_repair_verified" "Candidate L4 implementation status drifted"
    Assert-Equal ([string]$l4.path) ([string]$Candidate.artifact_hashes.l4_schema.path) "Candidate L4 activation path is not identity-bound"
    Assert-Equal ([string]$l4.sha256) ([string]$Candidate.artifact_hashes.l4_schema.sha256) "Candidate L4 activation hash is not identity-bound"
    $typed = @($l5Targets | Where-Object { [string]$_.artifact_role -eq "typed_outcome" })
    $projection = @($l5Targets | Where-Object { [string]$_.artifact_role -eq "projection_baseline" })
    $lifecycle = @($l5Targets | Where-Object { [string]$_.artifact_role -eq "lifecycle_baseline" })
    Assert-Equal $typed.Count 1 "Candidate must declare exactly one typed outcome activation target"
    Assert-Equal $projection.Count 1 "Candidate must declare exactly one retained projection target"
    Assert-Equal $lifecycle.Count 1 "Candidate must declare exactly one retained lifecycle target"
    Assert-Equal ([string]$typed[0].path) ([string]$Candidate.artifact_hashes.typed_outcome.path) "Candidate typed outcome activation path is not identity-bound"
    Assert-Equal ([string]$typed[0].sha256) ([string]$Candidate.artifact_hashes.typed_outcome.sha256) "Candidate typed outcome activation hash is not identity-bound"
    Assert-Equal ([string]$typed[0].authority_layer) "L5-result" "Candidate typed outcome authority layer drifted"
    Assert-Equal ([string]$typed[0].implementation_status) "active_repair_verified" "Candidate typed outcome implementation status drifted"
    $baselineProjection = @($ActiveAuthority.selected_targets | Where-Object { [string]$_.layer -eq "L5-projection" })
    $baselineLifecycle = @($ActiveAuthority.selected_targets | Where-Object { [string]$_.layer -eq "L5-lifecycle" })
    Assert-Equal $baselineProjection.Count 1 "Active authority must contain exactly one projection baseline"
    Assert-Equal $baselineLifecycle.Count 1 "Active authority must contain exactly one lifecycle baseline"
    Assert-Equal ([string]$projection[0].path) ([string]$baselineProjection[0].artifact) "Candidate retained projection path drifted from active authority"
    Assert-Equal ([string]$projection[0].sha256) ([string]$baselineProjection[0].sha256) "Candidate retained projection hash drifted from active authority"
    Assert-Equal ([string]$projection[0].activation_phase) ([string]$baselineProjection[0].activation_phase) "Candidate retained projection phase drifted from active authority"
    Assert-Equal ([string]$projection[0].authority_layer) "L5-projection" "Candidate retained projection authority layer drifted"
    Assert-Equal ([string]$projection[0].implementation_status) ([string]$baselineProjection[0].implementation_status) "Candidate retained projection status drifted from active authority"
    Assert-Equal ([string]$lifecycle[0].path) ([string]$baselineLifecycle[0].artifact) "Candidate retained lifecycle path drifted from active authority"
    Assert-Equal ([string]$lifecycle[0].sha256) ([string]$baselineLifecycle[0].sha256) "Candidate retained lifecycle hash drifted from active authority"
    Assert-Equal ([string]$lifecycle[0].activation_phase) ([string]$baselineLifecycle[0].activation_phase) "Candidate retained lifecycle phase drifted from active authority"
    Assert-Equal ([string]$lifecycle[0].authority_layer) "L5-lifecycle" "Candidate retained lifecycle authority layer drifted"
    Assert-Equal ([string]$lifecycle[0].implementation_status) ([string]$baselineLifecycle[0].implementation_status) "Candidate retained lifecycle status drifted from active authority"
}

function Assert-CandidateArtifactSchemaContract {
    param([string]$SchemaPath)
    $sha = "1" * 64
    $metrics = [ordered]@{
        transition_carrier_rate = @{formula = "committed_transition_with_reserved_tool / accepted_nonterminal_transition"; unit = "ratio"}
        carrier_execution_started_rate = @{formula = "handler_handoff_started / committed_transition_with_reserved_tool"; unit = "ratio"}
        correctness_rate = @{formula = "correct_runs / attempted_runs"; unit = "ratio"}
        standalone_nonterminal_count = @{formula = "count(accepted_nonterminal_transition_without_reserved_tool)"; unit = "count"}
        h003_count = @{formula = "count(TASKSPACE_REQUIRED_SIBLING_MISSING)"; unit = "count"}
        patch_input_exact_rate = @{formula = "byte_exact_patch_inputs / patch_inputs"; unit = "ratio"}
        typed_output_exact_rate = @{formula = "schema_and_payload_exact_outputs / carrier_outputs"; unit = "ratio"}
        request_count = @{formula = "count(provider_requests)"; unit = "count"}
        input_tokens = @{formula = "sum(provider_input_tokens)"; unit = "tokens"}
        output_tokens = @{formula = "sum(provider_output_tokens)"; unit = "tokens"}
        cache_hit_rate = @{formula = "cached_input_tokens / input_tokens"; unit = "ratio"}
        wall_time_ms = @{formula = "run_finished_at_ms - run_started_at_ms"; unit = "milliseconds"}
        provider_time_ms = @{formula = "sum(provider_request_duration_ms)"; unit = "milliseconds"}
        tool_time_ms = @{formula = "sum(tool_execution_duration_ms)"; unit = "milliseconds"}
    }
    $thresholds = [ordered]@{transition_carrier_rate_min = 1; correctness_rate_min = 1; standalone_nonterminal_count_max = 0; h003_count_max = 0; patch_input_exact_rate_min = 1; typed_output_exact_rate_min = 1; request_amplification_max = 1; input_token_amplification_max = 1; output_token_amplification_max = 1; wall_time_amplification_max = 1; cache_hit_rate_delta_min = 0}
    $oracles = @(
        @{id = "empty_map_initialize"; pre_state = "empty_map"; transition = "initialize_map"; commit = "committed"; tool_state = "reserved"; lease_effect = "root_and_next_bound"},
        @{id = "ready_bind"; pre_state = "ready_node"; transition = "bind_node"; commit = "committed"; tool_state = "reserved"; lease_effect = "ready_node_bound"},
        @{id = "complete_continue"; pre_state = "active_node"; transition = "complete_then_continue"; commit = "committed"; tool_state = "reserved"; lease_effect = "current_completed_next_bound"},
        @{id = "commit_cancel_boundary"; pre_state = "prepared_call"; transition = "complete_then_continue"; commit = "committed"; tool_state = "not_started"; lease_effect = "reservation_cancelled_with_fact"},
        @{id = "resume_lease"; pre_state = "resumed_session"; transition = "bind_node"; commit = "committed"; tool_state = "reserved"; lease_effect = "persisted_generation_reused"},
        @{id = "compaction_projection"; pre_state = "compacted_context"; transition = "complete_then_continue"; commit = "committed"; tool_state = "reserved"; lease_effect = "canonical_map_revision_preserved"}
    )
    $capabilityEntries = @()
    foreach ($api in @("deepseek_chat", "responses")) {
        $capabilityEntries += @(
            @{wire_api = $api; tool_spec = "Function"; tool_payload = "Function"; registration_source = "builtin"; invocation_origin = "direct"; route = "function"; disposition = "carrier"; reason_code = "shared_function_handler"},
            @{wire_api = $api; tool_spec = "Namespace"; tool_payload = "NotApplicable"; registration_source = "dynamic"; invocation_origin = "direct"; route = "namespace"; disposition = "container"; reason_code = "namespace_container"},
            @{wire_api = $api; tool_spec = "ToolSearch"; tool_payload = "ToolSearch"; registration_source = "builtin"; invocation_origin = "direct"; route = "tool_search"; disposition = "non_carrier"; reason_code = "provider_native"},
            @{wire_api = $api; tool_spec = "LocalShell"; tool_payload = "LocalShell"; registration_source = "builtin"; invocation_origin = "direct"; route = "local_shell"; disposition = "non_carrier"; reason_code = "provider_native"},
            @{wire_api = $api; tool_spec = "ImageGeneration"; tool_payload = "NotApplicable"; registration_source = "builtin"; invocation_origin = "direct"; route = "image_generation"; disposition = "non_carrier"; reason_code = "provider_native"},
            @{wire_api = $api; tool_spec = "WebSearch"; tool_payload = "NotApplicable"; registration_source = "builtin"; invocation_origin = "direct"; route = "web_search"; disposition = "non_carrier"; reason_code = "provider_native"},
            @{wire_api = $api; tool_spec = "Freeform"; tool_payload = "Custom"; registration_source = "builtin"; invocation_origin = "direct"; route = "apply_patch"; disposition = "projected_carrier"; reason_code = "taskspace_function_projection"},
            @{wire_api = $api; tool_spec = "Function"; tool_payload = "Mcp"; registration_source = "mcp"; invocation_origin = "nested"; route = "mcp"; disposition = "carrier"; reason_code = "decorated_mcp_function"}
        )
    }
    $sourceInventory = @{tool_spec_source = "third_party/codex-cli/codex-rs/tools/src/tool_spec.rs"; tool_payload_source = "third_party/codex-cli/codex-rs/core/src/tools/context.rs"; router_source = "third_party/codex-cli/codex-rs/core/src/tools/router.rs"; registry_source = "third_party/codex-cli/codex-rs/core/src/tools/registry.rs"; code_mode_source = "third_party/codex-cli/codex-rs/tools/src/code_mode.rs"; tool_spec_variants = @("Function", "Namespace", "ToolSearch", "LocalShell", "ImageGeneration", "WebSearch", "Freeform"); tool_payload_variants = @("Function", "ToolSearch", "Custom", "LocalShell", "Mcp")}
    $sourceHashes = @{tool_spec = $sha; tool_payload = $sha; router = $sha; registry = $sha; code_mode = $sha}
    $samples = @{simple = @{category = "simple"; fixture_sha256 = $sha; repeats = 3}; complex = @{category = "complex"; fixture_sha256 = $sha; repeats = 3}}
    $toolIdentity = { param($name, $wire) @{name = $name; wire_api = "responses"; wire_kind = $wire; carrier_field = "taskspace_transition"; business_schema_sha256 = $sha; standard_wire_sha256 = $sha; parser_identity = "$name-parser"; handler_identity = "$name-handler"} }
    $outcomeVariants = @{
        RejectedBeforeCommit = @{commit_state = "not_committed"; tool_state = "not_dispatched"; required_facts = @("pre_hook_fact", "failure")}
        CommittedNotExecuted = @{commit_state = "committed"; tool_state = "not_started"; required_facts = @("transition_fact", "pre_hook_fact", "cancellation_or_start_failure")}
        Executed = @{commit_state = "committed"; tool_state = "started"; required_facts = @("transition_fact", "pre_hook_fact", "execution", "post_hook_fact", "retention_fact", "deferred_authorization_fact", "delivery_fact")}
    }
    $statistics = @{aggregates = @("total", "mean", "median"); adaptive_stop = $false}
    $validBodies = @(
        @{schema_version = 1; artifact_role = "l4_schema"; provider_tools = @((& $toolIdentity "exec_command" "function"), (& $toolIdentity "apply_patch" "taskspace_function_projection"), (& $toolIdentity "exec" "taskspace_function_projection")); reserved_collision_policy = "reject_epoch"},
        @{schema_version = 1; artifact_role = "transition_schema"; transition_schema = @{schema_id = "r7-transition-v1"; action_contracts = @{initialize_map = @{required_fields = @("action", "expected_revision", "root_goal", "next_node_id"); state_effect = "create_root_and_bind_next"}; bind_node = @{required_fields = @("action", "expected_revision", "next_node_id"); state_effect = "bind_ready_node"}; complete_then_continue = @{required_fields = @("action", "expected_revision", "current_node_id", "next_node_id", "final_summary"); state_effect = "complete_current_and_bind_next"}}; standalone_nonterminal_allowed = $false}},
        @{schema_version = 1; artifact_role = "typed_outcome"; outcome_type = "TaskSpaceCarrierOutcome"; outcome_variants = $outcomeVariants; deferred_authorization_contract = @{NotRequired = @{required_fields = @("decision")}; Requested = @{required_fields = @("kind", "discovered_scope", "denial_hash", "decision", "grant_id")}; Approved = @{required_fields = @("kind", "discovered_scope", "denial_hash", "decision", "grant_id")}; Denied = @{required_fields = @("kind", "discovered_scope", "denial_hash", "decision", "grant_id", "factual_error")}}; tool_output_preservation = "opaque"; post_hook_separate = $true},
        @{schema_version = 1; artifact_role = "lifecycle_oracle_v2"; oracle_version = 2; oracles = $oracles},
        @{schema_version = 1; artifact_role = "entry_closure"; generator_version = 1; generated = $true; source_inventory = $sourceInventory; source_hashes = $sourceHashes; generation_digest = $sha; entries = $capabilityEntries},
        @{schema_version = 1; artifact_role = "capability_matrix"; matrix_id = "r7-carrier-matrix-v1"; entry_closure = @{path = "benchmarks/taskspace/r7/candidates/$sha/entry-closure.json"; sha256 = $sha}; entries = $capabilityEntries},
        @{schema_version = 1; artifact_role = "rollback_manifest"; baseline_authority_sha256 = $sha; baseline_production_sha256 = $sha; restore_targets = @(@{target_role = "authority"; path = "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json"; sha256 = $sha}, @{target_role = "production_manifest"; path = "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json"; sha256 = $sha}, @{target_role = "runtime_manifest"; path = "runtime-manifest.json"; sha256 = $sha}, @{target_role = "schema_parser"; path = "schema-parser.json"; sha256 = $sha}); verification_commands = @(@{id = "contract_tests"; command = "pwsh contract-tests"}, @{id = "rollback_drill"; command = "pwsh rollback-drill"})},
        @{schema_version = 1; artifact_role = "continuous_action_evaluation"; evaluation_id = "ca-eval-v1"; sealed = $true; seed = 7; arm_order = @("standard", "sibling_baseline", "fla3_5_candidate"); samples = $samples; sample_order = @("simple", "complex"); metrics = $metrics; thresholds = $thresholds; statistics = $statistics},
        @{schema_version = 1; artifact_role = "fla8_evaluation_v2"; evaluation_id = "fla8-v2"; sealed = $true; seed = 7; repeats = 3; arm_order = @("standard", "map-always-frozen-baseline", "map-always-candidate", "map-append-frozen-baseline", "map-append-candidate", "map-request-frozen-baseline", "map-request-candidate"); held_out_identity = @{suite_id = "held-out"; manifest_sha256 = $sha; sample_count = 2; sample_ids = @("held-simple", "held-complex"); content_mounted = $false; mount_assertion = "held_out_content_path_absent_in_ca0_ca1_ca5"}; metrics = $metrics; thresholds = $thresholds; statistics = $statistics}
    )
    foreach ($body in $validBodies) {
        $json = $body | ConvertTo-Json -Depth 30
        Assert-True ($json | Test-Json -SchemaFile $SchemaPath -ErrorAction Stop) "Role-specific artifact schema rejected $($body.artifact_role)"
    }
    $emptyPayloads = @(
        @{schema_version = 1; artifact_role = "l4_schema"; provider_tool = @{}},
        @{schema_version = 1; artifact_role = "transition_schema"; transition_schema = @{}},
        @{schema_version = 1; artifact_role = "typed_outcome"; outcome_variants = @("Executed")},
        @{schema_version = 1; artifact_role = "lifecycle_oracle_v2"; oracles = @("carrier")},
        @{schema_version = 1; artifact_role = "entry_closure"; generated = $true; source_inventory = @{}; entries = @()},
        @{schema_version = 1; artifact_role = "capability_matrix"; entries = @("exec")},
        @{schema_version = 1; artifact_role = "rollback_manifest"; restore_targets = @("authority")},
        @{schema_version = 1; artifact_role = "continuous_action_evaluation"; samples = @("simple"); metrics = @{}; thresholds = @{}},
        @{schema_version = 1; artifact_role = "fla8_evaluation_v2"; held_out_identity = @{}; metrics = @{}; thresholds = @{}}
    )
    foreach ($body in $emptyPayloads) {
        $json = $body | ConvertTo-Json -Depth 20
        Assert-True (-not ($json | Test-Json -SchemaFile $SchemaPath -ErrorAction SilentlyContinue)) "Role-specific artifact schema accepted an empty $($body.artifact_role) payload"
    }
    $hollowBodies = @()
    foreach ($index in 0..($validBodies.Count - 1)) {
        $hollowBodies += ($validBodies[$index] | ConvertTo-Json -Depth 40 | ConvertFrom-Json -Depth 40)
    }
    $hollowBodies[0].provider_tools[0].name = "placeholder"
    $hollowBodies[1].transition_schema.action_contracts.complete_then_continue.state_effect = "placeholder"
    $hollowBodies[2].outcome_variants.Executed.required_facts = @("transition_fact")
    $hollowBodies[3].oracles[0].pre_state = "arbitrary_state"
    foreach ($entry in @($hollowBodies[4].entries)) { $entry.disposition = "non_carrier" }
    foreach ($entry in @($hollowBodies[5].entries)) { $entry.disposition = "non_carrier" }
    foreach ($target in @($hollowBodies[6].restore_targets)) { $target.target_role = "authority" }
    $hollowBodies[7].metrics.transition_carrier_rate.formula = "meaningless / formula"
    $hollowBodies[8].arm_order[1] = $hollowBodies[8].arm_order[0]
    foreach ($body in $hollowBodies) {
        $json = $body | ConvertTo-Json -Depth 40
        Assert-True (-not ($json | Test-Json -SchemaFile $SchemaPath -ErrorAction SilentlyContinue)) "Role-specific artifact schema accepted a well-formed hollow $($body.artifact_role) payload"
    }
    $schema = Get-Content -Raw -Encoding UTF8 -LiteralPath $SchemaPath | ConvertFrom-Json -Depth 100
    $toolSpecInventory = Get-RustEnumVariants (Join-Path $repoRoot "third_party/codex-cli/codex-rs/tools/src/tool_spec.rs") "ToolSpec"
    $toolPayloadInventory = Get-RustEnumVariants (Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/tools/context.rs") "ToolPayload"
    Assert-Equal ($toolSpecInventory -join ",") (@($schema.'$defs'.entryClosure.properties.source_inventory.properties.tool_spec_variants.const) -join ",") "Entry closure ToolSpec inventory drifted from Rust source"
    Assert-Equal ($toolPayloadInventory -join ",") (@($schema.'$defs'.entryClosure.properties.source_inventory.properties.tool_payload_variants.const) -join ",") "Entry closure ToolPayload inventory drifted from Rust source"
}

function Assert-CandidateActivationContract {
    param([object]$Candidate)
    $activeAuthorityRaw = Get-GitBlobText ([string]$Candidate.active_authority.git_commit) ([string]$Candidate.active_authority.path)
    $activeAuthority = $activeAuthorityRaw | ConvertFrom-Json -Depth 50
    $activeProductionRaw = Get-GitBlobText ([string]$Candidate.active_production_manifest.git_commit) ([string]$Candidate.active_production_manifest.path)
    $activeProduction = $activeProductionRaw | ConvertFrom-Json -Depth 50
    Assert-CandidateActivationSnapshot $Candidate "evaluation_candidate" $activeAuthorityRaw $activeAuthority $activeProduction $activeProductionRaw ([string]$Candidate.active_authority.sha256) ([string]$Candidate.active_production_manifest.sha256)

    $driftedProduction = $activeProductionRaw | ConvertFrom-Json -Depth 50
    $driftedProduction.layers[3].selected_targets += [pscustomobject]@{artifact = "old-target"; sha256 = ("0" * 64); activation_phase = "old"}
    $driftedProductionRaw = $driftedProduction | ConvertTo-Json -Depth 50
    Assert-Throws {
        Assert-CandidateActivationSnapshot $Candidate "reverted" $activeAuthorityRaw $activeAuthority $driftedProduction $driftedProductionRaw ([string]$Candidate.active_authority.sha256) (Get-TextSha256 $driftedProductionRaw)
    } "Reverted candidate accepted a production manifest that was not byte-exact baseline"

    $promotedAuthority = Get-ExpectedPromotedAuthority $Candidate
    $promotedAuthorityRaw = $promotedAuthority | ConvertTo-Json -Depth 50
    $promotedProduction = Get-ExpectedPromotedProduction $Candidate (Get-TextSha256 $promotedAuthorityRaw)
    $promotedProductionRaw = $promotedProduction | ConvertTo-Json -Depth 50
    Assert-True ($promotedAuthorityRaw | Test-Json -SchemaFile $authoritySchemaPath -ErrorAction Stop) "Canonical promoted authority does not match authority schema"
    Assert-True ($promotedProductionRaw | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction Stop) "Canonical promoted production manifest does not match schema"
    Assert-CandidateActivationSnapshot $Candidate "promoted" $promotedAuthorityRaw $promotedAuthority $promotedProduction
    $roleSwapAuthority = $promotedAuthorityRaw | ConvertFrom-Json -Depth 50
    $resultTarget = @($roleSwapAuthority.selected_targets | Where-Object { [string]$_.layer -eq "L5-result" })[0]
    $projectionTarget = @($roleSwapAuthority.selected_targets | Where-Object { [string]$_.layer -eq "L5-projection" })[0]
    $resultTarget.layer = "L5-projection"
    $projectionTarget.layer = "L5-result"
    $roleSwapAuthorityRaw = $roleSwapAuthority | ConvertTo-Json -Depth 50
    $roleSwapProduction = $promotedProduction | ConvertTo-Json -Depth 50 | ConvertFrom-Json -Depth 50
    $roleSwapProduction.source_authority.sha256 = Get-TextSha256 $roleSwapAuthorityRaw
    Assert-Throws {
        Assert-CandidateActivationSnapshot $Candidate "promoted" $roleSwapAuthorityRaw $roleSwapAuthority $roleSwapProduction
    } "Promoted candidate accepted an authority role swap"
    $metadataAuthority = $promotedAuthorityRaw | ConvertFrom-Json -Depth 50
    $metadataL4 = @($metadataAuthority.selected_targets | Where-Object { [string]$_.layer -eq "L4" })[0]
    $metadataL4 | Add-Member -NotePropertyName required_next_call -NotePropertyValue "retained"
    $metadataAuthorityRaw = $metadataAuthority | ConvertTo-Json -Depth 50
    $metadataProduction = $promotedProduction | ConvertTo-Json -Depth 50 | ConvertFrom-Json -Depth 50
    $metadataProduction.source_authority.sha256 = Get-TextSha256 $metadataAuthorityRaw
    Assert-Throws {
        Assert-CandidateActivationSnapshot $Candidate "promoted" $metadataAuthorityRaw $metadataAuthority $metadataProduction
    } "Promoted candidate accepted undeclared sibling metadata"
    $unrelatedAuthority = $promotedAuthorityRaw | ConvertFrom-Json -Depth 50
    $unrelatedL1 = @($unrelatedAuthority.selected_targets | Where-Object { [string]$_.layer -eq "L1" })[0]
    $unrelatedL1.implementation_status = "selected_not_implemented"
    $unrelatedL1 | Add-Member -NotePropertyName sibling_metadata -NotePropertyValue "forbidden"
    $unrelatedRepair = @($unrelatedAuthority.blocking_repairs | Where-Object { [string]$_.id -eq [string]$Candidate.activation_targets.blocking_repair.id })[0]
    $unrelatedRepair | Add-Member -NotePropertyName required_next_call -NotePropertyValue "retained"
    $unrelatedAuthorityRaw = $unrelatedAuthority | ConvertTo-Json -Depth 50
    $unrelatedProduction = $promotedProduction | ConvertTo-Json -Depth 50 | ConvertFrom-Json -Depth 50
    $unrelatedProduction.source_authority.sha256 = Get-TextSha256 $unrelatedAuthorityRaw
    Assert-Throws {
        Assert-CandidateActivationSnapshot $Candidate "promoted" $unrelatedAuthorityRaw $unrelatedAuthority $unrelatedProduction
    } "Promoted candidate accepted unrelated L1 or blocking-repair drift"
    $promotedProduction.layers[3].selected_targets += [pscustomobject]@{artifact = "old-target"; sha256 = ("0" * 64); activation_phase = "old"}
    Assert-Throws {
        Assert-CandidateActivationSnapshot $Candidate "promoted" $promotedAuthorityRaw $promotedAuthority $promotedProduction
    } "Promoted candidate accepted an undeclared production target"
}

function Assert-CandidateHistoryMetaContract {
    param([object]$Candidate, [object]$ProductionManifest)
    $parentCommit = (& git -C $repoRoot rev-parse HEAD^1).Trim()
    Assert-Throws {
        Assert-CandidateManifestIntegrity $Candidate "" $parentCommit
    } "Candidate state event accepted a non-ancestor candidate commit"

    $supersedingId = "a" * 64
    $terminal = $Candidate | ConvertTo-Json -Depth 50 | ConvertFrom-Json -Depth 50
    $terminal.candidate_status = "rejected"
    $terminal | Add-Member -NotePropertyName superseded_by -NotePropertyValue $supersedingId
    $successor = $Candidate | ConvertTo-Json -Depth 50 | ConvertFrom-Json -Depth 50
    $successor.candidate_id = $supersedingId
    $successor.contract_id = "r7-taskspace-five-layer-candidate-$supersedingId"
    $successor.candidate_status = "promotion_pending"
    Assert-CandidateSetIntegrity @($terminal, $successor) $ProductionManifest
    $terminal.superseded_by = "b" * 64
    Assert-Throws {
        Assert-CandidateSetIntegrity @($terminal, $successor) $ProductionManifest
    } "Terminal candidate accepted an unretained superseding candidate"
    $nonTerminal = $Candidate | ConvertTo-Json -Depth 50 | ConvertFrom-Json -Depth 50
    $nonTerminal | Add-Member -NotePropertyName superseded_by -NotePropertyValue $supersedingId
    $nonTerminalJson = $nonTerminal | ConvertTo-Json -Depth 50
    Assert-True (-not ($nonTerminalJson | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction SilentlyContinue)) "Nonterminal candidate schema accepted superseded_by"
}

function Assert-CandidateManifestIntegrity {
    param([object]$Candidate, [string]$ManifestPath = "", [string]$EventCommit = "", [bool]$UseWorktree = $false)
    $candidateId = [string]$Candidate.candidate_id
    Assert-Equal ([string]$Candidate.contract_id) "r7-taskspace-five-layer-candidate-$candidateId" "Candidate contract id does not match candidate id"
    Assert-Equal (Get-CandidateContentId $Candidate) $candidateId "Candidate content id does not match active snapshot and artifact hashes"
    $candidateCommit = [string]$Candidate.candidate_commit
    & git -C $repoRoot cat-file -e "$candidateCommit^{commit}" 2>$null
    Assert-True ($LASTEXITCODE -eq 0) "Candidate commit is unavailable: $candidateCommit"
    if (-not [string]::IsNullOrWhiteSpace($EventCommit)) {
        $firstParentHistory = @(& git -C $repoRoot rev-list --first-parent $EventCommit)
        Assert-True ($firstParentHistory -contains $candidateCommit) "Candidate commit is not a first-parent ancestor of candidate state event: $candidateCommit -> $EventCommit"
    }
    Assert-Equal ([string]$Candidate.active_authority.contract_id) "r7-five-layer-contract-authority-v1" "Candidate active authority id drifted"
    Assert-Equal ([string]$Candidate.active_authority.path) "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json" "Candidate active authority path drifted"
    $authorityBlob = Get-GitBlobText ([string]$Candidate.active_authority.git_commit) ([string]$Candidate.active_authority.path)
    Assert-Equal (Get-GitBlobSha256 ([string]$Candidate.active_authority.git_commit) ([string]$Candidate.active_authority.path)) ([string]$Candidate.active_authority.sha256) "Candidate active authority snapshot hash drifted"
    $activeAuthorityBody = $authorityBlob | ConvertFrom-Json -Depth 50
    Assert-Equal ([string]$Candidate.source_authority.contract_id) ([string]$Candidate.active_authority.contract_id) "Candidate source and active authority ids differ"
    Assert-Equal ([string]$Candidate.source_authority.path) ([string]$Candidate.active_authority.path) "Candidate source and active authority paths differ"
    Assert-Equal ([string]$Candidate.source_authority.sha256) ([string]$Candidate.active_authority.sha256) "Candidate source and active authority hashes differ"
    $productionSnapshot = $Candidate.active_production_manifest
    $productionBlob = Get-GitBlobText ([string]$productionSnapshot.git_commit) ([string]$productionSnapshot.path)
    Assert-Equal (Get-GitBlobSha256 ([string]$productionSnapshot.git_commit) ([string]$productionSnapshot.path)) ([string]$productionSnapshot.sha256) "Candidate active production snapshot hash drifted"
    $productionSnapshotBody = $productionBlob | ConvertFrom-Json -Depth 50
    Assert-Equal ([string]$productionSnapshotBody.contract_id) ([string]$productionSnapshot.contract_id) "Candidate active production contract id drifted"
    Assert-Equal ([string]$productionSnapshotBody.source_authority.sha256) ([string]$Candidate.active_authority.sha256) "Candidate production snapshot does not use the active authority snapshot"
    Assert-CandidateActivationTargets $Candidate $activeAuthorityBody
    $namespaceRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "benchmarks/taskspace/r7/candidates/$candidateId"))
    $namespacePrefix = $namespaceRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    $seenPaths = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    $seenHashes = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    $artifactBodies = @{}
    foreach ($artifact in $Candidate.artifact_hashes.psobject.Properties) {
        Assert-Equal ([string]$artifact.Value.artifact_role) ([string]$artifact.Name) "Candidate artifact role marker drifted"
        $relativePath = [string]$artifact.Value.path
        $expectedPrefix = "benchmarks/taskspace/r7/candidates/$candidateId/"
        Assert-True $relativePath.StartsWith($expectedPrefix, [System.StringComparison]::Ordinal) "Candidate artifact escaped its namespace: $relativePath"
        $canonicalPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $relativePath))
        Assert-True $canonicalPath.StartsWith($namespacePrefix, [System.StringComparison]::Ordinal) "Candidate artifact escaped its canonical namespace: $relativePath"
        Assert-True ($seenPaths.Add($canonicalPath)) "Candidate artifact paths must be unique: $relativePath"
        Assert-True ($seenHashes.Add([string]$artifact.Value.sha256)) "Candidate artifact roles must not reuse one blob hash"
        if (-not [string]::IsNullOrWhiteSpace($ManifestPath)) {
            Assert-Equal (Get-GitBlobSha256 $candidateCommit $relativePath) ([string]$artifact.Value.sha256) "Candidate artifact was not frozen by candidate commit: $relativePath"
            if ($UseWorktree) {
                $pathCursor = [System.IO.Path]::GetFullPath($repoRoot)
                foreach ($segment in $relativePath.Split([char[]]@('/', '\'), [System.StringSplitOptions]::RemoveEmptyEntries)) {
                    $pathCursor = Join-Path $pathCursor $segment
                    if (Test-Path -LiteralPath $pathCursor) {
                        $pathItem = Get-Item -LiteralPath $pathCursor -Force
                        Assert-True (($pathItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -eq 0) "Candidate artifact path contains a symlink: $relativePath"
                    }
                }
                Assert-True (Test-Path -LiteralPath $canonicalPath -PathType Leaf) "Candidate artifact missing: $relativePath"
                $resolvedArtifactPath = (Resolve-Path -LiteralPath $canonicalPath).Path
                Assert-True $resolvedArtifactPath.StartsWith($namespacePrefix, [System.StringComparison]::Ordinal) "Candidate artifact escaped its resolved namespace: $relativePath"
                Assert-Equal (Get-Sha256 $canonicalPath) ([string]$artifact.Value.sha256) "Candidate artifact hash drifted: $relativePath"
                $artifactRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $canonicalPath
                $treeCommit = $candidateCommit
            } else {
                Assert-True (-not [string]::IsNullOrWhiteSpace($EventCommit)) "Historical artifact validation requires an event commit"
                $artifactRaw = Get-GitBlobText $EventCommit $relativePath
                Assert-Equal (Get-GitBlobSha256 $EventCommit $relativePath) ([string]$artifact.Value.sha256) "Historical candidate artifact hash drifted: $relativePath at $EventCommit"
                $treeCommit = $EventCommit
            }
            $treeEntry = (& git -C $repoRoot ls-tree $treeCommit -- $relativePath).Trim()
            Assert-True $treeEntry.StartsWith("100644 blob ", [System.StringComparison]::Ordinal) "Candidate artifact must be a regular non-executable Git blob: $relativePath"
            Assert-StrictJson $artifactRaw "candidate artifact $relativePath"
            $artifactBody = $artifactRaw | ConvertFrom-Json -Depth 100
            Assert-Equal ([string]$artifactBody.artifact_role) ([string]$artifact.Name) "Candidate artifact content role drifted: $relativePath"
            Assert-True (-not [string]::IsNullOrWhiteSpace([string]$artifactBody.schema_version)) "Candidate artifact schema_version missing: $relativePath"
            $artifactSchemaPath = Join-Path $repoRoot ([string]$authority.candidate_registry.artifact_schema)
            Assert-True ($artifactRaw | Test-Json -SchemaFile $artifactSchemaPath -ErrorAction Stop) "Candidate artifact does not match its role-specific schema: $relativePath"
            $artifactBodies[[string]$artifact.Name] = $artifactBody
        }
    }
    if (-not [string]::IsNullOrWhiteSpace($ManifestPath)) {
        Assert-CandidateArtifactSemantics $Candidate $artifactBodies $candidateCommit
        $expectedPath = Join-Path $repoRoot "benchmarks/taskspace/r7/candidates/$candidateId/manifest.json"
        Assert-Equal ([System.IO.Path]::GetFullPath($ManifestPath)) ([System.IO.Path]::GetFullPath($expectedPath)) "Candidate manifest path does not match candidate id"
    }
}

function Assert-CandidateSupersession {
    param([object]$Candidate, [string]$EventCommit, [bool]$AllowWorktree = $false, [bool]$RequirePending = $false)
    Assert-True (@("rejected", "reverted") -contains [string]$Candidate.candidate_status) "Only a terminal candidate may be superseded"
    $supersedingId = [string]$Candidate.superseded_by
    Assert-True ($supersedingId -cne [string]$Candidate.candidate_id) "Candidate cannot supersede itself"
    $supersedingPath = "benchmarks/taskspace/r7/candidates/$supersedingId/manifest.json"
    & git -C $repoRoot cat-file -e "${EventCommit}:$supersedingPath" 2>$null
    if ($LASTEXITCODE -eq 0) {
        $supersedingRaw = Get-GitBlobText $EventCommit $supersedingPath
    } elseif ($AllowWorktree -and (Test-Path -LiteralPath (Join-Path $repoRoot $supersedingPath) -PathType Leaf)) {
        $supersedingRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $repoRoot $supersedingPath)
    } else {
        throw "Superseding candidate did not exist in the same state event: $supersedingId"
    }
    $superseding = $supersedingRaw | ConvertFrom-Json -Depth 50
    Assert-Equal ([string]$superseding.candidate_id) $supersedingId "Superseding candidate identity drifted"
    Assert-True ([string]$superseding.candidate_status -ne "evaluation_candidate") "Evaluation-only candidate cannot supersede a terminal authority claim"
    if ($RequirePending) {
        Assert-Equal ([string]$superseding.candidate_status) "promotion_pending" "First supersession event must match successor promotion_pending"
        $pendingCommits = @(& git -C $repoRoot log $EventCommit --first-parent --reverse --format=%H -- $supersedingPath)
        $firstPendingCommit = ""
        foreach ($pendingCommit in $pendingCommits) {
            & git -C $repoRoot cat-file -e "${pendingCommit}:$supersedingPath" 2>$null
            if ($LASTEXITCODE -ne 0) { continue }
            $pendingRaw = Get-GitBlobText $pendingCommit $supersedingPath
            Assert-StrictJson $pendingRaw "superseding candidate at $pendingCommit"
            $pendingCandidate = $pendingRaw | ConvertFrom-Json -Depth 50
            if ([string]$pendingCandidate.candidate_status -eq "promotion_pending") {
                $firstPendingCommit = $pendingCommit
                break
            }
        }
        Assert-True (-not [string]::IsNullOrWhiteSpace($firstPendingCommit)) "Superseding candidate has no historical promotion_pending event"
        Assert-Equal $EventCommit $firstPendingCommit "First supersession assignment must share the successor's first promotion_pending commit"
    }
}

function Assert-CandidateHistoryIntegrity {
    param([string]$ManifestPath, [string]$CurrentRaw, [object]$Authority)
    $relativePath = [System.IO.Path]::GetRelativePath($repoRoot, $ManifestPath).Replace("\", "/")
    $candidateRootRelative = [System.IO.Path]::GetDirectoryName($relativePath).Replace("\", "/")
    $authorityRelativePath = "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json"
    $productionRelativePath = "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json"
    $historyCommits = @(& git -C $repoRoot log --first-parent --reverse --format=%H -- $candidateRootRelative $authorityRelativePath $productionRelativePath)
    $previousStatus = ""
    $previousSupersedingId = ""
    $lastRaw = $null
    $manifestSeen = $false
    foreach ($commit in $historyCommits) {
        & git -C $repoRoot cat-file -e "${commit}:$relativePath" 2>$null
        if ($LASTEXITCODE -ne 0) {
            Assert-True (-not $manifestSeen) "Candidate manifest history contains a deletion: $relativePath at $commit"
            continue
        }
        $manifestSeen = $true
        $manifestTreeEntry = (& git -C $repoRoot ls-tree $commit -- $relativePath).Trim()
        Assert-True $manifestTreeEntry.StartsWith("100644 blob ", [System.StringComparison]::Ordinal) "Historical candidate manifest must be a regular non-executable Git blob: $relativePath at $commit"
        $candidateRaw = Get-GitBlobText $commit $relativePath
        Assert-StrictJson $candidateRaw "historical candidate manifest $relativePath at $commit"
        Assert-True ($candidateRaw | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction Stop) "Historical candidate manifest does not match schema: $relativePath at $commit"
        $candidate = $candidateRaw | ConvertFrom-Json -Depth 50
        Assert-CandidateManifestIntegrity $candidate $ManifestPath $commit $false
        Assert-CandidateStateHistory $candidate $previousStatus $Authority
        $authorityRawAtCommit = Get-GitBlobText $commit $authorityRelativePath
        Assert-True ((& git -C $repoRoot ls-tree $commit -- $authorityRelativePath).Trim().StartsWith("100644 blob ", [System.StringComparison]::Ordinal)) "Historical authority must be a regular non-executable Git blob at $commit"
        Assert-StrictJson $authorityRawAtCommit "historical authority at $commit"
        Assert-True ($authorityRawAtCommit | Test-Json -SchemaFile $authoritySchemaPath -ErrorAction Stop) "Historical authority does not match schema at $commit"
        $authorityAtCommit = $authorityRawAtCommit | ConvertFrom-Json -Depth 50
        $productionRawAtCommit = Get-GitBlobText $commit $productionRelativePath
        Assert-True ((& git -C $repoRoot ls-tree $commit -- $productionRelativePath).Trim().StartsWith("100644 blob ", [System.StringComparison]::Ordinal)) "Historical production manifest must be a regular non-executable Git blob at $commit"
        Assert-StrictJson $productionRawAtCommit "historical production manifest at $commit"
        Assert-True ($productionRawAtCommit | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction Stop) "Historical production manifest does not match schema at $commit"
        $productionAtCommit = $productionRawAtCommit | ConvertFrom-Json -Depth 50
        if ([string]::IsNullOrWhiteSpace([string]$candidate.superseded_by)) {
            Assert-True ([string]::IsNullOrWhiteSpace($previousSupersedingId)) "Candidate superseded_by cannot be cleared after assignment"
            $authorityHashAtCommit = Get-GitBlobSha256 $commit $authorityRelativePath
            $productionHashAtCommit = Get-GitBlobSha256 $commit $productionRelativePath
            Assert-CandidateActivationSnapshot $candidate ([string]$candidate.candidate_status) $authorityRawAtCommit $authorityAtCommit $productionAtCommit $productionRawAtCommit $authorityHashAtCommit $productionHashAtCommit
        } else {
            if (-not [string]::IsNullOrWhiteSpace($previousSupersedingId)) {
                Assert-Equal ([string]$candidate.superseded_by) $previousSupersedingId "Candidate superseded_by changed after first assignment"
            }
            Assert-CandidateSupersession $candidate $commit $false ([string]::IsNullOrWhiteSpace($previousSupersedingId))
        }
        $previousStatus = [string]$candidate.candidate_status
        $previousSupersedingId = [string]$candidate.superseded_by
        $lastRaw = $candidateRaw
    }
    if ($null -eq $lastRaw -or (Get-TextSha256 $lastRaw) -cne (Get-TextSha256 $CurrentRaw)) {
        Assert-True ($CurrentRaw | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction Stop) "Worktree candidate manifest does not match schema: $relativePath"
        Assert-StrictJson $CurrentRaw "worktree candidate manifest $relativePath"
        $candidate = $CurrentRaw | ConvertFrom-Json -Depth 50
        Assert-CandidateManifestIntegrity $candidate $ManifestPath ((& git -C $repoRoot rev-parse HEAD).Trim()) $true
        Assert-CandidateStateHistory $candidate $previousStatus $Authority
        $currentAuthorityRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $authorityPath
        $currentProductionRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath
        $currentProduction = $currentProductionRaw | ConvertFrom-Json -Depth 50
        if ([string]::IsNullOrWhiteSpace([string]$candidate.superseded_by)) {
            Assert-True ([string]::IsNullOrWhiteSpace($previousSupersedingId)) "Worktree candidate cleared superseded_by after assignment"
            Assert-CandidateActivationSnapshot $candidate ([string]$candidate.candidate_status) $currentAuthorityRaw $Authority $currentProduction $currentProductionRaw (Get-Sha256 $authorityPath) (Get-Sha256 $manifestPath)
        } else {
            if (-not [string]::IsNullOrWhiteSpace($previousSupersedingId)) {
                Assert-Equal ([string]$candidate.superseded_by) $previousSupersedingId "Worktree candidate rewrote superseded_by"
            }
            Assert-CandidateSupersession $candidate ((& git -C $repoRoot rev-parse HEAD).Trim()) $true ([string]::IsNullOrWhiteSpace($previousSupersedingId))
        }
    }
    $currentCandidate = $CurrentRaw | ConvertFrom-Json -Depth 50
    $currentAuthorityRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $authorityPath
    $currentProductionRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath
    $currentProduction = $currentProductionRaw | ConvertFrom-Json -Depth 50
    if ([string]::IsNullOrWhiteSpace([string]$currentCandidate.superseded_by)) {
        Assert-CandidateActivationSnapshot $currentCandidate ([string]$currentCandidate.candidate_status) $currentAuthorityRaw $Authority $currentProduction $currentProductionRaw (Get-Sha256 $authorityPath) (Get-Sha256 $manifestPath)
    } else {
        if (-not [string]::IsNullOrWhiteSpace($previousSupersedingId)) {
            Assert-Equal ([string]$currentCandidate.superseded_by) $previousSupersedingId "Current candidate rewrote superseded_by"
        }
        Assert-CandidateSupersession $currentCandidate ((& git -C $repoRoot rev-parse HEAD).Trim()) $true
    }
}
