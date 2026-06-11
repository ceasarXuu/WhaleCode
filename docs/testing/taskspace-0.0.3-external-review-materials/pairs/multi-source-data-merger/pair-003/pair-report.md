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
- external_runtime_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-003\external-runtime-proof.json
- external_runner_equivalence_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-003\external-runner-equivalence-proof.json
- external_isolation_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-003\external-isolation-proof.json
- external_combined_proof_path: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-003\external-e3-proof.json
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
- jsonl: D:\whalecode-alpha\target\bench004-20260608-202551\runs\terminal_bench__multi-source-data-merger\20260608-220537-451\pair-003\oracle-isolation-probe\whale-exec.jsonl

## Scenario Warnings
- none

## Utility Assessment
- outcome: both_failed_or_inconclusive
- taskspace_tool_call_ratio: 3
- taskspace_wall_time_ratio: 5.06
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
- wall_time_ms: 66995
- tool_call_count: 7
- changed_paths: conflicts.json, merge_users.py, merged_users.parquet
- changed_file_inventory: conflicts.json[??] sha256=73af62cfdf03a85ffa9f07ea81616dd848b3a0092bdd2eb292137822b9ab0c5a size=732; merge_users.py[??] sha256=8f213ce14256bbc9abfe7fa0dd4130caa577ed08ebb388fbdd5145fb47f98f73 size=6843; merged_users.parquet[??] sha256=68f98cfd33555bb91713b3d6be6876da9d3de178fc3eb7673cc7dc63c76582f1 size=3461
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
- wall_time_ms: 339102
- tool_call_count: 21
- changed_paths: __pycache__/test_merge_pipeline.cpython-312-pytest-9.0.3.pyc.19492, __pycache__/test_merge_pipeline.cpython-312-pytest-9.0.3.pyc.22988, __pycache__/test_merge_pipeline.cpython-312-pytest-9.0.3.pyc.23756, conflicts.json, merge_users.py, merged_users.parquet, test_merge_pipeline.py
- changed_file_inventory: __pycache__/test_merge_pipeline.cpython-312-pytest-9.0.3.pyc.19492[??] sha256=acb06553bebd38d1ead1ca3d6ac77f86dca9d41dcbd523ba6c1bb9e422b11a81 size=23994; __pycache__/test_merge_pipeline.cpython-312-pytest-9.0.3.pyc.22988[??] sha256=acb06553bebd38d1ead1ca3d6ac77f86dca9d41dcbd523ba6c1bb9e422b11a81 size=23994; __pycache__/test_merge_pipeline.cpython-312-pytest-9.0.3.pyc.23756[??] sha256=acb06553bebd38d1ead1ca3d6ac77f86dca9d41dcbd523ba6c1bb9e422b11a81 size=23994; conflicts.json[??] sha256=89ae1fdb9418bfe94d307ad11bd508f6c23c7dddc92896380586363f3f2f994b size=732; merge_users.py[??] sha256=7d7dcc3ac20a8be2b2a810f23d9fd4e2904a29e9ff09a62773de637e72a9d586 size=4590; merged_users.parquet[??] sha256=b8b3fbb64201c9eb9973753ef91754934a3b57c5923cceffface0f94debec0cd size=3461; test_merge_pipeline.py[??] sha256=75e790157b291e7c49a007258e90e64787782c0f8712895d1a0a2e8247be5164 size=4492
- metrics_warnings: 
- metrics_taints: 
- validator_environment_failures: public_validation_timeout
- docker_build_result_path: 
- validator_environment_mismatch: False
- maps: 1
- nodes: 11
- edges: 10
- edge_order_violations: 0
- spawn_agent_calls: 0
- subagent_results: 0
- open_leaf_nodes: 0
- ordinary_before_binding: False
