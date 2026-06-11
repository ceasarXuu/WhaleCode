# TaskSpace Benchmark Pair Report

- scenario: terminal_bench__multi-source-data-merger
- level: L3
- requested_evidence_target: E3
- reported_evidence_level: E1
- oracle_isolation_policy: deferred_materialization_allowed
- valid_pair: True
- included_in_utility_aggregate: False
- left_logical_mode: standard
- right_logical_mode: taskspace
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
- external_runtime_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-001\external-runtime-proof.json
- external_runner_equivalence_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-001\external-runner-equivalence-proof.json
- external_isolation_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-001\external-isolation-proof.json
- external_combined_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-001\external-e3-proof.json
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
- path_mentioned: False
- jsonl: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-001\oracle-isolation-probe\whale-exec.jsonl

## Scenario Warnings
- none

## Utility Assessment
- outcome: both_failed_or_inconclusive
- taskspace_tool_call_ratio: 1.5
- taskspace_wall_time_ratio: 6.92
- taskspace_tool_call_ratio_warn: False
- taskspace_wall_time_ratio_warn: False
- note: excluded evidence is diagnostic only; it does not prove paired comparability or TaskSpace utility.

## left / standard
- business_success: False
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 124
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 95380
- tool_call_count: 10
- changed_paths: conflicts.json, merge_users.py, merged_users.parquet, verify_output.py
- changed_file_inventory: conflicts.json[??] sha256=89ae1fdb9418bfe94d307ad11bd508f6c23c7dddc92896380586363f3f2f994b size=732; merge_users.py[??] sha256=8446448ac9636f45a8275d874a7fab4a3dec8772797684e453cbdbe67a79bc58 size=7181; merged_users.parquet[??] sha256=b8b3fbb64201c9eb9973753ef91754934a3b57c5923cceffface0f94debec0cd size=3461; verify_output.py[??] sha256=ef296994413e21a792adfdd308878220312c699863cc0aecf96d3689cfd0ac7d size=2414
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

## right / taskspace
- business_success: False
- exec_exit_code: 0
- exec_timed_out: False
- public_validation_exit_code: 124
- hidden_oracle_exit_code: 0
- oracle_isolation_level: hard_sandbox
- wall_time_ms: 660118
- tool_call_count: 15
- changed_paths: __pycache__/test_merge_output.cpython-312-pytest-9.0.3.pyc.14380, conflicts.json, merge_users.py, merged_users.parquet, test_merge_output.py
- changed_file_inventory: __pycache__/test_merge_output.cpython-312-pytest-9.0.3.pyc.14380[??] sha256=15bec7b49abe9c23f9a3f210ea5e0a1675b6874d845fb32e5729c2e29c84177a size=22202; conflicts.json[??] sha256=89ae1fdb9418bfe94d307ad11bd508f6c23c7dddc92896380586363f3f2f994b size=732; merge_users.py[??] sha256=4e9367a85a3c3caf03dcde1fa4b954aec9d88f7955acbbb96ad1288d9593556c size=7963; merged_users.parquet[??] sha256=65cd5f42065d50345a85d0ea1ac3b46e432292af6a2c172bb85860424b1bfe9b size=3461; test_merge_output.py[??] sha256=059a6710183b4e41783e75416ad74a761d171c16d6548583c6dad2f3d1ab9e6e size=3353
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: public_validation_timeout
- docker_build_result_path: 
- validator_environment_mismatch: False
- maps: 1
- nodes: 21
- edges: 29
- edge_order_violations: 0
- spawn_agent_calls: 5
- subagent_results: 35
- open_leaf_nodes: 0
- ordinary_before_binding: False
