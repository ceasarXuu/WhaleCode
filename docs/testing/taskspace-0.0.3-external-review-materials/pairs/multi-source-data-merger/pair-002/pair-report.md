# TaskSpace Benchmark Pair Report

- scenario: terminal_bench__multi-source-data-merger
- level: L3
- requested_evidence_target: E3
- reported_evidence_level: E1
- oracle_isolation_policy: deferred_materialization_allowed
- valid_pair: True
- included_in_utility_aggregate: False
- left_logical_mode: taskspace
- right_logical_mode: standard
- included_in_e3_aggregate: False

## Evidence Gate Failures
- business_success_false
- manual_review_required

## E3 Gate
- sample_origin_type: external_benchmark
- human_review_required: True
- human_review_completed: False
- human_review_decision: 
- human_review_disagreement: False
- e3_minimum_repeats: 5
- claim_scope: Terminal-Bench coding/file/debug/data-processing subset
- declared_validator_runtime: terminal_bench_equivalent_docker_app
- declared_official_runner_or_equivalent: True
- declared_agent_cannot_read_validator_source: True
- declared_validator_e3_eligible: True
- declared_validator_downgrade_reason: 
- public_validation_timeout
- e3_external_validator_fidelity_unproven
- e3_external_validator_not_e3_eligible
- e3_human_review_not_completed
- external_runtime_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-002\external-runtime-proof.json
- external_runner_equivalence_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-002\external-runner-equivalence-proof.json
- external_isolation_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-002\external-isolation-proof.json
- external_combined_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-002\external-e3-proof.json
- proof_official_runner_or_equivalent: False
- proof_agent_cannot_read_validator_source: True
- proof_validator_e3_eligible: False
- audit_review_source_path: 
- audit_review_failures: audit_review_missing

## Variable Control
- failures: none

## Prompt Guard
- invalid_prompt: False
- manual_review_required: True
- hard_hits: 
- context_hits: Map

## Oracle Isolation Probe
- oracle_isolation_level: hard_deferred_materialization
- canary_leaked: False
- canary_materialized_during_probe: False
- path_mentioned: True
- jsonl: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-002\oracle-isolation-probe\whale-exec.jsonl

## Scenario Warnings
- none

## Utility Assessment
- outcome: both_failed_or_inconclusive
- taskspace_tool_call_ratio: 1.81
- taskspace_wall_time_ratio: 3.96
- taskspace_tool_call_ratio_warn: False
- taskspace_wall_time_ratio_warn: False
- note: excluded evidence is diagnostic only; it does not prove paired comparability or TaskSpace utility.

## left / taskspace
- business_success: False
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 124
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 442135
- tool_call_count: 29
- changed_paths: __pycache__/test_merge.cpython-312-pytest-9.0.3.pyc.19292, conflicts.json, merge_users.py, merged_users.parquet, test_merge.py
- changed_file_inventory: __pycache__/test_merge.cpython-312-pytest-9.0.3.pyc.19292[??] sha256=2d247db7d73f26d2c0ab1412000843b1b13ef17fefb0441bd082feb1f1c6bff2 size=13212; conflicts.json[??] sha256=a69ced5f2f7bab1f78cfb22cfd324c867875484bbbab977f85541a2ca5ff15ac size=941; merge_users.py[??] sha256=3e935b7ea5617961bcec4a21a2dcd1a3d4f87eb1f300f110416ab3936c8b2cd8 size=4752; merged_users.parquet[??] sha256=61cd58423cc249a7f05f0f8f96ddeca5045960b4bc54658b274553a545e12946 size=3848; test_merge.py[??] sha256=33dbf54307206fc1b4d34526abffb18d466142a9a6a0ceb0ad797737476e0e41 size=1800
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: public_validation_timeout
- docker_build_result_path: 
- validator_environment_mismatch: False
- maps: 1
- nodes: 12
- edges: 11
- edge_order_violations: 0
- spawn_agent_calls: 0
- subagent_results: 0
- open_leaf_nodes: 0
- ordinary_before_binding: False

## right / standard
- business_success: False
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 124
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 111635
- tool_call_count: 16
- changed_paths: conflicts.json, merge_users.py, merged_users.parquet
- changed_file_inventory: conflicts.json[??] sha256=89ae1fdb9418bfe94d307ad11bd508f6c23c7dddc92896380586363f3f2f994b size=732; merge_users.py[??] sha256=0c01cb5547aac42de672e19e34c6a535f9bc236a9635e2741bf16745447cd46a size=7472; merged_users.parquet[??] sha256=3ceb43b02a6febf2178630e209cf98fff71cc33535cfe0ef53151b01df57ddc7 size=3419
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: public_validation_timeout
- docker_build_result_path: 
- validator_environment_mismatch: False
- maps: 0
- nodes: 0
- edges: 0
- edge_order_violations: 0
- spawn_agent_calls: 0
- subagent_results: 0
- open_leaf_nodes: 0
- ordinary_before_binding: False
